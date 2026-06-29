use super::*;

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

#[test]
fn load_valid_profile() {
    let toml = r#"
name = "test"
version = "1.0"
type = "allow"
allOf = "forbid"
uniqueItems = "warn"

[structural]
require_object_root = true
max_object_depth = 10
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert_eq!(profile.name, "test");
    assert_eq!(profile.version, "1.0");
    assert_eq!(profile.keyword_map.len(), 3);
    assert_eq!(profile.keyword_map.get("type"), Some(&Severity::Allow));
    assert_eq!(profile.keyword_map.get("allOf"), Some(&Severity::Forbid));
    assert_eq!(
        profile.keyword_map.get("uniqueItems"),
        Some(&Severity::Warn)
    );
}

#[test]
fn lookup_severity() {
    let toml = r#"
name = "test"
version = "1.0"
allOf = "forbid"
uniqueItems = "warn"
type = "allow"

[structural]
require_object_root = false
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert_eq!(profile.keyword_map.get("allOf"), Some(&Severity::Forbid));
    assert_eq!(
        profile.keyword_map.get("uniqueItems"),
        Some(&Severity::Warn)
    );
    assert_eq!(profile.keyword_map.get("type"), Some(&Severity::Allow));
    assert_eq!(profile.keyword_map.get("not_present"), None);
}

#[test]
fn load_profile_with_comments_and_whitespace() {
    let toml = r#"
# Test profile
name = "test"
version = "1.0"

# Keywords
type = "allow"

[structural]
require_object_root = true
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert_eq!(profile.name, "test");
    assert_eq!(profile.keyword_map.get("type"), Some(&Severity::Allow));
}

#[test]
fn load_restriction_inline_table() {
    let toml = r#"
name = "test"
version = "1.0"
format = { kind = "restricted", allowed = ["date-time", "email"] }

[structural]
require_object_root = false
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert!(profile.restrictions.contains_key("format"));
    let restriction = profile.restrictions.get("format").unwrap();
    assert_eq!(restriction.allowed_values.len(), 2);
    assert_eq!(
        restriction.allowed_values[0],
        serde_json::json!("date-time")
    );
    assert_eq!(restriction.allowed_values[1], serde_json::json!("email"));
}

#[test]
fn load_restriction_array_of_tables() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false

[[restrictions]]
keyword = "format"
allowed = ["date-time", "email"]
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert!(profile.restrictions.contains_key("format"));
    let restriction = profile.restrictions.get("format").unwrap();
    assert_eq!(restriction.allowed_values.len(), 2);
}

#[test]
fn load_structural_limits() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = true
require_additional_properties_false = true
require_all_properties_in_required = true
max_object_depth = 15
max_total_properties = 3000
max_total_enum_values = 500
max_string_length_total = 60000
max_optional_properties = 24
max_union_properties = 16
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert_eq!(
        profile.structural,
        StructuralLimits {
            require_object_root: true,
            require_additional_properties_false: true,
            require_all_properties_in_required: true,
            max_object_depth: 15,
            max_total_properties: 3000,
            max_total_enum_values: 500,
            max_string_length_total: 60000,
            max_optional_properties: 24,
            max_union_properties: 16,
            external_refs: false,
            require_array_items: false,
            forbid_root_any_of: false,
            forbid_root_enum: false,
            forbid_empty_object: false,
            forbid_allof_with_ref: false,
        }
    );
}
