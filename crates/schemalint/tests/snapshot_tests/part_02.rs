#[test]
fn test_emit_human_edge_cases() {
    let path = std::path::PathBuf::from("special.json");
    let diags = vec![
        diag(
            "TST-SPECIAL",
            DiagnosticSeverity::Error,
            "message with special chars: \n tab\t backslash\\ and unicode 你好",
            "/special",
            src_span("special.json", Some(10), Some(5)),
            "test",
            Some("hint: with : colons and % percent"),
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
        diag(
            "TST-LONG",
            DiagnosticSeverity::Error,
            &"a".repeat(300),
            "/long",
            None,
            "test",
            None,
        ),
    ];
    let output = emit_human_to_string(&[(path, diags)], 2, 1, None);
    assert_snapshot_stable!(output);
}

#[test]
fn test_emit_human_source_variants() {
    let path = std::path::PathBuf::from("variants.json");
    let diags = vec![
        diag(
            "TST-FULL",
            DiagnosticSeverity::Error,
            "full source span with line and col",
            "/full",
            src_span("variants.json", Some(10), Some(5)),
            "test",
            None,
        ),
        diag(
            "TST-LINEONLY",
            DiagnosticSeverity::Error,
            "line only, no column",
            "/lineonly",
            src_span("variants.json", Some(20), None),
            "test",
            None,
        ),
        diag(
            "TST-FILEONLY",
            DiagnosticSeverity::Warning,
            "file only, no line or col",
            "/fileonly",
            src_span("variants.json", None, None),
            "test",
            None,
        ),
        diag(
            "TST-NOSOURCE",
            DiagnosticSeverity::Warning,
            "no source span at all — falls back to path",
            "/nosource",
            None,
            "test",
            None,
        ),
    ];
    let output = emit_human_to_string(&[(path, diags)], 2, 2, Some(0));
    assert_snapshot_stable!(output);
}

// ---------------------------------------------------------------------------
// emit_json_to_string — structural validation (JSON already at ~100%)
// ---------------------------------------------------------------------------

#[test]
fn test_emit_json_multi_diag() {
    let path = std::path::PathBuf::from("schema.json");
    let diags = vec![
        diag(
            "OAI-K-allOf",
            DiagnosticSeverity::Error,
            "keyword not supported",
            "/allOf",
            src_span("schema.json", Some(42), Some(8)),
            "openai.so",
            Some("hint text"),
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
    let output = emit_json_to_string(&[(path, diags)], 1, 1, &["openai.so".into()], Some(100));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["schema_version"], "1.0");
    assert_eq!(parsed["tool"]["name"], "schemalint");
    assert!(parsed["tool"]["version"].as_str().unwrap().len() > 0);
    assert_eq!(parsed["profiles"][0], "openai.so");
    assert_eq!(parsed["summary"]["total_issues"], 2);
    assert_eq!(parsed["summary"]["errors"], 1);
    assert_eq!(parsed["summary"]["warnings"], 1);
    assert_eq!(parsed["summary"]["schemas_checked"], 1);
    assert_eq!(parsed["summary"]["duration_ms"], 100);
    let d0 = &parsed["diagnostics"][0];
    assert_eq!(d0["code"], "OAI-K-allOf");
    assert_eq!(d0["severity"], "error");
    assert_eq!(d0["message"], "keyword not supported");
    assert_eq!(d0["pointer"], "/allOf");
    assert_eq!(d0["source"]["line"], 42);
    assert_eq!(d0["source"]["col"], 8);
    assert_eq!(d0["profile"], "openai.so");
    assert_eq!(d0["hint"], "hint text");
    assert_eq!(d0["seeUrl"], "https://1nder-labs.github.io/schemalint/rules/keyword/allOf");
    let d1 = &parsed["diagnostics"][1];
    assert_eq!(d1["severity"], "warning");
    assert!(d1.get("hint").is_none() || d1["hint"].is_null());
}

#[test]
fn test_emit_json_empty() {
    let output = emit_json_to_string(&[], 0, 0, &["openai.so".into()], Some(0));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["summary"]["total_issues"], 0);
    assert_eq!(parsed["summary"]["errors"], 0);
    assert_eq!(parsed["summary"]["warnings"], 0);
    assert_eq!(parsed["summary"]["schemas_checked"], 0);
    assert!(parsed["diagnostics"].as_array().unwrap().is_empty());
}

#[test]
fn test_emit_json_edge_cases() {
    let path = std::path::PathBuf::from("special.json");
    let diags = vec![
        diag(
            "TST-SPECIAL",
            DiagnosticSeverity::Error,
            "message with <xml> & \"quotes\" and \n newline",
            "/special",
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
    let output = emit_json_to_string(&[(path, diags)], 1, 1, &["test".into()], None);
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(
        parsed["diagnostics"][0]["message"],
        "message with <xml> & \"quotes\" and \n newline"
    );
    assert_eq!(parsed["diagnostics"][1]["message"], "");
    assert!(parsed["summary"]["duration_ms"].is_null());
}

#[test]
fn test_emit_json_source_variants() {
    let path = std::path::PathBuf::from("variants.json");
    let diags = vec![
        diag(
            "TST-FULL",
            DiagnosticSeverity::Error,
            "full span",
            "/full",
            src_span("variants.json", Some(10), Some(5)),
            "test",
            None,
        ),
        diag(
            "TST-NOSOURCE",
            DiagnosticSeverity::Warning,
            "no source span",
            "/nosource",
            None,
            "test",
            None,
        ),
    ];
    let output = emit_json_to_string(&[(path, diags)], 1, 1, &["test".into()], None);
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    // Full source span
    assert_eq!(parsed["diagnostics"][0]["source"]["file"], "variants.json");
    assert_eq!(parsed["diagnostics"][0]["source"]["line"], 10);
    assert_eq!(parsed["diagnostics"][0]["source"]["col"], 5);
    // No source span — should fall back to path
    assert_eq!(parsed["diagnostics"][1]["source"]["file"], "variants.json");
    assert!(parsed["diagnostics"][1]["source"]["line"].is_null());
    assert!(parsed["diagnostics"][1]["source"]["col"].is_null());
}

// ---------------------------------------------------------------------------
// emit_sarif_to_string — multi-diagnostic, empty, edge-cases, source variants
// ---------------------------------------------------------------------------

#[test]
fn test_emit_sarif_multi_diag() {
    let path = std::path::PathBuf::from("schema.json");
    let diags = vec![
        diag(
            "OAI-K-allOf",
            DiagnosticSeverity::Error,
            "keyword 'allOf' not supported",
            "/allOf",
            src_span("schema.json", Some(42), Some(8)),
            "openai.so",
            None,
        ),
        diag(
            "OAI-K-anyOf",
            DiagnosticSeverity::Warning,
            "keyword 'anyOf' is a warning",
            "/anyOf",
            None,
            "openai.so",
            None,
        ),
    ];
    let output = emit_sarif_to_string(&[(path, diags)]);
    assert_snapshot_stable!(output);
}

#[test]
fn test_emit_sarif_empty() {
    let output = emit_sarif_to_string(&[]);
    assert_snapshot_stable!(output);
}

#[test]
fn test_emit_sarif_edge_cases() {
    let path = std::path::PathBuf::from("special.json");
    let diags = vec![
        diag(
            "TST-XML",
            DiagnosticSeverity::Error,
            "contains <xml> & \"quotes\" 'single'",
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
    let output = emit_sarif_to_string(&[(path, diags)]);
    assert_snapshot_stable!(output);
}

#[test]
fn test_emit_sarif_source_variants() {
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
    let output = emit_sarif_to_string(&[(path, diags)]);
    assert_snapshot_stable!(output);
}

// ---------------------------------------------------------------------------
// emit_junit_to_string — multi-diagnostic, empty, edge-cases, source variants
// ---------------------------------------------------------------------------
