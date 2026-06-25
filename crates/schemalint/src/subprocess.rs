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
use std::io::{BufReader, Read, Write};
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
        // ponytail: per-line byte cap for stderr — lines longer than this are truncated
        // to avoid unbounded heap growth from a chatty sidecar. 1 MiB is generous for
        // any legitimate diagnostic message.
        const STDERR_MAX_LINE_BYTES: usize = 1 << 20; // 1 MiB
        let stderr_lines: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
        let stderr_capture = Arc::clone(&stderr_lines);
        thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            loop {
                let mut buf = Vec::with_capacity(256);
                let mut truncated = false;
                // Read bytes one at a time until '\n', EOF, or cap exceeded.
                let mut byte = [0u8; 1];
                loop {
                    match reader.read(&mut byte) {
                        Ok(0) => {
                            // EOF: flush whatever we have, then exit the outer loop.
                            if !buf.is_empty() {
                                let l = String::from_utf8_lossy(&buf).into_owned();
                                if let Some(prefix) = echo_prefix {
                                    eprintln!("[{}] {}", prefix, l);
                                }
                                let mut lines =
                                    stderr_capture.lock().unwrap_or_else(|e| e.into_inner());
                                if lines.len() >= STDERR_CAP {
                                    lines.pop_front();
                                }
                                lines.push_back(l);
                            }
                            return;
                        }
                        Ok(_) => {
                            if byte[0] == b'\n' {
                                break;
                            }
                            if buf.len() < STDERR_MAX_LINE_BYTES {
                                buf.push(byte[0]);
                            } else {
                                truncated = true;
                                // Keep reading past the cap to drain the line without
                                // buffering any more bytes.
                            }
                        }
                        Err(_) => return,
                    }
                }
                let mut l = String::from_utf8_lossy(&buf).into_owned();
                if truncated {
                    l.push_str("\u{2026}[truncated]");
                }
                if let Some(prefix) = echo_prefix {
                    eprintln!("[{}] {}", prefix, l);
                }
                let mut lines = stderr_capture.lock().unwrap_or_else(|e| e.into_inner());
                if lines.len() >= STDERR_CAP {
                    lines.pop_front();
                }
                lines.push_back(l);
            }
        });

        // Reader thread: deliver stdout lines to the main thread via channel.
        // ponytail: per-line byte cap for stdout JSON-RPC frames — a line exceeding
        // this cannot be a valid framed response and is treated as a protocol error,
        // closing the channel so send_discover fails cleanly instead of hanging.
        const STDOUT_MAX_LINE_BYTES: usize = 1 << 20; // 1 MiB
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut buf = Vec::with_capacity(512);
                let mut over_limit = false;
                let mut byte = [0u8; 1];
                loop {
                    match reader.read(&mut byte) {
                        Ok(0) => {
                            // EOF: send end-of-stream sentinel and exit.
                            let _ = tx.send(None);
                            return;
                        }
                        Ok(_) => {
                            if byte[0] == b'\n' {
                                break;
                            }
                            if buf.len() < STDOUT_MAX_LINE_BYTES {
                                buf.push(byte[0]);
                            } else {
                                over_limit = true;
                                // Keep draining the pipe to unblock the child, but
                                // we will report a protocol error after the newline.
                            }
                        }
                        Err(_) => {
                            // I/O error on stdout — send sentinel so send_discover
                            // fails immediately rather than timing out.
                            let _ = tx.send(None);
                            return;
                        }
                    }
                }
                if over_limit {
                    // Line exceeded cap: unparseable as JSON-RPC — signal protocol error
                    // by closing the channel (drop tx by returning without sending None).
                    // The Disconnected arm in send_discover converts this to InvalidResponse.
                    return;
                }
                let line = String::from_utf8_lossy(&buf).into_owned();
                if tx.send(Some(line)).is_err() {
                    return;
                }
            }
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
