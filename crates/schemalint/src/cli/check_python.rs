use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use crate::cli::args::{CheckPythonArgs, OutputFormat};
use crate::cli::pipeline::{aggregate_results, attach_source_spans, emit_output, process_schemas};
use crate::cli::pyproject;
use crate::rules::registry::RuleSet;

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

    let profile_rulesets: Vec<(&crate::profile::Profile, RuleSet)> = profiles
        .iter()
        .map(|p| (p, RuleSet::from_profile(p)))
        .collect();

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

    let mut discovered_models: Vec<crate::ingest::DiscoveredModel> = Vec::new();
    let mut discovery_failures = 0usize;
    for package in &packages {
        match helper.discover(package) {
            Ok(resp) => {
                for model in resp.models {
                    discovered_models.push(model);
                }
            }
            Err(e) => {
                eprintln!("error: discovery failed for package '{}': {}", package, e);
                discovery_failures += 1;
            }
        }
    }

    helper.shutdown();

    if discovered_models.is_empty() {
        if discovery_failures > 0 {
            eprintln!(
                "error: all {} package(s) failed discovery",
                discovery_failures
            );
            return 1;
        }
        if format == OutputFormat::Human {
            println!("0 issues found (0 errors, 0 warnings) across 0 schemas");
        } else if let Err(exit_code) = emit_output(
            format,
            &[],
            0,
            0,
            &profile_names,
            Some(0),
            args.output.as_deref(),
        ) {
            return exit_code;
        }
        return 0;
    }

    // -------------------------------------------------------------------
    // 6. Normalize and check schemas
    // -------------------------------------------------------------------
    let schema_entries: Vec<(PathBuf, String, serde_json::Value)> = discovered_models
        .iter()
        .map(|m| {
            (
                PathBuf::from(&m.module_path),
                m.name.clone(),
                m.schema.clone(),
            )
        })
        .collect();

    let results = process_schemas(schema_entries, &profile_rulesets);

    // -------------------------------------------------------------------
    // 7. Attach source spans from discovery
    // -------------------------------------------------------------------
    let all_diagnostics = attach_source_spans(results, &discovered_models);

    // -------------------------------------------------------------------
    // 8. Aggregate results
    // -------------------------------------------------------------------
    let (all_diagnostics, total_errors, total_warnings) = aggregate_results(all_diagnostics);

    // -------------------------------------------------------------------
    // 9. Emit output
    // -------------------------------------------------------------------
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

    if total_errors > 0 || discovery_failures > 0 {
        1
    } else {
        0
    }
}
