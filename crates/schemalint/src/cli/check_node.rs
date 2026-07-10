use std::io::IsTerminal;
use std::path::Path;

use crate::cli::args::{CheckNodeArgs, OutputFormat};
use crate::cli::discovery_policy::discover_batch;
use crate::cli::node_config;
use crate::cli::node_policy::{automatic_profile_ids, automatic_target_inputs};
use crate::cli::pipeline::{
    build_report, build_rulesets, emit_failure, emit_output, evaluate_targets,
    explicit_model_inputs, EnvelopePolicy,
};

use super::load_profiles_from_ids;

pub(super) fn run_check_node(args: CheckNodeArgs) -> i32 {
    let start = std::time::Instant::now();
    let format = args.format.unwrap_or_else(|| {
        if std::io::stdout().is_terminal() {
            OutputFormat::Human
        } else {
            OutputFormat::Json
        }
    });

    // -------------------------------------------------------------------
    // 1. Load package.json configuration
    // -------------------------------------------------------------------
    let config_path = args
        .config
        .as_deref()
        .unwrap_or_else(|| Path::new("package.json"));
    let node_config = match node_config::load_node_config(config_path) {
        Ok(c) => c,
        Err(e) => {
            return emit_failure(
                format,
                args.output.as_deref(),
                "config",
                e.to_string(),
                vec![],
                start.elapsed().as_millis() as u64,
            );
        }
    };

    // -------------------------------------------------------------------
    // 2. Merge CLI flags on top of config
    // -------------------------------------------------------------------
    let sources = if args.sources.is_empty() {
        node_config
            .as_ref()
            .map(|c| c.include.clone())
            .unwrap_or_default()
    } else {
        args.sources.clone()
    };

    let mut profile_args: Vec<String> = if args.profiles.is_empty() {
        node_config
            .as_ref()
            .map(|c| c.profiles.clone())
            .unwrap_or_default()
    } else {
        args.profiles
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    };

    let exclude_globs = if args.excludes.is_empty() {
        node_config
            .as_ref()
            .map(|c| c.exclude.clone())
            .unwrap_or_default()
    } else {
        args.excludes.clone()
    };

    if sources.is_empty() {
        return emit_failure(
            format,
            args.output.as_deref(),
            "sources",
            "no sources specified. Use --source or configure \"schemalint\" in package.json",
            vec![],
            start.elapsed().as_millis() as u64,
        );
    }

    let explicit_profiles = if profile_args.is_empty() {
        None
    } else {
        match load_profiles_from_ids(&profile_args) {
            Ok(profiles) => Some(profiles),
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
        }
    };

    // -------------------------------------------------------------------
    // 4. Discover schemas in isolated helpers. One wedged source cannot poison
    //    continuation for later sources on the serial JSON-RPC transport.
    // -------------------------------------------------------------------
    let discovery = discover_batch(
        &sources,
        &exclude_globs,
        args.continue_on_discovery_error,
        "source",
        |source, exclusions| {
            let mut helper = crate::node::NodeHelper::spawn(args.node_path.as_deref())?;
            let result = helper.discover_with_exclusions(source, exclusions);
            helper.shutdown();
            result
        },
    );
    for failure in &discovery.failures {
        eprintln!(
            "error: discovery failed for {}: {}",
            failure.target, failure.message
        );
    }
    for warning in &discovery.warnings {
        eprintln!("warning: {}: {}", warning.target, warning.message);
    }

    let total_discovered = discovery.models.len();
    if total_discovered == 0 {
        eprintln!("warning: no Zod schemas discovered in source globs");
    } else {
        eprintln!(
            "info: discovered {} Zod schema(s) in {} source glob(s)",
            total_discovered,
            sources.len()
        );
    }

    // -------------------------------------------------------------------
    // 5. Resolve automatic profiles from per-usage ownership. Ambiguous
    //    targets are retained and become typed pipeline failures below.
    // -------------------------------------------------------------------
    if profile_args.is_empty() {
        profile_args = automatic_profile_ids(&discovery.models);
        if !profile_args.is_empty() {
            eprintln!(
                "info: auto-selected per-target profile(s): {}",
                profile_args.join(", ")
            );
        }
    }
    let uses_explicit_profiles = explicit_profiles.is_some();
    let profiles = match explicit_profiles {
        Some(profiles) => profiles,
        None if profile_args.is_empty() => Vec::new(),
        None => match load_profiles_from_ids(&profile_args) {
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
        },
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

    // -------------------------------------------------------------------
    // 6. Normalize and check schemas
    // -------------------------------------------------------------------
    let inputs = if uses_explicit_profiles {
        explicit_model_inputs(
            &discovery.models,
            &profile_rulesets,
            EnvelopePolicy::Validate,
        )
    } else {
        automatic_target_inputs(&discovery.models, &profile_rulesets)
    };
    let results = evaluate_targets(inputs, &profile_rulesets);

    // -------------------------------------------------------------------
    // 8. Aggregate results
    // -------------------------------------------------------------------
    let report = build_report(
        discovery.coverage,
        discovery.failures,
        discovery.warnings,
        results,
        profile_names,
        Some(start.elapsed().as_millis() as u64),
    );

    // -------------------------------------------------------------------
    // 9. Emit output
    // -------------------------------------------------------------------
    if let Err(exit_code) = emit_output(format, &report, args.output.as_deref()) {
        return exit_code;
    }

    report.exit_code()
}
