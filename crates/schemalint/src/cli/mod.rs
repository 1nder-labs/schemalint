use std::fs;
use std::path::Path;
use std::process;

use crate::cli::args::{Cli, Commands};
use crate::profile::{load, Profile};

/// Built-in profile IDs. Shared between `resolve_profile` and the auto-detect
/// block in `check_node` so the strings are defined exactly once. Each is the
/// "latest" snapshot for its provider — the target of the bare `"openai"` /
/// `"anthropic"` and `.so.latest` aliases.
pub(super) const OPENAI_PROFILE_ID: &str = "openai.so.2026-04-30";
pub(super) const ANTHROPIC_PROFILE_ID: &str = "anthropic.so.2026-04-30";

/// Metadata for a built-in profile: its ID, provider alias, and a one-line
/// description. Reused by `schemalint profiles` and by the default-profile
/// auto-detection in `check` / `check_node`.
pub struct ProfileMeta {
    pub id: &'static str,
    pub provider: &'static str,
    pub description: &'static str,
}

/// All built-in profiles, in display order.
pub const PROFILES: &[ProfileMeta] = &[
    ProfileMeta {
        id: OPENAI_PROFILE_ID,
        provider: "openai",
        description: "OpenAI Structured Outputs schema rules (additionalProperties:false, all fields required, no optional/anyOf-of-objects, unsupported keywords, size/nesting limits).",
    },
    ProfileMeta {
        id: ANTHROPIC_PROFILE_ID,
        provider: "anthropic",
        description: "Anthropic tool-use schema rules (additionalProperties:false; forbids numeric/string-length/array-size keywords; up to 24 optional properties and 16 union members; no allOf combined with $ref).",
    },
];

pub mod args;
pub mod detect;
pub mod discover;
pub mod docs_url;
pub mod emit_gha;
pub mod emit_human;
pub mod emit_json;
pub mod emit_junit;
pub mod emit_sarif;
pub mod node_config;
pub mod pyproject;
pub mod server;

mod check;
mod check_node;
mod check_python;
mod discovery_policy;
mod glob;
mod pipeline;
mod profiles_cmd;
mod report;

/// CLI entry point.
pub fn run() {
    let cli = <Cli as clap::Parser>::parse();
    match cli.command {
        Commands::Check(check_args) => {
            let exit_code = check::run_check(check_args);
            process::exit(exit_code);
        }
        Commands::CheckPython(args) => {
            let exit_code = check_python::run_check_python(args);
            process::exit(exit_code);
        }
        Commands::CheckNode(args) => {
            let exit_code = check_node::run_check_node(args);
            process::exit(exit_code);
        }
        Commands::Profiles(_args) => {
            let exit_code = profiles_cmd::run_profiles();
            process::exit(exit_code);
        }
        Commands::Server(_args) => {
            server::run_server();
        }
    }
}

/// Resolve a profile identifier to raw TOML bytes.
///
/// If the input contains a path separator it is treated as a filesystem path.
/// This is intentional: users explicitly pass a path, and the CLI tool must read
/// files they specify, consistent with standard CLI behavior (e.g. `cat file.txt`).
///
/// Otherwise it is matched against built-in profile IDs. The bare provider
/// names `"openai"` / `"anthropic"` and the `"openai.so.latest"` /
/// `"anthropic.so.latest"` aliases resolve to that provider's latest dated
/// profile, so the exact dated ID never has to be typed for the common case.
/// The dated IDs themselves (e.g. `"openai.so.2026-04-30"`) always keep working.
pub fn resolve_profile(path_or_id: &str) -> Result<Vec<u8>, String> {
    if path_or_id.contains('/') || path_or_id.contains('\\') {
        fs::read(path_or_id).map_err(|e| format!("{e}"))
    } else {
        let resolved = match path_or_id {
            "openai" | "openai.so.latest" => OPENAI_PROFILE_ID,
            "anthropic" | "anthropic.so.latest" => ANTHROPIC_PROFILE_ID,
            other => other,
        };
        match resolved {
            OPENAI_PROFILE_ID => Ok(crate::profiles::OPENAI_SO_2026_04_30.as_bytes().to_vec()),
            ANTHROPIC_PROFILE_ID => {
                Ok(crate::profiles::ANTHROPIC_SO_2026_04_30.as_bytes().to_vec())
            }
            _ => Err(format!("unknown built-in profile '{path_or_id}'")),
        }
    }
}

/// Resolve a profile ID to raw TOML bytes, rejecting filesystem paths.
///
/// Only matches built-in profile IDs. Any input that looks like a path
/// (contains `/` or `\`) is rejected. This prevents path traversal issues
/// in server mode where profile selection comes from untrusted input.
pub fn resolve_builtin_profile(path_or_id: &str) -> Result<Vec<u8>, String> {
    if path_or_id.contains('/') || path_or_id.contains('\\') {
        return Err(format!(
            "profile ID '{}' must be a built-in name, not a filesystem path",
            path_or_id
        ));
    }
    resolve_profile(path_or_id)
}

/// Resolve a default `--profile` list when the caller didn't pass one
/// explicitly: detect providers from `package.json` dependencies near
/// `start_dir` (see `detect::detect_providers_from_deps`), falling back to
/// the OpenAI profile if nothing is detected.
///
/// Always returns a non-empty list, and always prints exactly one `info:`
/// line to stderr describing how the default was picked — callers should
/// never need to fall back to a hard error for a missing `--profile`.
pub(super) fn default_profile_ids(start_dir: &Path) -> Vec<String> {
    let providers = detect::detect_providers_from_deps(start_dir);
    if providers.is_empty() {
        eprintln!(
            "info: no --profile and no provider detected in package.json; defaulting to {OPENAI_PROFILE_ID} (pass --profile anthropic, or run 'schemalint profiles')"
        );
        return vec![OPENAI_PROFILE_ID.to_string()];
    }

    let ids: Vec<&'static str> = providers
        .iter()
        .map(|&provider| {
            PROFILES
                .iter()
                .find(|meta| meta.provider == provider)
                .map(|meta| meta.id)
                .unwrap_or(OPENAI_PROFILE_ID)
        })
        .collect();
    eprintln!(
        "info: no --profile given; detected {} from package.json → using {}",
        providers.join(", "),
        ids.join(", ")
    );
    ids.into_iter().map(str::to_string).collect()
}

pub(super) fn load_profiles_from_ids(profile_args: &[String]) -> Result<Vec<Profile>, String> {
    let mut profiles = Vec::new();
    for id in profile_args {
        let profile_bytes =
            resolve_profile(id).map_err(|e| format!("failed to read profile '{}': {}", id, e))?;
        let profile =
            load(&profile_bytes).map_err(|e| format!("failed to load profile '{}': {}", id, e))?;
        profiles.push(profile);
    }
    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    profiles.dedup_by_key(|p| p.name.clone());
    Ok(profiles)
}
