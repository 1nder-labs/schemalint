use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

#[test]
fn cli_valid_schema_exits_0_human() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    let schema = dir.path().join("schema.json");
    fs::write(&profile, minimal_profile()).unwrap();
    fs::write(&schema, r#"{"type": "string"}"#).unwrap();

    cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg("--format")
        .arg("human")
        .arg(&schema)
        .assert()
        .success()
        .stdout(predicate::str::contains("0 issues found"));
}

#[test]
fn cli_valid_schema_exits_0_json() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    let schema = dir.path().join("schema.json");
    fs::write(&profile, minimal_profile()).unwrap();
    fs::write(&schema, r#"{"type": "string"}"#).unwrap();

    cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg("--format")
        .arg("json")
        .arg(&schema)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"schema_version\""))
        .stdout(predicate::str::contains("\"diagnostics\""));
}

#[test]
fn cli_directory_with_schemas() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    let a = dir.path().join("a.json");
    let b = dir.path().join("b.json");
    fs::write(&profile, minimal_profile()).unwrap();
    fs::write(&a, r#"{"type": "string"}"#).unwrap();
    fs::write(&b, r#"{"type": "number"}"#).unwrap();

    cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg("--format")
        .arg("human")
        .arg(&dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("0 issues found"));
}

#[test]
fn cli_multiple_explicit_files() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    let a = dir.path().join("a.json");
    let b = dir.path().join("b.json");
    fs::write(&profile, minimal_profile()).unwrap();
    fs::write(&a, r#"{"type": "string"}"#).unwrap();
    fs::write(&b, r#"{"type": "number"}"#).unwrap();

    cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg(&a)
        .arg(&b)
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Error path
// ---------------------------------------------------------------------------

#[test]
fn cli_forbidden_keyword_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    let schema = dir.path().join("schema.json");
    fs::write(&profile, profile_with_forbid_allof()).unwrap();
    fs::write(&schema, r#"{"allOf": [{"type": "string"}]}"#).unwrap();

    cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg(&schema)
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("TEST-K-allOf"));
}

#[test]
fn cli_missing_profile_file() {
    let dir = tempfile::tempdir().unwrap();
    let schema = dir.path().join("schema.json");
    fs::write(&schema, r#"{}"#).unwrap();

    cmd()
        .arg("check")
        .arg("--profile")
        .arg("nonexistent.toml")
        .arg(&schema)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("failed to read profile"));
}

#[test]
fn cli_invalid_json_schema() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    let schema = dir.path().join("schema.json");
    fs::write(&profile, minimal_profile()).unwrap();
    fs::write(&schema, "not json").unwrap();

    cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg(&schema)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("invalid JSON"));
}

#[test]
fn cli_invalid_profile_toml() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    let schema = dir.path().join("schema.json");
    fs::write(&profile, "not toml [[[").unwrap();
    fs::write(&schema, r#"{}"#).unwrap();

    cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg(&schema)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("failed to load profile"));
}

#[test]
fn cli_empty_directory() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    fs::write(&profile, minimal_profile()).unwrap();

    cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg("--format")
        .arg("human")
        .arg(&dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("0 issues found"));
}

#[test]
fn cli_warnings_only_exit_0() {
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

    cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg(&schema)
        .assert()
        .success()
        .stdout(predicate::str::contains("TEST-K-uniqueItems"))
        .stdout(predicate::str::contains("warning"));
}

#[test]
fn cli_json_output_structure() {
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

    assert!(!output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["schema_version"], "1.0");
    assert_eq!(json["tool"]["name"], "schemalint");
    assert!(json["summary"]["errors"].as_u64().unwrap() > 0);
    assert!(json["diagnostics"].as_array().unwrap().len() > 0);
    let diag = &json["diagnostics"][0];
    assert!(diag["code"].as_str().unwrap().starts_with("TEST-K"));
    assert!(diag["pointer"].as_str().is_some());
    assert!(diag["source"]["file"].as_str().is_some());
}

#[test]
fn cli_batch_aggregates_counts() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    let a = dir.path().join("a.json");
    let b = dir.path().join("b.json");
    fs::write(&profile, profile_with_forbid_allof()).unwrap();
    fs::write(&a, r#"{"allOf": [{"type": "string"}]}"#).unwrap();
    fs::write(&b, r#"{"allOf": [{"type": "number"}]}"#).unwrap();

    cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .arg("--format")
        .arg("human")
        .arg(&a)
        .arg(&b)
        .assert()
        .failure()
        .stdout(predicate::str::contains("2 issues found"))
        .stdout(predicate::str::contains("2 errors"));
}

#[test]
fn cli_no_paths_provided() {
    let dir = tempfile::tempdir().unwrap();
    let profile = dir.path().join("profile.toml");
    fs::write(&profile, minimal_profile()).unwrap();

    cmd()
        .arg("check")
        .arg("--profile")
        .arg(&profile)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("no schema files"));
}

// ---------------------------------------------------------------------------
// Multi-profile
// ---------------------------------------------------------------------------

#[test]
fn cli_builtin_profile_resolution() {
    let dir = tempfile::tempdir().unwrap();
    let schema = dir.path().join("schema.json");
    fs::write(
        &schema,
        r#"{"type": "object", "properties": {}, "additionalProperties": false}"#,
    )
    .unwrap();

    cmd()
        .arg("check")
        .arg("--profile")
        .arg("openai.so.2026-04-30")
        .arg("--format")
        .arg("json")
        .arg(&schema)
        .assert()
        .success();
}

#[test]
fn cli_multi_profile_union_openai_error_only() {
    let dir = tempfile::tempdir().unwrap();
    let schema = dir.path().join("schema.json");
    // allOf is forbidden in OpenAI, allowed in Anthropic.
    // Include a property so EmptyObjectRule does not fire.
    fs::write(
        &schema,
        r#"{"type": "object", "properties": {"x": {}}, "required": ["x"], "additionalProperties": false, "allOf": [{"type": "string"}]}"#,
    )
    .unwrap();

    let output = cmd()
        .arg("check")
        .arg("--profile")
        .arg("openai.so.2026-04-30")
        .arg("--profile")
        .arg("anthropic.so.2026-04-30")
        .arg("--format")
        .arg("json")
        .arg(&schema)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let diags = json["diagnostics"].as_array().unwrap();

    // Should have at least the OpenAI allOf error and object-root error.
    let openai_diags: Vec<_> = diags
        .iter()
        .filter(|d| d["profile"].as_str() == Some("openai.so.2026-04-30"))
        .collect();
    let anthropic_diags: Vec<_> = diags
        .iter()
        .filter(|d| d["profile"].as_str() == Some("anthropic.so.2026-04-30"))
        .collect();

    assert!(
        !openai_diags.is_empty(),
        "expected OpenAI diagnostics, got: {:?}",
        diags
    );
    assert!(
        anthropic_diags.is_empty(),
        "expected no Anthropic diagnostics, got: {:?}",
        anthropic_diags
    );
}

#[test]
fn cli_multi_profile_union_both_errors() {
    let dir = tempfile::tempdir().unwrap();
    let schema = dir.path().join("schema.json");
    // allOf is forbidden in OpenAI; minimum is forbidden in Anthropic.
    fs::write(
        &schema,
        r#"{"type": "object", "additionalProperties": false, "allOf": [{"type": "string"}], "minimum": 1}"#,
    )
    .unwrap();

    let output = cmd()
        .arg("check")
        .arg("--profile")
        .arg("openai.so.2026-04-30")
        .arg("--profile")
        .arg("anthropic.so.2026-04-30")
        .arg("--format")
        .arg("json")
        .arg(&schema)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let diags = json["diagnostics"].as_array().unwrap();

    let has_openai = diags
        .iter()
        .any(|d| d["profile"].as_str() == Some("openai.so.2026-04-30"));
    let has_anthropic = diags
        .iter()
        .any(|d| d["profile"].as_str() == Some("anthropic.so.2026-04-30"));

    assert!(has_openai, "expected OpenAI diagnostics");
    assert!(has_anthropic, "expected Anthropic diagnostics");
}

#[test]
fn cli_multi_profile_exit_code_errors_any_profile() {
    let dir = tempfile::tempdir().unwrap();
    let schema = dir.path().join("schema.json");
    // Clean for OpenAI, error for Anthropic (minimum is forbid).
    fs::write(
        &schema,
        r#"{"type": "object", "additionalProperties": false, "minimum": 1}"#,
    )
    .unwrap();

    cmd()
        .arg("check")
        .arg("--profile")
        .arg("openai.so.2026-04-30")
        .arg("--profile")
        .arg("anthropic.so.2026-04-30")
        .arg(&schema)
        .assert()
        .failure()
        .code(1);
}

#[test]
fn cli_multi_profile_exit_code_clean_all_profiles() {
    let dir = tempfile::tempdir().unwrap();
    let schema = dir.path().join("schema.json");
    fs::write(
        &schema,
        r#"{"type": "object", "properties": {"x": {"type": "string"}}, "required": ["x"], "additionalProperties": false}"#,
    )
    .unwrap();

    cmd()
        .arg("check")
        .arg("--profile")
        .arg("openai.so.2026-04-30")
        .arg("--profile")
        .arg("anthropic.so.2026-04-30")
        .arg(&schema)
        .assert()
        .success();
}
