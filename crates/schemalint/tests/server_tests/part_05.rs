// ---------------------------------------------------------------------------
// CLI / JSON-RPC parity for automatic mixed-provider ownership
// ---------------------------------------------------------------------------

#[test]
fn check_node_rpc_matches_cli_for_mixed_provider_targets() {
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
    assert!(cli["report"]["targets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|target| target["provider"]["provider"] == "openai"));
    assert!(cli["report"]["targets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|target| target["provider"]["provider"] == "anthropic"));

    let shutdown = serde_json::json!({"jsonrpc": "2.0", "method": "shutdown", "id": 901});
    let _ = send_request(&mut child, &shutdown.to_string());
    assert!(child.wait().unwrap().success());
}
