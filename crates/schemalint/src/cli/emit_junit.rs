use crate::rules::registry::DiagnosticSeverity;
use crate::rules::Diagnostic;

/// Emit diagnostics as JUnit XML.
pub fn emit_junit_to_string(
    diagnostics: &[(std::path::PathBuf, Vec<Diagnostic>)],
    _total_errors: usize,
    _total_warnings: usize,
    _profile_names: &[String],
    _duration_ms: Option<u64>,
) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<testsuites>\n");

    for (path, diags) in diagnostics {
        let suite_file = diags
            .first()
            .and_then(|d| d.source.as_ref())
            .map(|s| s.file.clone())
            .unwrap_or_else(|| path.display().to_string());
        let tests = if diags.is_empty() { 1 } else { diags.len() };
        let failures = diags
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .count();
        let skipped = diags
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Warning)
            .count();

        out.push_str(&format!(
            "  <testsuite name=\"{}\" tests=\"{}\" failures=\"{}\" skipped=\"{}\" errors=\"0\" time=\"0\">\n",
            escape_xml(&suite_file),
            tests,
            failures,
            skipped
        ));

        if diags.is_empty() {
            out.push_str(&format!(
                "    <testcase name=\"{}\" classname=\"schemalint\" time=\"0\"/>\n",
                escape_xml(&suite_file)
            ));
        } else {
            for d in diags {
                let test_name = format!("{} - {}", d.code, d.message);
                let source_attrs = if let Some(span) = &d.source {
                    let file_attr = format!(" file=\"{}\"", escape_xml(&span.file));
                    let line_attr = span
                        .line
                        .map_or(String::new(), |l| format!(" line=\"{}\"", l));
                    format!("{}{}", file_attr, line_attr)
                } else {
                    String::new()
                };
                out.push_str(&format!(
                    "    <testcase name=\"{}\" classname=\"schemalint\" time=\"0\"{}>\n",
                    escape_xml(&test_name),
                    source_attrs
                ));
                match d.severity {
                    DiagnosticSeverity::Error => {
                        out.push_str(&format!(
                            "      <failure type=\"error\" message=\"{}\">{}</failure>\n",
                            escape_xml(&d.message),
                            escape_xml(&d.message)
                        ));
                    }
                    DiagnosticSeverity::Warning => {
                        out.push_str(&format!(
                            "      <skipped message=\"{}\">{}</skipped>\n",
                            escape_xml(&d.message),
                            escape_xml(&d.message)
                        ));
                    }
                }
                out.push_str("    </testcase>\n");
            }
        }

        out.push_str("  </testsuite>\n");
    }

    out.push_str("</testsuites>\n");
    out
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
