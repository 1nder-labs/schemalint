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

/// Missing `--profile` no longer hard-errors with "no profiles specified." —
/// schemalint now always resolves a default profile. In this case discovery
/// itself still fails (no tsconfig.json in the empty tmp dir), so the run
/// exits 1 via the discovery-failure path, not a profile error.
#[test]
fn check_node_no_profiles_falls_through_to_discovery_failure_not_profile_error() {
    let tmp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args(["check-node", "--source", "src/**/*.ts"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("no profiles specified."),
        "the old hard-error message must never appear, got:\n{stderr}"
    );
    assert!(
        stderr.contains("all 1 source(s) failed discovery"),
        "expected discovery-failure framing (no tsconfig.json), got:\n{stderr}"
    );
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
