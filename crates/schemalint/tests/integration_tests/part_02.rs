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
