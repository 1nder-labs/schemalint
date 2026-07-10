use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use crate::cli::args::{CheckNodeArgs, OutputFormat};
use crate::cli::discovery_policy::discover_batch;
use crate::cli::node_config;
use crate::cli::pipeline::{
    append_envelope_diagnostics, attach_source_spans, build_report, emit_output, process_schemas,
};
use crate::ingest::{DiscoveredModel, Provider, ProviderCertainty};
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
    let results = if uses_explicit_profiles {
        let mut results = process_schemas(schema_entries(&discovery.models), &profile_rulesets);
        append_envelope_diagnostics(&mut results, &discovery.models, &profile_rulesets);
        results
    } else {
        process_node_targets(&discovery.models, &profile_rulesets)
    };

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

fn automatic_profile_ids(models: &[DiscoveredModel]) -> Vec<String> {
    let has_openai = models.iter().any(|model| {
        model.provider.certainty != ProviderCertainty::Ambiguous
            && model.provider.provider == Some(Provider::Openai)
    });
    let has_anthropic = models.iter().any(|model| {
        model.provider.certainty != ProviderCertainty::Ambiguous
            && model.provider.provider == Some(Provider::Anthropic)
    });
    [
        (has_openai, OPENAI_PROFILE_ID),
        (has_anthropic, ANTHROPIC_PROFILE_ID),
    ]
    .into_iter()
    .filter(|(present, _)| *present)
    .map(|(_, profile)| profile.to_string())
    .collect()
}

fn schema_entries(models: &[DiscoveredModel]) -> Vec<(PathBuf, String, serde_json::Value)> {
    models
        .iter()
        .map(|model| {
            (
                PathBuf::from(&model.module_path),
                model.name.clone(),
                model.schema.clone(),
            )
        })
        .collect()
}

fn process_node_targets(
    models: &[DiscoveredModel],
    profile_rulesets: &[(&crate::profile::Profile, RuleSet)],
) -> Vec<(
    PathBuf,
    String,
    Result<Vec<crate::rules::Diagnostic>, String>,
)> {
    let mut results = Vec::with_capacity(models.len());
    let inferred_provider = single_owned_provider(models);
    for model in models {
        let profile_id = match (model.provider.certainty, model.provider.provider) {
            (
                ProviderCertainty::Definitive | ProviderCertainty::Inferred,
                Some(Provider::Openai),
            ) => OPENAI_PROFILE_ID,
            (
                ProviderCertainty::Definitive | ProviderCertainty::Inferred,
                Some(Provider::Anthropic),
            ) => ANTHROPIC_PROFILE_ID,
            (ProviderCertainty::Ambiguous, _) if inferred_provider == Some(Provider::Openai) => {
                OPENAI_PROFILE_ID
            }
            (ProviderCertainty::Ambiguous, _) if inferred_provider == Some(Provider::Anthropic) => {
                ANTHROPIC_PROFILE_ID
            }
            _ => {
                results.push((
                    PathBuf::from(&model.module_path),
                    model.name.clone(),
                    Err(format!(
                        "provider is ambiguous for target kind '{}'; pass --profile explicitly",
                        model.canonical_kind
                    )),
                ));
                continue;
            }
        };
        let Some(index) = profile_rulesets
            .iter()
            .position(|(profile, _)| profile.name == profile_id)
        else {
            results.push((
                PathBuf::from(&model.module_path),
                model.name.clone(),
                Err(format!(
                    "no ruleset loaded for provider profile '{profile_id}'"
                )),
            ));
            continue;
        };
        results.extend(process_schemas(
            vec![(
                PathBuf::from(&model.module_path),
                model.name.clone(),
                model.schema.clone(),
            )],
            std::slice::from_ref(&profile_rulesets[index]),
        ));
        if let Some((_, _, Ok(diagnostics))) = results.last_mut() {
            diagnostics.extend(crate::rules::envelope::check_envelope(
                model,
                profile_rulesets[index].0,
            ));
        }
    }
    results
}

fn single_owned_provider(models: &[DiscoveredModel]) -> Option<Provider> {
    let has_openai = models
        .iter()
        .any(|model| model.provider.provider == Some(Provider::Openai));
    let has_anthropic = models
        .iter()
        .any(|model| model.provider.provider == Some(Provider::Anthropic));
    match (has_openai, has_anthropic) {
        (true, false) => Some(Provider::Openai),
        (false, true) => Some(Provider::Anthropic),
        _ => None,
    }
}
