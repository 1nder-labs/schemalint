use std::io::IsTerminal;
use std::path::Path;

use crate::cli::args::{CheckPythonArgs, OutputFormat};
use crate::cli::discovery_policy::discover_batch;
use crate::cli::pipeline::{
    build_report, build_rulesets, emit_failure, emit_output, evaluate_targets,
    explicit_model_inputs, EnvelopePolicy,
};
use crate::cli::pyproject;

use super::load_profiles_from_ids;

pub(super) fn run_check_python(args: CheckPythonArgs) -> i32 {
    let start = std::time::Instant::now();
    let format = args.format.unwrap_or_else(|| {
        if std::io::stdout().is_terminal() {
            OutputFormat::Human
        } else {
            OutputFormat::Json
        }
    });

    // -------------------------------------------------------------------
    // 1. Load pyproject.toml configuration
    // -------------------------------------------------------------------
    let config_path = args
        .config
        .as_deref()
        .unwrap_or_else(|| Path::new("pyproject.toml"));
    let pyproject_config = match pyproject::load_pyproject_config(config_path) {
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
    let packages = if args.packages.is_empty() {
        pyproject_config
            .as_ref()
            .map(|c| c.packages.clone())
            .unwrap_or_default()
    } else {
        args.packages.clone()
    };

    let profile_args: Vec<String> = if args.profiles.is_empty() {
        pyproject_config
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
        pyproject_config
            .as_ref()
            .map(|config| config.exclude.clone())
            .unwrap_or_default()
    } else {
        args.excludes.clone()
    };

    if packages.is_empty() {
        return emit_failure(
            format,
            args.output.as_deref(),
            "packages",
            "no packages specified. Use --package or configure [tool.schemalint] in pyproject.toml",
            vec![],
            start.elapsed().as_millis() as u64,
        );
    }

    if profile_args.is_empty() {
        return emit_failure(
            format,
            args.output.as_deref(),
            "profiles",
            "no profiles specified. Use --profile or configure [tool.schemalint] in pyproject.toml",
            vec![],
            start.elapsed().as_millis() as u64,
        );
    }

    // -------------------------------------------------------------------
    // 3. Load profiles
    // -------------------------------------------------------------------
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

    // -------------------------------------------------------------------
    // 5. Discover packages in isolated helpers. User import hangs or protocol
    //    failures cannot poison continuation for later packages.
    // -------------------------------------------------------------------
    let discovery = discover_batch(
        &packages,
        &exclude_globs,
        args.continue_on_discovery_error,
        "package",
        |package, exclusions| {
            let mut helper = crate::python::PythonHelper::spawn(args.python_path.as_deref())?;
            let result = helper.discover_with_exclusions(package, exclusions);
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

    if discovery.models.is_empty() {
        eprintln!("warning: no Pydantic models discovered in packages");
    }

    // -------------------------------------------------------------------
    // 6. Normalize and check schemas
    // -------------------------------------------------------------------
    let inputs =
        explicit_model_inputs(&discovery.models, &profile_rulesets, EnvelopePolicy::Ignore);
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
