use std::collections::HashMap;
use std::io::{BufRead, Write};

use serde_json::{json, Value};

use crate::cache::Cache;
use crate::profile::Profile;

mod check;
mod discovery;
mod policy;

const MAX_PAYLOAD_BYTES: usize = 10_000_000;

#[derive(Default)]
pub(super) struct ServerState {
    schemas: Cache,
    profiles: HashMap<String, Profile>,
}

/// Run the line-delimited JSON-RPC 2.0 server over stdin/stdout.
pub fn run_server() {
    let mut state = ServerState::default();
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut output = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("error: failed to read line from stdin: {error}");
                break;
            }
        };
        let response = dispatch_line(&line, &mut state);
        let shutdown = response.get("result").is_some_and(Value::is_null)
            && response.get("id").is_some()
            && serde_json::from_str::<Value>(&line)
                .ok()
                .and_then(|request| request.get("method").cloned())
                == Some(json!("shutdown"));
        if writeln!(output, "{response}").is_err() || shutdown {
            break;
        }
    }
}

fn dispatch_line(line: &str, state: &mut ServerState) -> Value {
    if line.len() > MAX_PAYLOAD_BYTES {
        return rpc_error(json!(null), -32600, "Request payload exceeds 10 MB limit");
    }
    let request: Value = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(error) => return rpc_error(json!(null), -32700, &format!("Parse error: {error}")),
    };
    let id = request.get("id").cloned().unwrap_or(json!(null));
    if request.get("jsonrpc") != Some(&json!("2.0")) {
        return rpc_error(
            id,
            -32600,
            "Invalid JSON-RPC request: missing or incorrect jsonrpc field",
        );
    }
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
    match method {
        "check" => rpc_result(id, check::handle(params, state)),
        "checkNode" => rpc_result(id, discovery::handle_node(params, state)),
        "checkPython" => rpc_result(id, discovery::handle_python(params, state)),
        "shutdown" => rpc_result(id, Value::Null),
        "" => rpc_error(id, -32600, "Invalid JSON-RPC request: missing method"),
        other => rpc_error(id, -32601, &format!("Method not found: {other}")),
    }
}

fn rpc_result(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "result": result, "id": id})
}

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "error": {"code": code, "message": message}, "id": id})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_request_does_not_break_following_dispatch() {
        let mut state = ServerState::default();
        assert_eq!(
            dispatch_line("not json", &mut state)["error"]["code"],
            -32700
        );
        let response = dispatch_line(
            r#"{"jsonrpc":"2.0","method":"shutdown","id":2}"#,
            &mut state,
        );
        assert_eq!(response["id"], 2);
    }
}
