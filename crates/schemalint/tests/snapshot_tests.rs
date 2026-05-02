use assert_cmd::Command;
use schemalint::cli::emit_gha::emit_gha_to_string;
use schemalint::cli::emit_human::emit_human_to_string;
use schemalint::cli::emit_json::emit_json_to_string;
use schemalint::cli::emit_junit::emit_junit_to_string;
use schemalint::cli::emit_sarif::emit_sarif_to_string;
use schemalint::rules::registry::{DiagnosticSeverity, SourceSpan};
use schemalint::rules::Diagnostic;
use std::fs;

fn minimal_profile() -> &'static str {
    r##"
name = "test"
version = "1.0"

[structural]
require_object_root = false
"##
}

fn profile_with_forbid_allof() -> &'static str {
    r##"
name = "test"
version = "1.0"
allOf = "forbid"

[structural]
require_object_root = false
"##
}

fn cmd() -> Command {
    Command::cargo_bin("schemalint").unwrap()
}

/// Replace temp directory paths and non-deterministic durations in output with stable placeholders.
fn normalize_temp_paths(output: &str, temp_dir: &std::path::Path) -> String {
    let mut out = output.replace(&temp_dir.to_string_lossy().to_string(), "[TEMP_DIR]");
    // Strip human footer duration: " in 0ms" -> " in [DURATION]ms"
    let re_human = regex::Regex::new(r" in \d+ms").unwrap();
    out = re_human.replace_all(&out, " in [DURATION]ms").to_string();
    // Strip JSON duration_ms
    let re_json = regex::Regex::new("duration_ms\": [0-9]+").unwrap();
    out = re_json
        .replace_all(&out, "duration_ms\": [DURATION]")
        .to_string();
    out
}

// ---------------------------------------------------------------------------
// Snapshot: human output
// ---------------------------------------------------------------------------

#[test]
fn snapshot_human_clean_schema() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    let schema = dir.path().join("schema.json");
    fs::write(&profile, minimal_profile()).unwrap();
    fs::write(&schema, r#"{"type": "string"}"#).unwrap();

    let output = cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg("--format")
        .arg("human")
        .arg(&schema)
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(normalize_temp_paths(&stdout, dir.path()));
}

#[test]
fn snapshot_human_forbidden_keyword() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    let schema = dir.path().join("schema.json");
    fs::write(&profile, profile_with_forbid_allof()).unwrap();
    fs::write(&schema, r#"{"allOf": [{"type": "string"}]}"#).unwrap();

    let output = cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg("--format")
        .arg("human")
        .arg(&schema)
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(normalize_temp_paths(&stdout, dir.path()));
}

#[test]
fn snapshot_human_warning_only() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    let schema = dir.path().join("schema.json");
    fs::write(
        &profile,
        r##"
name = "test"
version = "1.0"
uniqueItems = "warn"

[structural]
require_object_root = false
"##,
    )
    .unwrap();
    fs::write(&schema, r#"{"uniqueItems": true}"#).unwrap();

    let output = cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg("--format")
        .arg("human")
        .arg(&schema)
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(normalize_temp_paths(&stdout, dir.path()));
}

// ---------------------------------------------------------------------------
// Snapshot: JSON output
// ---------------------------------------------------------------------------

#[test]
fn snapshot_json_clean_schema() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    let schema = dir.path().join("schema.json");
    fs::write(&profile, minimal_profile()).unwrap();
    fs::write(&schema, r#"{"type": "string"}"#).unwrap();

    let output = cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg("--format")
        .arg("json")
        .arg(&schema)
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(normalize_temp_paths(&stdout, dir.path()));
}

#[test]
fn snapshot_json_forbidden_keyword() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    let schema = dir.path().join("schema.json");
    fs::write(&profile, profile_with_forbid_allof()).unwrap();
    fs::write(&schema, r#"{"allOf": [{"type": "string"}]}"#).unwrap();

    let output = cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg("--format")
        .arg("json")
        .arg(&schema)
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(normalize_temp_paths(&stdout, dir.path()));
}

#[test]
fn snapshot_json_batch_with_errors() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    let a = dir.path().join("a.json");
    let b = dir.path().join("b.json");
    fs::write(&profile, profile_with_forbid_allof()).unwrap();
    fs::write(&a, r#"{"allOf": [{"type": "string"}]}"#).unwrap();
    fs::write(&b, r#"{"type": "string"}"#).unwrap();

    let output = cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg("--format")
        .arg("json")
        .arg(&a)
        .arg(&b)
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(normalize_temp_paths(&stdout, dir.path()));
}

// ---------------------------------------------------------------------------
// Snapshot: SARIF output
// ---------------------------------------------------------------------------

#[test]
fn snapshot_sarif_forbidden_keyword() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    let schema = dir.path().join("schema.json");
    fs::write(&profile, profile_with_forbid_allof()).unwrap();
    fs::write(&schema, r#"{"allOf": [{"type": "string"}]}"#).unwrap();

    let output = cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg("--format")
        .arg("sarif")
        .arg(&schema)
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(normalize_temp_paths(&stdout, dir.path()));
}

// ---------------------------------------------------------------------------
// Snapshot: GHA output
// ---------------------------------------------------------------------------

#[test]
fn snapshot_gha_forbidden_keyword() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    let schema = dir.path().join("schema.json");
    fs::write(&profile, profile_with_forbid_allof()).unwrap();
    fs::write(&schema, r#"{"allOf": [{"type": "string"}]}"#).unwrap();

    let output = cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg("--format")
        .arg("gha")
        .arg(&schema)
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(normalize_temp_paths(&stdout, dir.path()));
}

// ---------------------------------------------------------------------------
// Snapshot: JUnit output
// ---------------------------------------------------------------------------

#[test]
fn snapshot_junit_forbidden_keyword() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    let schema = dir.path().join("schema.json");
    fs::write(&profile, profile_with_forbid_allof()).unwrap();
    fs::write(&schema, r#"{"allOf": [{"type": "string"}]}"#).unwrap();

    let output = cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg("--format")
        .arg("junit")
        .arg(&schema)
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(normalize_temp_paths(&stdout, dir.path()));
}

// ===========================================================================
// Direct emitter coverage tests — U2 (Phase 6)
// ===========================================================================

fn diag(
    code: &str,
    severity: DiagnosticSeverity,
    message: &str,
    pointer: &str,
    source: Option<SourceSpan>,
    profile: &str,
    hint: Option<&str>,
) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity,
        message: message.to_string(),
        pointer: pointer.to_string(),
        source,
        profile: profile.to_string(),
        hint: hint.map(|s| s.to_string()),
    }
}

fn src_span(file: &str, line: Option<u32>, col: Option<u32>) -> Option<SourceSpan> {
    Some(SourceSpan {
        file: file.to_string(),
        line,
        col,
    })
}

// ---------------------------------------------------------------------------
// emit_human_to_string — multi-diagnostic, empty, edge-cases, source variants
// ---------------------------------------------------------------------------

#[test]
fn test_emit_human_multi_diag() {
    let path = std::path::PathBuf::from("schema.json");
    let diags = vec![
        diag(
            "OAI-K-allOf",
            DiagnosticSeverity::Error,
            "keyword 'allOf' is not supported",
            "/allOf",
            src_span("schema.json", Some(42), Some(8)),
            "openai.so",
            Some("remove allOf"),
        ),
        diag(
            "OAI-K-anyOf",
            DiagnosticSeverity::Error,
            "keyword 'anyOf' is not supported",
            "/anyOf",
            None,
            "openai.so",
            None,
        ),
        diag(
            "OAI-K-uniqueItems",
            DiagnosticSeverity::Warning,
            "keyword 'uniqueItems' is discouraged",
            "/properties/items",
            src_span("schema.json", Some(15), None),
            "openai.so",
            None,
        ),
    ];
    let output = emit_human_to_string(&[(path, diags)], 2, 1, Some(123));
    insta::assert_snapshot!(output);
}

#[test]
fn test_emit_human_empty() {
    let output = emit_human_to_string(&[], 0, 0, Some(42));
    insta::assert_snapshot!(output);
}

#[test]
fn test_emit_human_empty_per_file() {
    let path = std::path::PathBuf::from("clean.json");
    let output = emit_human_to_string(&[(path, vec![])], 0, 0, None);
    insta::assert_snapshot!(output);
}

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
    insta::assert_snapshot!(output);
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
    insta::assert_snapshot!(output);
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
    assert_eq!(d0["seeUrl"], "https://schemalint.dev/rules/OAI-K-allOf");
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
    let output = emit_sarif_to_string(&[(path, diags)], 0, 0, &[], None);
    insta::assert_snapshot!(output);
}

#[test]
fn test_emit_sarif_empty() {
    let output = emit_sarif_to_string(&[], 0, 0, &[], None);
    insta::assert_snapshot!(output);
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
    let output = emit_sarif_to_string(&[(path, diags)], 0, 0, &[], None);
    insta::assert_snapshot!(output);
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
    let output = emit_sarif_to_string(&[(path, diags)], 0, 0, &[], None);
    insta::assert_snapshot!(output);
}

// ---------------------------------------------------------------------------
// emit_junit_to_string — multi-diagnostic, empty, edge-cases, source variants
// ---------------------------------------------------------------------------

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
    let output = emit_junit_to_string(&[(path, diags)], 0, 0, &[], None);
    insta::assert_snapshot!(output);
}

#[test]
fn test_emit_junit_empty_global() {
    let output = emit_junit_to_string(&[], 0, 0, &[], None);
    insta::assert_snapshot!(output);
}

#[test]
fn test_emit_junit_empty_per_file() {
    let path = std::path::PathBuf::from("clean.json");
    let output = emit_junit_to_string(&[(path, vec![])], 0, 0, &[], None);
    insta::assert_snapshot!(output);
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
    let output = emit_junit_to_string(&[(path, diags)], 0, 0, &[], None);
    insta::assert_snapshot!(output);
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
    let output = emit_junit_to_string(&[(path, diags)], 0, 0, &[], None);
    insta::assert_snapshot!(output);
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
    let output = emit_gha_to_string(&[(path, diags)], 0, 0, &[], None);
    insta::assert_snapshot!(output);
}

#[test]
fn test_emit_gha_empty() {
    let output = emit_gha_to_string(&[], 0, 0, &[], None);
    insta::assert_snapshot!(output);
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
    let output = emit_gha_to_string(&[(path, diags)], 0, 0, &[], None);
    insta::assert_snapshot!(output);
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
    let output = emit_gha_to_string(&[(path, diags)], 0, 0, &[], None);
    insta::assert_snapshot!(output);
}
