use std::path::{Path, PathBuf};
use std::time::Duration;

use super::NodeError;
use crate::subprocess::probe_command;

/// Resolve the `tsx` runner command.
///
/// Returns `(executable, extra_args)` where extra_args are passed before the
/// bin path. Tries `tsx` directly first; falls back to `npx tsx`.
/// Each probe is bounded by a 2-second timeout to prevent hangs.
pub(super) fn resolve_tsx_cmd() -> Result<(String, Vec<String>), NodeError> {
    const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

    if let Some(local_tsx) = resolve_workspace_tsx() {
        let local_tsx = local_tsx.to_string_lossy().into_owned();
        if probe_command(&local_tsx, PROBE_TIMEOUT) {
            return Ok((local_tsx, vec![]));
        }
    }

    if probe_command("tsx", PROBE_TIMEOUT) {
        return Ok(("tsx".to_string(), vec![]));
    }
    if probe_command("npx", PROBE_TIMEOUT) {
        return Ok(("npx".to_string(), vec!["tsx".to_string()]));
    }
    Err(NodeError::NotInstalled(
        "tsx or npx not found - install tsx via: npm install -g tsx".to_string(),
    ))
}

fn resolve_workspace_tsx() -> Option<PathBuf> {
    let ws_root = workspace_root().ok()?;
    let bin_name = if cfg!(windows) { "tsx.cmd" } else { "tsx" };
    let path = ws_root
        .join("npm/schemalint/node_modules/.bin")
        .join(bin_name);
    path.exists().then_some(path)
}

/// Resolve the path to the `schemalint-zod` source helper bin entry.
pub(super) fn resolve_helper_path() -> Result<PathBuf, NodeError> {
    let ws_root = workspace_root()?;
    let bin_path = ws_root.join("npm/schemalint/bin/schemalint-zod.js");
    if !bin_path.exists() {
        return Err(NodeError::SpawnFailed(format!(
            "helper binary not found at '{}' - ensure npm/schemalint is built",
            bin_path.display()
        )));
    }
    Ok(bin_path)
}

pub(super) fn resolve_compiled_helper_path() -> Option<PathBuf> {
    // Explicit override: published distributions (npm) bundle the compiled Zod
    // sidecar and point here via SCHEMALINT_ZOD_HELPER, since the workspace-relative
    // dev path below only exists in the source checkout, not on an end user's machine.
    if let Some(raw) = std::env::var_os("SCHEMALINT_ZOD_HELPER") {
        let path = PathBuf::from(raw);
        if path.exists() {
            return Some(path);
        }
    }
    let ws_root = workspace_root().ok()?;
    let bin_path = ws_root.join("npm/schemalint/dist/main.js");
    bin_path.exists().then_some(bin_path)
}

fn workspace_root() -> Result<PathBuf, NodeError> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            NodeError::SpawnFailed(format!(
                "cannot resolve workspace root from CARGO_MANIFEST_DIR '{}'",
                manifest_dir.display()
            ))
        })
}
