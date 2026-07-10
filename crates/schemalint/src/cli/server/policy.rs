use serde_json::{json, Value};

use crate::cli::args::OutputFormat;
use crate::cli::pipeline::build_rulesets;
use crate::profile::{load, Profile};
use crate::rules::RuleSet;

use super::ProfileCache;

pub(super) fn output_format(params: &Value) -> Result<OutputFormat, Value> {
    let format = match params.get("format") {
        None => "json",
        Some(Value::String(format)) => format,
        Some(_) => {
            return Err(json!({
                "success": false,
                "error": "Invalid 'format' parameter (expected string)"
            }))
        }
    };
    match format {
        "human" => Ok(OutputFormat::Human),
        "json" => Ok(OutputFormat::Json),
        "sarif" => Ok(OutputFormat::Sarif),
        "gha" => Ok(OutputFormat::Gha),
        "junit" => Ok(OutputFormat::Junit),
        other => Err(json!({
            "success": false,
            "error": format!("Unknown format '{other}'; expected one of: human, json, sarif, gha, junit")
        })),
    }
}

pub(super) fn required_string_array(params: &Value, key: &str) -> Result<Vec<String>, Value> {
    let values = params
        .get(key)
        .ok_or_else(|| json!({"success": false, "error": format!("Missing '{key}' parameter")}))?
        .as_array()
        .ok_or_else(|| {
            json!({"success": false, "error": format!("Invalid '{key}' parameter (expected string array)")})
        })?;
    if !values.iter().all(Value::is_string) {
        return Err(json!({
            "success": false,
            "error": format!("Invalid '{key}' parameter (expected string array)")
        }));
    }
    Ok(values
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect())
}

pub(super) fn optional_string_array(params: &Value, key: &str) -> Result<Vec<String>, Value> {
    if params.get(key).is_none() {
        Ok(Vec::new())
    } else {
        required_string_array(params, key)
    }
}

pub(super) fn load_profiles(
    profile_ids: &[String],
    cache: &ProfileCache,
) -> Result<Vec<Profile>, Value> {
    let mut loaded = Vec::new();
    let mut cache = cache.lock().unwrap_or_else(|error| error.into_inner());
    for profile_id in profile_ids {
        let profile = if let Some(profile) = cache.get(profile_id) {
            profile.clone()
        } else {
            let bytes = crate::cli::resolve_builtin_profile(profile_id).map_err(|error| {
                json!({"success": false, "error": format!("Failed to resolve profile '{profile_id}': {error}")})
            })?;
            let profile = load(&bytes).map_err(|error| {
                json!({"success": false, "error": format!("Failed to load profile '{profile_id}': {error}")})
            })?;
            cache.insert(profile_id.clone(), profile.clone());
            profile
        };
        loaded.push(profile);
    }
    drop(cache);
    loaded.sort_by(|left, right| left.name.cmp(&right.name));
    loaded.dedup_by(|left, right| left.name == right.name);
    Ok(loaded)
}

pub(super) fn rulesets(profiles: &[Profile]) -> Result<Vec<(&Profile, RuleSet)>, Value> {
    build_rulesets(profiles).map_err(|error| {
        json!({"success": false, "error": format!("Failed to construct profile rules: {error}")})
    })
}
