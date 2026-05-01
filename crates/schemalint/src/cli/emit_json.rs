use serde::Serialize;

use crate::rules::registry::DiagnosticSeverity;
use crate::rules::Diagnostic;

#[derive(Serialize)]
struct JsonOutput {
    schema_version: String,
    tool: ToolMeta,
    profiles: Vec<String>,
    summary: Summary,
    diagnostics: Vec<JsonDiagnostic>,
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
}

#[derive(Serialize)]
struct JsonDiagnostic {
    code: String,
    severity: String,
    message: String,
    #[serde(rename = "schemaPath")]
    schema_path: String,
    #[serde(rename = "filePath")]
    file_path: String,
    profile: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<String>,
    #[serde(rename = "seeUrl")]
    see_url: String,
}

/// Emit diagnostics as structured JSON.
pub fn emit_json(
    diagnostics: &[(std::path::PathBuf, Vec<Diagnostic>)],
    total_errors: usize,
    total_warnings: usize,
    profile_names: &[String],
) {
    let mut json_diags = Vec::new();
    for (path, diags) in diagnostics {
        for d in diags {
            json_diags.push(JsonDiagnostic {
                code: d.code.clone(),
                severity: match d.severity {
                    DiagnosticSeverity::Error => "error".to_string(),
                    DiagnosticSeverity::Warning => "warning".to_string(),
                },
                message: d.message.clone(),
                schema_path: d.pointer.clone(),
                file_path: path.display().to_string(),
                profile: d.profile.clone(),
                hint: d.hint.clone(),
                see_url: format!("https://schemalint.dev/rules/{}", d.code),
            });
        }
    }

    let output = JsonOutput {
        schema_version: "1.0".to_string(),
        tool: ToolMeta {
            name: "schemalint".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        profiles: profile_names.to_vec(),
        summary: Summary {
            total_issues: total_errors + total_warnings,
            errors: total_errors,
            warnings: total_warnings,
            schemas_checked: diagnostics.len(),
        },
        diagnostics: json_diags,
    };

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
