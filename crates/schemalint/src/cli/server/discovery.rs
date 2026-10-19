use std::path::Path;
use std::time::Instant;

use serde_json::{json, Value};

use crate::cli::args::OutputFormat;
use crate::cli::discovery_policy::{discover_batch, DiscoveryBatch};
use crate::cli::pipeline::{
    build_report, evaluate_targets, explicit_model_inputs, render_output, TargetInput,
};
use crate::profile::Profile;
use crate::rules::RuleSet;

use super::policy::{
    load_profiles, optional_string_array, output_format, required_string_array, rulesets,
};
use super::ServerState;

struct PreparedRequest {
    targets: Vec<String>,
    exclusions: Vec<String>,
    continue_on_error: bool,
    format: OutputFormat,
    profile_ids: Option<Vec<String>>,
}

pub(super) fn handle_node(params: Value, state: &mut ServerState) -> Value {
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
    let profile_ids = match resolve_profile_ids(prepared.profile_ids.as_deref()) {
        Ok(profile_ids) => profile_ids,
        Err(error) => return json!({"success": false, "error": error}),
    };
    let profiles = match load_profiles(&profile_ids, &mut state.profiles) {
        Ok(profiles) => profiles,
        Err(error) => return error,
    };
    let rules = match rulesets(&profiles) {
        Ok(rules) => rules,
        Err(error) => return error,
    };
    let inputs = explicit_model_inputs(&discovery.models, &rules);
    finish(discovery, &prepared, &profiles, &rules, start, inputs)
}

/// Resolve profiles exactly like `check-node`: always load and validate the
/// `"schemalint"` config in `package.json`; explicit profiles win, else the
/// configured profiles, else the package.json-detected providers.
fn resolve_profile_ids(explicit: Option<&[String]>) -> Result<Vec<String>, String> {
    let config_path = Path::new("package.json");
    let configured = crate::cli::node_config::load_node_config(config_path)?
        .map(|config| config.profiles)
        .unwrap_or_default();
    Ok(match explicit {
        Some(profile_ids) => profile_ids.to_vec(),
        None if configured.is_empty() => crate::cli::default_profile_ids(config_path),
        None => configured,
    })
}

pub(super) fn handle_python(params: Value, state: &mut ServerState) -> Value {
    let prepared = match prepare(params, "packages", "Python package names", true) {
        Ok(prepared) => prepared,
        Err(error) => return error,
    };
    let profiles = match load_profiles(
        prepared.profile_ids.as_deref().unwrap_or_default(),
        &mut state.profiles,
    ) {
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
    let inputs = explicit_model_inputs(&discovery.models, &rules);
    finish(discovery, &prepared, &profiles, &rules, start, inputs)
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
    inputs: Vec<TargetInput>,
) -> Value {
    let names: Vec<String> = profiles
        .iter()
        .map(|profile| profile.name.clone())
        .collect();
    let results = evaluate_targets(inputs, rules);
    let report = build_report(
        discovery.coverage,
        discovery.failures,
        discovery.warnings,
        results,
        names,
        Some(start.elapsed().as_millis() as u64),
    );
    report.rpc_result(render_output(prepared.format, &report))
}
