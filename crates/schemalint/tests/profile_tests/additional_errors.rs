use super::*;

// ---------------------------------------------------------------------------
// Error paths — additional coverage
// ---------------------------------------------------------------------------

#[test]
fn invalid_toml_duplicate_top_level_key() {
    let toml = r#"
name = "test"
version = "1.0"
type = "allow"
type = "forbid"

[structural]
require_object_root = false
"#;

    let err = load(toml.as_bytes()).unwrap_err();
    assert!(
        matches!(err, ProfileError::InvalidToml(_)),
        "expected InvalidToml for duplicate key, got {:?}",
        err
    );
}

#[test]
fn invalid_toml_unclosed_table() {
    let toml = r#"
name = "test"
version = "1.0"

[structural
require_object_root = false
"#;

    let err = load(toml.as_bytes()).unwrap_err();
    assert!(
        matches!(err, ProfileError::InvalidToml(_)),
        "expected InvalidToml for unclosed table, got {:?}",
        err
    );
}

#[test]
fn invalid_toml_duplicate_restriction_keyword() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false

[[restrictions]]
keyword = "format"
allowed = ["date-time"]

[[restrictions]]
keyword = "format"
allowed = ["email"]
"#;

    // Second entry overwrites first in HashMap — this is valid TOML and should parse.
    let profile = load(toml.as_bytes()).unwrap();
    let r = profile.restrictions.get("format").unwrap();
    assert_eq!(r.allowed_values.len(), 1);
    assert_eq!(r.allowed_values[0], serde_json::json!("email"));
}

#[test]
fn empty_string_profile_errors() {
    // An empty byte array parses as an empty TOML table (valid TOML),
    // but it lacks the required `name` field.
    let err = load(b"").unwrap_err();
    assert!(
        matches!(err, ProfileError::MissingField("name")),
        "expected MissingField(name) for empty profile, got {:?}",
        err
    );
}

#[test]
fn missing_name_field_nested() {
    let toml = r#"
[profile]
name = "test"

[profile.structural]
require_object_root = false
"#;

    // The root-level "name" is missing; profile.name is in a sub-table.
    let err = load(toml.as_bytes()).unwrap_err();
    assert!(
        matches!(err, ProfileError::MissingField("name")),
        "expected MissingField(name), got {:?}",
        err
    );
}

#[test]
fn unknown_keyword_in_profile_body() {
    let toml = r#"
name = "test"
version = "1.0"
nonsense = "allow"

[structural]
require_object_root = false
"#;

    let err = load(toml.as_bytes()).unwrap_err();
    assert!(
        matches!(err, ProfileError::UnknownKeyword(ref keyword) if keyword == "nonsense"),
        "expected UnknownKeyword(nonsense), got {:?}",
        err
    );
}

#[test]
fn invalid_value_for_keyword_integer() {
    let toml = r#"
name = "test"
version = "1.0"
type = 42

[structural]
require_object_root = false
"#;

    let err = load(toml.as_bytes()).unwrap_err();
    assert!(
        matches!(err, ProfileError::InvalidKeywordValue(ref keyword) if keyword == "type"),
        "expected InvalidKeywordValue(type), got {:?}",
        err
    );
}

#[test]
fn invalid_value_for_keyword_array() {
    let toml = r#"
name = "test"
version = "1.0"
type = ["allow", "warn"]

[structural]
require_object_root = false
"#;

    let err = load(toml.as_bytes()).unwrap_err();
    assert!(
        matches!(err, ProfileError::InvalidKeywordValue(ref keyword) if keyword == "type"),
        "expected InvalidKeywordValue(type), got {:?}",
        err
    );
}

#[test]
fn restricted_table_missing_allowed() {
    let toml = r#"
name = "test"
version = "1.0"
format = { kind = "restricted" }

[structural]
require_object_root = false
"#;

    let err = load(toml.as_bytes()).unwrap_err();
    assert!(
        matches!(err, ProfileError::InvalidRestriction(ref k) if k == "format"),
        "expected InvalidRestriction for format, got {:?}",
        err
    );
}

#[test]
fn restrictions_array_entry_missing_keyword() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false

[[restrictions]]
allowed = ["date-time"]
"#;

    let err = load(toml.as_bytes()).unwrap_err();
    assert!(
        matches!(err, ProfileError::MissingField("restrictions.keyword")),
        "expected MissingField(restrictions.keyword), got {:?}",
        err
    );
}

#[test]
fn restrictions_array_entry_missing_allowed() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false

[[restrictions]]
keyword = "format"
"#;

    let err = load(toml.as_bytes()).unwrap_err();
    assert!(
        matches!(err, ProfileError::InvalidRestriction(ref k) if k == "format"),
        "expected InvalidRestriction for format, got {:?}",
        err
    );
}

#[test]
fn restrictions_not_an_array_is_rejected() {
    let toml = r#"
name = "test"
version = "1.0"
restrictions = "not an array of tables"

[structural]
require_object_root = false
"#;

    assert!(matches!(
        load(toml.as_bytes()).unwrap_err(),
        ProfileError::InvalidRestrictionsContainer
    ));
}

#[test]
fn restriction_with_unknown_keyword_is_rejected_during_load() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false

[[restrictions]]
keyword = "typoKeyword"
allowed = [true]
"#;

    assert!(matches!(
        load(toml.as_bytes()).unwrap_err(),
        ProfileError::UnknownKeyword(ref keyword) if keyword == "typoKeyword"
    ));
}

#[test]
fn severity_and_restriction_for_same_keyword_conflict() {
    let toml = r#"
name = "test"
version = "1.0"
type = "forbid"

[structural]
require_object_root = false

[[restrictions]]
keyword = "type"
allowed = ["object"]
"#;

    assert!(matches!(
        load(toml.as_bytes()).unwrap_err(),
        ProfileError::ConflictingKeyword(ref keyword) if keyword == "type"
    ));
}
