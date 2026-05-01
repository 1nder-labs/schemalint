use std::fs;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process;

use rayon::prelude::*;

use crate::cache::{hash_bytes, Cache};
use crate::cli::args::{Cli, Commands, OutputFormat};
use crate::normalize::normalize;
use crate::profile::load;
use crate::rules::registry::{DiagnosticSeverity, RuleSet};

pub mod args;
pub mod discover;
pub mod emit_gha;
pub mod emit_human;
pub mod emit_json;
pub mod emit_junit;
pub mod emit_sarif;
pub mod server;

/// CLI entry point.
pub fn run() {
    let cli = <Cli as clap::Parser>::parse();
    match cli.command {
        Commands::Check(check_args) => {
            let exit_code = run_check(check_args);
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
            "openai.so.2026-04-30" => Ok(schemalint_profiles::OPENAI_SO_2026_04_30
                .as_bytes()
                .to_vec()),
            "anthropic.so.2026-04-30" => Ok(schemalint_profiles::ANTHROPIC_SO_2026_04_30
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
    match path_or_id {
        "openai.so.2026-04-30" => Ok(schemalint_profiles::OPENAI_SO_2026_04_30
            .as_bytes()
            .to_vec()),
        "anthropic.so.2026-04-30" => Ok(schemalint_profiles::ANTHROPIC_SO_2026_04_30
            .as_bytes()
            .to_vec()),
        other => Err(format!("unknown built-in profile '{other}'")),
    }
}

fn run_check(args: args::CheckArgs) -> i32 {
    let start = std::time::Instant::now();
    // -----------------------------------------------------------------------
    // Load profiles
    // -----------------------------------------------------------------------
    let mut profiles = Vec::new();
    for path_or_id in &args.profiles {
        let path_or_id_str = path_or_id.to_string_lossy();
        let profile_bytes = match resolve_profile(&path_or_id_str) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("error: failed to read profile '{path_or_id_str}': {e}");
                return 1;
            }
        };
        let profile = match load(&profile_bytes) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("error: failed to load profile '{path_or_id_str}': {e}");
                return 1;
            }
        };
        profiles.push(profile);
    }

    // Deduplicate by name (first occurrence wins after sort)
    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    profiles.dedup_by_key(|p| p.name.clone());

    let profile_rulesets: Vec<(&crate::profile::Profile, RuleSet)> = profiles
        .iter()
        .map(|p| (p, RuleSet::from_profile(p)))
        .collect();

    let profile_names: Vec<String> = profiles.iter().map(|p| p.name.clone()).collect();

    // -----------------------------------------------------------------------
    // Determine output format
    // -----------------------------------------------------------------------
    let format = args.format.unwrap_or_else(|| {
        if std::io::stdout().is_terminal() {
            OutputFormat::Human
        } else {
            OutputFormat::Json
        }
    });

    // -----------------------------------------------------------------------
    // Discover schema files
    // -----------------------------------------------------------------------
    if args.paths.is_empty() {
        eprintln!("error: no schema files or directories provided");
        return 1;
    }
    let files = discover::discover(&args.paths);
    if files.is_empty() {
        if format == OutputFormat::Human {
            println!("0 issues found (0 errors, 0 warnings) across 0 schemas");
        } else {
            print!(
                "{}",
                emit_json::emit_json_to_string(&[], 0, 0, &profile_names, Some(0))
            );
        }
        return 0;
    }

    // -----------------------------------------------------------------------
    // Process schemas (parallel)
    // -----------------------------------------------------------------------
    let cache = std::sync::Mutex::new(Cache::new());

    let results: Vec<(PathBuf, Result<Vec<crate::rules::Diagnostic>, String>)> = files
        .into_par_iter()
        .map(|path| {
            let bytes = match fs::read(&path) {
                Ok(b) => b,
                Err(e) => return (path, Err(format!("failed to read file: {}", e))),
            };

            let hash = hash_bytes(&bytes);
            let cached_schema = {
                let cache_guard = cache.lock().unwrap();
                cache_guard.get(hash).cloned()
            };
            if let Some(cached) = cached_schema {
                let mut diags = Vec::new();
                for (profile, ruleset) in &profile_rulesets {
                    diags.extend(ruleset.check_all(&cached.arena, profile));
                }
                return (path, Ok(diags));
            }

            let value = match serde_json::from_slice::<serde_json::Value>(&bytes) {
                Ok(v) => v,
                Err(e) => return (path, Err(format!("invalid JSON: {}", e))),
            };

            let normalized = match normalize(value) {
                Ok(n) => n,
                Err(e) => return (path, Err(format!("normalization failed: {}", e))),
            };

            let mut diags = Vec::new();
            for (profile, ruleset) in &profile_rulesets {
                diags.extend(ruleset.check_all(&normalized.arena, profile));
            }
            cache.lock().unwrap().insert(hash, normalized);
            (path, Ok(diags))
        })
        .collect();

    // -----------------------------------------------------------------------
    // Aggregate results
    // -----------------------------------------------------------------------
    let mut all_diagnostics: Vec<(PathBuf, Vec<crate::rules::Diagnostic>)> = Vec::new();
    let mut total_errors = 0usize;
    let mut total_warnings = 0usize;
    let mut fatal_errors = 0usize;

    for (path, result) in results {
        match result {
            Ok(diags) => {
                for d in &diags {
                    match d.severity {
                        DiagnosticSeverity::Error => total_errors += 1,
                        DiagnosticSeverity::Warning => total_warnings += 1,
                    }
                }
                all_diagnostics.push((path, diags));
            }
            Err(msg) => {
                eprintln!("error: {}: {}", path.display(), msg);
                fatal_errors += 1;
            }
        }
    }

    // Sort by path, then by profile name for deterministic output
    all_diagnostics.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, diags) in &mut all_diagnostics {
        diags.sort_by(|a, b| a.profile.cmp(&b.profile));
    }

    // -----------------------------------------------------------------------
    // Emit output
    // -----------------------------------------------------------------------
    let duration_ms = Some(start.elapsed().as_millis() as u64);
    let output_text = match format {
        OutputFormat::Human => emit_human::emit_human_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            duration_ms,
        ),
        OutputFormat::Json => emit_json::emit_json_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            &profile_names,
            duration_ms,
        ),
        OutputFormat::Sarif => emit_sarif::emit_sarif_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            &profile_names,
            duration_ms,
        ),
        OutputFormat::Gha => emit_gha::emit_gha_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            &profile_names,
            duration_ms,
        ),
        OutputFormat::Junit => emit_junit::emit_junit_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            &profile_names,
            duration_ms,
        ),
    };

    if let Some(out_path) = &args.output {
        if let Err(e) = fs::write(out_path, &output_text) {
            eprintln!(
                "error: failed to write output to '{}': {}",
                out_path.display(),
                e
            );
            return 2;
        }
    } else {
        print!("{}", output_text);
    }

    // -----------------------------------------------------------------------
    // Exit code
    // -----------------------------------------------------------------------
    // 0 = no lint errors (warnings alone are OK)
    // 1 = lint errors or fatal parse/normalization error
    // 2 = I/O error (e.g. --output file write failure)
    if total_errors > 0 || fatal_errors > 0 {
        1
    } else {
        0
    }
}
