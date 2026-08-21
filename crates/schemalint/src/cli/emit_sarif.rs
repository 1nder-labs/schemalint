use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::cli::docs_url::{rule_url, DOCS_BASE_URL};
use crate::rules::registry::DiagnosticSeverity;
use crate::rules::Diagnostic;

/// Emit diagnostics as SARIF v2.1.0 JSON.
pub fn emit_sarif_to_string(diagnostics: &[(std::path::PathBuf, Vec<Diagnostic>)]) -> String {
    let mut results = Vec::new();
    let mut rule_ids: BTreeMap<String, Option<crate::profile::ProviderEvidence>> = BTreeMap::new();

    for (path, diags) in diagnostics {
        for d in diags {
            rule_ids
                .entry(d.code.clone())
                .and_modify(|evidence| {
                    if evidence.is_none() {
                        evidence.clone_from(&d.provider_evidence);
                    }
                })
                .or_insert_with(|| d.provider_evidence.clone());

            let level = match d.severity {
                DiagnosticSeverity::Error => "error",
                DiagnosticSeverity::Warning => "warning",
            };

            let file = d
                .source
                .as_ref()
                .map(|s| s.file.clone())
                .unwrap_or_else(|| path.display().to_string());

            let mut physical_location = json!({
                "artifactLocation": {
                    "uri": file
                }
            });

            if let Some(source) = &d.source {
                if source.line.is_some() || source.col.is_some() {
                    let mut region = serde_json::Map::new();
                    if let Some(line) = source.line {
                        region.insert("startLine".to_string(), json!(line));
                    }
                    if let Some(col) = source.col {
                        region.insert("startColumn".to_string(), json!(col));
                    }
                    physical_location["region"] = Value::Object(region);
                }
            }

            let result = json!({
                "ruleId": d.code,
                "level": level,
                "message": {
                    "text": d.message
                },
                "locations": [
                    {
                        "physicalLocation": physical_location
                    }
                ]
            });

            results.push(result);
        }
    }

    let rules: Vec<_> = rule_ids
        .into_iter()
        .map(|(id, evidence)| {
            let mut descriptor = json!({
                "id": id,
                "helpUri": rule_url(&id)
            });
            if let Some(evidence) = evidence {
                descriptor["properties"] = json!({ "providerEvidence": evidence });
            }
            descriptor
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
                        "informationUri": DOCS_BASE_URL,
                        "rules": rules
                    }
                },
                "results": results
            }
        ]
    });

    serde_json::to_string_pretty(&output).unwrap() + "\n"
}
