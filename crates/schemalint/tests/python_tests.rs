use std::fs;
use std::path::PathBuf;
use std::process::Command;

use assert_cmd::prelude::*;
use tempfile::TempDir;

use clap::Parser;
use schemalint::cli::args::{Cli, Commands, OutputFormat};
use schemalint::python::PythonError;

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

// ---------------------------------------------------------------------------
// PythonError display formatting
// ---------------------------------------------------------------------------

#[test]
fn python_error_not_installed_display() {
    let err = PythonError::NotInstalled("python3, python".into());
    assert!(err.to_string().contains("python interpreter not found"));
    assert!(err.to_string().contains("python3, python"));
}

#[test]
fn python_error_spawn_failed_display() {
    let err = PythonError::SpawnFailed("command not found: python3".into());
    assert!(err.to_string().contains("failed to spawn python helper"));
    assert!(err.to_string().contains("command not found: python3"));
}

#[test]
fn python_error_timeout_display() {
    let err = PythonError::Timeout(60);
    assert!(err.to_string().contains("timed out after 60s"));
}

#[test]
fn python_error_invalid_response_display() {
    let err = PythonError::InvalidResponse("response parse error: missing field".into());
    assert!(err
        .to_string()
        .contains("invalid response from python helper"));
    assert!(err.to_string().contains("response parse error"));
}

#[test]
fn python_error_discover_failed_display() {
    let err = PythonError::DiscoverFailed("package not found: myapp.models".into());
    assert!(err.to_string().contains("discovery failed"));
    assert!(err.to_string().contains("package not found: myapp.models"));
}

#[test]
fn python_error_request_failed_display() {
    let err = PythonError::RequestFailed("write error: broken pipe".into());
    assert!(err
        .to_string()
        .contains("failed to communicate with python helper"));
    assert!(err.to_string().contains("broken pipe"));
}

// ---------------------------------------------------------------------------
// Additional CLI argument parsing edge cases
// ---------------------------------------------------------------------------

#[test]
fn parse_check_python_all_formats() {
    for (flag, expected) in [
        ("human", OutputFormat::Human),
        ("json", OutputFormat::Json),
        ("sarif", OutputFormat::Sarif),
        ("gha", OutputFormat::Gha),
        ("junit", OutputFormat::Junit),
    ] {
        let cli = Cli::parse_from([
            "schemalint",
            "check-python",
            "-P",
            "myapp.models",
            "-p",
            "openai.so.2026-04-30",
            "-f",
            flag,
        ]);
        match cli.command {
            Commands::CheckPython(args) => {
                assert_eq!(args.format, Some(expected), "format flag -f {flag}");
            }
            _ => unreachable!(),
        }
    }
}

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
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no packages specified.") || !output.status.success());
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
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown built-in profile")
            || stderr.contains("failed to read profile")
            || !output.status.success()
    );
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
    assert!(stderr.contains("invalidfmt") || stderr.contains("error"));
}
