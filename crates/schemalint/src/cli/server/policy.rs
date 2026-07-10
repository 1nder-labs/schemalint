use serde_json::{json, Value};

use crate::cli::args::OutputFormat;
use crate::profile::{load, Profile};
use crate::rules::RuleSet;

use super::ProfileCache;

pub(super) fn output_format(params: &Value) -> Result<OutputFormat, Value> {
    match params
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or("json")
    {
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

pub(super) fn string_array(params: &Value, key: &str) -> Vec<String> {
    params
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
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
    profiles
        .iter()
        .map(|profile| {
            RuleSet::from_profile(profile)
                .map(|rules| (profile, rules))
                .map_err(|error| {
                    json!({"success": false, "error": format!("Failed to construct profile rules: {error}")})
                })
        })
        .collect()
}
