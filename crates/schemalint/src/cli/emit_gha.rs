use crate::rules::registry::DiagnosticSeverity;
use crate::rules::Diagnostic;

/// Percent-encode characters that would break GitHub Actions workflow commands.
///
/// GHA uses `::` as delimiters and interprets `%` as an escape prefix.
/// We encode `%`, `\r`, `\n`, and `:` to prevent injection.
fn encode_gha_value(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
        .replace(':', "%3A")
        .replace(',', "%2C")
}

/// Emit diagnostics as GitHub Actions workflow commands.
pub fn emit_gha_to_string(
    diagnostics: &[(std::path::PathBuf, Vec<Diagnostic>)],
    _total_errors: usize,
    _total_warnings: usize,
    _profile_names: &[String],
    _duration_ms: Option<u64>,
) -> String {
    let mut out = String::new();
    for (path, diags) in diagnostics {
        for d in diags {
            let cmd = match d.severity {
                DiagnosticSeverity::Error => "error",
                DiagnosticSeverity::Warning => "warning",
            };
            let file = match &d.source {
                Some(span) => encode_gha_value(&span.file),
                None => encode_gha_value(&path.display().to_string()),
            };
            let code = encode_gha_value(&d.code);
            let message = encode_gha_value(&format!("{} [profile: {}]", d.message, d.profile));

            let mut params = format!("file={file},title={code}");
            if let Some(span) = &d.source {
                if let Some(line) = span.line {
                    params.push_str(&format!(",line={line}"));
                }
                if let Some(col) = span.col {
                    params.push_str(&format!(",col={col}"));
                }
            }
            out.push_str(&format!("::{cmd} {params}::{message}\n"));
        }
    }
    out
}
