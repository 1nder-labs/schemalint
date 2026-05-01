use serde_json::json;
use std::collections::HashSet;

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
    let mut rule_ids = HashSet::new();

    for (path, diags) in diagnostics {
        for d in diags {
            rule_ids.insert(d.code.clone());

            let level = match d.severity {
                DiagnosticSeverity::Error => "error",
                DiagnosticSeverity::Warning => "warning",
            };

            let result = json!({
                "ruleId": d.code,
                "level": level,
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

            results.push(result);
        }
    }

    let rules: Vec<_> = rule_ids
        .into_iter()
        .map(|id| {
            json!({
                "id": id,
                "helpUri": format!("https://schemalint.dev/rules/{}", id)
            })
        })
        .collect();

    let output = json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "schemalint",
                        "informationUri": "https://schemalint.dev",
                        "rules": rules
                    }
                },
                "results": results
            }
        ]
    });

    serde_json::to_string_pretty(&output).unwrap() + "\n"
}
