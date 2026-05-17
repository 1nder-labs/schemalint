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
