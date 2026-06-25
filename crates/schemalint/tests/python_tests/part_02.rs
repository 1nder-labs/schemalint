#[test]
fn parse_check_python_with_output_flag() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-python",
        "-P",
        "myapp.models",
        "-p",
        "openai.so.2026-04-30",
        "-o",
        "results.json",
    ]);
    match cli.command {
        Commands::CheckPython(args) => {
            assert_eq!(args.output, Some(std::path::PathBuf::from("results.json")));
        }
        _ => unreachable!(),
    }
}

#[test]
fn parse_check_python_with_output_and_format() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-python",
        "-P",
        "myapp.models",
        "-p",
        "openai.so.2026-04-30",
        "-f",
        "sarif",
        "-o",
        "results.sarif",
    ]);
    match cli.command {
        Commands::CheckPython(args) => {
            assert_eq!(args.format, Some(OutputFormat::Sarif));
            assert_eq!(args.output, Some(std::path::PathBuf::from("results.sarif")));
        }
        _ => unreachable!(),
    }
}

#[test]
fn parse_check_python_empty_package_list() {
    let cli = Cli::parse_from(["schemalint", "check-python", "-p", "openai.so.2026-04-30"]);
    match cli.command {
        Commands::CheckPython(args) => {
            assert!(args.packages.is_empty());
            assert!(!args.profiles.is_empty());
        }
        _ => unreachable!(),
    }
}

#[test]
fn parse_check_python_minimal_args() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-python",
        "-P",
        "myapp",
        "-p",
        "openai.so.2026-04-30",
    ]);
    match cli.command {
        Commands::CheckPython(args) => {
            assert_eq!(args.packages, vec!["myapp"]);
            assert_eq!(args.profiles.len(), 1);
            assert!(args.format.is_none());
            assert!(args.config.is_none());
            assert!(args.python_path.is_none());
            assert!(args.output.is_none());
        }
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// CLI errors without Python (argument-level validation)
// ---------------------------------------------------------------------------

#[test]
fn check_python_missing_required_package_flag() {
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    let output = cmd
        .args(["check-python", "-p", "openai.so.2026-04-30"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no packages specified."));
}

#[test]
fn check_python_invalid_profile_name() {
    let tmp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args([
            "check-python",
            "-P",
            "myapp.models",
            "-p",
            "nonexistent-zzz-profile",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown built-in profile"));
}

#[test]
fn check_python_invalid_format_cli_rejects() {
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    let output = cmd
        .args([
            "check-python",
            "-P",
            "myapp.models",
            "-p",
            "openai.so.2026-04-30",
            "-f",
            "invalidfmt",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalidfmt"));
}

// ---------------------------------------------------------------------------
// New error branches: config present but missing packages, explicit --config
// ---------------------------------------------------------------------------

/// pyproject.toml exists with a `[tool.schemalint]` section but no `packages`
/// list, and no --package flag → "no packages specified." exit 1.
///
/// Covers check_python.rs line 55: the `packages.is_empty()` guard when the
/// config is present but yields an empty packages list (packages defaults to []).
#[test]
fn check_python_pyproject_schemalint_no_packages_errors() {
    let tmp = TempDir::new().unwrap();
    let pyproject = tmp.path().join("pyproject.toml");
    // [tool.schemalint] block exists with profiles but no packages key.
    fs::write(
        &pyproject,
        r#"
[tool.schemalint]
profiles = ["openai.so.2026-04-30"]
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd.args(["check-python"]).output().unwrap();
    assert!(
        !output.status.success(),
        "exit code should be 1 when packages is empty"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no packages specified."),
        "expected 'no packages specified.' in stderr, got:\n{stderr}"
    );
}

/// Explicit --config pointing to a non-default name with invalid TOML → error
/// reported on exit 1, referencing that exact config path.
///
/// Covers the `args.config.as_deref()` Some-branch in check_python.rs line 20-22
/// combined with the parse-error path in pyproject::load_pyproject_config.
/// No existing test passes --config with a non-default filename for check-python.
#[test]
fn check_python_explicit_config_invalid_toml_errors() {
    let tmp = TempDir::new().unwrap();
    let custom_config = tmp.path().join("custom-pyproject.toml");
    fs::write(&custom_config, "this is definitely not valid toml {{{").unwrap();

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args([
            "check-python",
            "--config",
            custom_config.to_str().unwrap(),
            "--profile",
            "openai.so.2026-04-30",
            "--package",
            "myapp.models",
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "exit code should be 1 for malformed --config TOML"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid TOML in"),
        "expected 'invalid TOML in' in stderr, got:\n{stderr}"
    );
    // The error message must include the explicit config filename.
    assert!(
        stderr.contains("custom-pyproject.toml"),
        "expected config filename in error message, got:\n{stderr}"
    );
}

/// Explicit --config pointing to a nonexistent file falls through to None
/// (load_pyproject_config returns Ok(None) when !path.exists()), so the CLI
/// continues with CLI args only. With --package provided, it proceeds past
/// the packages guard and only fails later (spawn), NOT with a config error.
///
/// Confirms that a missing --config path does NOT produce a "failed to read"
/// error.
#[test]
fn check_python_explicit_config_nonexistent_falls_through() {
    let tmp = TempDir::new().unwrap();
    let nonexistent_config = tmp.path().join("does-not-exist.toml");
    assert!(!nonexistent_config.exists());

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args([
            "check-python",
            "--config",
            nonexistent_config.to_str().unwrap(),
            "--profile",
            "openai.so.2026-04-30",
        ])
        .output()
        .unwrap();
    // No --package and config is missing → falls through to "no packages specified."
    assert!(
        !output.status.success(),
        "exit code should be 1 (no packages)"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no packages specified."),
        "expected 'no packages specified.' (not a read error), got:\n{stderr}"
    );
    assert!(
        !stderr.contains("failed to read"),
        "nonexistent --config should NOT produce a read error, got:\n{stderr}"
    );
}
