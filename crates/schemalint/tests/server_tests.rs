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
    reader.read_line(&mut line).expect("should read line from stdout");
    serde_json::from_str(&line).expect("should parse JSON response")
}

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

#[test]
fn server_check_single_profile_json() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "check",
        "params": {
            "schema": {"type": "object", "properties": {"x": {"type": "string"}}, "required": ["x"], "additionalProperties": false},
            "profiles": ["openai.so.2026-04-30"],
            "format": "json"
        },
        "id": 1
    });

    let response = send_request(&mut child, &request.to_string());
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["result"]["success"].as_bool().unwrap());
    assert_eq!(response["id"], 1);

    // Shutdown
    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 2});
    let response = send_request(&mut child, &shutdown.to_string());
    assert_eq!(response["result"], serde_json::Value::Null);

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

#[test]
fn server_check_multi_profile() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "check",
        "params": {
            "schema": {"type": "object", "properties": {"x": {"type": "string"}}, "required": ["x"], "additionalProperties": false},
            "profiles": ["openai.so.2026-04-30", "anthropic.so.2026-04-30"],
            "format": "json"
        },
        "id": 1
    });

    let response = send_request(&mut child, &request.to_string());
    assert!(response["result"]["success"].as_bool().unwrap());

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 2});
    let _response = send_request(&mut child, &shutdown.to_string());

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

// ---------------------------------------------------------------------------
// Error paths
// ---------------------------------------------------------------------------

#[test]
fn server_invalid_jsonrpc_missing_field() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let request = serde_json::json!({"method": "check", "id": 1});
    let response = send_request(&mut child, &request.to_string());
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32600);

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 2});
    let _response = send_request(&mut child, &shutdown.to_string());

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

#[test]
fn server_unknown_method() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let request = serde_json::json!({"jsonrpc": "2.0", "method": "foo", "id": 1});
    let response = send_request(&mut child, &request.to_string());
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32601);

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 2});
    let _response = send_request(&mut child, &shutdown.to_string());

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

#[test]
fn server_check_unknown_profile() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "check",
        "params": {
            "schema": {"type": "string"},
            "profiles": ["nonexistent-profile"]
        },
        "id": 1
    });

    let response = send_request(&mut child, &request.to_string());
    assert!(response["result"]["success"].as_bool() == Some(false));
    assert!(response["result"]["error"].as_str().unwrap().contains("unknown built-in profile"));

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 2});
    let _response = send_request(&mut child, &shutdown.to_string());

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}
