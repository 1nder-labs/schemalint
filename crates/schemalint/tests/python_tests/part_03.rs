// ---------------------------------------------------------------------------
// Python sidecar failure branches
//
// These tests target the currently-untested branches in:
//   - src/python/mod.rs   (spawn / discover / augment_error error arms)
//   - src/subprocess.rs   (send_discover DiscoverFailed / InvalidResponse paths)
//
// All tests here are deterministic: they either control the PATH environment
// or use a tiny fake shell-script sidecar that emits predictable responses.
// ---------------------------------------------------------------------------

/// Convenience: path to a fixture shell script relative to CARGO_MANIFEST_DIR.
fn fixture(name: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/tests/fixtures/{}", manifest_dir, name)
}

// ---------------------------------------------------------------------------
// PythonError::NotInstalled — empty PATH, no python3/python available
//
// Exercises src/python/mod.rs `resolve_python()` when neither `python3` nor
// `python` is found on PATH: the probe loop exhausts all candidates and
// returns Err(PythonError::NotInstalled).
// ---------------------------------------------------------------------------

#[test]
fn check_python_not_installed_when_path_is_empty() {
    let tmp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    // Point PATH at a temp directory that contains no python3 or python binary.
    // This causes probe_command to fail for all candidates in resolve_python().
    cmd.env("PATH", tmp.path());
    let output = cmd
        .args([
            "check-python",
            "--package",
            "myapp.models",
            "--profile",
            "openai.so.2026-04-30",
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "exit code should be 1 when python is not found"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // PythonError::NotInstalled message: "python interpreter not found: tried python3, python"
    assert!(
        stderr.contains("python interpreter not found"),
        "expected NotInstalled message, got:\n{stderr}"
    );
    assert!(
        stderr.contains("python3") && stderr.contains("python"),
        "expected candidate list in error, got:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Real-sidecar DiscoverFailed — augment_error early-return (no stderr)
//
// When the real schemalint_pydantic sidecar is asked to discover a package
// that doesn't exist, it returns a JSON-RPC DiscoverFailed error without
// writing to stderr.  This exercises the `lines.is_empty() → return err`
// early-return branch in `augment_error`.
// ---------------------------------------------------------------------------

#[test]
fn check_python_real_sidecar_discover_failed_nonexistent_package() {
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    let output = cmd
        .args([
            "check-python",
            "--package",
            "definitely.does.not.exist.xyz",
            "--profile",
            "openai.so.2026-04-30",
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "exit code should be 1 when package does not exist"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The DiscoverFailed message should mention the non-existent package.
    assert!(
        stderr.contains("discovery failed"),
        "expected DiscoverFailed message, got:\n{stderr}"
    );
    // Should not contain the augmented stderr block because the real sidecar
    // emits no stderr for missing packages.
    assert!(
        !stderr.contains("--- Python stderr ---"),
        "real sidecar should produce no stderr block, got:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Fake-sidecar: DiscoverFailed with ≤10 stderr lines (short arm of augment_error)
//
// Exercises `python/mod.rs` augment_error:
//   - PythonError::DiscoverFailed path (msg gets stderr appended)
//   - Short stderr: `lines.len() <= 10` → simple "--- Python stderr ---" format
// Also exercises subprocess.rs send_discover DiscoverFailed arm.
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn check_python_fake_sidecar_discover_failed_few_stderr_lines() {
    let script = fixture("fake_discover_error_few_stderr.sh");

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    let output = cmd
        .args([
            "check-python",
            "--package",
            "myapp.models",
            "--profile",
            "openai.so.2026-04-30",
            "--python-path",
            &script,
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "exit code should be 1 when discover fails"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Exact error message emitted by the fake sidecar.
    assert!(
        stderr.contains("fake short-stderr discovery error"),
        "expected fake DiscoverFailed message, got:\n{stderr}"
    );
    // Short-arm format: no "last N of M" prefix.
    assert!(
        stderr.contains("--- Python stderr ---"),
        "expected short-arm stderr header, got:\n{stderr}"
    );
    assert!(
        stderr.contains("fake-sidecar: line one"),
        "expected first stderr line attached, got:\n{stderr}"
    );
    // Must NOT use the truncated-tail header (only appears for >10 lines).
    assert!(
        !stderr.contains("last 10 of"),
        "short-arm should not show truncated header, got:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Fake-sidecar: InvalidResponse with >10 stderr lines (truncated-tail arm)
//
// Exercises `python/mod.rs` augment_error:
//   - PythonError::InvalidResponse path (msg gets stderr appended)
//   - Long stderr: `lines.len() > 10` → "last 10 of N lines" truncated tail
// Also exercises subprocess.rs send_discover InvalidResponse arm via the
// non-JSON response the fake sidecar emits.
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn check_python_fake_sidecar_invalid_response_many_stderr_lines() {
    let script = fixture("fake_invalid_response.sh");

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    let output = cmd
        .args([
            "check-python",
            "--package",
            "myapp.models",
            "--profile",
            "openai.so.2026-04-30",
            "--python-path",
            &script,
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "exit code should be 1 on invalid response"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // InvalidResponse error type must appear.
    assert!(
        stderr.contains("invalid response from python helper"),
        "expected InvalidResponse error, got:\n{stderr}"
    );
    // The json parse error detail from serde_json.
    assert!(
        stderr.contains("response parse error"),
        "expected parse error detail, got:\n{stderr}"
    );
    // The long-stderr arm uses "last N of M lines" truncated-tail header.
    assert!(
        stderr.contains("last 10 of"),
        "expected truncated stderr header for >10 lines, got:\n{stderr}"
    );
    assert!(
        stderr.contains("--- Python stderr (last 10 of"),
        "expected full truncated header format, got:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Fake-sidecar: DiscoverFailed with 3 stderr lines via main fake fixture
//
// Exercises the complete path: PythonHelper::discover → send_discover →
// DiscoverFailed → augment_error → stderr lines attached to error message.
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn check_python_fake_sidecar_discover_failed_with_stderr() {
    let script = fixture("fake_discover_error.sh");

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    let output = cmd
        .args([
            "check-python",
            "--package",
            "myapp.models",
            "--profile",
            "openai.so.2026-04-30",
            "--python-path",
            &script,
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "exit code should be 1 on discovery failure"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Exact error text from the fake sidecar.
    assert!(
        stderr.contains("fake discovery failure: module not found"),
        "expected fake DiscoverFailed message, got:\n{stderr}"
    );
    // Stderr lines are appended as context.
    assert!(
        stderr.contains("fake-sidecar: starting up"),
        "expected augmented stderr context, got:\n{stderr}"
    );
    // Correct failure framing from the CLI.
    assert!(
        stderr.contains("discovery failed for package"),
        "expected 'discovery failed for package' framing, got:\n{stderr}"
    );
}
