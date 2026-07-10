use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::Mutex;

use rayon::prelude::*;

use crate::cache::{hash_bytes, Cache};
use crate::cli::args::{CheckArgs, OutputFormat};
use crate::cli::discover;
use crate::cli::pipeline::{aggregate_results, check_rulesets, emit_empty_output, emit_output};
use crate::normalize::normalize;
use crate::rules::registry::{Diagnostic, RuleSet};

use super::load_profiles_from_ids;

pub(super) fn run_check(args: CheckArgs) -> i32 {
    let start = std::time::Instant::now();
    let mut profile_args: Vec<String> = args
        .profiles
        .iter()
        .map(|profile| profile.to_string_lossy().to_string())
        .collect();

    // No --profile given: auto-detect a default from package.json
    // dependencies near the first schema path (or cwd if none given), always
    // falling back to the openai profile rather than hard-erroring.
    if profile_args.is_empty() {
        let start_dir = args
            .paths
            .first()
            .map(|p| PathBuf::from(p.as_str()))
            .unwrap_or_else(|| PathBuf::from("."));
        profile_args = super::default_profile_ids(&start_dir);
    }

    let profiles = match load_profiles_from_ids(&profile_args) {
        Ok(profiles) => profiles,
        Err(e) => {
            eprintln!("error: {}", e);
            return 1;
        }
    };

    let profile_rulesets: Vec<(&crate::profile::Profile, RuleSet)> = match profiles
        .iter()
        .map(|p| (p, RuleSet::from_profile(p)))
        .map(|(profile, ruleset)| ruleset.map(|ruleset| (profile, ruleset)))
        .collect()
    {
        Ok(rulesets) => rulesets,
        Err(e) => {
            eprintln!("error: failed to construct profile rules: {e}");
            return 1;
        }
    };

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
        return emit_empty_output(format, &profile_names, args.output.as_deref());
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
    let (all_diagnostics, total_errors, total_warnings, fatal_errors) = aggregate_results(
        results
            .into_iter()
            .map(|(p, r)| (p, String::new(), r))
            .collect(),
    );

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
