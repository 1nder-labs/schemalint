// ---------------------------------------------------------------------------
// Depth-limit guard: deeply-nested schemas are rejected before any recursive
// work (normalize, traverse) can overflow the stack.
// ---------------------------------------------------------------------------

#[test]
fn server_deeply_nested_schema_rejected_not_crashed() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    // Build a chain {"a":{"a":{"a":...}}} 1_100 levels deep.
    // This is only ~11 KB and ~1 100 nodes, so it passes the byte and node
    // guards — the depth guard must catch it before any recursive processing.
    //
    // Defense-in-depth note: serde_json has its own default recursion limit
    // (~128) for deserializing JSON strings. A >128-deep schema string will
    // hit that limit at the serde_json::from_str call in the server loop and
    // come back as a JSON-RPC parse error (code -32700) before it even reaches
    // handle_check. In that case our count_nodes_bounded depth guard never
    // fires because the Value never gets constructed. Either way — parse-time
    // rejection or our explicit depth check — the server must NOT crash and
    // must return a well-formed error response.
    let depth = 1_100usize;
    let mut schema = serde_json::Value::Object(serde_json::Map::new());
    for _ in 0..depth {
        let mut map = serde_json::Map::new();
        map.insert("a".to_string(), schema);
        schema = serde_json::Value::Object(map);
    }

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "check",
        "params": {
            "schema": schema,
            "profiles": ["openai.so.2026-04-30"]
        },
        "id": 1
    });

    let response = send_request(&mut child, &request.to_string());

    // The server must return a well-formed error — either:
    // (a) a JSON-RPC parse error at the wire level (serde_json recursion limit),
    // (b) a handle_check-level error with success:false (our depth guard).
    // It must NOT crash or hang.
    let is_parse_error = response["error"]["code"].as_i64() == Some(-32700);
    let is_handle_error = response["result"]["success"].as_bool() == Some(false);
    assert!(
        is_parse_error || is_handle_error,
        "over-deep schema must produce a clean error response (parse or handle level); got: {response}"
    );

    if is_parse_error {
        let msg = response["error"]["message"].as_str().unwrap_or("");
        assert!(
            msg.contains("recursion") || msg.contains("Parse error"),
            "parse-level rejection should mention recursion; got: {msg}"
        );
    } else {
        let err = response["result"]["error"].as_str().unwrap_or("");
        assert!(
            err.contains("complexity limits") || err.contains("depth"),
            "handle-level rejection should mention depth or complexity limits; got: {err}"
        );
    }

    // Server must still be alive and well after rejecting the pathological input.
    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 2});
    let _response = send_request(&mut child, &shutdown.to_string());

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success(), "server should exit cleanly after depth rejection");
}

#[test]
fn server_normal_depth_schema_not_rejected() {
    // A legitimately nested schema (e.g. 20 levels) must pass the depth guard.
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let depth = 20usize;
    let mut schema = serde_json::Value::Object(serde_json::Map::new());
    for _ in 0..depth {
        let mut map = serde_json::Map::new();
        map.insert("a".to_string(), schema);
        schema = serde_json::Value::Object(map);
    }

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "check",
        "params": {
            "schema": schema,
            "profiles": ["openai.so.2026-04-30"]
        },
        "id": 1
    });

    let response = send_request(&mut child, &request.to_string());
    // Must not be rejected by the depth guard (success or a rule-level error is fine).
    let result = &response["result"];
    assert!(
        result.is_object(),
        "should get a result object for a normal-depth schema; got: {response}"
    );
    // The guard specifically must not fire — the error, if any, must not mention depth.
    if result["success"].as_bool() == Some(false) {
        let err = result["error"].as_str().unwrap_or("");
        assert!(
            !err.contains("complexity limits") && !err.contains("depth"),
            "normal-depth schema should not be rejected by depth guard; got: {err}"
        );
    }

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 2});
    let _response = send_request(&mut child, &shutdown.to_string());

    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}
