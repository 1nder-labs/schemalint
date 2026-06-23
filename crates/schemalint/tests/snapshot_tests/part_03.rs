#[test]
fn test_emit_junit_multi_diag() {
    let path = std::path::PathBuf::from("schema.json");
    let diags = vec![
        diag(
            "OAI-K-allOf",
            DiagnosticSeverity::Error,
            "keyword not supported",
            "/allOf",
            src_span("schema.json", Some(42), Some(8)),
            "openai.so",
            None,
        ),
        diag(
            "OAI-K-anyOf",
            DiagnosticSeverity::Warning,
            "warning message",
            "/anyOf",
            None,
            "openai.so",
            None,
        ),
    ];
    let output = emit_junit_to_string(&[(path, diags)]);
    assert_snapshot_stable!(output);
}

#[test]
fn test_emit_junit_empty_global() {
    let output = emit_junit_to_string(&[]);
    assert_snapshot_stable!(output);
}

#[test]
fn test_emit_junit_empty_per_file() {
    let path = std::path::PathBuf::from("clean.json");
    let output = emit_junit_to_string(&[(path, vec![])]);
    assert_snapshot_stable!(output);
}

#[test]
fn test_emit_junit_edge_cases() {
    let path = std::path::PathBuf::from("special.json");
    let diags = vec![
        diag(
            "TST-XML",
            DiagnosticSeverity::Error,
            "contains <xml> & \"quotes\" 'apos'",
            "/xml",
            src_span("special.json", Some(1), Some(1)),
            "test",
            None,
        ),
        diag(
            "TST-EMPTY",
            DiagnosticSeverity::Warning,
            "",
            "",
            None,
            "test",
            None,
        ),
    ];
    let output = emit_junit_to_string(&[(path, diags)]);
    assert_snapshot_stable!(output);
}

#[test]
fn test_emit_junit_source_variants() {
    let path = std::path::PathBuf::from("variants.json");
    let diags = vec![
        diag(
            "TST-FULL",
            DiagnosticSeverity::Error,
            "full span with line and col",
            "/full",
            src_span("variants.json", Some(10), Some(5)),
            "test",
            None,
        ),
        diag(
            "TST-LINEONLY",
            DiagnosticSeverity::Error,
            "line only, no col",
            "/lineonly",
            src_span("variants.json", Some(20), None),
            "test",
            None,
        ),
        diag(
            "TST-NOSOURCE",
            DiagnosticSeverity::Warning,
            "no source span at all",
            "/nosource",
            None,
            "test",
            None,
        ),
    ];
    let output = emit_junit_to_string(&[(path, diags)]);
    assert_snapshot_stable!(output);
}

// ---------------------------------------------------------------------------
// emit_sarif_to_string — rule_id ordering
// ---------------------------------------------------------------------------

#[test]
fn test_emit_sarif_rule_ids_sorted() {
    let path = std::path::PathBuf::from("schema.json");
    let diags = vec![
        diag(
            "Z-K-last",
            DiagnosticSeverity::Error,
            "should appear last when sorted",
            "/z",
            None,
            "test",
            None,
        ),
        diag(
            "A-K-first",
            DiagnosticSeverity::Warning,
            "should appear first when sorted",
            "/a",
            None,
            "test",
            None,
        ),
    ];
    let output = emit_sarif_to_string(&[(path, diags)]);
    // Verify alphabetical ordering of ruleIds in the driver rules.
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    let rules = parsed["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .unwrap();
    assert_eq!(rules[0]["id"], "A-K-first");
    assert_eq!(rules[1]["id"], "Z-K-last");
    assert_snapshot_stable!(output);
}

// ---------------------------------------------------------------------------
// emit_gha_to_string — multi-diagnostic, empty, edge-cases, source variants
// ---------------------------------------------------------------------------

#[test]
fn test_emit_gha_multi_diag() {
    let path = std::path::PathBuf::from("schema.json");
    let diags = vec![
        diag(
            "OAI-K-allOf",
            DiagnosticSeverity::Error,
            "keyword not supported",
            "/allOf",
            src_span("schema.json", Some(42), Some(8)),
            "openai.so",
            None,
        ),
        diag(
            "OAI-K-anyOf",
            DiagnosticSeverity::Warning,
            "warning message",
            "/anyOf",
            None,
            "openai.so",
            None,
        ),
    ];
    let output = emit_gha_to_string(&[(path, diags)]);
    assert_snapshot_stable!(output);
}

#[test]
fn test_emit_gha_empty() {
    let output = emit_gha_to_string(&[]);
    assert_snapshot_stable!(output);
}

#[test]
fn test_emit_gha_edge_cases() {
    let path = std::path::PathBuf::from("special.json");
    let diags = vec![
        diag(
            "TST-PERCENT",
            DiagnosticSeverity::Error,
            "message with % percent sign",
            "/percent",
            src_span("special.json", Some(1), Some(1)),
            "test",
            None,
        ),
        diag(
            "TST-COLONS",
            DiagnosticSeverity::Error,
            "message with : colons :: double",
            "/colons",
            src_span("special.json", Some(2), Some(1)),
            "test",
            None,
        ),
        diag(
            "TST-NEWLINES",
            DiagnosticSeverity::Warning,
            "message with\nnewline and\r\nwindows",
            "/newlines",
            None,
            "test",
            None,
        ),
        diag(
            "TST-COMMAS",
            DiagnosticSeverity::Error,
            "message, with, commas",
            "/commas",
            None,
            "test",
            None,
        ),
    ];
    let output = emit_gha_to_string(&[(path, diags)]);
    assert_snapshot_stable!(output);
}

#[test]
fn test_emit_gha_source_variants() {
    let path = std::path::PathBuf::from("variants.json");
    let diags = vec![
        diag(
            "TST-FULL",
            DiagnosticSeverity::Error,
            "full span with line and col",
            "/full",
            src_span("variants.json", Some(10), Some(5)),
            "test",
            None,
        ),
        diag(
            "TST-LINEONLY",
            DiagnosticSeverity::Error,
            "line only, no col",
            "/lineonly",
            src_span("variants.json", Some(20), None),
            "test",
            None,
        ),
        diag(
            "TST-NOSOURCE",
            DiagnosticSeverity::Warning,
            "no source span at all",
            "/nosource",
            None,
            "test",
            None,
        ),
    ];
    let output = emit_gha_to_string(&[(path, diags)]);
    assert_snapshot_stable!(output);
}
