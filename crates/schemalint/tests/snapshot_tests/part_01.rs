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
    assert_snapshot_stable!(normalize_temp_paths(&stdout, dir.path()));
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
    assert_snapshot_stable!(normalize_temp_paths(&stdout, dir.path()));
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
    assert_snapshot_stable!(normalize_temp_paths(&stdout, dir.path()));
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
    assert_snapshot_stable_json!(normalize_temp_paths(&stdout, dir.path()));
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
    assert_snapshot_stable_json!(normalize_temp_paths(&stdout, dir.path()));
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
    assert_snapshot_stable_json!(normalize_temp_paths(&stdout, dir.path()));
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
    assert_snapshot_stable!(normalize_temp_paths(&stdout, dir.path()));
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
    assert_snapshot_stable!(normalize_temp_paths(&stdout, dir.path()));
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
    assert_snapshot_stable!(normalize_temp_paths(&stdout, dir.path()));
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
    assert_snapshot_stable!(output);
}

#[test]
fn test_emit_human_empty() {
    let output = emit_human_to_string(&[], 0, 0, Some(42));
    assert_snapshot_stable!(output);
}

#[test]
fn test_emit_human_empty_per_file() {
    let path = std::path::PathBuf::from("clean.json");
    let output = emit_human_to_string(&[(path, vec![])], 0, 0, None);
    assert_snapshot_stable!(output);
}
