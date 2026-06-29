use super::*;

// ---------------------------------------------------------------------------
// Severity parse — additional coverage
// ---------------------------------------------------------------------------

#[test]
fn severity_parse_unknown_literally() {
    assert_eq!(Severity::parse("unknown").unwrap(), Severity::Unknown);
}

#[test]
fn severity_parse_strip() {
    assert_eq!(Severity::parse("strip").unwrap(), Severity::Strip);
}

#[test]
fn severity_parse_forbid() {
    assert_eq!(Severity::parse("forbid").unwrap(), Severity::Forbid);
}

#[test]
fn severity_parse_invalid() {
    let err = Severity::parse("nonsense").unwrap_err();
    assert!(
        matches!(err, ProfileError::InvalidSeverity(ref s) if s == "nonsense"),
        "expected InvalidSeverity('nonsense'), got {:?}",
        err
    );
}
