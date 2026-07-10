use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::Mutex;

use rayon::prelude::*;

use crate::cache::{hash_bytes, Cache};
use crate::cli::args::{CheckArgs, OutputFormat};
use crate::cli::discover;
use crate::cli::pipeline::{
    build_report, build_rulesets, check_rulesets, emit_failure, emit_output, raw_check_result,
};
use crate::normalize::normalize;
use crate::rules::registry::Diagnostic;

use super::load_profiles_from_ids;

pub(super) fn run_check(args: CheckArgs) -> i32 {
    let start = std::time::Instant::now();
    let format = args.format.unwrap_or_else(|| {
        if std::io::stdout().is_terminal() {
            OutputFormat::Human
        } else {
            OutputFormat::Json
        }
    });
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
            return emit_failure(
                format,
                args.output.as_deref(),
                "profiles",
                e.to_string(),
                vec![],
                start.elapsed().as_millis() as u64,
            );
        }
    };
    let profile_names: Vec<String> = profiles
        .iter()
        .map(|profile| profile.name.clone())
        .collect();

    let profile_rulesets = match build_rulesets(&profiles) {
        Ok(rulesets) => rulesets,
        Err(e) => {
            return emit_failure(
                format,
                args.output.as_deref(),
                "profiles",
                format!("failed to construct profile rules: {e}"),
                profile_names,
                start.elapsed().as_millis() as u64,
            );
        }
    };

    // -----------------------------------------------------------------------
    // Discover schema files
    // -----------------------------------------------------------------------
    if args.paths.is_empty() {
        return emit_failure(
            format,
            args.output.as_deref(),
            "input",
            "no schema files or directories provided",
            profile_names,
            start.elapsed().as_millis() as u64,
        );
    }
    let discovery = discover::discover(&args.paths, &args.excludes);

    // -----------------------------------------------------------------------
    // Process schemas (parallel)
    // -----------------------------------------------------------------------
    let cache = Mutex::new(Cache::new());

    let results: Vec<(PathBuf, Result<Vec<Diagnostic>, String>)> = discovery
        .files
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
    let check_results = results
        .into_iter()
        .map(|(path, result)| raw_check_result(path, String::new(), result, &profile_names))
        .collect();
    let report = build_report(
        discovery.coverage,
        discovery.failures,
        vec![],
        check_results,
        profile_names,
        Some(start.elapsed().as_millis() as u64),
    );

    // -----------------------------------------------------------------------
    // Emit output
    // -----------------------------------------------------------------------
    if let Err(exit_code) = emit_output(format, &report, args.output.as_deref()) {
        return exit_code;
    }

    // -----------------------------------------------------------------------
    // Exit code
    // -----------------------------------------------------------------------
    report.exit_code()
}
