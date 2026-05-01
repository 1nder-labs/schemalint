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
pub mod emit_human;
pub mod emit_json;

/// CLI entry point.
pub fn run() {
    let cli = <Cli as clap::Parser>::parse();
    match cli.command {
        Commands::Check(check_args) => {
            let exit_code = run_check(check_args);
            process::exit(exit_code);
        }
    }
}

fn run_check(args: args::CheckArgs) -> i32 {
    // -----------------------------------------------------------------------
    // Load profile
    // -----------------------------------------------------------------------
    let profile_bytes = match fs::read(&args.profile) {
        Ok(b) => b,
        Err(e) => {
            eprintln!(
                "error: failed to read profile '{}': {}",
                args.profile.display(),
                e
            );
            return 1;
        }
    };
    let profile = match load(&profile_bytes) {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "error: failed to load profile '{}': {}",
                args.profile.display(),
                e
            );
            return 1;
        }
    };

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
            emit_json::emit_json(&[], 0, 0, std::slice::from_ref(&profile.name));
        }
        return 0;
    }

    // -----------------------------------------------------------------------
    // Process schemas (parallel)
    // -----------------------------------------------------------------------
    let cache = std::sync::Mutex::new(Cache::new());
    let ruleset = RuleSet::from_profile(&profile);

    let results: Vec<(PathBuf, Result<Vec<crate::rules::Diagnostic>, String>)> = files
        .into_par_iter()
        .map(|path| {
            let bytes = match fs::read(&path) {
                Ok(b) => b,
                Err(e) => return (path, Err(format!("failed to read file: {}", e))),
            };

            let hash = hash_bytes(&bytes);
            {
                let cache_guard = cache.lock().unwrap();
                if let Some(cached) = cache_guard.get(hash) {
                    let diags = ruleset.check_all(&cached.arena, &profile);
                    return (path, Ok(diags));
                }
            }

            let value = match serde_json::from_slice::<serde_json::Value>(&bytes) {
                Ok(v) => v,
                Err(e) => return (path, Err(format!("invalid JSON: {}", e))),
            };

            let normalized = match normalize(value) {
                Ok(n) => n,
                Err(e) => return (path, Err(format!("normalization failed: {}", e))),
            };

            let diags = ruleset.check_all(&normalized.arena, &profile);
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

    // Sort by path for deterministic output
    all_diagnostics.sort_by(|a, b| a.0.cmp(&b.0));

    // -----------------------------------------------------------------------
    // Emit output
    // -----------------------------------------------------------------------
    match format {
        OutputFormat::Human => {
            emit_human::emit_human(&all_diagnostics, total_errors, total_warnings);
        }
        OutputFormat::Json => {
            emit_json::emit_json(
                &all_diagnostics,
                total_errors,
                total_warnings,
                &[profile.name],
            );
        }
    }

    // -----------------------------------------------------------------------
    // Exit code
    // -----------------------------------------------------------------------
    // 0 if no errors (warnings alone are OK), 1 if any error or fatal parse error.
    if total_errors > 0 || fatal_errors > 0 {
        1
    } else {
        0
    }
}
