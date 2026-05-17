use super::*;

// ---------------------------------------------------------------------------
// NodeError display formatting
// ---------------------------------------------------------------------------

#[test]
fn node_error_not_installed_display() {
    let err = NodeError::NotInstalled("tsx or npx".into());
    assert!(err.to_string().contains("npx not found"));
    assert!(err.to_string().contains("tsx or npx"));
}

#[test]
fn node_error_spawn_failed_display() {
    let err = NodeError::SpawnFailed("command not found: tsx".into());
    assert!(err.to_string().contains("failed to spawn node helper"));
    assert!(err.to_string().contains("command not found: tsx"));
}

#[test]
fn node_error_timeout_display() {
    let err = NodeError::Timeout(60);
    assert!(err.to_string().contains("timed out after 60s"));
}

#[test]
fn node_error_invalid_response_display() {
    let err = NodeError::InvalidResponse("response parse error: missing field".into());
    assert!(err
        .to_string()
        .contains("invalid response from node helper"));
    assert!(err.to_string().contains("response parse error"));
}

#[test]
fn node_error_discover_failed_display() {
    let err = NodeError::DiscoverFailed("no zod schemas exported".into());
    assert!(err.to_string().contains("discovery failed"));
    assert!(err.to_string().contains("no zod schemas exported"));
}

#[test]
fn node_error_request_failed_display() {
    let err = NodeError::RequestFailed("write error: broken pipe".into());
    assert!(err
        .to_string()
        .contains("failed to communicate with node helper"));
    assert!(err.to_string().contains("broken pipe"));
}
