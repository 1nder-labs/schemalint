use super::*;

// ---------------------------------------------------------------------------
// package.json config integration
// ---------------------------------------------------------------------------

#[test]
fn check_node_loads_package_json_config() {
    let tmp = TempDir::new().unwrap();
    let pkg = tmp.path().join("package.json");
    fs::write(
        &pkg,
        r#"{
  "schemalint": {
    "profiles": ["openai.so.2026-04-30"],
    "include": ["src/**/*.ts"]
  }
}"#,
    )
    .unwrap();

    // This will try to spawn the Node helper. The key assertion: config was
    // loaded, NOT "no sources specified".
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd.args(["check-node"]).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("no sources specified."));
}

#[test]
fn check_node_cli_overrides_package_json_profiles() {
    let tmp = TempDir::new().unwrap();
    let pkg = tmp.path().join("package.json");
    fs::write(
        &pkg,
        r#"{
  "schemalint": {
    "profiles": ["anthropic.so.2026-04-30"],
    "include": ["src/**/*.ts"]
  }
}"#,
    )
    .unwrap();

    // CLI --profile should override package.json profiles
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args(["check-node", "--profile", "openai.so.2026-04-30"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("no profiles specified."));
}

#[test]
fn check_node_invalid_package_json_errors() {
    let tmp = TempDir::new().unwrap();
    let pkg = tmp.path().join("package.json");
    fs::write(&pkg, "this is not valid json {{{").unwrap();

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd.args(["check-node"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid JSON in"));
}

#[test]
fn check_node_missing_package_json_no_config_ok() {
    let tmp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    // No package.json, and no --source → should error about no sources
    let output = cmd
        .args(["check-node", "--profile", "openai.so.2026-04-30"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no sources specified."));
}

// ---------------------------------------------------------------------------
// New error branches: config present but missing fields, explicit --config path
// ---------------------------------------------------------------------------

/// package.json exists with a `schemalint` key but no `include` list, and no
/// --source flag → "no sources specified." exit 1.
///
/// Covers check_node.rs line 59: the `sources.is_empty()` guard when the config
/// is present but yields an empty include list (Some(config) with include=[]).
#[test]
fn check_node_package_json_schemalint_no_include_errors() {
    let tmp = TempDir::new().unwrap();
    let pkg = tmp.path().join("package.json");
    // schemalint block exists, profiles set, but include[] is absent (defaults to []).
    fs::write(
        &pkg,
        r#"{
  "schemalint": {
    "profiles": ["openai.so.2026-04-30"]
  }
}"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd.args(["check-node"]).output().unwrap();
    assert!(
        !output.status.success(),
        "exit code should be 1 when include is empty"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no sources specified."),
        "expected 'no sources specified.' in stderr, got:\n{stderr}"
    );
}

/// Explicit --config pointing to a non-default name with invalid JSON → error
/// reported on exit 1, referencing that exact config path.
///
/// Covers the `args.config.as_deref()` Some-branch in check_node.rs line 19-21
/// combined with the parse-error path in node_config::load_node_config.
/// No existing test passes --config with a non-default filename.
#[test]
fn check_node_explicit_config_invalid_json_errors() {
    let tmp = TempDir::new().unwrap();
    let custom_config = tmp.path().join("custom-package.json");
    fs::write(&custom_config, "{ definitely: not: valid: json }").unwrap();

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args([
            "check-node",
            "--config",
            custom_config.to_str().unwrap(),
            "--profile",
            "openai.so.2026-04-30",
            "--source",
            "src/**/*.ts",
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "exit code should be 1 for malformed --config JSON"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid JSON in"),
        "expected 'invalid JSON in' in stderr, got:\n{stderr}"
    );
    // The error message must include the explicit config filename so the user
    // knows which file caused the problem.
    assert!(
        stderr.contains("custom-package.json"),
        "expected config filename in error message, got:\n{stderr}"
    );
}

/// Explicit --config pointing to a nonexistent file falls through to None
/// (load_node_config returns Ok(None) when !path.exists()), so the CLI
/// continues to the "no sources specified." guard and exits 1.
///
/// Confirms that a missing --config path does NOT produce a "failed to read"
/// error, only the downstream "no sources specified." message.
#[test]
fn check_node_explicit_config_nonexistent_falls_through_to_no_sources() {
    let tmp = TempDir::new().unwrap();
    let nonexistent_config = tmp.path().join("does-not-exist.json");
    // Confirm the file does not exist.
    assert!(!nonexistent_config.exists());

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args([
            "check-node",
            "--config",
            nonexistent_config.to_str().unwrap(),
            "--profile",
            "openai.so.2026-04-30",
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "exit code should be 1 (no sources)"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // load_node_config returns Ok(None) for missing files; next failure is "no sources".
    assert!(
        stderr.contains("no sources specified."),
        "expected 'no sources specified.' (not a read error), got:\n{stderr}"
    );
    assert!(
        !stderr.contains("failed to read"),
        "nonexistent --config should NOT produce a read error, got:\n{stderr}"
    );
}
