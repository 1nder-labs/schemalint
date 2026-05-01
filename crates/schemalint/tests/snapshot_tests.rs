use assert_cmd::Command;
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
    out = re_json.replace_all(&out, "duration_ms\": [DURATION]").to_string();
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
