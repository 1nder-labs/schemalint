use std::time::Instant;

use serde_json::{json, Value};

use crate::cli::args::OutputFormat;
use crate::cli::discovery_policy::{discover_batch, DiscoveryBatch};
use crate::cli::node_policy::{automatic_profile_ids, automatic_target_inputs};
use crate::cli::pipeline::{
    build_report, evaluate_targets, explicit_model_inputs, render_output, EnvelopePolicy,
    TargetInput,
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
    let automatic = prepared.profile_ids.is_none();
    let profile_ids = match &prepared.profile_ids {
        Some(profile_ids) => profile_ids.clone(),
        None => automatic_profile_ids(&discovery.models),
    };
    let profiles = match load_profiles(&profile_ids, &mut state.profiles) {
        Ok(profiles) => profiles,
        Err(error) => return error,
    };
    let rules = match rulesets(&profiles) {
        Ok(rules) => rules,
        Err(error) => return error,
    };
    let inputs = if automatic {
        automatic_target_inputs(&discovery.models, &rules)
    } else {
        explicit_model_inputs(&discovery.models, &rules, EnvelopePolicy::Validate)
    };
    finish(discovery, &prepared, &profiles, &rules, start, inputs)
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
    let inputs = explicit_model_inputs(&discovery.models, &rules, EnvelopePolicy::Ignore);
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
