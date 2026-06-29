use super::*;

// ---------------------------------------------------------------------------
// Error paths
// ---------------------------------------------------------------------------

#[test]
fn check_node_no_sources_no_config_errors() {
    let tmp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args(["check-node", "--profile", "openai.so.2026-04-30"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no sources specified."));
}

#[test]
fn check_node_no_profiles_errors() {
    let tmp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args(["check-node", "--source", "src/**/*.ts"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no profiles specified."));
}

#[test]
fn check_node_nonexistent_node_path_errors() {
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    let output = cmd
        .args([
            "check-node",
            "--source",
            "src/**/*.ts",
            "--profile",
            "openai.so.2026-04-30",
            "--node-path",
            "/nonexistent/node/binary",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to start"));
}
