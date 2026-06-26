use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::cache::{hash_bytes, DiskCache};
use crate::cli::args::OutputFormat;
use crate::cli::check_rulesets;
use crate::cli::pipeline::{
    aggregate_results, attach_source_spans, process_schemas, render_output,
};
use crate::normalize::normalize;
use crate::profile::load;
use crate::rules::{Diagnostic, DiagnosticSeverity, RuleSet};

const MAX_PAYLOAD_BYTES: usize = 10_000_000;
const MAX_CHECK_SECONDS: u64 = 30;

// DoS input bounds: reject pathological schemas before any expensive work.
// A real-world JSON Schema is almost always under 100 KiB, a few thousand
// nodes, and fewer than 50 levels deep. These limits are generous enough to
// never affect legitimate usage while preventing CPU/memory exhaustion from
// crafted inputs.
const MAX_SCHEMA_BYTES: usize = 5 * 1024 * 1024; // 5 MiB serialized
const MAX_SCHEMA_NODES: usize = 200_000; // recursive object/array/value count
                                         // Depth guard: a chain like {"a":{"a":...}} 200k levels deep is only ~1.2 MiB
                                         // and ~200k nodes, so it passes both guards above — but then causes a stack
                                         // overflow in count_nodes_bounded itself and in normalize/traverse, crashing
                                         // the server. Bounding depth here prevents that: once rejected, neither the
                                         // counter nor any downstream recursive walk ever receives an over-deep tree.
const MAX_SCHEMA_DEPTH: usize = 1_000; // real schemas are always well under 50

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
            Err(e) => {
                eprintln!("error: failed to read line from stdin: {}", e);
                break;
            }
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
            if writeln!(stdout_lock, "{}", error_response).is_err() {
                break;
            }
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
                if writeln!(stdout_lock, "{}", error_response).is_err() {
                    break;
                }
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
            if writeln!(stdout_lock, "{}", error_response).is_err() {
                break;
            }
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
                if writeln!(stdout_lock, "{}", response).is_err() {
                    break;
                }
            }
            "checkNode" => {
                let params = request.get("params").cloned().unwrap_or(json!({}));
                let result = handle_check_node(params, &profile_cache);
                let response = json!({
                    "jsonrpc": "2.0",
                    "result": result,
                    "id": id
                });
                if writeln!(stdout_lock, "{}", response).is_err() {
                    break;
                }
            }
            "checkPython" => {
                let params = request.get("params").cloned().unwrap_or(json!({}));
                let result = handle_check_python(params, &profile_cache);
                let response = json!({
                    "jsonrpc": "2.0",
                    "result": result,
                    "id": id
                });
                if writeln!(stdout_lock, "{}", response).is_err() {
                    break;
                }
            }
            "shutdown" => {
                let response = json!({
                    "jsonrpc": "2.0",
                    "result": null,
                    "id": id
                });
                if writeln!(stdout_lock, "{}", response).is_err() {
                    break;
                }
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
                if writeln!(stdout_lock, "{}", error_response).is_err() {
                    break;
                }
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
                if writeln!(stdout_lock, "{}", error_response).is_err() {
                    break;
                }
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
        // Use unwrap_or_else to recover from a poisoned lock (a prior request
        // panicked while holding it). The inner data is still valid — we just
        // clear the poison flag and continue rather than taking down the server.
        let mut cache_guard = profile_cache.lock().unwrap_or_else(|e| e.into_inner());
        for &profile_id in &profiles {
            let profile = if let Some(cached) = cache_guard.get(profile_id) {
                cached.clone()
            } else {
                let bytes = match crate::cli::resolve_builtin_profile(profile_id) {
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

    // Deduplicate loaded profiles by name
    loaded_profiles.sort_by(|a, b| a.name.cmp(&b.name));
    loaded_profiles.dedup_by_key(|p| p.name.clone());

    let profile_rulesets: Vec<(&crate::profile::Profile, RuleSet)> = loaded_profiles
        .iter()
        .map(|p| (p, RuleSet::from_profile(p)))
        .collect();

    let profile_names: Vec<String> = loaded_profiles.iter().map(|p| p.name.clone()).collect();

    // --- Input bounds: primary DoS protection ---
    // These checks run before normalize/check_rulesets so a crafted schema
    // cannot consume unbounded CPU or memory. The 30 s elapsed check below
    // remains as a secondary backstop for unforeseen edge cases.

    // 1. Byte-length guard on the serialized schema value.
    let schema_bytes = serde_json::to_vec(&schema).unwrap_or_default();
    if schema_bytes.len() > MAX_SCHEMA_BYTES {
        return json!({
            "success": false,
            "error": format!(
                "Schema serialized size ({} bytes) exceeds the {} byte limit",
                schema_bytes.len(),
                MAX_SCHEMA_BYTES
            )
        });
    }

    // 2. JSON node-count + depth guard with early exit.
    // count_nodes_bounded tracks both a shared node budget and the current
    // nesting depth. It returns false as soon as either limit is hit, so the
    // counter itself is O(min(actual_nodes, MAX_SCHEMA_NODES)) work and can
    // never recurse more than MAX_SCHEMA_DEPTH frames — no overflow while
    // validating. Subsequent normalize/traverse calls therefore also never
    // receive an over-deep tree.
    fn count_nodes_bounded(value: &Value, remaining: &mut usize, depth: usize) -> bool {
        if depth > MAX_SCHEMA_DEPTH {
            return false;
        }
        if *remaining == 0 {
            return false;
        }
        *remaining -= 1;
        match value {
            Value::Array(arr) => arr
                .iter()
                .all(|v| count_nodes_bounded(v, remaining, depth + 1)),
            Value::Object(map) => map
                .values()
                .all(|v| count_nodes_bounded(v, remaining, depth + 1)),
            _ => true,
        }
    }
    let mut budget = MAX_SCHEMA_NODES;
    if !count_nodes_bounded(&schema, &mut budget, 0) {
        return json!({
            "success": false,
            "error": format!(
                "Schema exceeds complexity limits (max depth {MAX_SCHEMA_DEPTH}, \
                 max nodes {MAX_SCHEMA_NODES}); rejected to prevent resource exhaustion"
            )
        });
    }

    let start = Instant::now();

    // Normalize schema. Reuse schema_bytes already produced by the byte-length
    // guard above — no need to re-serialize.
    let hash = hash_bytes(&schema_bytes);

    let normalized = match cache.get(hash, &schema_bytes) {
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
            cache.insert(hash, schema_bytes.clone(), n.clone());
            n
        }
    };

    // Check rules
    let mut total_errors = 0usize;
    let mut total_warnings = 0usize;

    let diags = check_rulesets(&normalized.arena, &profile_rulesets);
    for d in &diags {
        match d.severity {
            DiagnosticSeverity::Error => total_errors += 1,
            DiagnosticSeverity::Warning => total_warnings += 1,
        }
    }
    let all_diagnostics: Vec<(PathBuf, Vec<Diagnostic>)> = vec![(PathBuf::from("<inline>"), diags)];

    if start.elapsed() > Duration::from_secs(MAX_CHECK_SECONDS) {
        return json!({
            "success": false,
            "error": "Check execution exceeded 30 second limit"
        });
    }

    let duration_ms = Some(start.elapsed().as_millis() as u64);

    let output_text = render_output(
        format,
        &all_diagnostics,
        total_errors,
        total_warnings,
        &profile_names,
        duration_ms,
    );

    json!({
        "success": true,
        "output": output_text,
        "total_errors": total_errors,
        "total_warnings": total_warnings,
    })
}

// ---------------------------------------------------------------------------
// checkNode — JSON contract
//
// Request params:
//   {
//     "sources":  ["src/**/*.ts", ...]   // required; TypeScript source globs
//     "profiles": ["openai.so.2026-04-30", ...]  // required; built-in profile IDs only
//     "format":   "json" | "human" | "sarif" | "gha" | "junit"  // optional, default "json"
//   }
//
// Success response result (some sources succeeded):
//   { "success": true, "output": "<rendered text>",
//     "total_errors": N, "total_warnings": N,
//     "discovery_errors": ["..."]   // omitted when empty; present when SOME sources failed
//     "discovery_warnings": ["..."] // omitted when empty; non-fatal warnings from sidecar
//   }
//
// Failure response result (ALL sources failed or no models found):
//   { "success": false, "error": "<message>" }
//
// Lifecycle: per-request spawn → discover-loop → shutdown → process pipeline.
// No pooled/long-lived helper. Only built-in profile IDs are accepted
// (resolve_builtin_profile rejects filesystem paths).
// ---------------------------------------------------------------------------
fn handle_check_node(
    params: Value,
    profile_cache: &Arc<Mutex<HashMap<String, crate::profile::Profile>>>,
) -> Value {
    // --- 1. Parse params ---
    let sources = match params.get("sources").and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<_>>(),
        None => {
            return json!({
                "success": false,
                "error": "Missing 'sources' parameter (expected array of glob strings)"
            });
        }
    };
    if sources.is_empty() {
        return json!({
            "success": false,
            "error": "Empty 'sources' array; at least one source glob is required"
        });
    }

    let profiles = match params.get("profiles").and_then(|v| v.as_array()) {
        Some(arr) => arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>(),
        None => {
            return json!({
                "success": false,
                "error": "Missing 'profiles' parameter (expected array of built-in profile IDs)"
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

    // --- 2. Load profiles (cached) ---
    let mut loaded_profiles = Vec::new();
    {
        let mut cache_guard = profile_cache.lock().unwrap_or_else(|e| e.into_inner());
        for &profile_id in &profiles {
            let profile = if let Some(cached) = cache_guard.get(profile_id) {
                cached.clone()
            } else {
                let bytes = match crate::cli::resolve_builtin_profile(profile_id) {
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
    loaded_profiles.sort_by(|a, b| a.name.cmp(&b.name));
    loaded_profiles.dedup_by_key(|p| p.name.clone());

    let profile_rulesets: Vec<(&crate::profile::Profile, RuleSet)> = loaded_profiles
        .iter()
        .map(|p| (p, RuleSet::from_profile(p)))
        .collect();
    let profile_names: Vec<String> = loaded_profiles.iter().map(|p| p.name.clone()).collect();

    let start = Instant::now();

    // --- 3. Spawn Node helper ---
    // Per-request spawn keeps a hung helper isolated to one request; pooling is a future optimization if a hot caller appears.
    let mut helper = match crate::node::NodeHelper::spawn(None) {
        Ok(h) => h,
        Err(e) => {
            return json!({
                "success": false,
                "error": format!("Failed to spawn Node helper: {e}")
            });
        }
    };

    // --- 4. Discover schemas from each source glob ---
    let mut discovered_models: Vec<crate::ingest::DiscoveredModel> = Vec::new();
    let mut discovery_errors: Vec<String> = Vec::new();
    let mut discovery_warnings: Vec<String> = Vec::new();

    for source in &sources {
        match helper.discover(source) {
            Ok(resp) => {
                for model in resp.models {
                    discovered_models.push(model);
                }
                for warning in &resp.warnings {
                    // Log to stderr (doesn't corrupt the JSON-RPC stream) and collect for the response.
                    eprintln!(
                        "[checkNode] warning: discovery warning for '{}' in source '{}': {}",
                        warning.model, source, warning.message
                    );
                    discovery_warnings.push(format!(
                        "source '{}', model '{}': {}",
                        source, warning.model, warning.message
                    ));
                }
            }
            Err(e) => {
                discovery_errors.push(format!("discovery failed for source '{}': {}", source, e));
            }
        }
    }

    // Unconditionally shut down the helper before any failure handling.
    helper.shutdown();

    if discovered_models.is_empty() && !discovery_errors.is_empty() {
        return json!({
            "success": false,
            "error": discovery_errors.join("; ")
        });
    }

    if discovered_models.is_empty() {
        // No models found, no failures — return clean empty result.
        let output_text = render_output(format, &[], 0, 0, &profile_names, Some(0));
        let mut response = json!({
            "success": true,
            "output": output_text,
            "total_errors": 0,
            "total_warnings": 0,
        });
        if !discovery_warnings.is_empty() {
            response["discovery_warnings"] = json!(discovery_warnings);
        }
        return response;
    }

    // --- 5. Run the normalize → check → aggregate pipeline ---
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
    let results_with_spans = attach_source_spans(results, &discovered_models);
    let (all_diagnostics, total_errors, total_warnings, fatal_errors) =
        aggregate_results(results_with_spans);

    let duration_ms = Some(start.elapsed().as_millis() as u64);

    if fatal_errors > 0 || (!discovery_errors.is_empty() && discovered_models.is_empty()) {
        return json!({
            "success": false,
            "error": format!("{} schema(s) failed normalization/checking", fatal_errors)
        });
    }

    let output_text = render_output(
        format,
        &all_diagnostics,
        total_errors,
        total_warnings,
        &profile_names,
        duration_ms,
    );

    // Build the success response, adding discovery_errors / discovery_warnings only when non-empty.
    // This preserves the existing response shape for callers that see no partial failures.
    let mut response = json!({
        "success": true,
        "output": output_text,
        "total_errors": total_errors,
        "total_warnings": total_warnings,
    });
    if !discovery_errors.is_empty() {
        response["discovery_errors"] = json!(discovery_errors);
    }
    if !discovery_warnings.is_empty() {
        response["discovery_warnings"] = json!(discovery_warnings);
    }
    response
}

// ---------------------------------------------------------------------------
// checkPython — JSON contract
//
// Request params:
//   {
//     "packages": ["my_package", ...]   // required; Python package names to discover Pydantic models from
//     "profiles": ["openai.so.2026-04-30", ...]  // required; built-in profile IDs only
//     "format":   "json" | "human" | "sarif" | "gha" | "junit"  // optional, default "json"
//   }
//
// Success response result (some packages succeeded):
//   { "success": true, "output": "<rendered text>",
//     "total_errors": N, "total_warnings": N,
//     "discovery_errors": ["..."]   // omitted when empty; present when SOME packages failed
//     "discovery_warnings": ["..."] // omitted when empty; non-fatal warnings from sidecar
//   }
//
// Failure response result (ALL packages failed or no models found):
//   { "success": false, "error": "<message>" }
//
// Lifecycle: per-request spawn → discover-loop → shutdown → process pipeline.
// No pooled/long-lived helper. Only built-in profile IDs are accepted
// (resolve_builtin_profile rejects filesystem paths).
// If the Python interpreter or pydantic module is not available the helper
// spawn or discovery will fail and a structured {"success":false,"error":"..."}
// is returned — no panic.
// ---------------------------------------------------------------------------
fn handle_check_python(
    params: Value,
    profile_cache: &Arc<Mutex<HashMap<String, crate::profile::Profile>>>,
) -> Value {
    // --- 1. Parse params ---
    let packages = match params.get("packages").and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<_>>(),
        None => {
            return json!({
                "success": false,
                "error": "Missing 'packages' parameter (expected array of Python package names)"
            });
        }
    };
    if packages.is_empty() {
        return json!({
            "success": false,
            "error": "Empty 'packages' array; at least one package name is required"
        });
    }

    let profiles = match params.get("profiles").and_then(|v| v.as_array()) {
        Some(arr) => arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>(),
        None => {
            return json!({
                "success": false,
                "error": "Missing 'profiles' parameter (expected array of built-in profile IDs)"
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

    // --- 2. Load profiles (cached) ---
    let mut loaded_profiles = Vec::new();
    {
        let mut cache_guard = profile_cache.lock().unwrap_or_else(|e| e.into_inner());
        for &profile_id in &profiles {
            let profile = if let Some(cached) = cache_guard.get(profile_id) {
                cached.clone()
            } else {
                let bytes = match crate::cli::resolve_builtin_profile(profile_id) {
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
    loaded_profiles.sort_by(|a, b| a.name.cmp(&b.name));
    loaded_profiles.dedup_by_key(|p| p.name.clone());

    let profile_rulesets: Vec<(&crate::profile::Profile, RuleSet)> = loaded_profiles
        .iter()
        .map(|p| (p, RuleSet::from_profile(p)))
        .collect();
    let profile_names: Vec<String> = loaded_profiles.iter().map(|p| p.name.clone()).collect();

    let start = Instant::now();

    // --- 3. Spawn Python helper ---
    // Per-request spawn keeps a hung helper isolated to one request; pooling is a future optimization if a hot caller appears.
    let mut helper = match crate::python::PythonHelper::spawn(None) {
        Ok(h) => h,
        Err(e) => {
            return json!({
                "success": false,
                "error": format!("Failed to spawn Python helper: {e}")
            });
        }
    };

    // --- 4. Discover models from each package ---
    let mut discovered_models: Vec<crate::ingest::DiscoveredModel> = Vec::new();
    let mut discovery_errors: Vec<String> = Vec::new();
    let mut discovery_warnings: Vec<String> = Vec::new();

    for package in &packages {
        match helper.discover(package) {
            Ok(resp) => {
                for model in resp.models {
                    discovered_models.push(model);
                }
                for warning in &resp.warnings {
                    discovery_warnings.push(format!(
                        "package '{}', model '{}': {}",
                        package, warning.model, warning.message
                    ));
                }
            }
            Err(e) => {
                discovery_errors.push(format!("discovery failed for package '{}': {}", package, e));
            }
        }
    }

    // Unconditionally shut down the helper before any failure handling.
    helper.shutdown();

    if discovered_models.is_empty() && !discovery_errors.is_empty() {
        return json!({
            "success": false,
            "error": discovery_errors.join("; ")
        });
    }

    if discovered_models.is_empty() {
        // No models found, no failures — return clean empty result.
        let output_text = render_output(format, &[], 0, 0, &profile_names, Some(0));
        let mut response = json!({
            "success": true,
            "output": output_text,
            "total_errors": 0,
            "total_warnings": 0,
        });
        if !discovery_warnings.is_empty() {
            response["discovery_warnings"] = json!(discovery_warnings);
        }
        return response;
    }

    // --- 5. Run the normalize → check → aggregate pipeline ---
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
    let results_with_spans = attach_source_spans(results, &discovered_models);
    let (all_diagnostics, total_errors, total_warnings, fatal_errors) =
        aggregate_results(results_with_spans);

    let duration_ms = Some(start.elapsed().as_millis() as u64);

    if fatal_errors > 0 && discovered_models.is_empty() {
        return json!({
            "success": false,
            "error": format!("{} schema(s) failed normalization/checking", fatal_errors)
        });
    }

    let output_text = render_output(
        format,
        &all_diagnostics,
        total_errors,
        total_warnings,
        &profile_names,
        duration_ms,
    );

    // Build the success response, adding discovery_errors / discovery_warnings only when non-empty.
    // This preserves the existing response shape for callers that see no partial failures.
    let mut response = json!({
        "success": true,
        "output": output_text,
        "total_errors": total_errors,
        "total_warnings": total_warnings,
    });
    if !discovery_errors.is_empty() {
        response["discovery_errors"] = json!(discovery_errors);
    }
    if !discovery_warnings.is_empty() {
        response["discovery_warnings"] = json!(discovery_warnings);
    }
    response
}
