use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use crate::cli::args::{CheckNodeArgs, OutputFormat};
use crate::cli::glob::glob_match;
use crate::cli::node_config;
use crate::cli::pipeline::{aggregate_results, attach_source_spans, emit_output, process_schemas};
use crate::rules::registry::RuleSet;

use super::{load_profiles_from_ids, ANTHROPIC_PROFILE_ID, OPENAI_PROFILE_ID};

pub(super) fn run_check_node(args: CheckNodeArgs) -> i32 {
    let start = std::time::Instant::now();

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
            eprintln!("error: {}", e);
            return 1;
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

    let exclude_globs: Vec<String> = node_config
        .as_ref()
        .map(|c| c.exclude.clone())
        .unwrap_or_default();

    if sources.is_empty() {
        eprintln!(
            "error: no sources specified. Use --source or configure \"schemalint\" in package.json"
        );
        return 1;
    }

    let explicit_profiles = if profile_args.is_empty() {
        None
    } else {
        match load_profiles_from_ids(&profile_args) {
            Ok(profiles) => Some(profiles),
            Err(e) => {
                eprintln!("error: {}", e);
                return 1;
            }
        }
    };

    // -------------------------------------------------------------------
    // 3. Determine output format
    // -------------------------------------------------------------------
    let format = args.format.unwrap_or_else(|| {
        if std::io::stdout().is_terminal() {
            OutputFormat::Human
        } else {
            OutputFormat::Json
        }
    });

    // -------------------------------------------------------------------
    // 4. Spawn Node helper and discover schemas
    // -------------------------------------------------------------------
    let mut helper = match crate::node::NodeHelper::spawn(args.node_path.as_deref()) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("error: {}", e);
            return 1;
        }
    };

    let mut discovered_models: Vec<crate::ingest::DiscoveredModel> = Vec::new();
    let mut discovery_failures = 0usize;
    let mut provider_hint: Option<String> = None;
    for source in &sources {
        match helper.discover(source) {
            Ok(resp) => {
                if provider_hint.is_none() {
                    provider_hint = resp.provider_hint.clone();
                }
                for model in resp.models {
                    discovered_models.push(model);
                }
                // Log discovery warnings
                for warning in &resp.warnings {
                    eprintln!(
                        "warning: discovery warning for '{}' in source '{}': {}",
                        warning.model, source, warning.message
                    );
                }
            }
            Err(e) => {
                eprintln!("error: discovery failed for source '{}': {}", source, e);
                discovery_failures += 1;
            }
        }
    }

    // Apply exclude patterns
    if !exclude_globs.is_empty() {
        discovered_models.retain(|m| {
            !exclude_globs.iter().any(|g| {
                let core = g.trim_start_matches("**/");
                let core = core
                    .strip_suffix("/**")
                    .or_else(|| core.strip_suffix("/*"))
                    .unwrap_or(core);
                glob_match(core, &m.module_path)
            })
        });
    }

    let total_discovered = discovered_models.len();
    if total_discovered == 0 {
        eprintln!("warning: no Zod schemas discovered in source globs");
    } else {
        eprintln!(
            "info: discovered {} Zod schema(s) in {} source glob(s)",
            total_discovered,
            sources.len()
        );
    }

    helper.shutdown();

    if discovered_models.is_empty() && discovery_failures > 0 {
        // If no profiles configured yet, show the profiles error instead of
        // the generic discovery failure — the user may have forgotten to
        // configure profiles/packages.json.
        if profile_args.is_empty() {
            eprintln!(
                "error: no profiles specified. Use --profile or configure \"schemalint\" in package.json"
            );
            return 1;
        }
        eprintln!(
            "error: all {} source(s) failed discovery",
            discovery_failures
        );
        return 1;
    }

    // -------------------------------------------------------------------
    // 5. Auto-detect profile from provider_hint if none specified
    // -------------------------------------------------------------------
    if profile_args.is_empty() {
        if let Some(ref hint) = provider_hint {
            let resolved = match hint.as_str() {
                "openai" => OPENAI_PROFILE_ID.to_string(),
                "anthropic" => ANTHROPIC_PROFILE_ID.to_string(),
                other => {
                    eprintln!("error: unknown provider hint '{}' from source files", other);
                    return 1;
                }
            };
            eprintln!(
                "info: auto-detected provider '{}' from source imports → using profile '{}'",
                hint, resolved
            );
            profile_args.push(resolved);
        } else {
            eprintln!(
                "error: no profiles specified. Use --profile or configure \"schemalint\" in package.json"
            );
            return 1;
        }
    }
    let profiles = match explicit_profiles {
        Some(profiles) => profiles,
        None => match load_profiles_from_ids(&profile_args) {
            Ok(profiles) => profiles,
            Err(e) => {
                eprintln!("error: {}", e);
                return 1;
            }
        },
    };

    let profile_rulesets: Vec<(&crate::profile::Profile, RuleSet)> = profiles
        .iter()
        .map(|p| (p, RuleSet::from_profile(p)))
        .collect();

    let profile_names: Vec<String> = profiles.iter().map(|p| p.name.clone()).collect();

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
    let (all_diagnostics, total_errors, total_warnings, fatal_errors) =
        aggregate_results(all_diagnostics);

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

    if total_errors > 0 || fatal_errors > 0 || discovery_failures > 0 {
        1
    } else {
        0
    }
}
