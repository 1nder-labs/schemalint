use crate::rules::registry::DiagnosticSeverity;
use crate::rules::Diagnostic;

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
            let file = path.display();
            // GitHub Actions commands: ::error file=...,title=...::message
            out.push_str(&format!(
                "::{cmd} file={file},title={code}::{message} [profile: {profile}]\n",
                code = d.code,
                message = d.message,
                profile = d.profile
            ));
        }
    }
    out
}
