use super::*;

// ---------------------------------------------------------------------------
// Structural limits — boundary values
// ---------------------------------------------------------------------------

#[test]
fn structural_limit_zero() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false
max_object_depth = 0
max_total_properties = 0
max_total_enum_values = 0
max_string_length_total = 0
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert_eq!(profile.structural.max_object_depth, 0);
    assert_eq!(profile.structural.max_total_properties, 0);
    assert_eq!(profile.structural.max_total_enum_values, 0);
    assert_eq!(profile.structural.max_string_length_total, 0);
}

#[test]
fn structural_limit_max_u32() {
    let toml = format!(
        r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false
max_object_depth = {}
max_total_properties = {}
max_total_enum_values = {}
max_string_length_total = {}
"#,
        u32::MAX,
        u32::MAX,
        u32::MAX,
        u32::MAX,
    );

    let profile = load(toml.as_bytes()).unwrap();
    assert_eq!(profile.structural.max_object_depth, u32::MAX);
    assert_eq!(profile.structural.max_total_properties, u32::MAX);
    assert_eq!(profile.structural.max_total_enum_values, u32::MAX);
    assert_eq!(profile.structural.max_string_length_total, u32::MAX);
}

#[test]
fn structural_limit_negative_value_errors() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false
max_object_depth = -1
"#;

    let err = load(toml.as_bytes()).unwrap_err();
    // Negative integer should fail TOML parsing on the negative sign,
    // or be treated as an unknown keyword value error.
    // The TOML library may actually parse this but fail to convert from i64.
    match err {
        ProfileError::InvalidToml(_) => {} // likely: unexpected `-`
        ProfileError::InvalidSeverity(ref s) if s.contains("max_object_depth") => {}
        other => panic!("unexpected error for negative limit: {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Unknown structural keys fail closed
// ---------------------------------------------------------------------------

#[test]
fn structural_unknown_key_is_rejected() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false
some_future_key_not_yet_known = true
another_unknown_integer = 42
"#;

    let error = load(toml.as_bytes()).unwrap_err();
    assert!(matches!(error, ProfileError::InvalidToml(_)));
    assert!(error.to_string().contains("some_future_key_not_yet_known"));
}

// ---------------------------------------------------------------------------
// unknown_keyword_policy
// ---------------------------------------------------------------------------

#[test]
fn unknown_keyword_policy_absent_defaults_to_warn() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert_eq!(
        profile.structural.unknown_keyword_policy,
        UnknownKeywordPolicy::Warn
    );
}

#[test]
fn unknown_keyword_policy_parses_allow_warn_forbid() {
    for (value, expected) in [
        ("allow", UnknownKeywordPolicy::Allow),
        ("warn", UnknownKeywordPolicy::Warn),
        ("forbid", UnknownKeywordPolicy::Forbid),
    ] {
        let toml = format!(
            r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false
unknown_keyword_policy = "{value}"
"#
        );
        let profile = load(toml.as_bytes()).unwrap();
        assert_eq!(
            profile.structural.unknown_keyword_policy, expected,
            "unknown_keyword_policy = \"{value}\" did not parse to {expected:?}"
        );
    }
}

#[test]
fn unknown_keyword_policy_invalid_value_errors() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false
unknown_keyword_policy = "strip"
"#;

    let error = load(toml.as_bytes()).unwrap_err();
    assert!(matches!(error, ProfileError::InvalidToml(_)));
}
