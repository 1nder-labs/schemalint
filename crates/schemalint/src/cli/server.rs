use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::cache::{hash_bytes, DiskCache};
use crate::cli::args::OutputFormat;
use crate::cli::{emit_gha, emit_human, emit_json, emit_junit, emit_sarif};
use crate::normalize::normalize;
use crate::profile::load;
use crate::rules::registry::{DiagnosticSeverity, RuleSet};

const MAX_PAYLOAD_BYTES: usize = 10_000_000;
const MAX_CHECK_SECONDS: u64 = 30;

/// Run the JSON-RPC 2.0 server over stdin/stdout.
///
/// Reads one JSON-RPC request per line, dispatches to the appropriate
/// handler, and writes the response back as a single line.
pub fn run_server() {
    let cache = Arc::new(DiskCache::new());
    let profile_cache: Arc<Mutex<HashMap<String, crate::profile::Profile>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.len() > MAX_PAYLOAD_BYTES {
            let error_response = json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32600,
                    "message": "Request payload exceeds 10 MB limit"
                },
                "id": null
            });
            let _ = writeln!(stdout_lock, "{}", error_response);
            continue;
        }

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let error_response = json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {e}")
                    },
                    "id": null
                });
                let _ = writeln!(stdout_lock, "{}", error_response);
                continue;
            }
        };

        if request.get("jsonrpc") != Some(&json!("2.0")) {
            let id = request.get("id").cloned().unwrap_or(json!(null));
            let error_response = json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32600,
                    "message": "Invalid JSON-RPC request: missing or incorrect jsonrpc field"
                },
                "id": id
            });
            let _ = writeln!(stdout_lock, "{}", error_response);
            continue;
        }

        let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let id = request.get("id").cloned().unwrap_or(json!(null));

        match method {
            "check" => {
                let params = request.get("params").cloned().unwrap_or(json!({}));
                let result = handle_check(params, &cache, &profile_cache);
                let response = json!({
                    "jsonrpc": "2.0",
                    "result": result,
                    "id": id
                });
                let _ = writeln!(stdout_lock, "{}", response);
            }
            "shutdown" => {
                let response = json!({
                    "jsonrpc": "2.0",
                    "result": null,
                    "id": id
                });
                let _ = writeln!(stdout_lock, "{}", response);
                break;
            }
            "" => {
                let error_response = json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32600,
                        "message": "Invalid JSON-RPC request: missing method"
                    },
                    "id": id
                });
                let _ = writeln!(stdout_lock, "{}", error_response);
            }
            _ => {
                let error_response = json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {method}")
                    },
                    "id": id
                });
                let _ = writeln!(stdout_lock, "{}", error_response);
            }
        }
    }
}

fn handle_check(
    params: Value,
    cache: &Arc<DiskCache>,
    profile_cache: &Arc<Mutex<HashMap<String, crate::profile::Profile>>>,
) -> Value {
    let schema = match params.get("schema") {
        Some(v) => v.clone(),
        None => {
            return json!({
                "success": false,
                "error": "Missing 'schema' parameter"
            });
        }
    };

    let profiles = match params.get("profiles").and_then(|v| v.as_array()) {
        Some(arr) => arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>(),
        None => {
            return json!({
                "success": false,
                "error": "Missing 'profiles' parameter"
            });
        }
    };

    let format_str = params
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("json");
    let format = match format_str {
        "human" => OutputFormat::Human,
        "json" => OutputFormat::Json,
        "sarif" => OutputFormat::Sarif,
        "gha" => OutputFormat::Gha,
        "junit" => OutputFormat::Junit,
        other => {
            return json!({
                "success": false,
                "error": format!("Unknown format '{}'; expected one of: human, json, sarif, gha, junit", other)
            });
        }
    };

    // Load profiles (cached across requests)
    let mut loaded_profiles = Vec::new();
    {
        let mut cache_guard = profile_cache.lock().unwrap();
        for &profile_id in &profiles {
            let profile = if let Some(cached) = cache_guard.get(profile_id) {
                cached.clone()
            } else {
                let bytes = match crate::cli::resolve_profile(profile_id) {
                    Ok(b) => b,
                    Err(e) => {
                        return json!({
                            "success": false,
                            "error": format!("Failed to resolve profile '{profile_id}': {e}")
                        });
                    }
                };
                let profile = match load(&bytes) {
                    Ok(p) => p,
                    Err(e) => {
                        return json!({
                            "success": false,
                            "error": format!("Failed to load profile '{profile_id}': {e}")
                        });
                    }
                };
                cache_guard.insert(profile_id.to_string(), profile.clone());
                profile
            };
            loaded_profiles.push(profile);
        }
    }

    let profile_rulesets: Vec<(&crate::profile::Profile, RuleSet)> = loaded_profiles
        .iter()
        .map(|p| (p, RuleSet::from_profile(p)))
        .collect();

    let profile_names: Vec<String> = loaded_profiles.iter().map(|p| p.name.clone()).collect();

    let start = Instant::now();

    // Normalize schema
    let bytes = serde_json::to_vec(&schema).unwrap_or_default();
    let hash = hash_bytes(&bytes);

    let normalized = match cache.get(hash) {
        Some(n) => n,
        None => {
            let n = match normalize(schema) {
                Ok(n) => n,
                Err(e) => {
                    return json!({
                        "success": false,
                        "error": format!("Normalization failed: {e}")
                    });
                }
            };
            cache.insert(hash, n.clone());
            n
        }
    };

    // Check rules
    let mut all_diagnostics = Vec::new();
    let mut total_errors = 0usize;
    let mut total_warnings = 0usize;

    for (profile, ruleset) in &profile_rulesets {
        let diags = ruleset.check_all(&normalized.arena, profile);
        for d in &diags {
            match d.severity {
                DiagnosticSeverity::Error => total_errors += 1,
                DiagnosticSeverity::Warning => total_warnings += 1,
            }
        }
        all_diagnostics.push((PathBuf::from("<inline>"), diags));
    }

    if start.elapsed() > Duration::from_secs(MAX_CHECK_SECONDS) {
        return json!({
            "success": false,
            "error": "Check execution exceeded 30 second limit"
        });
    }

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

    json!({
        "success": true,
        "output": output_text,
        "total_errors": total_errors,
        "total_warnings": total_warnings,
    })
}
