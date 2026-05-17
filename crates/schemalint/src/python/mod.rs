use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::Deserialize;

// Re-export shared ingestion types for backward compat.
pub use crate::ingest::{DiscoverResponse, DiscoveredModel};

const DISCOVER_TIMEOUT_SECS: u64 = 60;
const SHUTDOWN_TIMEOUT_SECS: u64 = 5;

/// Errors produced by Python helper operations.
#[derive(Debug, thiserror::Error)]
pub enum PythonError {
    #[error("python interpreter not found: tried {0}")]
    NotInstalled(String),
    #[error("failed to spawn python helper: {0}")]
    SpawnFailed(String),
    #[error("failed to communicate with python helper: {0}")]
    RequestFailed(String),
    #[error("discover request timed out after {0}s")]
    Timeout(u64),
    #[error("invalid response from python helper: {0}")]
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

/// Manages a Python subprocess running the `schemalint-pydantic` JSON-RPC server.
///
/// The helper is intentionally not `Sync` — it owns a `Child` with piped I/O
/// and should be used sequentially before any parallel processing phase.
pub struct PythonHelper {
    child: Child,
    stdin: ChildStdin,
    request_id: u64,
    stdout_rx: mpsc::Receiver<Option<String>>,
    stderr_lines: Arc<Mutex<Vec<String>>>,
}

impl PythonHelper {
    /// Spawn the Python helper subprocess.
    ///
    /// Resolves the Python interpreter via `python3` → `python` fallback unless
    /// `python_path` provides an explicit executable.
    pub fn spawn(python_path: Option<&str>) -> Result<Self, PythonError> {
        let python = resolve_python(python_path)?;

        let mut child = Command::new(&python)
            .args(["-m", "schemalint_pydantic"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                PythonError::SpawnFailed(format!("failed to start '{}': {}", python, e))
            })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| PythonError::SpawnFailed("no stdout pipe available".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| PythonError::SpawnFailed("no stderr pipe available".to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| PythonError::SpawnFailed("no stdin pipe available".to_string()))?;

        // Drain stderr continuously to prevent pipe-buffer deadlock.
        // Capture lines for inclusion in error messages on failure.
        let stderr_lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let stderr_capture = Arc::clone(&stderr_lines);
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        eprintln!("[schemalint-pydantic] {}", l);
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

        Ok(PythonHelper {
            child,
            stdin,
            request_id: 1,
            stdout_rx: rx,
            stderr_lines,
        })
    }

    /// Send a `discover` request for the given package and return discovered models.
    pub fn discover(&mut self, package: &str) -> Result<DiscoverResponse, PythonError> {
        let id = self.request_id;
        self.request_id += 1;

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "discover",
            "params": { "package": package },
            "id": id,
        });
        let request_str = serde_json::to_string(&request)
            .map_err(|e| PythonError::RequestFailed(format!("serialize error: {}", e)))?;

        writeln!(self.stdin, "{}", request_str)
            .map_err(|e| PythonError::RequestFailed(format!("write error: {}", e)))?;
        self.stdin
            .flush()
            .map_err(|e| PythonError::RequestFailed(format!("flush error: {}", e)))?;

        const MAX_STALE_DRAIN: usize = 4;

        for _ in 0..=MAX_STALE_DRAIN {
            let line = match self
                .stdout_rx
                .recv_timeout(Duration::from_secs(DISCOVER_TIMEOUT_SECS))
            {
                Ok(Some(line)) => line,
                Ok(None) => {
                    return Err(self.augment_error(PythonError::InvalidResponse(
                        "helper process closed stdout unexpectedly".to_string(),
                    )))
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(self.augment_error(PythonError::Timeout(DISCOVER_TIMEOUT_SECS)))
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(self.augment_error(PythonError::InvalidResponse(
                        "stdout reader thread disconnected".to_string(),
                    )))
                }
            };

            let response: JsonRpcResponse = serde_json::from_str(&line).map_err(|e| {
                self.augment_error(PythonError::InvalidResponse(format!(
                    "response parse error: {}",
                    e
                )))
            })?;

            if response.id != Some(id) {
                // Stale response from a previous timed-out request — drain and retry.
                continue;
            }

            if let Some(error) = response.error {
                return Err(self.augment_error(PythonError::DiscoverFailed(error.message)));
            }

            if response.jsonrpc.as_deref() != Some("2.0") {
                return Err(self.augment_error(PythonError::InvalidResponse(
                    "response missing or has incorrect jsonrpc version".to_string(),
                )));
            }

            let result = response.result.ok_or_else(|| {
                self.augment_error(PythonError::InvalidResponse(
                    "response missing result field".to_string(),
                ))
            })?;

            return serde_json::from_value(result).map_err(|e| {
                self.augment_error(PythonError::InvalidResponse(format!(
                    "result parse error: {}",
                    e
                )))
            });
        }

        Err(self.augment_error(PythonError::InvalidResponse(
            "too many stale responses — helper may be in a corrupted state".to_string(),
        )))
    }

    /// Drain captured stderr lines and append them to the error message.
    fn augment_error(&self, err: PythonError) -> PythonError {
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
                "\n--- Python stderr (last {} of {} lines) ---\n{}\n--- end stderr ---",
                10,
                lines.len(),
                tail.into_iter().rev().collect::<Vec<_>>().join("\n")
            )
        } else {
            format!(
                "\n--- Python stderr ---\n{}\n--- end stderr ---",
                lines.join("\n")
            )
        };
        match err {
            PythonError::DiscoverFailed(msg) => {
                PythonError::DiscoverFailed(format!("{}{}", msg, stderr_tail))
            }
            PythonError::InvalidResponse(msg) => {
                PythonError::InvalidResponse(format!("{}{}", msg, stderr_tail))
            }
            PythonError::Timeout(secs) => PythonError::Timeout(secs),
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

impl Drop for PythonHelper {
    fn drop(&mut self) {
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                eprintln!("warning: python helper still running, attempting shutdown");
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

fn resolve_python(python_path: Option<&str>) -> Result<String, PythonError> {
    if let Some(path) = python_path {
        return Ok(path.to_string());
    }
    let candidates = ["python3", "python"];
    let mut tried = String::new();
    const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
    for candidate in &candidates {
        if !tried.is_empty() {
            tried.push_str(", ");
        }
        tried.push_str(candidate);
        if probe_command(candidate, PROBE_TIMEOUT) {
            return Ok(candidate.to_string());
        }
    }
    Err(PythonError::NotInstalled(tried))
}

/// Check whether a command is available on PATH with a bounded timeout.
fn probe_command(cmd: &str, timeout: Duration) -> bool {
    match Command::new(cmd)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            let start = Instant::now();
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => return status.success(),
                    Ok(None) => {
                        if Instant::now() - start >= timeout {
                            let _ = child.kill();
                            let _ = child.wait();
                            return false;
                        }
                        thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => return false,
                }
            }
        }
        Err(_) => false,
    }
}
