use std::fs;
use std::path::PathBuf;
use std::process::Command;

use assert_cmd::prelude::*;
use tempfile::TempDir;

use clap::Parser;
use schemalint::cli::args::{Cli, Commands, OutputFormat};
use serde::Deserialize;
use std::path::Path;

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
    let status = Command::new("npm")
        .args(["install", "--silent", "zod@^3.23"])
        .current_dir(dir)
        .status()
        .expect("npm install zod failed");
    assert!(status.success(), "npm install zod exited non-zero");
}

/// Run schemalint check-node in dir with given args, return parsed JSON output.
fn run_check_node_json(dir: &Path, args: &[&str]) -> JsonOutput {
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(dir);
    let mut full_args = vec!["check-node", "-f", "json"];
    full_args.extend(args);
    let output = cmd.args(&full_args).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "JSON parse failed: {e}\nstdout:\n{stdout}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

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

// ---------------------------------------------------------------------------
// End-to-end pipeline tests (real Node helper + TypeScript project)
// ---------------------------------------------------------------------------

#[test]
fn e2e_forbidden_format_produces_diagnostic_with_source_span() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "forbidden.ts",
            r#"import { z } from "zod";
export const Bad = z.object({ website: z.string().url() });
"#,
        )],
    );

    let out = run_check_node_json(
        tmp.path(),
        &[
            "--source",
            "src/**/*.ts",
            "--profile",
            "openai.so.2026-04-30",
        ],
    );

    assert_eq!(out.profiles, vec!["openai.so.2026-04-30"]);
    assert_eq!(out.summary.errors, 1);
    assert_eq!(out.summary.warnings, 0);
    assert_eq!(out.summary.schemas_checked, 1);

    let diag = &out.diagnostics[0];
    assert_eq!(diag.code, "OAI-K-format-restricted");
    assert_eq!(diag.severity, "error");
    assert_eq!(diag.pointer, "/properties/website");
    assert_eq!(diag.profile, "openai.so.2026-04-30");

    let src = diag
        .source
        .as_ref()
        .expect("source span should be populated");
    assert!(src.file.ends_with("/forbidden.ts"), "file={}", src.file);
    assert_eq!(src.line, Some(2));
}

#[test]
fn e2e_clean_schema_exits_zero() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "clean.ts",
            r#"import { z } from "zod";
export const Good = z.object({ name: z.string(), age: z.number() });
"#,
        )],
    );

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args([
            "check-node",
            "-S",
            "src/**/*.ts",
            "-p",
            "openai.so.2026-04-30",
            "-f",
            "json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "exit code should be 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out: JsonOutput = serde_json::from_str(&stdout).unwrap();
    assert_eq!(out.summary.total_issues, 0);
}

#[test]
fn e2e_multi_schema_single_file_separate_source_spans() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "multi.ts",
            r#"import { z } from "zod";

export const UserSchema = z.object({
  email: z.string().url(),
});

export const AddressSchema = z.object({
  street: z.string(),
  city: z.string(),
});
"#,
        )],
    );

    let out = run_check_node_json(
        tmp.path(),
        &[
            "--source",
            "src/**/*.ts",
            "--profile",
            "openai.so.2026-04-30",
        ],
    );

    let user_diag = out
        .diagnostics
        .iter()
        .find(|d| d.pointer == "/properties/email")
        .expect("should diagnose /properties/email from UserSchema");

    let src = user_diag.source.as_ref().unwrap();
    assert!(src.file.ends_with("/multi.ts"));
    assert_eq!(src.line, Some(4), "url() is on line 4 of multi.ts");
}

#[test]
fn e2e_package_json_driven_without_cli_flags() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "schema.ts",
            r#"import { z } from "zod";
export const My = z.object({ site: z.string().url() });
"#,
        )],
    );

    fs::write(
        tmp.path().join("package.json"),
        r#"{
  "schemalint": {
    "profiles": ["openai.so.2026-04-30"],
    "include": ["src/**/*.ts"]
  }
}"#,
    )
    .unwrap();

    let out = run_check_node_json(tmp.path(), &[]);
    assert_eq!(out.summary.errors, 1);
    assert_eq!(out.diagnostics[0].code, "OAI-K-format-restricted");
}

#[test]
fn e2e_cli_source_overrides_package_json_include() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let sub = src.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(
        sub.join("nested.ts"),
        r#"import { z } from "zod";
export const Nested = z.object({ url: z.string().url() });
"#,
    )
    .unwrap();

    let status = Command::new("npm")
        .args(["install", "--silent", "zod@^3.23"])
        .current_dir(tmp.path())
        .status()
        .unwrap();
    assert!(status.success());

    fs::write(
        tmp.path().join("tsconfig.json"),
        r#"{"compilerOptions":{"module":"ESNext","moduleResolution":"bundler","strict":true},"include":["src/**/*.ts"]}"#,
    )
    .unwrap();

    fs::write(
        tmp.path().join("package.json"),
        r#"{
  "schemalint": {
    "profiles": ["openai.so.2026-04-30"],
    "include": ["src/nonexistent/**/*.ts"]
  }
}"#,
    )
    .unwrap();

    let out = run_check_node_json(
        tmp.path(),
        &[
            "--source",
            "src/**/*.ts",
            "--profile",
            "openai.so.2026-04-30",
        ],
    );

    assert_eq!(out.summary.errors, 1);
    assert_eq!(out.diagnostics[0].code, "OAI-K-format-restricted");
}

#[test]
fn e2e_anthropic_profile_allows_uri_format() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "schema.ts",
            r#"import { z } from "zod";
export const My = z.object({ site: z.string().url() });
"#,
        )],
    );

    let out = run_check_node_json(
        tmp.path(),
        &[
            "--source",
            "src/**/*.ts",
            "--profile",
            "anthropic.so.2026-04-30",
        ],
    );

    assert!(
        !out.diagnostics
            .iter()
            .any(|d| d.code == "OAI-K-format-restricted"),
        "Anthropic profile should not produce OpenAI format-restricted diagnostics"
    );
}

#[test]
fn e2e_intersection_not_discovered_gracefully() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "intersection.ts",
            r#"import { z } from "zod";

const Person = z.object({ name: z.string() });
const Employee = z.object({ id: z.number() });

export const Combo = z.intersection(Person, Employee);
"#,
        )],
    );

    // z.intersection() is NOT discovered — the AST walker only finds
    // z.object() call expressions. This is documented behavior (scope
    // boundary: "Schemas constructed from imported factory functions...
    // are not discoverable via AST walking"). The pipeline should exit
    // cleanly with 0 schemas rather than crashing.
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args([
            "check-node",
            "-S",
            "src/**/*.ts",
            "-p",
            "openai.so.2026-04-30",
            "-f",
            "json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "should exit 0 (no schemas found, no error)"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out: JsonOutput = serde_json::from_str(&stdout).unwrap();
    assert_eq!(out.summary.schemas_checked, 0);
}
