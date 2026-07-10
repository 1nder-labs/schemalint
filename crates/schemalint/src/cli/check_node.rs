use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use crate::cli::args::{CheckNodeArgs, OutputFormat};
use crate::cli::discovery_policy::discover_batch;
use crate::cli::node_config;
use crate::cli::pipeline::{attach_source_spans, build_report, emit_output, process_schemas};
use crate::rules::registry::RuleSet;

use super::{default_profile_ids, load_profiles_from_ids, ANTHROPIC_PROFILE_ID, OPENAI_PROFILE_ID};

pub(super) fn run_check_node(args: CheckNodeArgs) -> i32 {
    let start = std::time::Instant::now();

    // -------------------------------------------------------------------
    // 1. Load package.json configuration
    // -------------------------------------------------------------------
    let config_path = args
        .config
        .as_deref()
        .unwrap_or_else(|| Path::new("package.json"));
    // Directory to search for provider-detection purposes if we fall all the
    // way through to the deps-based default (step 5 below).
    let profile_start_dir: &Path = config_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
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

    let exclude_globs = if args.excludes.is_empty() {
        node_config
            .as_ref()
            .map(|c| c.exclude.clone())
            .unwrap_or_default()
    } else {
        args.excludes.clone()
    };

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

    let discovery = discover_batch(
        &sources,
        &exclude_globs,
        args.continue_on_discovery_error,
        "source",
        |source, exclusions| helper.discover_with_exclusions(source, exclusions),
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

    helper.shutdown();

    // -------------------------------------------------------------------
    // 5. Resolve a default profile if none was given yet: source-import
    //    provider hint first (existing signal), then package.json
    //    dependencies, then the openai default. This never hard-errors — it
    //    always resolves to a profile and prints an `info:` line explaining
    //    the choice.
    // -------------------------------------------------------------------
    if profile_args.is_empty() {
        match discovery.provider_hint.as_deref() {
            Some("openai") => {
                eprintln!(
                    "info: auto-detected provider 'openai' from source imports → using profile '{}'",
                    OPENAI_PROFILE_ID
                );
                profile_args.push(OPENAI_PROFILE_ID.to_string());
            }
            Some("anthropic") => {
                eprintln!(
                    "info: auto-detected provider 'anthropic' from source imports → using profile '{}'",
                    ANTHROPIC_PROFILE_ID
                );
                profile_args.push(ANTHROPIC_PROFILE_ID.to_string());
            }
            Some(other) => {
                eprintln!("error: unknown provider hint '{}' from source files", other);
                return 1;
            }
            None => {
                profile_args = default_profile_ids(profile_start_dir);
            }
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

    // -------------------------------------------------------------------
    // 6. Normalize and check schemas
    // -------------------------------------------------------------------
    let schema_entries: Vec<(PathBuf, String, serde_json::Value)> = discovery
        .models
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
