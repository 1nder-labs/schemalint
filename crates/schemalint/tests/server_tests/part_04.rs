// ---------------------------------------------------------------------------
// checkNode and checkPython JSON-RPC method tests
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// checkNode — end-to-end: schema with a known violation (OAI-K-format-restricted)
//
// Uses a Zod source with z.string().url() which triggers OAI-K-format-restricted
// under the openai.so.2026-04-30 profile. The server is spawned with the temp
// project as its CWD so the Node sidecar resolves tsconfig.json and node_modules
// relative to the project root.
// ---------------------------------------------------------------------------

#[test]
fn server_check_node_forbidden_format_returns_diagnostics() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "schema.ts",
            r#"import { z } from "zod";
export const Bad = z.object({ website: z.string().url() });
"#,
        )],
    );

    let mut child = cmd()
        .current_dir(tmp.path())
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "checkNode",
        "params": {
            "sources": ["src/**/*.ts"],
            "profiles": ["openai.so.2026-04-30"],
            "format": "json"
        },
        "id": 1
    });

    let response = send_request(&mut child, &request.to_string());
    assert_eq!(response["jsonrpc"], "2.0", "unexpected response: {response}");
    assert_eq!(response["id"], 1);

    let result = &response["result"];
    assert!(
        result["success"].as_bool().unwrap_or(false),
        "checkNode should succeed; got: {result}"
    );

    // The url() format must fire at least one error under the openai profile.
    let total_errors = result["total_errors"].as_u64().unwrap_or(0);
    assert!(
        total_errors >= 1,
        "expected at least 1 error (OAI-K-format-restricted) but got {total_errors}; result: {result}"
    );

    // Verify the output text is valid JSON containing the expected diagnostic.
    let output_str = result["output"].as_str().expect("output should be a string");
    let output_json: serde_json::Value =
        serde_json::from_str(output_str).expect("output should be valid JSON");
    let diags = output_json["diagnostics"].as_array().expect("diagnostics array");
    let has_format_error = diags
        .iter()
        .any(|d| d["code"].as_str() == Some("OAI-K-format-restricted"));
    assert!(
        has_format_error,
        "expected OAI-K-format-restricted in diagnostics; got: {output_json}"
    );

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 2});
    let _ = send_request(&mut child, &shutdown.to_string());
    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

// ---------------------------------------------------------------------------
// checkNode — clean schema: no diagnostics
// ---------------------------------------------------------------------------

#[test]
fn server_check_node_clean_schema_zero_errors() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "clean.ts",
            r#"import { z } from "zod";
export const Good = z.object({ name: z.string(), age: z.number() }).strict();
"#,
        )],
    );

    let mut child = cmd()
        .current_dir(tmp.path())
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "checkNode",
        "params": {
            "sources": ["src/**/*.ts"],
            "profiles": ["openai.so.2026-04-30"],
            "format": "json"
        },
        "id": 10
    });

    let response = send_request(&mut child, &request.to_string());
    let result = &response["result"];
    assert!(
        result["success"].as_bool().unwrap_or(false),
        "checkNode should succeed on clean schema; got: {result}"
    );
    // A clean .strict() schema with string/number only should produce 0 format errors.
    let diags_with_format: Vec<_> = {
        let output_str = result["output"].as_str().unwrap_or("{}");
        let output_json: serde_json::Value = serde_json::from_str(output_str).unwrap_or(serde_json::json!({}));
        output_json["diagnostics"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter(|d| {
                        d["code"].as_str() == Some("OAI-K-format-restricted")
                            || d["code"].as_str() == Some("OAI-K-allOf-forbidden")
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    };
    assert!(
        diags_with_format.is_empty(),
        "clean schema should produce no format/allOf errors; got: {diags_with_format:?}"
    );

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 11});
    let _ = send_request(&mut child, &shutdown.to_string());
    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

// ---------------------------------------------------------------------------
// checkNode — missing required params
// ---------------------------------------------------------------------------

#[test]
fn server_check_node_missing_sources_returns_error() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "checkNode",
        "params": {
            "profiles": ["openai.so.2026-04-30"]
        },
        "id": 20
    });

    let response = send_request(&mut child, &request.to_string());
    assert_eq!(response["result"]["success"], false);
    let err = response["result"]["error"].as_str().unwrap_or("");
    assert!(
        err.contains("sources"),
        "error should mention 'sources'; got: {err}"
    );

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 21});
    let _ = send_request(&mut child, &shutdown.to_string());
    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

#[test]
fn server_check_node_missing_profiles_returns_error() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "checkNode",
        "params": {
            "sources": ["src/**/*.ts"]
        },
        "id": 22
    });

    let response = send_request(&mut child, &request.to_string());
    assert_eq!(response["result"]["success"], false);
    let err = response["result"]["error"].as_str().unwrap_or("");
    assert!(
        err.contains("profiles"),
        "error should mention 'profiles'; got: {err}"
    );

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 23});
    let _ = send_request(&mut child, &shutdown.to_string());
    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

// ---------------------------------------------------------------------------
// checkNode — unknown profile returns structured error (not crash)
// ---------------------------------------------------------------------------

#[test]
fn server_check_node_unknown_profile_returns_error() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "checkNode",
        "params": {
            "sources": ["src/**/*.ts"],
            "profiles": ["nonexistent-profile"]
        },
        "id": 24
    });

    let response = send_request(&mut child, &request.to_string());
    assert_eq!(
        response["result"]["success"].as_bool(),
        Some(false),
        "expected success:false for unknown profile; got: {response}"
    );
    let err = response["result"]["error"].as_str().unwrap_or("");
    assert!(
        err.contains("unknown built-in profile") || err.contains("nonexistent-profile"),
        "error should reference the unknown profile; got: {err}"
    );

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 25});
    let _ = send_request(&mut child, &shutdown.to_string());
    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

// ---------------------------------------------------------------------------
// checkNode — invalid format returns structured error
// ---------------------------------------------------------------------------

#[test]
fn server_check_node_invalid_format_returns_error() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "checkNode",
        "params": {
            "sources": ["src/**/*.ts"],
            "profiles": ["openai.so.2026-04-30"],
            "format": "xml"
        },
        "id": 26
    });

    let response = send_request(&mut child, &request.to_string());
    assert_eq!(response["result"]["success"].as_bool(), Some(false));
    let err = response["result"]["error"].as_str().unwrap_or("");
    assert!(
        err.contains("Unknown format"),
        "error should mention 'Unknown format'; got: {err}"
    );

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 27});
    let _ = send_request(&mut child, &shutdown.to_string());
    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

// ---------------------------------------------------------------------------
// checkPython — structured response (tolerant test)
//
// The Python sidecar requires `schemalint_pydantic` to be importable. This
// test does NOT assert pydantic-specific output: it only verifies that the
// server returns a well-formed result object with a boolean `success` field
// and, on failure, a non-empty string `error` field. The server must never
// panic or hang regardless of whether pydantic is installed.
// ---------------------------------------------------------------------------

#[test]
fn server_check_python_returns_structured_response() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "checkPython",
        "params": {
            "packages": ["schemalint_test_nonexistent_pkg"],
            "profiles": ["openai.so.2026-04-30"],
            "format": "json"
        },
        "id": 30
    });

    let response = send_request(&mut child, &request.to_string());
    assert_eq!(response["jsonrpc"], "2.0", "unexpected envelope: {response}");
    assert_eq!(response["id"], 30);

    // The result must be a well-formed object with a boolean success field.
    let result = &response["result"];
    assert!(
        result.is_object(),
        "result must be an object; got: {response}"
    );
    let success = result["success"].as_bool();
    assert!(
        success.is_some(),
        "result must contain a boolean 'success' field; got: {result}"
    );

    // On failure (expected: no pydantic sidecar, or package doesn't exist),
    // the error field must be a non-empty string.
    if success == Some(false) {
        let err = result["error"].as_str().unwrap_or("");
        assert!(
            !err.is_empty(),
            "failure result must include a non-empty 'error' string; got: {result}"
        );
    } else {
        // If somehow it succeeded (pydantic installed, empty package found),
        // verify output is present.
        assert!(
            result["output"].is_string(),
            "success result must include 'output' string; got: {result}"
        );
    }

    // Server must still be alive after this request.
    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 31});
    let shutdown_resp = send_request(&mut child, &shutdown.to_string());
    assert_eq!(shutdown_resp["result"], serde_json::Value::Null);

    let status = child.wait().expect("should exit cleanly");
    assert!(
        status.success(),
        "server should exit cleanly after checkPython"
    );
}

// ---------------------------------------------------------------------------
// checkPython — missing params return structured errors
// ---------------------------------------------------------------------------

#[test]
fn server_check_python_missing_packages_returns_error() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "checkPython",
        "params": {
            "profiles": ["openai.so.2026-04-30"]
        },
        "id": 32
    });

    let response = send_request(&mut child, &request.to_string());
    assert_eq!(response["result"]["success"].as_bool(), Some(false));
    let err = response["result"]["error"].as_str().unwrap_or("");
    assert!(
        err.contains("packages"),
        "error should mention 'packages'; got: {err}"
    );

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 33});
    let _ = send_request(&mut child, &shutdown.to_string());
    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

#[test]
fn server_check_python_missing_profiles_returns_error() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "checkPython",
        "params": {
            "packages": ["my_package"]
        },
        "id": 34
    });

    let response = send_request(&mut child, &request.to_string());
    assert_eq!(response["result"]["success"].as_bool(), Some(false));
    let err = response["result"]["error"].as_str().unwrap_or("");
    assert!(
        err.contains("profiles"),
        "error should mention 'profiles'; got: {err}"
    );

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 35});
    let _ = send_request(&mut child, &shutdown.to_string());
    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

// ---------------------------------------------------------------------------
// checkPython — unknown profile returns structured error (not crash)
// ---------------------------------------------------------------------------

#[test]
fn server_check_python_unknown_profile_returns_error() {
    let mut child = cmd()
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "checkPython",
        "params": {
            "packages": ["my_package"],
            "profiles": ["nonexistent-profile"]
        },
        "id": 36
    });

    let response = send_request(&mut child, &request.to_string());
    assert_eq!(response["result"]["success"].as_bool(), Some(false));
    let err = response["result"]["error"].as_str().unwrap_or("");
    assert!(
        err.contains("unknown built-in profile") || err.contains("nonexistent-profile"),
        "error should reference the unknown profile; got: {err}"
    );

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 37});
    let _ = send_request(&mut child, &shutdown.to_string());
    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}

// ---------------------------------------------------------------------------
// checkNode — server stays alive and handles further requests after checkNode
// ---------------------------------------------------------------------------

#[test]
fn server_check_node_then_check_json_still_works() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "schema.ts",
            r#"import { z } from "zod";
export const Foo = z.object({ x: z.string() }).strict();
"#,
        )],
    );

    let mut child = cmd()
        .current_dir(tmp.path())
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn server");

    // First: a checkNode request
    let node_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "checkNode",
        "params": {
            "sources": ["src/**/*.ts"],
            "profiles": ["openai.so.2026-04-30"],
            "format": "json"
        },
        "id": 40
    });
    let node_resp = send_request(&mut child, &node_req.to_string());
    assert!(
        node_resp["result"]["success"].as_bool().is_some(),
        "checkNode should return a result; got: {node_resp}"
    );

    // Then: a plain check request — server must still be live
    let check_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "check",
        "params": {
            "schema": {"type": "string"},
            "profiles": ["openai.so.2026-04-30"],
            "format": "json"
        },
        "id": 41
    });
    let check_resp = send_request(&mut child, &check_req.to_string());
    assert!(
        check_resp["result"]["success"].as_bool().unwrap_or(false),
        "check should succeed after checkNode; got: {check_resp}"
    );

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 42});
    let _ = send_request(&mut child, &shutdown.to_string());
    let status = child.wait().expect("should exit cleanly");
    assert!(status.success());
}
