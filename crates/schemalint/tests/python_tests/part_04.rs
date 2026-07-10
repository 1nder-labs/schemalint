// ---------------------------------------------------------------------------
// Real-sidecar end-to-end tests
//
// These tests exercise the full check-python pipeline with the actual
// schemalint_pydantic sidecar: discovery → normalize → rules → JSON output.
//
// Precondition: `schemalint_pydantic` must be importable (installed via
//   from crates/schemalint-python/python) and `pydantic` must be present.
//   CI sets PYTHONPATH to the canonical wheel source and installs Pydantic;
//   locally, set PYTHONPATH=crates/schemalint-python/python.
//
// If the sidecar is absent the schemalint process exits 1 with "No module named
// schemalint_pydantic" in stderr — the test will fail with a clear message
// rather than silently skipping, so a broken CI setup is immediately visible.
// ---------------------------------------------------------------------------

/// Minimal deserialization types for the JSON output of `check-python -f json`.
/// Mirror the analogous types in node_tests.rs.
#[derive(Debug, serde::Deserialize)]
struct PyJsonOutput {
    profiles: Vec<String>,
    summary: PyJsonSummary,
    diagnostics: Vec<PyJsonDiagnostic>,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct PyJsonSummary {
    total_issues: u32,
    errors: u32,
    warnings: u32,
    schemas_checked: u32,
}

#[derive(Debug, serde::Deserialize)]
struct PyJsonDiagnostic {
    code: String,
    severity: String,
    #[serde(default)]
    pointer: String,
    #[serde(default)]
    profile: String,
}

/// Path to the `tests/fixtures/` directory.
fn fixtures_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

// ---------------------------------------------------------------------------
// Real-sidecar discovery: OAI violations in a Pydantic model
//
// Exercises the full chain end-to-end:
//   1. schemalint spawns `python3 -m schemalint_pydantic` (real sidecar).
//   2. The sidecar imports `pydantic_fixture` (a tiny package in tests/fixtures/).
//   3. It discovers `ViolatingModel`, extracts its JSON Schema.
//   4. schemalint normalizes the schema and runs the openai.so.2026-04-30 rules.
//   5. Two diagnostics are expected:
//        - OAI-S-additional-properties-false  (missing additionalProperties: false)
//        - OAI-K-format-restricted            (format: "uri" not in allowed list)
//   6. Output is parsed from stdout as JSON and asserted on code presence.
//
// `pydantic_fixture/` lives at tests/fixtures/pydantic_fixture/__init__.py.
// We set PYTHONPATH to both the canonical wheel source and tests/fixtures/ so
// the sidecar and `pydantic_fixture` resolve without installing a second
// SchemaLint package.
// ---------------------------------------------------------------------------

#[test]
fn real_sidecar_pydantic_fixture_triggers_oai_violations() {
    let fixtures = fixtures_dir();
    assert!(
        fixtures.join("pydantic_fixture").exists(),
        "fixture directory missing: {}",
        fixtures.join("pydantic_fixture").display()
    );

    let python_source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../schemalint-python/python");
    let python_path = std::env::join_paths([python_source, fixtures.clone()]).unwrap();
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.env("PYTHONPATH", python_path);
    let output = cmd
        .args([
            "check-python",
            "--package",
            "pydantic_fixture",
            "--profile",
            "openai.so.2026-04-30",
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Parse the JSON output — if the sidecar is missing the process would have
    // exited 1 with stderr containing "No module named schemalint_pydantic",
    // and the JSON parse would fail with a clear diagnostic.
    let out: PyJsonOutput = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "JSON parse failed: {e}\n\
             stdout:\n{stdout}\n\
             stderr:\n{stderr}\n\
             \n\
             If stderr shows 'No module named schemalint_pydantic', set:\n\
             PYTHONPATH=crates/schemalint-python/python"
        )
    });

    // The profile should be the one we requested.
    assert!(
        out.profiles.iter().any(|p| p == "openai.so.2026-04-30"),
        "expected openai.so.2026-04-30 in profiles, got: {:?}",
        out.profiles
    );

    // Exactly one schema (ViolatingModel) must have been checked.
    assert_eq!(
        out.summary.schemas_checked, 1,
        "expected 1 schema checked, got {}",
        out.summary.schemas_checked
    );

    // `format: "uri"` is not in the OpenAI allowed format list — must fire.
    assert!(
        out.diagnostics
            .iter()
            .any(|d| d.code == "OAI-K-format-restricted"),
        "expected OAI-K-format-restricted diagnostic, got codes: {:?}",
        out.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()
    );

    // The format-restricted diagnostic must be at the /properties/website pointer.
    let format_diag = out
        .diagnostics
        .iter()
        .find(|d| d.code == "OAI-K-format-restricted")
        .unwrap();
    assert_eq!(
        format_diag.pointer, "/properties/website",
        "OAI-K-format-restricted should point to /properties/website"
    );
    assert_eq!(format_diag.severity, "error");
    assert_eq!(format_diag.profile, "openai.so.2026-04-30");

    // Pydantic v2 omits additionalProperties: false — the structural rule fires.
    assert!(
        out.diagnostics
            .iter()
            .any(|d| d.code == "OAI-S-additional-properties-false"),
        "expected OAI-S-additional-properties-false diagnostic"
    );

    // Overall: errors > 0, exit code 1.
    assert!(out.summary.errors > 0, "expected at least 1 error");
    assert!(
        !output.status.success(),
        "process should exit 1 when there are errors"
    );
}

// ---------------------------------------------------------------------------
// Real-sidecar: nonexistent package → DiscoverFailed → exit 1
//
// Asserts only the deterministic parts:
//   - exit code 1
//   - CLI framing message "discovery failed for package" in stderr
// Does NOT assert on "--- Python stderr ---" or any async-drain content
// because those are racy under CPU load.
// ---------------------------------------------------------------------------

#[test]
fn real_sidecar_nonexistent_package_discover_failed() {
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    let output = cmd
        .args([
            "check-python",
            "--package",
            "nonexistent_zzz_package_schemalint",
            "--profile",
            "openai.so.2026-04-30",
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "exit code should be 1 when discovery fails"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("discovery failed for package"),
        "expected 'discovery failed for package' framing in stderr, got:\n{stderr}"
    );
}
