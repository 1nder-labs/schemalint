use std::io::{BufRead, Write};
use std::process::{Command, Stdio};

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
// Happy path
// ---------------------------------------------------------------------------

include!("server_tests/part_01.rs");
include!("server_tests/part_02.rs");
