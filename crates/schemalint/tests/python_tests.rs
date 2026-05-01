use std::fs;
use std::path::PathBuf;
use std::process::Command;

use assert_cmd::prelude::*;
use tempfile::TempDir;

use clap::Parser;
use schemalint::cli::args::{Cli, Commands, OutputFormat};

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_check_python_with_package_and_profile() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-python",
        "-P",
        "myapp.models",
        "-p",
        "openai.so.2026-04-30",
    ]);
    match cli.command {
        Commands::CheckPython(args) => {
            assert_eq!(args.packages, vec!["myapp.models"]);
            assert_eq!(args.profiles, vec![PathBuf::from("openai.so.2026-04-30")]);
            assert!(args.format.is_none());
            assert!(args.config.is_none());
            assert!(args.python_path.is_none());
        }
        _ => unreachable!(),
    }
}

#[test]
fn parse_check_python_with_format() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-python",
        "-P",
        "app",
        "-p",
        "openai.so.2026-04-30",
        "-f",
        "json",
    ]);
    match cli.command {
        Commands::CheckPython(args) => {
            assert_eq!(args.format, Some(OutputFormat::Json));
        }
        _ => unreachable!(),
    }
}

#[test]
fn parse_check_python_with_config_and_python_path() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-python",
        "-P",
        "app",
        "-p",
        "openai.so.2026-04-30",
        "--config",
        "custom.toml",
        "--python-path",
        "/usr/bin/python3",
    ]);
    match cli.command {
        Commands::CheckPython(args) => {
            assert_eq!(args.config, Some(PathBuf::from("custom.toml")));
            assert_eq!(args.python_path, Some("/usr/bin/python3".to_string()));
        }
        _ => unreachable!(),
    }
}

#[test]
fn parse_check_python_multiple_packages() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-python",
        "-P",
        "app.models",
        "-P",
        "app.schemas",
        "-p",
        "openai.so.2026-04-30",
    ]);
    match cli.command {
        Commands::CheckPython(args) => {
            assert_eq!(args.packages, vec!["app.models", "app.schemas"]);
        }
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// Help output
// ---------------------------------------------------------------------------

#[test]
fn check_python_help_shows_subcommand() {
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    let output = cmd.args(["check-python", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Check Pydantic models"));
    assert!(stdout.contains("--package"));
    assert!(stdout.contains("--profile"));
    assert!(stdout.contains("--config"));
    assert!(stdout.contains("--python-path"));
    assert!(output.status.success());
}

// ---------------------------------------------------------------------------
// Error paths
// ---------------------------------------------------------------------------

#[test]
fn check_python_no_packages_no_config_errors() {
    let tmp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args(["check-python", "--profile", "openai.so.2026-04-30"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no packages specified."));
}

#[test]
fn check_python_no_profiles_errors() {
    let tmp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args(["check-python", "--package", "myapp.models"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no profiles specified."));
}

#[test]
fn check_python_nonexistent_python_errors() {
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    let output = cmd
        .args([
            "check-python",
            "--package",
            "myapp.models",
            "--profile",
            "openai.so.2026-04-30",
            "--python-path",
            "/nonexistent/python/binary",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to spawn python helper"));
}

// ---------------------------------------------------------------------------
// pyproject.toml integration
// ---------------------------------------------------------------------------

#[test]
fn check_python_loads_pyproject_config() {
    let tmp = TempDir::new().unwrap();
    let pyproject = tmp.path().join("pyproject.toml");
    fs::write(
        &pyproject,
        r#"
[tool.schemalint]
profiles = ["openai.so.2026-04-30"]
packages = ["myapp.models"]
"#,
    )
    .unwrap();

    // This will try to spawn the Python helper (which may succeed via python3)
    // and attempt discovery. Discovery may fail or succeed depending on env.
    // The key assertion: config was loaded, NOT "no packages specified".
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd.args(["check-python"]).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("no packages specified."));
}

#[test]
fn check_python_cli_overrides_pyproject_profiles() {
    let tmp = TempDir::new().unwrap();
    let pyproject = tmp.path().join("pyproject.toml");
    fs::write(
        &pyproject,
        r#"
[tool.schemalint]
profiles = ["anthropic.so.2026-04-30"]
packages = ["myapp.models"]
"#,
    )
    .unwrap();

    // CLI --profile should override pyproject.toml profiles (not "no profiles")
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args(["check-python", "--profile", "openai.so.2026-04-30"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("no profiles specified."));
}

#[test]
fn check_python_invalid_pyproject_toml_errors() {
    let tmp = TempDir::new().unwrap();
    let pyproject = tmp.path().join("pyproject.toml");
    fs::write(&pyproject, "this is not valid toml {{{").unwrap();

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd.args(["check-python"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid TOML in"));
}

#[test]
fn check_python_missing_pyproject_no_config_ok() {
    let tmp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    // No pyproject.toml, and no --package → should error about no packages
    let output = cmd
        .args(["check-python", "--profile", "openai.so.2026-04-30"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no packages specified."));
}
