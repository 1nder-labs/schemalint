use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::ingest::DiscoverResponse;

const DISCOVER_TIMEOUT_SECS: u64 = 60;
const SHUTDOWN_TIMEOUT_SECS: u64 = 5;

/// Errors produced by Node helper operations.
#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("npx not found; tried {0}")]
    NotInstalled(String),
    #[error("failed to spawn node helper: {0}")]
    SpawnFailed(String),
    #[error("failed to communicate with node helper: {0}")]
    RequestFailed(String),
    #[error("discover request timed out after {0}s")]
    Timeout(u64),
    #[error("invalid response from node helper: {0}")]
    InvalidResponse(String),
    #[error("discovery failed: {0}")]
    DiscoverFailed(String),
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[serde(default)]
    jsonrpc: Option<String>,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
    #[allow(dead_code)]
    id: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    #[allow(dead_code)]
    code: i64,
    message: String,
}

/// Manages a Node subprocess running the `schemalint-zod` JSON-RPC server.
///
/// The helper is intentionally not `Sync` — it owns a `Child` with piped I/O
/// and should be used sequentially before any parallel processing phase.
pub struct NodeHelper {
    child: Child,
    stdin: ChildStdin,
    request_id: u64,
    stdout_rx: mpsc::Receiver<Option<String>>,
    stderr_lines: Arc<Mutex<Vec<String>>>,
}

impl NodeHelper {
    /// Spawn the Node helper subprocess.
    ///
    /// Resolves `tsx` as the TypeScript runner, then resolves the helper bin
    /// path relative to the workspace. Uses `npx tsx` fallback if `tsx` is not
    /// on PATH. If `node_path` provides an explicit executable, it is used as
    /// the runner with the bin path as the only argument.
    pub fn spawn(node_path: Option<&str>) -> Result<Self, NodeError> {
        let (runner, args): (String, Vec<String>) = if let Some(path) = node_path {
            (path.to_string(), vec![])
        } else {
            let bin = resolve_helper_path().to_string_lossy().to_string();
            let (runner_name, extra_args) = resolve_tsx_cmd()?;
            let mut all_args = extra_args;
            all_args.push(bin);
            (runner_name, all_args)
        };

        let mut cmd = Command::new(&runner);
        cmd.args(&args);
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                let args_display = args.join(" ");
                NodeError::SpawnFailed(format!(
                    "failed to start '{} {}': {}",
                    runner, args_display, e
                ))
            })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| NodeError::SpawnFailed("no stdout pipe available".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| NodeError::SpawnFailed("no stderr pipe available".to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| NodeError::SpawnFailed("no stdin pipe available".to_string()))?;

        // Drain stderr continuously to prevent pipe-buffer deadlock.
        let stderr_lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let stderr_capture = Arc::clone(&stderr_lines);
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        eprintln!("[schemalint-zod] {}", l);
                        if let Ok(mut lines) = stderr_capture.lock() {
                            lines.push(l);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Reader thread for stdout with line-delimited JSON delivery.
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if tx.send(Some(l)).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            let _ = tx.send(None);
        });

        Ok(NodeHelper {
            child,
            stdin,
            request_id: 1,
            stdout_rx: rx,
            stderr_lines,
        })
    }

    /// Send a `discover` request for the given source glob and return discovered models.
    pub fn discover(&mut self, source: &str) -> Result<DiscoverResponse, NodeError> {
        let id = self.request_id;
        self.request_id += 1;

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "discover",
            "params": { "source": source },
            "id": id,
        });
        let request_str = serde_json::to_string(&request)
            .map_err(|e| NodeError::RequestFailed(format!("serialize error: {}", e)))?;

        writeln!(self.stdin, "{}", request_str)
            .map_err(|e| NodeError::RequestFailed(format!("write error: {}", e)))?;
        self.stdin
            .flush()
            .map_err(|e| NodeError::RequestFailed(format!("flush error: {}", e)))?;

        let line = match self
            .stdout_rx
            .recv_timeout(Duration::from_secs(DISCOVER_TIMEOUT_SECS))
        {
            Ok(Some(line)) => line,
            Ok(None) => {
                return Err(self.augment_error(NodeError::InvalidResponse(
                    "helper process closed stdout unexpectedly".to_string(),
                )))
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                return Err(self.augment_error(NodeError::Timeout(DISCOVER_TIMEOUT_SECS)))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(self.augment_error(NodeError::InvalidResponse(
                    "stdout reader thread disconnected".to_string(),
                )))
            }
        };

        let response: JsonRpcResponse = serde_json::from_str(&line).map_err(|e| {
            self.augment_error(NodeError::InvalidResponse(format!(
                "response parse error: {}",
                e
            )))
        })?;

        if let Some(error) = response.error {
            return Err(self.augment_error(NodeError::DiscoverFailed(error.message)));
        }

        if response.jsonrpc.as_deref() != Some("2.0") {
            return Err(self.augment_error(NodeError::InvalidResponse(
                "response missing or has incorrect jsonrpc version".to_string(),
            )));
        }

        let result = response.result.ok_or_else(|| {
            self.augment_error(NodeError::InvalidResponse(
                "response missing result field".to_string(),
            ))
        })?;

        serde_json::from_value(result)
            .map_err(|e| NodeError::InvalidResponse(format!("result parse error: {}", e)))
    }

    /// Drain captured stderr lines and append them to the error message.
    fn augment_error(&self, err: NodeError) -> NodeError {
        let lines: Vec<String> = {
            let mut guard = self.stderr_lines.lock().unwrap_or_else(|e| e.into_inner());
            std::mem::take(&mut *guard)
        };
        if lines.is_empty() {
            return err;
        }
        let stderr_tail = if lines.len() > 10 {
            let tail: Vec<_> = lines.iter().rev().take(10).map(|s| s.as_str()).collect();
            format!(
                "\n--- Node stderr (last {} of {} lines) ---\n{}\n--- end stderr ---",
                10,
                lines.len(),
                tail.into_iter().rev().collect::<Vec<_>>().join("\n")
            )
        } else {
            format!(
                "\n--- Node stderr ---\n{}\n--- end stderr ---",
                lines.join("\n")
            )
        };
        match err {
            NodeError::DiscoverFailed(msg) => {
                NodeError::DiscoverFailed(format!("{}{}", msg, stderr_tail))
            }
            NodeError::InvalidResponse(msg) => {
                NodeError::InvalidResponse(format!("{}{}", msg, stderr_tail))
            }
            NodeError::Timeout(secs) => NodeError::Timeout(secs),
            other => other,
        }
    }

    /// Send a `shutdown` request and wait for the child process to exit.
    pub fn shutdown(&mut self) {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "shutdown",
            "id": self.request_id,
        });
        if let Ok(req) = serde_json::to_string(&request) {
            let _ = writeln!(self.stdin, "{}", req);
            let _ = self.stdin.flush();
        }

        let deadline = Instant::now() + Duration::from_secs(SHUTDOWN_TIMEOUT_SECS);
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = self.child.kill();
                        let _ = self.child.wait();
                        return;
                    }
                    thread::sleep(Duration::from_millis(100));
                }
                Err(_) => {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    return;
                }
            }
        }
    }
}

impl Drop for NodeHelper {
    fn drop(&mut self) {
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                eprintln!("warning: node helper still running, attempting shutdown");
                let request = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "shutdown",
                    "id": self.request_id,
                });
                if let Ok(req) = serde_json::to_string(&request) {
                    let _ = writeln!(self.stdin, "{}", req);
                    let _ = self.stdin.flush();
                }
                let deadline = Instant::now() + Duration::from_secs(2);
                loop {
                    match self.child.try_wait() {
                        Ok(Some(_)) => return,
                        Ok(None) => {
                            if Instant::now() >= deadline {
                                let _ = self.child.kill();
                                let _ = self.child.wait();
                                return;
                            }
                            thread::sleep(Duration::from_millis(100));
                        }
                        Err(_) => {
                            let _ = self.child.kill();
                            let _ = self.child.wait();
                            return;
                        }
                    }
                }
            }
            Err(_) => {
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
        }
    }
}

/// Resolve the `tsx` runner command.
///
/// Returns `(executable, extra_args)` where extra_args are passed before the
/// bin path. Tries `tsx` directly first; falls back to `npx tsx`.
fn resolve_tsx_cmd() -> Result<(String, Vec<String>), NodeError> {
    // Try standalone tsx
    if Command::new("tsx")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
    {
        return Ok(("tsx".to_string(), vec![]));
    }
    // Fall back to npx tsx
    if Command::new("npx")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
    {
        return Ok(("npx".to_string(), vec!["tsx".to_string()]));
    }
    Err(NodeError::NotInstalled(
        "tsx or npx not found — install tsx via: npm install -g tsx".to_string(),
    ))
}

/// Resolve the path to the `schemalint-zod` helper bin entry.
///
/// Returns the absolute path to `typescript/schemalint-zod/bin/schemalint-zod.js`
/// relative to the workspace root.
fn resolve_helper_path() -> std::path::PathBuf {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // CARGO_MANIFEST_DIR = <workspace>/crates/schemalint
    // Workspace root = ../../ from the manifest dir
    let ws_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(std::path::Path::new("."));
    ws_root.join("typescript/schemalint-zod/bin/schemalint-zod.js")
}
