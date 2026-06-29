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
// P4: Unknown keys inside [structural] are silently ignored (no deny_unknown_fields)
// ---------------------------------------------------------------------------

/// Guard that StructuralLimits deserialization does NOT use `deny_unknown_fields`.
/// Adding new fields to the struct (or third-party profiles that use keys we
/// don't yet recognise) must not break profile loading.
#[test]
fn structural_unknown_key_is_ignored() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false
some_future_key_not_yet_known = true
another_unknown_integer = 42
"#;

    // Must succeed — unknown keys must be silently ignored, not rejected.
    let profile = load(toml.as_bytes()).unwrap();
    // Known fields still deserialize correctly.
    assert_eq!(profile.structural.require_object_root, false);
}
