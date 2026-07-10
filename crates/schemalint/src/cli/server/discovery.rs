use std::time::Instant;

use serde_json::{json, Value};

use crate::cli::args::OutputFormat;
use crate::cli::discovery_policy::{discover_batch, DiscoveryBatch};
use crate::cli::node_policy::{automatic_profile_ids, process_node_targets};
use crate::cli::pipeline::{
    append_envelope_diagnostics, attach_source_spans, build_report, process_schemas, render_output,
    schema_entries,
};
use crate::profile::Profile;
use crate::rules::RuleSet;

use super::policy::{
    load_profiles, optional_string_array, output_format, required_string_array, rulesets,
};
use super::ProfileCache;

struct PreparedRequest {
    targets: Vec<String>,
    exclusions: Vec<String>,
    continue_on_error: bool,
    format: OutputFormat,
    profile_ids: Option<Vec<String>>,
}

#[derive(Clone, Copy)]
enum DiscoveryMode {
    NodeAutomatic,
    NodeExplicit,
    PythonExplicit,
}

pub(super) fn handle_node(params: Value, cache: &ProfileCache) -> Value {
    let prepared = match prepare(params, "sources", "source globs", false) {
        Ok(prepared) => prepared,
        Err(error) => return error,
    };
    let start = Instant::now();
    let discovery = discover_batch(
        &prepared.targets,
        &prepared.exclusions,
        prepared.continue_on_error,
        "source",
        |source, exclusions| {
            let mut helper = crate::node::NodeHelper::spawn(None)?;
            let result = helper.discover_with_exclusions(source, exclusions);
            helper.shutdown();
            result
        },
    );
    let (profile_ids, mode) = match &prepared.profile_ids {
        Some(profile_ids) => (profile_ids.clone(), DiscoveryMode::NodeExplicit),
        None => (
            automatic_profile_ids(&discovery.models),
            DiscoveryMode::NodeAutomatic,
        ),
    };
    let profiles = match load_profiles(&profile_ids, cache) {
        Ok(profiles) => profiles,
        Err(error) => return error,
    };
    let rules = match rulesets(&profiles) {
        Ok(rules) => rules,
        Err(error) => return error,
    };
    finish(discovery, &prepared, &profiles, &rules, start, mode)
}

pub(super) fn handle_python(params: Value, cache: &ProfileCache) -> Value {
    let prepared = match prepare(params, "packages", "Python package names", true) {
        Ok(prepared) => prepared,
        Err(error) => return error,
    };
    let profiles = match load_profiles(prepared.profile_ids.as_deref().unwrap_or_default(), cache) {
        Ok(profiles) => profiles,
        Err(error) => return error,
    };
    let rules = match rulesets(&profiles) {
        Ok(rules) => rules,
        Err(error) => return error,
    };
    let start = Instant::now();
    let discovery = discover_batch(
        &prepared.targets,
        &prepared.exclusions,
        prepared.continue_on_error,
        "package",
        |package, exclusions| {
            let mut helper = crate::python::PythonHelper::spawn(None)?;
            let result = helper.discover_with_exclusions(package, exclusions);
            helper.shutdown();
            result
        },
    );
    finish(
        discovery,
        &prepared,
        &profiles,
        &rules,
        start,
        DiscoveryMode::PythonExplicit,
    )
}

fn prepare(
    params: Value,
    target_key: &str,
    target_description: &str,
    profiles_required: bool,
) -> Result<PreparedRequest, Value> {
    let targets = required_string_array(&params, target_key)?;
    if targets.is_empty() {
        return Err(json!({
            "success": false,
            "error": format!("Empty '{target_key}' array; at least one entry from {target_description} is required")
        }));
    }
    let profile_ids = if params.get("profiles").is_some() {
        let profile_ids = required_string_array(&params, "profiles")?;
        if profile_ids.is_empty() {
            return Err(json!({
                "success": false,
                "error": "Empty 'profiles' array; at least one profile is required"
            }));
        }
        Some(profile_ids)
    } else if profiles_required {
        return Err(json!({
            "success": false,
            "error": "Missing 'profiles' parameter (expected string array)"
        }));
    } else {
        None
    };
    let format = output_format(&params)?;
    let exclusions = optional_string_array(&params, "exclude")?;
    let continue_on_error = match params.get("continue_on_discovery_error") {
        None => false,
        Some(Value::Bool(value)) => *value,
        Some(_) => {
            return Err(json!({
                "success": false,
                "error": "Invalid 'continue_on_discovery_error' parameter (expected boolean)"
            }))
        }
    };
    Ok(PreparedRequest {
        targets,
        exclusions,
        continue_on_error,
        format,
        profile_ids,
    })
}

fn finish(
    discovery: DiscoveryBatch,
    prepared: &PreparedRequest,
    profiles: &[Profile],
    rules: &[(&Profile, RuleSet)],
    start: Instant,
    mode: DiscoveryMode,
) -> Value {
    let names: Vec<String> = profiles
        .iter()
        .map(|profile| profile.name.clone())
        .collect();
    let mut results = match mode {
        DiscoveryMode::NodeAutomatic => process_node_targets(&discovery.models, rules),
        DiscoveryMode::NodeExplicit | DiscoveryMode::PythonExplicit => {
            process_schemas(schema_entries(&discovery.models, &names), rules)
        }
    };
    if matches!(mode, DiscoveryMode::NodeExplicit) {
        append_envelope_diagnostics(&mut results, &discovery.models, rules);
    }
    let diagnostics = attach_source_spans(results);
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
