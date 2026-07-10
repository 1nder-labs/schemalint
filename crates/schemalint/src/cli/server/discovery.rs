use std::time::Instant;

use serde_json::{json, Value};

use crate::cli::args::OutputFormat;
use crate::cli::discovery_policy::{discover_batch, DiscoveryBatch};
use crate::cli::pipeline::{
    append_envelope_diagnostics, attach_source_spans, build_report, process_schemas, render_output,
    schema_entries,
};
use crate::profile::Profile;
use crate::rules::RuleSet;

use super::policy::{load_profiles, output_format, required_string_array, rulesets, string_array};
use super::ProfileCache;

struct PreparedRequest {
    targets: Vec<String>,
    exclusions: Vec<String>,
    continue_on_error: bool,
    format: OutputFormat,
    profiles: Vec<Profile>,
}

pub(super) fn handle_node(params: Value, cache: &ProfileCache) -> Value {
    let prepared = match prepare(params, "sources", "source globs", cache) {
        Ok(prepared) => prepared,
        Err(error) => return error,
    };
    let rules = match rulesets(&prepared.profiles) {
        Ok(rules) => rules,
        Err(error) => return error,
    };
    let start = Instant::now();
    let mut helper = match crate::node::NodeHelper::spawn(None) {
        Ok(helper) => helper,
        Err(error) => {
            return json!({"success": false, "error": format!("Failed to spawn Node helper: {error}")});
        }
    };
    let discovery = discover_batch(
        &prepared.targets,
        &prepared.exclusions,
        prepared.continue_on_error,
        "source",
        |source, exclusions| helper.discover_with_exclusions(source, exclusions),
    );
    helper.shutdown();
    finish(discovery, &prepared, &rules, start, true)
}

pub(super) fn handle_python(params: Value, cache: &ProfileCache) -> Value {
    let prepared = match prepare(params, "packages", "Python package names", cache) {
        Ok(prepared) => prepared,
        Err(error) => return error,
    };
    let rules = match rulesets(&prepared.profiles) {
        Ok(rules) => rules,
        Err(error) => return error,
    };
    let start = Instant::now();
    let mut helper = match crate::python::PythonHelper::spawn(None) {
        Ok(helper) => helper,
        Err(error) => {
            return json!({"success": false, "error": format!("Failed to spawn Python helper: {error}")});
        }
    };
    let discovery = discover_batch(
        &prepared.targets,
        &prepared.exclusions,
        prepared.continue_on_error,
        "package",
        |package, exclusions| helper.discover_with_exclusions(package, exclusions),
    );
    helper.shutdown();
    finish(discovery, &prepared, &rules, start, false)
}

fn prepare(
    params: Value,
    target_key: &str,
    target_description: &str,
    cache: &ProfileCache,
) -> Result<PreparedRequest, Value> {
    let targets = required_string_array(&params, target_key)
        .ok_or_else(|| {
            json!({
                "success": false,
                "error": format!("Missing '{target_key}' parameter (expected array of {target_description})")
            })
        })?;
    if targets.is_empty() {
        return Err(json!({
            "success": false,
            "error": format!("Empty '{target_key}' array; at least one {} is required", if target_key == "sources" { "source glob" } else { "package name" })
        }));
    }
    let profile_ids = required_string_array(&params, "profiles").ok_or_else(|| {
        json!({
            "success": false,
            "error": "Missing 'profiles' parameter (expected array of built-in profile IDs)"
        })
    })?;
    let format = output_format(&params)?;
    let exclusions = string_array(&params, "exclude");
    let continue_on_error = params
        .get("continue_on_discovery_error")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let profiles = load_profiles(&profile_ids, cache)?;
    Ok(PreparedRequest {
        targets,
        exclusions,
        continue_on_error,
        format,
        profiles,
    })
}

fn finish(
    discovery: DiscoveryBatch,
    prepared: &PreparedRequest,
    rules: &[(&Profile, RuleSet)],
    start: Instant,
    validate_envelopes: bool,
) -> Value {
    let mut results = process_schemas(schema_entries(&discovery.models), rules);
    if validate_envelopes {
        append_envelope_diagnostics(&mut results, &discovery.models, rules);
    }
    let diagnostics = attach_source_spans(results, &discovery.models);
    let names = prepared
        .profiles
        .iter()
        .map(|profile| profile.name.clone())
        .collect();
    let report = build_report(
        discovery.coverage,
        discovery.failures,
        discovery.warnings,
        diagnostics,
        names,
        Some(start.elapsed().as_millis() as u64),
    );
    report.rpc_result(render_output(prepared.format, &report))
}
