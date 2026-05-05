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
                "openai" => "openai.so.2026-04-30".to_string(),
                "anthropic" => "anthropic.so.2026-04-30".to_string(),
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

/// Simple glob matcher for exclude patterns.
///
/// Handles `*` (match anything within a single path segment except `/`)
/// and `**` (match across path segments — handled by caller via `trim_start_matches`/`strip_suffix`).
/// `?` is not supported; use `*` instead.
fn glob_match(pattern: &str, path: &str) -> bool {
    // Split on *, match each literal segment in order.
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return path.contains(parts[0]);
    }

    let mut pos = 0usize;
    for part in &parts {
        if part.is_empty() {
            continue;
        }
        match path[pos..].find(part) {
            Some(offset) => {
                pos += offset + part.len();
            }
            None => return false,
        }
    }

    let last_part = parts.last().copied().unwrap_or("");
    last_part.is_empty() || path.ends_with(last_part)
}

/// Attach source spans from discovered models to diagnostics.
///
/// Each result carries a `(module_path, model_name)` composite key.
/// When multiple schemas share the same `module_path` (e.g., `UserSchema` and
/// `AddressSchema` in `schemas/models.ts`), each model's source map is matched
/// independently — no merging, no first-write-wins collision.
fn attach_source_spans(
    results: Vec<(
        PathBuf,
        String,
        Result<Vec<crate::rules::Diagnostic>, String>,
    )>,
    models: &[crate::ingest::DiscoveredModel],
) -> Vec<(
    PathBuf,
    String,
    Result<Vec<crate::rules::Diagnostic>, String>,
)> {
    // Build per-model lookup: (module_path, model_name) → source_map
    use std::collections::HashMap;
    let model_maps: HashMap<(&str, &str), &HashMap<String, crate::rules::registry::SourceSpan>> =
        models
            .iter()
            .map(|m| ((m.module_path.as_str(), m.name.as_str()), &m.source_map))
            .collect();

    results
        .into_iter()
        .map(|(key, model_name, result)| match result {
            Ok(mut diags) => {
                if let Some(source_map) =
                    model_maps.get(&(key.to_string_lossy().as_ref(), model_name.as_str()))
                {
                    for d in &mut diags {
                        if let Some(span) = source_map.get(&d.pointer) {
                            d.source = Some(span.clone());
                        }
                    }
                }
                (key, model_name, Ok(diags))
            }
            Err(e) => (key, model_name, Err(e)),
        })
        .collect()
}

fn aggregate_results(
    results: Vec<(
        PathBuf,
        String,
        Result<Vec<crate::rules::Diagnostic>, String>,
    )>,
) -> (Vec<(PathBuf, Vec<crate::rules::Diagnostic>)>, usize, usize) {
    let mut all_diagnostics: Vec<(PathBuf, Vec<crate::rules::Diagnostic>)> = Vec::new();
    let mut total_errors = 0usize;
    let mut total_warnings = 0usize;

    for (path, _model_name, result) in results {
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
/// Takes pre-parsed JSON values with their source keys and model names, and
/// returns diagnostics grouped by source key. The model name is carried through
/// to enable per-model source span lookups (avoiding collisions when multiple
/// schemas share a module_path).
pub(crate) fn process_schemas(
    schemas: Vec<(PathBuf, String, serde_json::Value)>,
    profile_rulesets: &[(&crate::profile::Profile, RuleSet)],
) -> Vec<(
    PathBuf,
    String,
    Result<Vec<crate::rules::Diagnostic>, String>,
)> {
    schemas
        .into_iter()
        .map(|(key, model_name, value)| {
            let normalized = match normalize(value) {
                Ok(n) => n,
                Err(e) => return (key, model_name, Err(format!("normalization failed: {}", e))),
            };
            let diags = check_rulesets(&normalized.arena, profile_rulesets);
            (key, model_name, Ok(diags))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::registry::{DiagnosticSeverity, SourceSpan};

    fn make_diag(pointer: &str) -> crate::rules::Diagnostic {
        crate::rules::Diagnostic {
            code: "TEST-001".into(),
            severity: DiagnosticSeverity::Error,
            message: "test diagnostic".into(),
            pointer: pointer.to_string(),
            source: None,
            profile: "test".into(),
            hint: None,
        }
    }

    fn make_model(
        name: &str,
        module_path: &str,
        spans: Vec<(&str, &str, u32)>,
    ) -> crate::ingest::DiscoveredModel {
        let mut source_map = std::collections::HashMap::new();
        for (pointer, file, line) in spans {
            source_map.insert(
                pointer.to_string(),
                SourceSpan {
                    file: file.to_string(),
                    line: Some(line),
                    col: Some(1),
                },
            );
        }
        crate::ingest::DiscoveredModel {
            name: name.to_string(),
            module_path: module_path.to_string(),
            schema: serde_json::json!({}),
            source_map,
        }
    }

    #[test]
    fn attach_source_spans_single_model() {
        let model = make_model(
            "UserSchema",
            "src/models.ts",
            vec![("/properties/email", "src/models.ts", 5)],
        );
        let diags = vec![make_diag("/properties/email")];
        let results = vec![(
            PathBuf::from("src/models.ts"),
            "UserSchema".into(),
            Ok(diags),
        )];

        let out = attach_source_spans(results, &[model]);
        let (_, _, result) = &out[0];
        let diags = result.as_ref().unwrap();
        assert_eq!(diags[0].source.as_ref().unwrap().file, "src/models.ts");
        assert_eq!(diags[0].source.as_ref().unwrap().line, Some(5));
    }

    #[test]
    fn attach_source_spans_two_models_same_file_no_collision() {
        // UserSchema and AddressSchema share src/models.ts but have different
        // properties. Each model's diagnostics must resolve to its own spans.
        let user_model = make_model(
            "UserSchema",
            "src/models.ts",
            vec![("/properties/email", "src/models.ts", 5)],
        );
        let addr_model = make_model(
            "AddressSchema",
            "src/models.ts",
            vec![("/properties/street", "src/models.ts", 20)],
        );

        let user_diags = vec![make_diag("/properties/email")];
        let addr_diags = vec![make_diag("/properties/street")];

        let results = vec![
            (
                PathBuf::from("src/models.ts"),
                "UserSchema".into(),
                Ok(user_diags),
            ),
            (
                PathBuf::from("src/models.ts"),
                "AddressSchema".into(),
                Ok(addr_diags),
            ),
        ];

        let out = attach_source_spans(results, &[user_model, addr_model]);
        let (_, _, r1) = &out[0];
        let (_, _, r2) = &out[1];
        assert_eq!(
            r1.as_ref().unwrap()[0].source.as_ref().unwrap().line,
            Some(5)
        );
        assert_eq!(
            r2.as_ref().unwrap()[0].source.as_ref().unwrap().line,
            Some(20)
        );
    }

    #[test]
    fn attach_source_spans_two_models_same_file_pointer_collision() {
        // Both models define /properties/name at different lines.
        // Each model's diagnostic must resolve to its OWN span, NOT the
        // other model's span (first-write-wins would break this).
        let user_model = make_model(
            "UserSchema",
            "src/models.ts",
            vec![("/properties/name", "src/models.ts", 5)],
        );
        let addr_model = make_model(
            "AddressSchema",
            "src/models.ts",
            vec![("/properties/name", "src/models.ts", 20)],
        );

        let user_diags = vec![make_diag("/properties/name")];
        let addr_diags = vec![make_diag("/properties/name")];

        let results = vec![
            (
                PathBuf::from("src/models.ts"),
                "UserSchema".into(),
                Ok(user_diags),
            ),
            (
                PathBuf::from("src/models.ts"),
                "AddressSchema".into(),
                Ok(addr_diags),
            ),
        ];

        let out = attach_source_spans(results, &[user_model, addr_model]);
        let (_, _, r1) = &out[0];
        let (_, _, r2) = &out[1];
        // UserSchema's /properties/name → line 5
        assert_eq!(
            r1.as_ref().unwrap()[0].source.as_ref().unwrap().line,
            Some(5)
        );
        // AddressSchema's /properties/name → line 20 (NOT line 5!)
        assert_eq!(
            r2.as_ref().unwrap()[0].source.as_ref().unwrap().line,
            Some(20)
        );
    }

    #[test]
    fn attach_source_spans_unmatched_pointer_leaves_source_none() {
        let model = make_model(
            "UserSchema",
            "src/models.ts",
            vec![("/properties/email", "src/models.ts", 5)],
        );
        let diags = vec![make_diag("/properties/nonexistent")];
        let results = vec![(
            PathBuf::from("src/models.ts"),
            "UserSchema".into(),
            Ok(diags),
        )];

        let out = attach_source_spans(results, &[model]);
        let (_, _, result) = &out[0];
        let diags = result.as_ref().unwrap();
        assert!(diags[0].source.is_none());
    }
}
