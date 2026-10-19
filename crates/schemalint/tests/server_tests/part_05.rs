// ---------------------------------------------------------------------------
// CLI / JSON-RPC parity for default-profile selection
// ---------------------------------------------------------------------------

#[test]
fn check_node_rpc_matches_cli_for_multiple_targets() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[
            (
                "openai.ts",
                r#"import { z } from "zod";
import { zodResponseFormat } from "openai/helpers/zod";
const OpenAI = z.object({ website: z.string().url() });
zodResponseFormat(OpenAI, "openai_response");
"#,
            ),
            (
                "anthropic.ts",
                r#"import { z } from "zod";
import { betaZodTool } from "@anthropic-ai/sdk/helpers/beta/zod";
betaZodTool({
  name: "anthropic_tool",
  inputSchema: z.object({ count: z.number().min(1) }),
});
"#,
            ),
        ],
    );

    let cli_output = cmd()
        .current_dir(tmp.path())
        .args(["check-node", "--source", "src/**/*.ts", "--format", "json"])
        .output()
        .unwrap();
    assert!(!cli_output.status.success());
    let cli: serde_json::Value = serde_json::from_slice(&cli_output.stdout).unwrap_or_else(|error| {
        panic!(
            "CLI JSON parse failed: {error}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&cli_output.stdout),
            String::from_utf8_lossy(&cli_output.stderr)
        )
    });

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
        "params": {"sources": ["src/**/*.ts"], "format": "json"},
        "id": 900
    });
    let response = send_request(&mut child, &request.to_string());
    let rpc_result = &response["result"];
    let rpc_output: serde_json::Value =
        serde_json::from_str(rpc_result["output"].as_str().unwrap()).unwrap();

    assert_eq!(rpc_result["report"], cli["report"]);
    assert_eq!(rpc_output["report"], cli["report"]);
    assert_eq!(rpc_output["diagnostics"], cli["diagnostics"]);
    assert_eq!(cli["report"]["coverage"]["status"], "complete");
    assert_eq!(cli["report"]["targets"].as_array().unwrap().len(), 2);
    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 901});
    let _ = send_request(&mut child, &shutdown.to_string());
    assert!(child.wait().unwrap().success());
}

#[test]
fn check_node_rpc_honors_package_json_profiles_like_cli() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "schema.ts",
            r#"import { z } from "zod";
export const Item = z.object({ id: z.string(), note: z.string().optional() });
"#,
        )],
    );
    std::fs::write(
        tmp.path().join("package.json"),
        r#"{"dependencies":{"zod":"^3","openai":"^4","@anthropic-ai/sdk":"^0.30.0"},"schemalint":{"profiles":["anthropic"]}}"#,
    )
    .unwrap();

    let cli_output = cmd()
        .current_dir(tmp.path())
        .args(["check-node", "--source", "src/**/*.ts", "--format", "json"])
        .output()
        .unwrap();
    let cli: serde_json::Value = serde_json::from_slice(&cli_output.stdout).unwrap();

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
        "params": {"sources": ["src/**/*.ts"], "format": "json"},
        "id": 910
    });
    let response = send_request(&mut child, &request.to_string());
    let rpc_output: serde_json::Value =
        serde_json::from_str(response["result"]["output"].as_str().unwrap()).unwrap();

    assert_eq!(cli["profiles"], serde_json::json!(["anthropic.so.2026-04-30"]));
    assert_eq!(rpc_output["profiles"], cli["profiles"]);
    assert_eq!(rpc_output["diagnostics"], cli["diagnostics"]);

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 911});
    let _ = send_request(&mut child, &shutdown.to_string());
    assert!(child.wait().unwrap().success());
}
