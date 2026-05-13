use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::Mutex;

use rayon::prelude::*;

use crate::cache::{hash_bytes, Cache};
use crate::cli::args::{CheckArgs, OutputFormat};
use crate::cli::pipeline::{check_rulesets, emit_output};
use crate::cli::{discover, emit_json};
use crate::normalize::normalize;
use crate::rules::registry::{Diagnostic, DiagnosticSeverity, RuleSet};

use super::load_profiles_from_ids;

pub(super) fn run_check(args: CheckArgs) -> i32 {
    let start = std::time::Instant::now();
    let profile_args: Vec<String> = args
        .profiles
        .iter()
        .map(|profile| profile.to_string_lossy().to_string())
        .collect();
    let profiles = match load_profiles_from_ids(&profile_args) {
        Ok(profiles) => profiles,
        Err(e) => {
            eprintln!("error: {}", e);
            return 1;
        }
    };

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
    let cache = Mutex::new(Cache::new());

    let results: Vec<(PathBuf, Result<Vec<Diagnostic>, String>)> = files
        .into_par_iter()
        .map(|path| {
            let bytes = match std::fs::read(&path) {
                Ok(b) => b,
                Err(e) => return (path, Err(format!("failed to read file: {}", e))),
            };

            let hash = hash_bytes(&bytes);
            let cached_schema = {
                let cache_guard = cache.lock().unwrap();
                cache_guard.get(hash, &bytes).cloned()
            };
            if let Some(cached) = cached_schema {
                let diags = check_rulesets(&cached.arena, &profile_rulesets);
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

            let diags = check_rulesets(&normalized.arena, &profile_rulesets);
            cache.lock().unwrap().insert(hash, bytes, normalized);
            (path, Ok(diags))
        })
        .collect();

    // -----------------------------------------------------------------------
    // Aggregate results
    // -----------------------------------------------------------------------
    let mut all_diagnostics: Vec<(PathBuf, Vec<Diagnostic>)> = Vec::new();
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
    if let Err(exit_code) = emit_output(
        format,
        &all_diagnostics,
        total_errors,
        total_warnings,
        &profile_names,
        duration_ms,
        args.output.as_deref(),
    ) {
        return exit_code;
    }

    // -----------------------------------------------------------------------
    // Exit code
    // -----------------------------------------------------------------------
    if total_errors > 0 || fatal_errors > 0 {
        1
    } else {
        0
    }
}
