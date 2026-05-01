use std::fs;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process;

use rayon::prelude::*;

use crate::cache::{hash_bytes, Cache};
use crate::cli::args::{Cli, Commands, OutputFormat};
use crate::normalize::normalize;
use crate::profile::load;
use crate::rules::registry::{DiagnosticSeverity, RuleSet};

pub mod args;
pub mod discover;
pub mod emit_human;
pub mod emit_json;

/// CLI entry point.
pub fn run() {
    let cli = <Cli as clap::Parser>::parse();
    match cli.command {
        Commands::Check(check_args) => {
            let exit_code = run_check(check_args);
            process::exit(exit_code);
        }
        Commands::Server(_args) => {
            eprintln!("error: server mode is not yet implemented");
            process::exit(1);
        }
    }
}

/// Resolve a profile identifier to raw TOML bytes.
///
/// If the input contains a path separator it is treated as a filesystem path.
/// Otherwise it is matched against built-in profile IDs.
pub fn resolve_profile(path_or_id: &str) -> Result<Vec<u8>, String> {
    if path_or_id.contains('/') || path_or_id.contains('\\') {
        fs::read(path_or_id).map_err(|e| format!("{e}"))
    } else {
        match path_or_id {
            "openai.so.2026-04-30" => Ok(schemalint_profiles::OPENAI_SO_2026_04_30.as_bytes().to_vec()),
            "anthropic.so.2026-04-30" => Ok(schemalint_profiles::ANTHROPIC_SO_2026_04_30.as_bytes().to_vec()),
            other => Err(format!("unknown built-in profile '{other}'")),
        }
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

    let profile = &profiles[0];

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
                emit_json::emit_json_to_string(
                    &[],
                    0,
                    0,
                    std::slice::from_ref(&profile.name),
                    Some(0)
                )
            );
        }
        return 0;
    }

    // -----------------------------------------------------------------------
    // Process schemas (parallel)
    // -----------------------------------------------------------------------
    let cache = std::sync::Mutex::new(Cache::new());
    let ruleset = RuleSet::from_profile(&profile);

    let results: Vec<(PathBuf, Result<Vec<crate::rules::Diagnostic>, String>)> = files
        .into_par_iter()
        .map(|path| {
            let bytes = match fs::read(&path) {
                Ok(b) => b,
                Err(e) => return (path, Err(format!("failed to read file: {}", e))),
            };

            let hash = hash_bytes(&bytes);
            {
                let cache_guard = cache.lock().unwrap();
                if let Some(cached) = cache_guard.get(hash) {
                    let diags = ruleset.check_all(&cached.arena, &profile);
                    return (path, Ok(diags));
                }
            }

            let value = match serde_json::from_slice::<serde_json::Value>(&bytes) {
                Ok(v) => v,
                Err(e) => return (path, Err(format!("invalid JSON: {}", e))),
            };

            let normalized = match normalize(value) {
                Ok(n) => n,
                Err(e) => return (path, Err(format!("normalization failed: {}", e))),
            };

            let diags = ruleset.check_all(&normalized.arena, &profile);
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

    // Sort by path for deterministic output
    all_diagnostics.sort_by(|a, b| a.0.cmp(&b.0));

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
            &[profile.name.clone()],
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
    // 0 = no lint errors (warnings alone are OK)
    // 1 = lint errors or fatal parse/normalization error
    // 2 = I/O error (e.g. --output file write failure)
    if total_errors > 0 || fatal_errors > 0 {
        1
    } else {
        0
    }
}
