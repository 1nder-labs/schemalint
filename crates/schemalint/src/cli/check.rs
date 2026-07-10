use std::io::IsTerminal;
use std::path::PathBuf;

use crate::cli::args::{CheckArgs, OutputFormat};
use crate::cli::discover;
use crate::cli::pipeline::{
    build_report, build_rulesets, emit_failure, emit_output, evaluate_targets, raw_target_input,
};

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
    let inputs = discovery
        .files
        .into_iter()
        .map(|path| raw_target_input(path, &profile_names, profile_rulesets.len()))
        .collect();
    let results = evaluate_targets(inputs, &profile_rulesets);

    // -----------------------------------------------------------------------
    // Aggregate results
    // -----------------------------------------------------------------------
    let report = build_report(
        discovery.coverage,
        discovery.failures,
        vec![],
        results,
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
