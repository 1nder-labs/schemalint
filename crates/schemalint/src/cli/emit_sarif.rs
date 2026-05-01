use serde_json::json;

use crate::rules::registry::DiagnosticSeverity;
use crate::rules::Diagnostic;

/// Emit diagnostics as SARIF v2.1.0 JSON.
pub fn emit_sarif_to_string(
    diagnostics: &[(std::path::PathBuf, Vec<Diagnostic>)],
    _total_errors: usize,
    _total_warnings: usize,
    _profile_names: &[String],
    _duration_ms: Option<u64>,
) -> String {
    let mut results = Vec::new();

    for (path, diags) in diagnostics {
        for d in diags {
            let mut result = json!({
                "ruleId": d.code,
                "message": {
                    "text": d.message
                },
                "locations": [
                    {
                        "physicalLocation": {
                            "artifactLocation": {
                                "uri": path.display().to_string()
                            }
                        }
                    }
                ]
            });

            // Source spans not yet available (Phase 3+), so we omit region.
            let _ = &result["locations"][0]["physicalLocation"];

            let level = match d.severity {
                DiagnosticSeverity::Error => "error",
                DiagnosticSeverity::Warning => "warning",
            };
            if let Some(obj) = result.as_object_mut() {
                obj.insert("level".to_string(), json!(level));
            }

            results.push(result);
        }
    }

    let output = json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "schemalint",
                        "informationUri": "https://schemalint.dev"
                    }
                },
                "results": results
            }
        ]
    });

    serde_json::to_string_pretty(&output).unwrap() + "\n"
}
