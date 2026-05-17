use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::prelude::*;
use tempfile::TempDir;

use clap::Parser;
use schemalint::cli::args::{Cli, Commands, OutputFormat};
use schemalint::node::NodeError;
use serde::Deserialize;

/// Minimal JSON output structure for asserting key fields.
#[derive(Debug, Deserialize)]
struct JsonOutput {
    profiles: Vec<String>,
    summary: JsonSummary,
    diagnostics: Vec<JsonDiagnostic>,
}

#[derive(Debug, Deserialize)]
struct JsonSummary {
    total_issues: u32,
    errors: u32,
    warnings: u32,
    schemas_checked: u32,
}

#[derive(Debug, Deserialize)]
struct JsonDiagnostic {
    code: String,
    severity: String,
    #[serde(default)]
    pointer: String,
    #[serde(default)]
    source: Option<JsonSource>,
    #[serde(default)]
    profile: String,
}

#[derive(Debug, Deserialize)]
struct JsonSource {
    file: String,
    #[serde(default)]
    line: Option<u32>,
}

/// Create a minimal TypeScript project with zod installed.
fn setup_ts_project(dir: &Path, files: &[(&str, &str)]) {
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();
    for (name, content) in files {
        fs::write(src.join(name), content).unwrap();
    }
    fs::write(
        dir.join("tsconfig.json"),
        r#"{"compilerOptions":{"module":"ESNext","moduleResolution":"bundler","strict":true},"include":["src/**/*.ts"]}"#,
    )
    .unwrap();
    link_workspace_node_modules(dir);
}

fn link_workspace_node_modules(dir: &Path) {
    let target = workspace_root().join("typescript/schemalint-zod/node_modules");
    assert!(
        target.join("zod").exists(),
        "missing workspace zod dependency at {}",
        target.display()
    );
    let link = dir.join("node_modules");
    if link.exists() {
        return;
    }
    create_dir_symlink(&target, &link).unwrap_or_else(|err| {
        panic!(
            "failed to link {} to {}: {}",
            link.display(),
            target.display(),
            err
        )
    });
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("schemalint crate should be inside workspace/crates")
        .to_path_buf()
}

#[cfg(unix)]
fn create_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn create_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

/// Run schemalint check-node in dir with given args, return parsed JSON output.
fn run_check_node_json(dir: &Path, args: &[&str]) -> JsonOutput {
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(dir);
    let mut full_args = vec!["check-node", "-f", "json"];
    full_args.extend(args);
    let output = cmd.args(&full_args).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed: JsonOutput = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("JSON parse failed: {e}\nstdout:\n{stdout}\nstderr:\n{stderr}"));
    assert_eq!(
        parsed.summary.total_issues,
        parsed.summary.errors + parsed.summary.warnings
    );
    parsed
}

#[path = "node_tests/args.rs"]
mod args;
#[path = "node_tests/args_more.rs"]
mod args_more;
#[path = "node_tests/config.rs"]
mod config;
#[path = "node_tests/e2e.rs"]
mod e2e;
#[path = "node_tests/errors.rs"]
mod errors;
#[path = "node_tests/node_error_display.rs"]
mod node_error_display;
