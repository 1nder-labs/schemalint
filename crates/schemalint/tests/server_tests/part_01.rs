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
