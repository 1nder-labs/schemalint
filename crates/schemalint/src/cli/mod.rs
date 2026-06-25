use std::fs;
use std::process;

use crate::cli::args::{Cli, Commands};
use crate::profile::{load, Profile};

/// Built-in profile IDs. Shared between `resolve_profile` and the auto-detect
/// block in `check_node` so the strings are defined exactly once.
pub(super) const OPENAI_PROFILE_ID: &str = "openai.so.2026-04-30";
pub(super) const ANTHROPIC_PROFILE_ID: &str = "anthropic.so.2026-04-30";

pub mod args;
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
mod glob;
mod pipeline;

pub(crate) use pipeline::check_rulesets;

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
/// Otherwise it is matched against built-in profile IDs.
pub fn resolve_profile(path_or_id: &str) -> Result<Vec<u8>, String> {
    if path_or_id.contains('/') || path_or_id.contains('\\') {
        fs::read(path_or_id).map_err(|e| format!("{e}"))
    } else {
        match path_or_id {
            OPENAI_PROFILE_ID => Ok(schemalint_profiles::OPENAI_SO_2026_04_30
                .as_bytes()
                .to_vec()),
            ANTHROPIC_PROFILE_ID => Ok(schemalint_profiles::ANTHROPIC_SO_2026_04_30
                .as_bytes()
                .to_vec()),
            other => Err(format!("unknown built-in profile '{other}'")),
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
