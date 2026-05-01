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
fn parse_check_node_with_source_and_profile() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-node",
        "-S",
        "src/**/*.ts",
        "-p",
        "openai.so.2026-04-30",
    ]);
    match cli.command {
        Commands::CheckNode(args) => {
            assert_eq!(args.sources, vec!["src/**/*.ts"]);
            assert_eq!(args.profiles, vec![PathBuf::from("openai.so.2026-04-30")]);
            assert!(args.format.is_none());
            assert!(args.config.is_none());
            assert!(args.node_path.is_none());
        }
        _ => unreachable!(),
    }
}

#[test]
fn parse_check_node_with_format() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-node",
        "-S",
        "app.ts",
        "-p",
        "openai.so.2026-04-30",
        "-f",
        "json",
    ]);
    match cli.command {
        Commands::CheckNode(args) => {
            assert_eq!(args.format, Some(OutputFormat::Json));
        }
        _ => unreachable!(),
    }
}

#[test]
fn parse_check_node_with_config_and_node_path() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-node",
        "-S",
        "app.ts",
        "-p",
        "openai.so.2026-04-30",
        "--config",
        "custom-package.json",
        "--node-path",
        "/usr/local/bin/tsx",
    ]);
    match cli.command {
        Commands::CheckNode(args) => {
            assert_eq!(args.config, Some(PathBuf::from("custom-package.json")));
            assert_eq!(args.node_path, Some("/usr/local/bin/tsx".to_string()));
        }
        _ => unreachable!(),
    }
}

#[test]
fn parse_check_node_multiple_sources() {
    let cli = Cli::parse_from([
        "schemalint",
        "check-node",
        "-S",
        "src/models.ts",
        "-S",
        "src/schemas.ts",
        "-p",
        "openai.so.2026-04-30",
    ]);
    match cli.command {
        Commands::CheckNode(args) => {
            assert_eq!(args.sources, vec!["src/models.ts", "src/schemas.ts"]);
        }
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// Help output
// ---------------------------------------------------------------------------

#[test]
fn check_node_help_shows_subcommand() {
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    let output = cmd.args(["check-node", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Check Zod schemas"));
    assert!(stdout.contains("--source"));
    assert!(stdout.contains("--profile"));
    assert!(stdout.contains("--config"));
    assert!(stdout.contains("--node-path"));
    assert!(output.status.success());
}

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
