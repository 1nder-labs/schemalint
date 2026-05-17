use super::*;

// ---------------------------------------------------------------------------
// Error paths
// ---------------------------------------------------------------------------

#[test]
fn invalid_toml_syntax() {
    let toml = r#"
name = "test"
version = "1.0"
[structural
"#;

    let err = load(toml.as_bytes()).unwrap_err();
    assert!(
        matches!(err, ProfileError::InvalidToml(_)),
        "expected InvalidToml, got {:?}",
        err
    );
}

#[test]
fn unknown_severity_string() {
    let toml = r#"
name = "test"
version = "1.0"
allOf = "reject"

[structural]
require_object_root = false
"#;

    let err = load(toml.as_bytes()).unwrap_err();
    assert!(
        matches!(err, ProfileError::InvalidSeverity(ref s) if s == "reject"),
        "expected InvalidSeverity, got {:?}",
        err
    );
}

#[test]
fn missing_structural_section() {
    let toml = r#"
name = "test"
version = "1.0"
allOf = "forbid"
"#;

    let err = load(toml.as_bytes()).unwrap_err();
    assert!(
        matches!(err, ProfileError::MissingField("[structural] section")),
        "expected MissingField([structural] section), got {:?}",
        err
    );
}

#[test]
fn missing_name_field() {
    let toml = r#"
version = "1.0"

[structural]
require_object_root = false
"#;

    let err = load(toml.as_bytes()).unwrap_err();
    assert!(
        matches!(err, ProfileError::MissingField("name")),
        "expected MissingField(name), got {:?}",
        err
    );
}
