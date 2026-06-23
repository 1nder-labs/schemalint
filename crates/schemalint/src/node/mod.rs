use std::process::{Command, Stdio};

use crate::ingest::DiscoverResponse;
use crate::subprocess::{SubprocessClient, SubprocessError};

mod resolve;

use resolve::{resolve_compiled_helper_path, resolve_helper_path, resolve_tsx_cmd};

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

/// Manages a Node subprocess running the `schemalint-zod` JSON-RPC server.
///
/// The helper is intentionally not `Sync` — it owns a `SubprocessClient` with
/// piped I/O and should be used sequentially before any parallel processing phase.
pub struct NodeHelper {
    client: SubprocessClient,
}

impl NodeHelper {
    /// Spawn the Node helper subprocess.
    ///
    /// Prefers the compiled helper from `typescript/schemalint-zod/dist`.
    /// Falls back to the TypeScript source helper through `tsx` when the
    /// compiled entry is unavailable. If `node_path` provides an explicit
    /// executable, it is used as the runner with the resolved helper entry.
    pub fn spawn(node_path: Option<&str>) -> Result<Self, NodeError> {
        let compiled_bin = resolve_compiled_helper_path();

        let (runner, args): (String, Vec<String>) = if let Some(path) = node_path {
            let source_bin;
            let entry = match compiled_bin.as_ref() {
                Some(compiled) => compiled,
                None => {
                    source_bin = resolve_helper_path()?;
                    &source_bin
                }
            };
            (path.to_string(), vec![entry.to_string_lossy().into_owned()])
        } else if let Some(compiled) = compiled_bin {
            (
                "node".to_string(),
                vec![compiled.to_string_lossy().into_owned()],
            )
        } else {
            let source_bin = resolve_helper_path()?;
            let bin_str = source_bin.to_string_lossy().into_owned();
            let (runner_name, extra_args) = resolve_tsx_cmd()?;
            let mut all_args = extra_args;
            all_args.push(bin_str);
            (runner_name, all_args)
        };

        let mut cmd = Command::new(&runner);
        cmd.args(&args);
        let child = cmd
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

        // Node does not echo stderr lines unconditionally (user project stderr
        // may contain secrets); pass echo_prefix = None.
        let client = SubprocessClient::from_child(child, None, "node").map_err(|e| match e {
            SubprocessError::SpawnFailed(msg) => NodeError::SpawnFailed(msg),
            _ => unreachable!("from_child only returns SpawnFailed"),
        })?;

        Ok(NodeHelper { client })
    }

    /// Send a `discover` request for the given source glob and return discovered models.
    ///
    /// Drains stale responses (from previous timed-out requests) by checking the
    /// `id` field against the request id. Stale lines are silently discarded.
    /// After `MAX_STALE_DRAIN` mismatches, an error is returned to prevent
    /// infinite loops in a corrupted protocol state.
    pub fn discover(&mut self, source: &str) -> Result<DiscoverResponse, NodeError> {
        let params = serde_json::json!({ "source": source });
        let result = self.client.send_discover(params);
        result.map_err(|e| match e {
            // serialize/write/flush: no stderr augmentation
            SubprocessError::RequestFailed(msg) => NodeError::RequestFailed(msg),
            // in-loop errors: augment with stderr context
            SubprocessError::Timeout(secs) => self.augment_error(NodeError::Timeout(secs)),
            SubprocessError::InvalidResponse(msg) => {
                self.augment_error(NodeError::InvalidResponse(msg))
            }
            SubprocessError::DiscoverFailed(msg) => {
                self.augment_error(NodeError::DiscoverFailed(msg))
            }
            SubprocessError::SpawnFailed(msg) => NodeError::SpawnFailed(msg),
        })
    }

    /// Drain captured stderr lines and append them to the error message.
    fn augment_error(&self, err: NodeError) -> NodeError {
        let lines = self.client.take_stderr();
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
            NodeError::Timeout(secs) => {
                eprintln!(
                    "[schemalint-zod] Node stderr during timeout:\n{}",
                    stderr_tail
                );
                NodeError::Timeout(secs)
            }
            other => {
                eprintln!(
                    "[schemalint-zod] Node stderr during error:\n{}",
                    stderr_tail
                );
                other
            }
        }
    }

    /// Send a `shutdown` request and wait for the child process to exit.
    pub fn shutdown(&mut self) {
        self.client.shutdown();
    }
}
