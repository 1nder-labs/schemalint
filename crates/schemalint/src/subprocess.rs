//! Shared subprocess plumbing for JSON-RPC sidecar helpers (Node and Python).
//!
//! This module provides:
//! - `probe_command`: bounded-timeout PATH availability check (used by both
//!   `node::resolve` and `python::mod`).
//! - `JsonRpcResponse` / `JsonRpcError`: wire-level deserialization types shared
//!   by both helpers.
//! - `DISCOVER_TIMEOUT_SECS` / `SHUTDOWN_TIMEOUT_SECS`: protocol timeout consts.
//! - `SubprocessClient`: owns a spawned child process with piped stdio and exposes
//!   the common request/discover/shutdown lifecycle. Thin wrappers (`NodeHelper`,
//!   `PythonHelper`) supply command-resolution and error-type mapping.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::ingest::DiscoverResponse;

// ── Protocol timeouts ────────────────────────────────────────────────────────

pub(crate) const DISCOVER_TIMEOUT_SECS: u64 = 60;
pub(crate) const SHUTDOWN_TIMEOUT_SECS: u64 = 5;

// ── Wire types ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct JsonRpcResponse {
    #[serde(default)]
    pub jsonrpc: Option<String>,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
    pub id: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JsonRpcError {
    #[allow(dead_code)]
    pub code: i64,
    pub message: String,
}

// ── probe_command ─────────────────────────────────────────────────────────────

/// Check whether a command is available on PATH with a bounded timeout.
pub(crate) fn probe_command(cmd: &str, timeout: Duration) -> bool {
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

// ── SubprocessError ───────────────────────────────────────────────────────────

/// Internal error type used by `SubprocessClient`. Callers map to their own
/// public error type (`NodeError` / `PythonError`) to preserve observable API.
#[derive(Debug)]
pub(crate) enum SubprocessError {
    SpawnFailed(String),
    RequestFailed(String),
    Timeout(u64),
    InvalidResponse(String),
    DiscoverFailed(String),
}

// ── SubprocessClient ──────────────────────────────────────────────────────────

/// Low-level subprocess manager: owns the child process, its piped stdio
/// handles, and the background threads that drain stderr/stdout.
///
/// Both `NodeHelper` and `PythonHelper` compose this instead of duplicating
/// the plumbing. They retain responsibility for:
/// - spawning the concrete command and constructing a `SubprocessClient` via
///   `SubprocessClient::from_child`.
/// - mapping `SubprocessError` → their own public error type.
/// - implementing `augment_error` with the appropriate stderr header/labels.
pub(crate) struct SubprocessClient {
    pub child: Child,
    pub stdin: ChildStdin,
    pub request_id: u64,
    pub stdout_rx: mpsc::Receiver<Option<String>>,
    pub stderr_lines: Arc<Mutex<VecDeque<String>>>,
    /// Human-readable name used in the `Drop` warning message ("node" / "python").
    name: &'static str,
}

impl SubprocessClient {
    /// Wire up the background I/O threads for an already-spawned `child`.
    ///
    /// `echo_prefix` — if `Some("prefix")`, each stderr line is echoed to the
    /// process stderr as `[prefix] <line>` (Python does this; Node does not).
    pub(crate) fn from_child(
        mut child: Child,
        echo_prefix: Option<&'static str>,
        name: &'static str,
    ) -> Result<Self, SubprocessError> {
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| SubprocessError::SpawnFailed("no stdout pipe available".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| SubprocessError::SpawnFailed("no stderr pipe available".to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| SubprocessError::SpawnFailed("no stdin pipe available".to_string()))?;

        // Drain stderr continuously to prevent pipe-buffer deadlock.
        // ponytail: keep last 1000 stderr lines; raise if a helper legitimately needs more
        const STDERR_CAP: usize = 1000;
        let stderr_lines: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
        let stderr_capture = Arc::clone(&stderr_lines);
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if let Some(prefix) = echo_prefix {
                            eprintln!("[{}] {}", prefix, l);
                        }
                        let mut lines = stderr_capture.lock().unwrap_or_else(|e| e.into_inner());
                        if lines.len() >= STDERR_CAP {
                            lines.pop_front();
                        }
                        lines.push_back(l);
                    }
                    Err(_) => break,
                }
            }
        });

        // Reader thread: deliver stdout lines to the main thread via channel.
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

        Ok(SubprocessClient {
            child,
            stdin,
            request_id: 1,
            stdout_rx: rx,
            stderr_lines,
            name,
        })
    }

    /// Drain captured stderr lines and return them in order (clears the buffer).
    pub(crate) fn take_stderr(&self) -> Vec<String> {
        let mut guard = self.stderr_lines.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *guard).into()
    }

    /// Send a JSON-RPC `discover` request and return the raw parsed response.
    ///
    /// The `params` object is caller-supplied so each helper can use its own
    /// parameter name ("source" for Node, "package" for Python).
    ///
    /// Returns `Err(RequestFailed)` without stderr augmentation for
    /// serialize/write/flush failures; all in-loop errors (Timeout,
    /// InvalidResponse, DiscoverFailed) are returned as-is for the caller to
    /// augment with stderr context.
    pub(crate) fn send_discover(
        &mut self,
        params: serde_json::Value,
    ) -> Result<DiscoverResponse, SubprocessError> {
        let id = self.request_id;
        self.request_id += 1;

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "discover",
            "params": params,
            "id": id,
        });
        let request_str = serde_json::to_string(&request)
            .map_err(|e| SubprocessError::RequestFailed(format!("serialize error: {}", e)))?;

        writeln!(self.stdin, "{}", request_str)
            .map_err(|e| SubprocessError::RequestFailed(format!("write error: {}", e)))?;
        self.stdin
            .flush()
            .map_err(|e| SubprocessError::RequestFailed(format!("flush error: {}", e)))?;

        const MAX_STALE_DRAIN: usize = 4;

        for _ in 0..=MAX_STALE_DRAIN {
            let line = match self
                .stdout_rx
                .recv_timeout(Duration::from_secs(DISCOVER_TIMEOUT_SECS))
            {
                Ok(Some(line)) => line,
                Ok(None) => {
                    return Err(SubprocessError::InvalidResponse(
                        "helper process closed stdout unexpectedly".to_string(),
                    ))
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(SubprocessError::Timeout(DISCOVER_TIMEOUT_SECS))
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(SubprocessError::InvalidResponse(
                        "stdout reader thread disconnected".to_string(),
                    ))
                }
            };

            let response: JsonRpcResponse = serde_json::from_str(&line).map_err(|e| {
                SubprocessError::InvalidResponse(format!("response parse error: {}", e))
            })?;

            if response.id != Some(id) {
                // Stale response from a previous timed-out request — drain and retry.
                continue;
            }

            if let Some(error) = response.error {
                return Err(SubprocessError::DiscoverFailed(error.message));
            }

            if response.jsonrpc.as_deref() != Some("2.0") {
                return Err(SubprocessError::InvalidResponse(
                    "response missing or has incorrect jsonrpc version".to_string(),
                ));
            }

            let result = response.result.ok_or_else(|| {
                SubprocessError::InvalidResponse("response missing result field".to_string())
            })?;

            return serde_json::from_value(result).map_err(|e| {
                SubprocessError::InvalidResponse(format!("result parse error: {}", e))
            });
        }

        Err(SubprocessError::InvalidResponse(
            "too many stale responses \u{2014} helper may be in a corrupted state".to_string(),
        ))
    }

    /// Send a `shutdown` request and wait up to `SHUTDOWN_TIMEOUT_SECS` for the
    /// child to exit, killing it if it does not.
    pub(crate) fn shutdown(&mut self) {
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

impl Drop for SubprocessClient {
    fn drop(&mut self) {
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                eprintln!(
                    "warning: {} helper still running, attempting shutdown",
                    self.name
                );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_command_returns_false_for_absent_binary() {
        // A guaranteed-absent binary name — must not exist on any real PATH.
        assert!(!probe_command(
            "schemalint-definitely-not-a-real-binary-xyz",
            Duration::from_millis(200),
        ));
    }

    #[test]
    fn stderr_cap_evicts_oldest_entry() {
        // Pure logic test: verify the cap eviction behaviour without spawning a subprocess.
        const STDERR_CAP: usize = 1000;
        let mut lines: VecDeque<String> = VecDeque::new();
        for i in 0..=STDERR_CAP {
            if lines.len() >= STDERR_CAP {
                lines.pop_front();
            }
            lines.push_back(format!("line-{}", i));
        }
        // After inserting 1001 entries the buffer must be exactly at the cap.
        assert_eq!(lines.len(), STDERR_CAP);
        // The very first entry ("line-0") must have been evicted.
        assert_ne!(lines.front().map(String::as_str), Some("line-0"));
        // The newest entry must be present at the back.
        assert_eq!(lines.back().map(String::as_str), Some("line-1000"));
    }
}
