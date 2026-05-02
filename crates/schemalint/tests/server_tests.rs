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
    assert!(response["result"]["error"]
        .as_str()
        .unwrap()
        .contains("unknown built-in profile"));

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 2});
    let _response = send_request(&mut child, &shutdown.to_string());

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

// ---------------------------------------------------------------------------
// Malformed JSON / parse error
// ---------------------------------------------------------------------------

#[test]
fn server_malformed_json_returns_parse_error_and_stays_alive() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let response = send_request(&mut child, "this is not valid json {{{");
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32700);
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Parse error"));

    // Server must still be alive — send a valid request
    let ok = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 1});
    let response = send_request(&mut child, &ok.to_string());
    assert_eq!(response["result"], serde_json::Value::Null);

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

#[test]
fn server_malformed_json_then_valid_request_stays_alive() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let response = send_request(&mut child, "42");
    assert!(response.get("error").is_some() || response.get("jsonrpc").is_some());

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 1});
    let response = send_request(&mut child, &shutdown.to_string());
    assert_eq!(response["result"], serde_json::Value::Null);

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

// ---------------------------------------------------------------------------
// Missing / invalid method
// ---------------------------------------------------------------------------

#[test]
fn server_missing_method_field() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let request = serde_json::json!({"jsonrpc": "2.0", "id": 1});
    let response = send_request(&mut child, &request.to_string());
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32600);
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("missing method"));

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 2});
    let _response = send_request(&mut child, &shutdown.to_string());

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

// ---------------------------------------------------------------------------
// Missing params fields in check
// ---------------------------------------------------------------------------

#[test]
fn server_check_missing_schema_param() {
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
        "params": {"profiles": ["openai.so.2026-04-30"]},
        "id": 1
    });
    let response = send_request(&mut child, &request.to_string());
    assert_eq!(response["result"]["success"], false);
    assert!(response["result"]["error"]
        .as_str()
        .unwrap()
        .contains("Missing 'schema' parameter"));

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 2});
    let _response = send_request(&mut child, &shutdown.to_string());

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

#[test]
fn server_check_missing_profiles_param() {
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
        "params": {"schema": {"type": "string"}},
        "id": 1
    });
    let response = send_request(&mut child, &request.to_string());
    assert_eq!(response["result"]["success"], false);
    assert!(response["result"]["error"]
        .as_str()
        .unwrap()
        .contains("Missing 'profiles' parameter"));

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 2});
    let _response = send_request(&mut child, &shutdown.to_string());

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

// ---------------------------------------------------------------------------
// Invalid schema JSON
// ---------------------------------------------------------------------------

#[test]
fn server_check_invalid_schema_root_type() {
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
            "schema": [1, 2, 3],
            "profiles": ["openai.so.2026-04-30"]
        },
        "id": 1
    });
    let response = send_request(&mut child, &request.to_string());
    assert_eq!(response["result"]["success"], false);
    assert!(response["result"]["error"]
        .as_str()
        .unwrap()
        .contains("Normalization failed"));

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 2});
    let _response = send_request(&mut child, &shutdown.to_string());

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

#[test]
fn server_check_schema_number_root_type() {
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
            "schema": 3.14,
            "profiles": ["openai.so.2026-04-30"]
        },
        "id": 1
    });
    let response = send_request(&mut child, &request.to_string());
    assert_eq!(response["result"]["success"], false);
    let err = response["result"]["error"].as_str().unwrap();
    assert!(err.contains("Normalization failed"), "got: {err}");

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 2});
    let _response = send_request(&mut child, &shutdown.to_string());

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

// ---------------------------------------------------------------------------
// Invalid format
// ---------------------------------------------------------------------------

#[test]
fn server_check_invalid_format() {
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
            "profiles": ["openai.so.2026-04-30"],
            "format": "xml"
        },
        "id": 1
    });
    let response = send_request(&mut child, &request.to_string());
    assert_eq!(response["result"]["success"], false);
    let err = response["result"]["error"].as_str().unwrap();
    assert!(err.contains("Unknown format"), "got: {err}");

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 2});
    let _response = send_request(&mut child, &shutdown.to_string());

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

// ---------------------------------------------------------------------------
// Large payload
// ---------------------------------------------------------------------------

#[test]
fn server_large_schema_payload_processed() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let mut props = serde_json::Map::new();
    for i in 0..5000 {
        props.insert(
            format!("prop_{:04}", i),
            serde_json::json!({"type": "string"}),
        );
    }
    let mut schema = serde_json::Map::new();
    schema.insert("type".to_string(), serde_json::json!("object"));
    schema.insert("properties".to_string(), serde_json::Value::Object(props));

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "check",
        "params": {
            "schema": schema,
            "profiles": ["openai.so.2026-04-30"]
        },
        "id": 1
    });

    let req_str = request.to_string();
    assert!(
        req_str.len() > 100_000,
        "expected request >100KB, got {} bytes",
        req_str.len()
    );

    let response = send_request(&mut child, &req_str);
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["result"].is_object());

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 2});
    let _response = send_request(&mut child, &shutdown.to_string());

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

#[test]
fn server_oversized_payload_boundary() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let huge = "x".repeat(10_000_001);
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "check",
        "params": {"huge": huge},
        "id": 1
    });

    let stdin = child.stdin.as_mut().expect("stdin should be open");
    writeln!(stdin, "{}", request).expect("should write to stdin");

    let stdout = child.stdout.as_mut().expect("stdout should be open");
    let mut reader = std::io::BufReader::new(stdout);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .expect("should read response line");
    let response: serde_json::Value =
        serde_json::from_str(&line).expect("should parse error response");
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32600);

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 2});
    let _response = send_request(&mut child, &shutdown.to_string());

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

// ---------------------------------------------------------------------------
// Dedicated shutdown test
// ---------------------------------------------------------------------------

#[test]
fn server_shutdown_clean_exit() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 1});
    let response = send_request(&mut child, &shutdown.to_string());
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["result"], serde_json::Value::Null);
    assert_eq!(response["id"], 1);

    let status = child.wait().expect("should exit cleanly");
    assert_eq!(status.code(), Some(0), "shutdown should exit 0");
}

// ---------------------------------------------------------------------------
// Multi-request stress: error recovery across multiple requests
// ---------------------------------------------------------------------------

#[test]
fn server_multiple_errors_then_success() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let r1 = send_request(&mut child, "garbage");
    assert_eq!(r1["error"]["code"], -32700);

    let r2 = send_request(&mut child, r#"{"jsonrpc":"2.0","method":"bogus","id":2}"#);
    assert_eq!(r2["error"]["code"], -32601);

    let r3 = send_request(
        &mut child,
        r#"{"jsonrpc":"2.0","method":"check","params":{"profiles":["openai.so.2026-04-30"]},"id":3}"#,
    );
    assert_eq!(r3["result"]["success"], false);

    let r4 = send_request(
        &mut child,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "method": "check",
            "params": {
                "schema": {"type": "string"},
                "profiles": ["openai.so.2026-04-30"]
            },
            "id": 4
        })
        .to_string(),
    );
    assert!(r4["result"]["success"].as_bool().unwrap());

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 5});
    let _response = send_request(&mut child, &shutdown.to_string());

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}
