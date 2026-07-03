// ---------------------------------------------------------------------------
// Default profile detection (`check` without `--profile`)
// ---------------------------------------------------------------------------
//
// `--profile` is now optional on `check`. These tests cover the fallback
// chain implemented in `check.rs` / `cli::default_profile_ids`:
//   1. no package.json / no recognized deps → openai.so.2026-04-30 default
//   2. package.json deps name one provider  → that provider's latest profile
//   3. package.json deps name both          → both profiles
// Each tier must print exactly one `info:` line and never the old
// "no schema files"-adjacent hard error for a missing profile.

#[test]
fn check_no_profile_no_deps_defaults_to_openai() {
    let dir = tempfile::tempdir().unwrap();
    let schema = dir.path().join("schema.json");
    fs::write(
        &schema,
        r#"{"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"], "additionalProperties": false}"#,
    )
    .unwrap();

    let output = cmd()
        .current_dir(dir.path())
        .arg("check")
        .arg("--format")
        .arg("json")
        .arg("schema.json")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "info: no --profile and no provider detected in package.json; defaulting to openai.so.2026-04-30"
        ),
        "expected openai-default info line, got:\n{stderr}"
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["profiles"], serde_json::json!(["openai.so.2026-04-30"]));
}

#[test]
fn check_no_profile_detects_anthropic_from_package_json_deps() {
    let dir = tempfile::tempdir().unwrap();
    let schema = dir.path().join("schema.json");
    fs::write(
        &schema,
        r#"{"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"], "additionalProperties": false}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies": {"@anthropic-ai/sdk": "^0.30.0"}}"#,
    )
    .unwrap();

    let output = cmd()
        .current_dir(dir.path())
        .arg("check")
        .arg("--format")
        .arg("json")
        .arg("schema.json")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("info: no --profile given; detected anthropic from package.json"),
        "expected anthropic auto-detect info line, got:\n{stderr}"
    );
    assert!(
        stderr.contains("anthropic.so.2026-04-30"),
        "expected resolved profile id in info line, got:\n{stderr}"
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        json["profiles"],
        serde_json::json!(["anthropic.so.2026-04-30"])
    );
}

#[test]
fn check_no_profile_detects_both_providers_from_package_json_deps() {
    let dir = tempfile::tempdir().unwrap();
    let schema = dir.path().join("schema.json");
    fs::write(
        &schema,
        r#"{"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"], "additionalProperties": false}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies": {"openai": "^4.0.0", "@anthropic-ai/sdk": "^0.30.0"}}"#,
    )
    .unwrap();

    let output = cmd()
        .current_dir(dir.path())
        .arg("check")
        .arg("--format")
        .arg("json")
        .arg("schema.json")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("info: no --profile given; detected openai, anthropic from package.json"),
        "expected both-provider auto-detect info line, got:\n{stderr}"
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let profiles = json["profiles"].as_array().unwrap();
    let names: Vec<&str> = profiles.iter().map(|p| p.as_str().unwrap()).collect();
    assert!(names.contains(&"openai.so.2026-04-30"), "got: {:?}", names);
    assert!(
        names.contains(&"anthropic.so.2026-04-30"),
        "got: {:?}",
        names
    );
}

#[test]
fn check_no_profile_detects_deps_from_directory_arg() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"devDependencies": {"@ai-sdk/openai": "^1.0.0"}}"#,
    )
    .unwrap();
    let schema = dir.path().join("schema.json");
    fs::write(
        &schema,
        r#"{"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"], "additionalProperties": false}"#,
    )
    .unwrap();

    // Pass the directory itself (not a file), as an absolute path, as the
    // schema path arg.
    let output = cmd()
        .arg("check")
        .arg("--format")
        .arg("json")
        .arg(dir.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("info: no --profile given; detected openai from package.json"),
        "expected openai auto-detect info line for directory arg, got:\n{stderr}"
    );
}
