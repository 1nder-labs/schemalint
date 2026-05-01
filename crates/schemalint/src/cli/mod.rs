use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process;

use rayon::prelude::*;

use crate::cache::{hash_bytes, Cache};
use crate::cli::args::{Cli, Commands, OutputFormat};
use crate::normalize::normalize;
use crate::profile::load;
use crate::rules::registry::{DiagnosticSeverity, RuleSet};

pub mod args;
pub mod discover;
pub mod emit_gha;
pub mod emit_human;
pub mod emit_json;
pub mod emit_junit;
pub mod emit_sarif;
pub mod node_config;
pub mod pyproject;
pub mod server;

/// CLI entry point.
pub fn run() {
    let cli = <Cli as clap::Parser>::parse();
    match cli.command {
        Commands::Check(check_args) => {
            let exit_code = run_check(check_args);
            process::exit(exit_code);
        }
        Commands::CheckPython(args) => {
            let exit_code = run_check_python(args);
            process::exit(exit_code);
        }
        Commands::CheckNode(args) => {
            let exit_code = run_check_node(args);
            process::exit(exit_code);
        }
        Commands::Server(_args) => {
            server::run_server();
        }
    }
}

/// Resolve a profile identifier to raw TOML bytes.
///
/// If the input contains a path separator it is treated as a filesystem path.
/// This is intentional: users explicitly pass a path, and the CLI tool must read
/// files they specify, consistent with standard CLI behavior (e.g. `cat file.txt`).
///
/// Otherwise it is matched against built-in profile IDs.
pub fn resolve_profile(path_or_id: &str) -> Result<Vec<u8>, String> {
    if path_or_id.contains('/') || path_or_id.contains('\\') {
        fs::read(path_or_id).map_err(|e| format!("{e}"))
    } else {
        match path_or_id {
            "openai.so.2026-04-30" => Ok(schemalint_profiles::OPENAI_SO_2026_04_30
                .as_bytes()
                .to_vec()),
            "anthropic.so.2026-04-30" => Ok(schemalint_profiles::ANTHROPIC_SO_2026_04_30
                .as_bytes()
                .to_vec()),
            other => Err(format!("unknown built-in profile '{other}'")),
        }
    }
}

/// Resolve a profile ID to raw TOML bytes, rejecting filesystem paths.
///
/// Only matches built-in profile IDs. Any input that looks like a path
/// (contains `/` or `\`) is rejected. This prevents path traversal issues
/// in server mode where profile selection comes from untrusted input.
pub fn resolve_builtin_profile(path_or_id: &str) -> Result<Vec<u8>, String> {
    if path_or_id.contains('/') || path_or_id.contains('\\') {
        return Err(format!(
            "profile ID '{}' must be a built-in name, not a filesystem path",
            path_or_id
        ));
    }
    match path_or_id {
        "openai.so.2026-04-30" => Ok(schemalint_profiles::OPENAI_SO_2026_04_30
            .as_bytes()
            .to_vec()),
        "anthropic.so.2026-04-30" => Ok(schemalint_profiles::ANTHROPIC_SO_2026_04_30
            .as_bytes()
            .to_vec()),
        other => Err(format!("unknown built-in profile '{other}'")),
    }
}

fn run_check(args: args::CheckArgs) -> i32 {
    let start = std::time::Instant::now();
    // -----------------------------------------------------------------------
    // Load profiles
    // -----------------------------------------------------------------------
    let mut profiles = Vec::new();
    for path_or_id in &args.profiles {
        let path_or_id_str = path_or_id.to_string_lossy();
        let profile_bytes = match resolve_profile(&path_or_id_str) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("error: failed to read profile '{path_or_id_str}': {e}");
                return 1;
            }
        };
        let profile = match load(&profile_bytes) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("error: failed to load profile '{path_or_id_str}': {e}");
                return 1;
            }
        };
        profiles.push(profile);
    }

    // Deduplicate by name (first occurrence wins after sort)
    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    profiles.dedup_by_key(|p| p.name.clone());

    let profile_rulesets: Vec<(&crate::profile::Profile, RuleSet)> = profiles
        .iter()
        .map(|p| (p, RuleSet::from_profile(p)))
        .collect();

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
        if format == OutputFormat::Human {
            println!("0 issues found (0 errors, 0 warnings) across 0 schemas");
        } else {
            print!(
                "{}",
                emit_json::emit_json_to_string(&[], 0, 0, &profile_names, Some(0))
            );
        }
        return 0;
    }

    // -----------------------------------------------------------------------
    // Process schemas (parallel)
    // -----------------------------------------------------------------------
    let cache = std::sync::Mutex::new(Cache::new());

    let results: Vec<(PathBuf, Result<Vec<crate::rules::Diagnostic>, String>)> = files
        .into_par_iter()
        .map(|path| {
            let bytes = match fs::read(&path) {
                Ok(b) => b,
                Err(e) => return (path, Err(format!("failed to read file: {}", e))),
            };

            let hash = hash_bytes(&bytes);
            let cached_schema = {
                let cache_guard = cache.lock().unwrap();
                cache_guard.get(hash).cloned()
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
            cache.lock().unwrap().insert(hash, normalized);
            (path, Ok(diags))
        })
        .collect();

    // -----------------------------------------------------------------------
    // Aggregate results
    // -----------------------------------------------------------------------
    let mut all_diagnostics: Vec<(PathBuf, Vec<crate::rules::Diagnostic>)> = Vec::new();
    let mut total_errors = 0usize;
    let mut total_warnings = 0usize;
    let mut fatal_errors = 0usize;

    for (path, result) in results {
        match result {
            Ok(diags) => {
                for d in &diags {
                    match d.severity {
                        DiagnosticSeverity::Error => total_errors += 1,
                        DiagnosticSeverity::Warning => total_warnings += 1,
                    }
                }
                all_diagnostics.push((path, diags));
            }
            Err(msg) => {
                eprintln!("error: {}: {}", path.display(), msg);
                fatal_errors += 1;
            }
        }
    }

    // Sort by path, then by profile name for deterministic output
    all_diagnostics.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, diags) in &mut all_diagnostics {
        diags.sort_by(|a, b| a.profile.cmp(&b.profile));
    }

    // -----------------------------------------------------------------------
    // Emit output
    // -----------------------------------------------------------------------
    let duration_ms = Some(start.elapsed().as_millis() as u64);
    let output_text = match format {
        OutputFormat::Human => emit_human::emit_human_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            duration_ms,
        ),
        OutputFormat::Json => emit_json::emit_json_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            &profile_names,
            duration_ms,
        ),
        OutputFormat::Sarif => emit_sarif::emit_sarif_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            &profile_names,
            duration_ms,
        ),
        OutputFormat::Gha => emit_gha::emit_gha_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            &profile_names,
            duration_ms,
        ),
        OutputFormat::Junit => emit_junit::emit_junit_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            &profile_names,
            duration_ms,
        ),
    };

    if let Some(out_path) = &args.output {
        if let Err(e) = fs::write(out_path, &output_text) {
            eprintln!(
                "error: failed to write output to '{}': {}",
                out_path.display(),
                e
            );
            return 2;
        }
    } else {
        print!("{}", output_text);
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

fn run_check_python(args: args::CheckPythonArgs) -> i32 {
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
    let mut profiles = Vec::new();
    for id in &profile_args {
        let profile_bytes = match resolve_profile(id) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("error: failed to read profile '{}': {}", id, e);
                return 1;
            }
        };
        let profile = match load(&profile_bytes) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("error: failed to load profile '{}': {}", id, e);
                return 1;
            }
        };
        profiles.push(profile);
    }

    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    profiles.dedup_by_key(|p| p.name.clone());

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
        } else {
            print!(
                "{}",
                emit_json::emit_json_to_string(&[], 0, 0, &profile_names, Some(0))
            );
        }
        return 0;
    }

    // -------------------------------------------------------------------
    // 6. Normalize and check schemas
    // -------------------------------------------------------------------
    let schema_entries: Vec<(PathBuf, serde_json::Value)> = discovered_models
        .iter()
        .map(|m| (PathBuf::from(&m.module_path), m.schema.clone()))
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
    let output_text = match format {
        OutputFormat::Human => emit_human::emit_human_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            duration_ms,
        ),
        OutputFormat::Json => emit_json::emit_json_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            &profile_names,
            duration_ms,
        ),
        OutputFormat::Sarif => emit_sarif::emit_sarif_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            &profile_names,
            duration_ms,
        ),
        OutputFormat::Gha => emit_gha::emit_gha_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            &profile_names,
            duration_ms,
        ),
        OutputFormat::Junit => emit_junit::emit_junit_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            &profile_names,
            duration_ms,
        ),
    };

    if let Some(out_path) = &args.output {
        if let Err(e) = fs::write(out_path, &output_text) {
            eprintln!(
                "error: failed to write output to '{}': {}",
                out_path.display(),
                e
            );
            return 2;
        }
    } else {
        print!("{}", output_text);
    }

    if total_errors > 0 || discovery_failures > 0 {
        1
    } else {
        0
    }
}

fn run_check_node(args: args::CheckNodeArgs) -> i32 {
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

    let profile_args: Vec<String> = if args.profiles.is_empty() {
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

    if profile_args.is_empty() {
        eprintln!(
            "error: no profiles specified. Use --profile or configure \"schemalint\" in package.json"
        );
        return 1;
    }

    // -------------------------------------------------------------------
    // 3. Load profiles
    // -------------------------------------------------------------------
    let mut profiles = Vec::new();
    for id in &profile_args {
        let profile_bytes = match resolve_profile(id) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("error: failed to read profile '{}': {}", id, e);
                return 1;
            }
        };
        let profile = match load(&profile_bytes) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("error: failed to load profile '{}': {}", id, e);
                return 1;
            }
        };
        profiles.push(profile);
    }

    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    profiles.dedup_by_key(|p| p.name.clone());

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
    // 5. Spawn Node helper and discover schemas
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
    for source in &sources {
        match helper.discover(source) {
            Ok(resp) => {
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

    // Apply exclude patterns: filter discovered models by module_path.
    // Simple glob matching: strips leading **/ and trailing /** (or /*) then
    // does a substring match on the remaining path component.
    if !exclude_globs.is_empty() {
        discovered_models.retain(|m| {
            !exclude_globs.iter().any(|g| {
                let core = g.trim_start_matches("**/");
                let core = core
                    .strip_suffix("/**")
                    .or_else(|| core.strip_suffix("/*"))
                    .unwrap_or(core);
                m.module_path.contains(core)
            })
        });
    }

    helper.shutdown();

    if discovered_models.is_empty() {
        if discovery_failures > 0 {
            eprintln!(
                "error: all {} source(s) failed discovery",
                discovery_failures
            );
            return 1;
        }
        if format == OutputFormat::Human {
            println!("0 issues found (0 errors, 0 warnings) across 0 schemas");
        } else {
            print!(
                "{}",
                emit_json::emit_json_to_string(&[], 0, 0, &profile_names, Some(0))
            );
        }
        return 0;
    }

    // -------------------------------------------------------------------
    // 6. Normalize and check schemas
    // -------------------------------------------------------------------
    let schema_entries: Vec<(PathBuf, serde_json::Value)> = discovered_models
        .iter()
        .map(|m| (PathBuf::from(&m.module_path), m.schema.clone()))
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
    let output_text = match format {
        OutputFormat::Human => emit_human::emit_human_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            duration_ms,
        ),
        OutputFormat::Json => emit_json::emit_json_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            &profile_names,
            duration_ms,
        ),
        OutputFormat::Sarif => emit_sarif::emit_sarif_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            &profile_names,
            duration_ms,
        ),
        OutputFormat::Gha => emit_gha::emit_gha_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            &profile_names,
            duration_ms,
        ),
        OutputFormat::Junit => emit_junit::emit_junit_to_string(
            &all_diagnostics,
            total_errors,
            total_warnings,
            &profile_names,
            duration_ms,
        ),
    };

    if let Some(out_path) = &args.output {
        if let Err(e) = fs::write(out_path, &output_text) {
            eprintln!(
                "error: failed to write output to '{}': {}",
                out_path.display(),
                e
            );
            return 2;
        }
    } else {
        print!("{}", output_text);
    }

    if total_errors > 0 || discovery_failures > 0 {
        1
    } else {
        0
    }
}

/// Attach source spans from discovered models to diagnostics.
///
/// When multiple schemas share the same `module_path` (e.g., `UserSchema` and
/// `AddressSchema` in `schemas/models.ts`), their source maps are merged so
/// diagnostics on any schema in the file resolve to the correct span.
fn attach_source_spans(
    results: Vec<(PathBuf, Result<Vec<crate::rules::Diagnostic>, String>)>,
    models: &[crate::ingest::DiscoveredModel],
) -> Vec<(PathBuf, Result<Vec<crate::rules::Diagnostic>, String>)> {
    // Build merged source maps keyed by module_path.
    use std::collections::HashMap;
    let mut merged_maps: HashMap<String, HashMap<String, crate::rules::registry::SourceSpan>> =
        HashMap::new();
    for model in models {
        let entry = merged_maps.entry(model.module_path.clone()).or_default();
        for (pointer, span) in &model.source_map {
            entry.entry(pointer.clone()).or_insert_with(|| span.clone());
        }
    }

    results
        .into_iter()
        .map(|(key, result)| match result {
            Ok(mut diags) => {
                if let Some(merged_map) = merged_maps.get(&key.to_string_lossy().to_string()) {
                    for d in &mut diags {
                        if let Some(span) = merged_map.get(&d.pointer) {
                            d.source = Some(span.clone());
                        }
                    }
                }
                (key, Ok(diags))
            }
            Err(e) => (key, Err(e)),
        })
        .collect()
}

fn aggregate_results(
    results: Vec<(PathBuf, Result<Vec<crate::rules::Diagnostic>, String>)>,
) -> (Vec<(PathBuf, Vec<crate::rules::Diagnostic>)>, usize, usize) {
    let mut all_diagnostics: Vec<(PathBuf, Vec<crate::rules::Diagnostic>)> = Vec::new();
    let mut total_errors = 0usize;
    let mut total_warnings = 0usize;

    for (path, result) in results {
        match result {
            Ok(diags) => {
                for d in &diags {
                    match d.severity {
                        DiagnosticSeverity::Error => total_errors += 1,
                        DiagnosticSeverity::Warning => total_warnings += 1,
                    }
                }
                all_diagnostics.push((path, diags));
            }
            Err(msg) => {
                eprintln!("error: {}: {}", path.display(), msg);
            }
        }
    }

    all_diagnostics.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, diags) in &mut all_diagnostics {
        diags.sort_by(|a, b| a.profile.cmp(&b.profile));
    }

    (all_diagnostics, total_errors, total_warnings)
}

// ---------------------------------------------------------------------------
// Shared pipeline helpers — reused by run_check, handle_check, and run_check_python
// ---------------------------------------------------------------------------

/// Run all profile rulesets against a normalized arena and collect diagnostics.
pub(crate) fn check_rulesets(
    arena: &crate::ir::Arena,
    profile_rulesets: &[(&crate::profile::Profile, RuleSet)],
) -> Vec<crate::rules::Diagnostic> {
    let mut diags = Vec::new();
    for (profile, ruleset) in profile_rulesets {
        diags.extend(ruleset.check_all(arena, profile));
    }
    diags
}

/// Process schemas through the normalize → check pipeline.
///
/// Takes pre-parsed JSON values with their source keys and returns diagnostics
/// grouped by source key. Used by the Python check pipeline (and available for
/// any batch processing of raw JSON schemas).
pub(crate) fn process_schemas(
    schemas: Vec<(PathBuf, serde_json::Value)>,
    profile_rulesets: &[(&crate::profile::Profile, RuleSet)],
) -> Vec<(PathBuf, Result<Vec<crate::rules::Diagnostic>, String>)> {
    schemas
        .into_iter()
        .map(|(key, value)| {
            let normalized = match normalize(value) {
                Ok(n) => n,
                Err(e) => return (key, Err(format!("normalization failed: {}", e))),
            };
            let diags = check_rulesets(&normalized.arena, profile_rulesets);
            (key, Ok(diags))
        })
        .collect()
}
