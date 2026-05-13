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
