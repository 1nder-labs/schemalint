use std::process::{Command, Stdio};
use std::time::Duration;

use crate::subprocess::{probe_command, SubprocessClient, SubprocessError};

// Re-export shared ingestion types for backward compat.
pub use crate::ingest::{DiscoverResponse, DiscoveredModel};

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

/// Manages a Python subprocess running the `schemalint-pydantic` JSON-RPC server.
///
/// The helper is intentionally not `Sync` — it owns a `SubprocessClient` with
/// piped I/O and should be used sequentially before any parallel processing phase.
pub struct PythonHelper {
    client: SubprocessClient,
}

impl PythonHelper {
    /// Spawn the Python helper subprocess.
    ///
    /// Resolves the Python interpreter via `python3` → `python` fallback unless
    /// `python_path` provides an explicit executable.
    pub fn spawn(python_path: Option<&str>) -> Result<Self, PythonError> {
        let python = resolve_python(python_path)?;

        let child = Command::new(&python)
            .args(["-m", "schemalint_pydantic"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                PythonError::SpawnFailed(format!("failed to start '{}': {}", python, e))
            })?;

        // Python echoes each stderr line as `[schemalint-pydantic] <line>`.
        let client = SubprocessClient::from_child(child, Some("schemalint-pydantic"), "python")
            .map_err(|e| match e {
                SubprocessError::SpawnFailed(msg) => PythonError::SpawnFailed(msg),
                _ => unreachable!("from_child only returns SpawnFailed"),
            })?;

        Ok(PythonHelper { client })
    }

    /// Send a `discover` request for the given package and return discovered models.
    pub fn discover(&mut self, package: &str) -> Result<DiscoverResponse, PythonError> {
        let params = serde_json::json!({ "package": package });
        let result = self.client.send_discover(params);
        result.map_err(|e| match e {
            // serialize/write/flush: no stderr augmentation
            SubprocessError::RequestFailed(msg) => PythonError::RequestFailed(msg),
            // in-loop errors: augment with stderr context
            SubprocessError::Timeout(secs) => self.augment_error(PythonError::Timeout(secs)),
            SubprocessError::InvalidResponse(msg) => {
                self.augment_error(PythonError::InvalidResponse(msg))
            }
            SubprocessError::DiscoverFailed(msg) => {
                self.augment_error(PythonError::DiscoverFailed(msg))
            }
            SubprocessError::SpawnFailed(msg) => PythonError::SpawnFailed(msg),
        })
    }

    /// Drain captured stderr lines and append them to the error message.
    fn augment_error(&self, err: PythonError) -> PythonError {
        let lines = self.client.take_stderr();
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
        self.client.shutdown();
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
