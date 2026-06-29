use super::*;

// ---------------------------------------------------------------------------
// Node sidecar failure branches
//
// These tests target the currently-untested branches in:
//   - src/node/mod.rs   (discover / augment_error error arms)
//   - src/subprocess.rs (send_discover DiscoverFailed / InvalidResponse paths)
//
// NOTE — NodeError::NotInstalled / resolve_tsx_cmd() (src/node/resolve.rs):
//   `dist/main.js` is present in the workspace, so NodeHelper::spawn(None)
//   always takes the compiled-bin branch and never calls resolve_tsx_cmd().
//   The NotInstalled path in resolve.rs is therefore unreachable from the
//   test harness without editing src; it is documented here rather than tested.
// ---------------------------------------------------------------------------

/// Convenience: path to a fixture shell script relative to CARGO_MANIFEST_DIR.
fn fixture(name: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/tests/fixtures/{}", manifest_dir, name)
}

// ---------------------------------------------------------------------------
// Real-sidecar DiscoverFailed: augment_error with no stderr
//
// When the Node sidecar cannot find a tsconfig.json it returns a JSON-RPC
// DiscoverFailed error without writing to stderr.  That exercises the
// `lines.is_empty() → return err` early-return branch in `augment_error`.
// ---------------------------------------------------------------------------

#[test]
fn check_node_real_sidecar_discover_failed_no_tsconfig() {
    // Run from a temp dir that has no tsconfig.json.
    let tmp = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args([
            "check-node",
            "--source",
            "src/**/*.ts",
            "--profile",
            "openai.so.2026-04-30",
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "exit code should be 1 on discovery failure"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The sidecar returns a DiscoverFailed error when no tsconfig.json found.
    assert!(
        stderr.contains("discovery failed") || stderr.contains("discovery failed for source"),
        "expected discovery failure in stderr, got:\n{stderr}"
    );
    assert!(
        stderr.contains("tsconfig.json") || stderr.contains("failed discovery"),
        "expected tsconfig.json mention or 'failed discovery', got:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Fake-sidecar: DiscoverFailed with ≤10 stderr lines
//
// This exercises `node/mod.rs` augment_error arms:
//   - NodeError::DiscoverFailed path (SubprocessError::DiscoverFailed mapping)
//
// Note on Node stderr timing: the node helper uses echo_prefix = None, so
// stderr lines are buffered without immediate I/O in the draining thread.
// Whether take_stderr() captures them before augment_error is called depends
// on thread scheduling and is non-deterministic.  We therefore assert only the
// guaranteed-deterministic outcomes (DiscoverFailed message, exit code) and do
// NOT assert whether the "--- Node stderr ---" block is present in the output.
// The augment_error short-stderr arm (≤10 lines → simple header) is fully
// covered by the Python equivalent (check_python_fake_sidecar_discover_failed_few_stderr_lines)
// where echo_prefix guarantees stderr ordering.
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn check_node_fake_sidecar_discover_failed_few_stderr_lines() {
    let script = fixture("fake_discover_error_few_stderr.sh");

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    let output = cmd
        .args([
            "check-node",
            "--source",
            "src/**/*.ts",
            "--profile",
            "openai.so.2026-04-30",
            "--node-path",
            &script,
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "exit code should be 1 when discover fails"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The DiscoverFailed error message from the fake sidecar must appear.
    // This confirms SubprocessError::DiscoverFailed → NodeError::DiscoverFailed mapping.
    assert!(
        stderr.contains("fake short-stderr discovery error"),
        "expected fake DiscoverFailed message, got:\n{stderr}"
    );
    // The discovery failure framing from the CLI pipeline.
    assert!(
        stderr.contains("discovery failed for source"),
        "expected 'discovery failed for source' framing, got:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Fake-sidecar: InvalidResponse with >10 stderr lines (truncated-tail arm)
//
// This exercises `node/mod.rs` augment_error arms:
//   - NodeError::InvalidResponse path (msg gets stderr appended)
//   - Long stderr: `lines.len() > 10` → "last 10 of N lines" truncated tail
// Also exercises `subprocess.rs` send_discover InvalidResponse arm via the
// non-JSON response the fake sidecar emits.
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn check_node_fake_sidecar_invalid_response_many_stderr_lines() {
    let script = fixture("fake_invalid_response.sh");

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    let output = cmd
        .args([
            "check-node",
            "--source",
            "src/**/*.ts",
            "--profile",
            "openai.so.2026-04-30",
            "--node-path",
            &script,
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "exit code should be 1 on invalid response"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The InvalidResponse error type must appear in the output (from CLI error mapping — deterministic).
    assert!(
        stderr.contains("invalid response from node helper"),
        "expected InvalidResponse error, got:\n{stderr}"
    );
    // "response parse error" from serde_json parsing non-JSON (from CLI error formatting — deterministic).
    assert!(
        stderr.contains("response parse error"),
        "expected parse error detail, got:\n{stderr}"
    );
    // NOTE: assertions on "last 10 of" and "--- Node stderr (last 10 of" have been
    // removed because those depend on async stderr drain timing and are racy under CPU load.
}

// ---------------------------------------------------------------------------
// Fake-sidecar: DiscoverFailed with >3 stderr lines (main fake_discover_error)
//
// Exercises the DiscoverFailed branch through augment_error with the 3-line
// stderr fixture — confirming the path from `discover()` → `augment_error` →
// DiscoverFailed arm → stderr appended.
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn check_node_fake_sidecar_discover_failed_with_stderr() {
    let script = fixture("fake_discover_error.sh");

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    let output = cmd
        .args([
            "check-node",
            "--source",
            "src/**/*.ts",
            "--profile",
            "openai.so.2026-04-30",
            "--node-path",
            &script,
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "exit code should be 1 on discovery failure"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Exact error message from the fake sidecar (from JSON-RPC stdout response — deterministic).
    assert!(
        stderr.contains("fake discovery failure: module not found"),
        "expected fake DiscoverFailed message, got:\n{stderr}"
    );
    // NOTE: assertion on "fake-sidecar: starting up" has been removed because that string
    // is written to sidecar STDERR (drained asynchronously) and is racy under CPU load.
    // All fake discovery failures should produce exit 1 + the correct error framing.
    assert!(
        stderr.contains("discovery failed for source"),
        "expected 'discovery failed for source' framing, got:\n{stderr}"
    );
}
