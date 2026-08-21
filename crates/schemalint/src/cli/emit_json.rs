use serde::Serialize;

use crate::cli::docs_url::rule_url;
use crate::cli::report::{CheckReport, CoverageCounts};
use crate::profile::ProviderEvidence;
use crate::rules::registry::{DiagnosticSeverity, SourceSpan};
use crate::rules::Diagnostic;

#[derive(Serialize)]
struct JsonOutput {
    schema_version: String,
    tool: ToolMeta,
    profiles: Vec<String>,
    summary: Summary,
    diagnostics: Vec<JsonDiagnostic>,
    report: serde_json::Value,
}

#[derive(Serialize)]
struct ToolMeta {
    name: String,
    version: String,
}

#[derive(Serialize)]
struct Summary {
    total_issues: usize,
    errors: usize,
    warnings: usize,
    schemas_checked: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<u64>,
}

#[derive(Serialize)]
struct JsonDiagnostic {
    code: String,
    severity: String,
    message: String,
    pointer: String,
    source: Option<SourceSpan>,
    profile: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<String>,
    #[serde(rename = "seeUrl")]
    see_url: String,
    #[serde(rename = "providerEvidence", skip_serializing_if = "Option::is_none")]
    provider_evidence: Option<ProviderEvidence>,
}

/// Emit diagnostics as structured JSON.
pub fn emit_json_to_string(
    diagnostics: &[(std::path::PathBuf, Vec<Diagnostic>)],
    total_errors: usize,
    total_warnings: usize,
    profile_names: &[String],
    duration_ms: Option<u64>,
) -> String {
    let checked = diagnostics.len();
    let report = CheckReport {
        coverage: CoverageCounts {
            attempted: checked,
            discovered: checked,
            checked,
            ..CoverageCounts::default()
        },
        failures: vec![],
        warnings: vec![],
        targets: vec![],
        diagnostics: diagnostics.to_vec(),
        total_errors,
        total_warnings,
        profiles: profile_names.to_vec(),
        duration_ms,
    };
    emit_report_to_string(&report)
}

pub(crate) fn emit_report_to_string(report: &CheckReport) -> String {
    let mut json_diags = Vec::new();
    for (path, diags) in &report.diagnostics {
        for d in diags {
            let source = d.source.clone().map(|mut s| {
                s.file = s.file.replace('\\', "/");
                s
            });
            json_diags.push(JsonDiagnostic {
                code: d.code.clone(),
                severity: match d.severity {
                    DiagnosticSeverity::Error => "error".to_string(),
                    DiagnosticSeverity::Warning => "warning".to_string(),
                },
                message: d.message.clone(),
                pointer: d.pointer.clone(),
                source: source.or_else(|| {
                    Some(SourceSpan {
                        file: path.display().to_string().replace('\\', "/"),
                        line: None,
                        col: None,
                    })
                }),
                profile: d.profile.clone(),
                hint: d.hint.clone(),
                see_url: rule_url(&d.code),
                provider_evidence: d.provider_evidence.clone(),
            });
        }
    }

    let output = JsonOutput {
        schema_version: "1.2".to_string(),
        tool: ToolMeta {
            name: "schemalint".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        profiles: report.profiles.clone(),
        summary: Summary {
            total_issues: report.total_errors + report.total_warnings,
            errors: report.total_errors,
            warnings: report.total_warnings,
            schemas_checked: report.coverage.checked,
            duration_ms: report.duration_ms,
        },
        diagnostics: json_diags,
        report: report.report_json(),
    };

    serde_json::to_string_pretty(&output).unwrap() + "\n"
}
