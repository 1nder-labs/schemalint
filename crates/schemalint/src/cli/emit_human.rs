use crate::rules::Diagnostic;
use crate::rules::registry::DiagnosticSeverity;

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
pub fn emit_human(
    diagnostics: &[(std::path::PathBuf, Vec<Diagnostic>)],
    total_errors: usize,
    total_warnings: usize,
) {
    for (path, diags) in diagnostics {
        for d in diags {
            let severity_label = match d.severity {
                DiagnosticSeverity::Error => "error",
                DiagnosticSeverity::Warning => "warning",
            };
            println!("{}[{}]: {}", severity_label, d.code, d.message);
            println!("   --> {}", path.display());
            println!("     |");
            println!("     = profile: {}", d.profile);
            println!("     = schema path: {}", d.pointer);
            if let Some(hint) = &d.hint {
                println!("     = hint: {}", hint);
            }
            println!("     = see: https://schemalint.dev/rules/{}", d.code);
            println!();
        }
    }

    let total = total_errors + total_warnings;
    let schema_count = diagnostics.len();
    println!(
        "{} issue{} found ({} error{}, {} warning{}) across {} schema{}",
        total,
        if total == 1 { "" } else { "s" },
        total_errors,
        if total_errors == 1 { "" } else { "s" },
        total_warnings,
        if total_warnings == 1 { "" } else { "s" },
        schema_count,
        if schema_count == 1 { "" } else { "s" }
    );
}
