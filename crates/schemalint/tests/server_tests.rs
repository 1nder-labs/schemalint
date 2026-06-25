use std::fs;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tempfile::TempDir;

fn cmd() -> Command {
    let exe = std::env::current_exe().expect("current_exe should be available");
    let dir = exe.parent().expect("exe should have parent");
    // When running via cargo test, the test binary is in target/debug/deps/,
    // so we go up one level to target/debug/ where the main binary lives.
    let bin = if dir.file_name() == Some(std::ffi::OsStr::new("deps")) {
        dir.parent().unwrap().join("schemalint")
    } else {
        dir.join("schemalint")
    };
    Command::new(bin)
}

fn send_request(child: &mut std::process::Child, request: &str) -> serde_json::Value {
    let stdin = child.stdin.as_mut().expect("stdin should be open");
    writeln!(stdin, "{}", request).expect("should write to stdin");

    let stdout = child.stdout.as_mut().expect("stdout should be open");
    let mut reader = std::io::BufReader::new(stdout);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .expect("should read line from stdout");
    serde_json::from_str(&line).expect("should parse JSON response")
}

// ---------------------------------------------------------------------------
// TypeScript project helpers — shared with part_04.rs (checkNode tests)
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
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

/// Symlink the workspace node_modules into `dir` so the Node sidecar can
/// resolve `zod` without a full `npm install` in the temp directory.
fn link_workspace_node_modules(dir: &Path) {
    let target = workspace_root().join("typescript/schemalint-zod/node_modules");
    assert!(
        target.join("zod").exists(),
        "missing workspace zod dependency at {}; run npm ci first",
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

/// Create a minimal TypeScript project with zod installed.
///
/// Writes `files` into `dir/src/`, creates a tsconfig.json, and symlinks
/// `node_modules` from the workspace so the sidecar can resolve `zod`.
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

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

include!("server_tests/part_01.rs");
include!("server_tests/part_02.rs");
include!("server_tests/part_03.rs");
include!("server_tests/part_04.rs");
