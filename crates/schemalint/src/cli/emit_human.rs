use crate::rules::registry::DiagnosticSeverity;
use crate::rules::Diagnostic;

/// Emit diagnostics in rustc-style human-readable format.
///
/// For raw `.json` files, line:col and source snippets are omitted because
/// `serde_json` does not provide byte offsets in Phase 1.
///
/// Format per diagnostic:
/// ```text
/// error[OAI-K-allOf]: keyword 'allOf' is not supported by OpenAI Structured Outputs
///    --> schema.json
///      |
///      = profile: openai.so.2026-04-30
///      = schema path: /properties/items
///      = see: https://schemalint.dev/rules/OAI-K-allOf
/// ```
pub fn emit_human_to_string(
    diagnostics: &[(std::path::PathBuf, Vec<Diagnostic>)],
    total_errors: usize,
    total_warnings: usize,
    duration_ms: Option<u64>,
) -> String {
    let mut out = String::new();
    for (path, diags) in diagnostics {
        for d in diags {
            let severity_label = match d.severity {
                DiagnosticSeverity::Error => "error",
                DiagnosticSeverity::Warning => "warning",
            };
            out.push_str(&format!("{}[{}]: {}\n", severity_label, d.code, d.message));
            out.push_str(&format!("   --> {}\n", path.display()));
            out.push_str("     |\n");
            out.push_str(&format!("     = profile: {}\n", d.profile));
            out.push_str(&format!("     = schema path: {}\n", d.pointer));
            if let Some(hint) = &d.hint {
                out.push_str(&format!("     = hint: {}\n", hint));
            }
            out.push_str(&format!("     = see: https://schemalint.dev/rules/{}\n", d.code));
            out.push('\n');
        }
    }

    let total = total_errors + total_warnings;
    let schema_count = diagnostics.len();
    let duration_part = duration_ms.map_or(String::new(), |d| format!(" in {}ms", d));
    out.push_str(&format!(
        "{} issue{} found ({} error{}, {} warning{}) across {} schema{}{}\n",
        total,
        if total == 1 { "" } else { "s" },
        total_errors,
        if total_errors == 1 { "" } else { "s" },
        total_warnings,
        if total_warnings == 1 { "" } else { "s" },
        schema_count,
        if schema_count == 1 { "" } else { "s" },
        duration_part
    ));
    out
}
