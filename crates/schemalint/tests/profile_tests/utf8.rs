use super::*;

// ---------------------------------------------------------------------------
// Invalid UTF-8
// ---------------------------------------------------------------------------

#[test]
fn invalid_utf8_profile_bytes() {
    let bytes: &[u8] = &[0x80, 0x81, 0x82, 0x83];
    let err = load(bytes).unwrap_err();
    assert!(
        matches!(err, ProfileError::InvalidSeverity(ref s) if s.contains("invalid UTF-8")),
        "expected InvalidSeverity with 'invalid UTF-8', got {:?}",
        err
    );
}
