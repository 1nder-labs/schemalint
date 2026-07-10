use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::cache::hash_bytes;
use crate::cli::pipeline::{build_report, check_rulesets, render_output};
use crate::cli::report::CoverageCounts;
use crate::normalize::normalize;

use super::policy::{load_profiles, output_format, rulesets};
use super::{ProfileCache, SchemaCache};

const MAX_CHECK_SECONDS: u64 = 30;
const MAX_SCHEMA_BYTES: usize = 5 * 1024 * 1024;
const MAX_SCHEMA_NODES: usize = 200_000;
const MAX_SCHEMA_DEPTH: usize = 1_000;

pub(super) fn handle(params: Value, cache: &SchemaCache, profiles: &ProfileCache) -> Value {
    let schema = match params.get("schema") {
        Some(schema) => schema.clone(),
        None => return json!({"success": false, "error": "Missing 'schema' parameter"}),
    };
    let profile_ids: Vec<String> = match params.get("profiles").and_then(Value::as_array) {
        Some(values) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        None => return json!({"success": false, "error": "Missing 'profiles' parameter"}),
    };
    let format = match output_format(&params) {
        Ok(format) => format,
        Err(error) => return error,
    };
    let loaded = match load_profiles(&profile_ids, profiles) {
        Ok(loaded) => loaded,
        Err(error) => return error,
    };
    let rules = match rulesets(&loaded) {
        Ok(rules) => rules,
        Err(error) => return error,
    };
    let names = loaded.iter().map(|profile| profile.name.clone()).collect();

    let schema_bytes = serde_json::to_vec(&schema).unwrap_or_default();
    if schema_bytes.len() > MAX_SCHEMA_BYTES {
        return json!({
            "success": false,
            "error": format!("Schema serialized size ({} bytes) exceeds the {} byte limit", schema_bytes.len(), MAX_SCHEMA_BYTES)
        });
    }
    let mut remaining = MAX_SCHEMA_NODES;
    if !within_complexity_bounds(&schema, &mut remaining, 0) {
        return json!({
            "success": false,
            "error": format!("Schema exceeds complexity limits (max depth {MAX_SCHEMA_DEPTH}, max nodes {MAX_SCHEMA_NODES}); rejected to prevent resource exhaustion")
        });
    }

    let start = Instant::now();
    let hash = hash_bytes(&schema_bytes);
    let cached = cache
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .get(hash, &schema_bytes)
        .cloned();
    let normalized = match cached {
        Some(schema) => schema,
        None => {
            let normalized = match normalize(schema) {
                Ok(normalized) => normalized,
                Err(error) => {
                    return json!({"success": false, "error": format!("Normalization failed: {error}")});
                }
            };
            cache
                .write()
                .unwrap_or_else(|error| error.into_inner())
                .insert(hash, schema_bytes, normalized.clone());
            normalized
        }
    };
    let diagnostics = check_rulesets(&normalized.arena, &rules);
    if start.elapsed() > Duration::from_secs(MAX_CHECK_SECONDS) {
        return json!({"success": false, "error": "Check execution exceeded 30 second limit"});
    }

    let report = build_report(
        CoverageCounts {
            attempted: 1,
            discovered: 1,
            ..CoverageCounts::default()
        },
        vec![],
        vec![],
        vec![(PathBuf::from("<inline>"), String::new(), Ok(diagnostics))],
        names,
        Some(start.elapsed().as_millis() as u64),
    );
    report.rpc_result(render_output(format, &report))
}

fn within_complexity_bounds(value: &Value, remaining: &mut usize, depth: usize) -> bool {
    if depth > MAX_SCHEMA_DEPTH || *remaining == 0 {
        return false;
    }
    *remaining -= 1;
    match value {
        Value::Array(values) => values
            .iter()
            .all(|value| within_complexity_bounds(value, remaining, depth + 1)),
        Value::Object(values) => values
            .values()
            .all(|value| within_complexity_bounds(value, remaining, depth + 1)),
        _ => true,
    }
}
