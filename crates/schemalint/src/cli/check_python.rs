use std::io::IsTerminal;
use std::path::Path;

use crate::cli::args::{CheckPythonArgs, OutputFormat};
use crate::cli::discovery_policy::discover_batch;
use crate::cli::pipeline::{
    attach_source_spans, build_report, build_rulesets, emit_output, process_schemas, schema_entries,
};
use crate::cli::pyproject;

use super::load_profiles_from_ids;

pub(super) fn run_check_python(args: CheckPythonArgs) -> i32 {
    let start = std::time::Instant::now();

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
            eprintln!("error: {}", e);
            return 1;
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
        eprintln!(
            "error: no packages specified. Use --package or configure [tool.schemalint] in pyproject.toml"
        );
        return 1;
    }

    if profile_args.is_empty() {
        eprintln!(
            "error: no profiles specified. Use --profile or configure [tool.schemalint] in pyproject.toml"
        );
        return 1;
    }

    // -------------------------------------------------------------------
    // 3. Load profiles
    // -------------------------------------------------------------------
    let profiles = match load_profiles_from_ids(&profile_args) {
        Ok(profiles) => profiles,
        Err(e) => {
            eprintln!("error: {}", e);
            return 1;
        }
    };

    let profile_rulesets = match build_rulesets(&profiles) {
        Ok(rulesets) => rulesets,
        Err(e) => {
            eprintln!("error: failed to construct profile rules: {e}");
            return 1;
        }
    };

    let profile_names: Vec<String> = profiles.iter().map(|p| p.name.clone()).collect();

    // -------------------------------------------------------------------
    // 4. Determine output format
    // -------------------------------------------------------------------
    let format = args.format.unwrap_or_else(|| {
        if std::io::stdout().is_terminal() {
            OutputFormat::Human
        } else {
            OutputFormat::Json
        }
    });

    // -------------------------------------------------------------------
    // 5. Spawn Python helper and discover models
    // -------------------------------------------------------------------
    let mut helper = match crate::python::PythonHelper::spawn(args.python_path.as_deref()) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("error: {}", e);
            return 1;
        }
    };

    let discovery = discover_batch(
        &packages,
        &exclude_globs,
        args.continue_on_discovery_error,
        "package",
        |package, exclusions| helper.discover_with_exclusions(package, exclusions),
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

    helper.shutdown();

    if discovery.models.is_empty() {
        eprintln!("warning: no Pydantic models discovered in packages");
    }

    // -------------------------------------------------------------------
    // 6. Normalize and check schemas
    // -------------------------------------------------------------------
    let results = process_schemas(schema_entries(&discovery.models), &profile_rulesets);

    // -------------------------------------------------------------------
    // 7. Attach source spans from discovery
    // -------------------------------------------------------------------
    let all_diagnostics = attach_source_spans(results, &discovery.models);

    // -------------------------------------------------------------------
    // 8. Aggregate results
    // -------------------------------------------------------------------
    let report = build_report(
        discovery.coverage,
        discovery.failures,
        discovery.warnings,
        all_diagnostics,
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
