use schemalint::profile::{load, ProfileError, Severity, StructuralLimits};

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
    assert_eq!(profile.keyword_map.get("uniqueItems"), Some(&Severity::Warn));
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
    assert_eq!(profile.keyword_map.get("uniqueItems"), Some(&Severity::Warn));
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
    assert_eq!(restriction.allowed_values[0], serde_json::json!("date-time"));
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
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert_eq!(profile.structural, StructuralLimits {
        require_object_root: true,
        require_additional_properties_false: true,
        require_all_properties_in_required: true,
        max_object_depth: 15,
        max_total_properties: 3000,
        max_total_enum_values: 500,
        max_string_length_total: 60000,
    });
}

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

// ---------------------------------------------------------------------------
// Integration: OpenAI built-in profile
// ---------------------------------------------------------------------------

#[test]
fn openai_profile_loads() {
    let bytes = schemalint_profiles::OPENAI_SO_2026_04_30.as_bytes();
    let profile = load(bytes).unwrap();
    assert_eq!(profile.name, "openai.so.2026-04-30");
    assert_eq!(profile.version, "2026-04-30");
}

#[test]
fn openai_profile_has_zero_unknown_for_pydantic_zod_keywords() {
    let bytes = schemalint_profiles::OPENAI_SO_2026_04_30.as_bytes();
    let profile = load(bytes).unwrap();

    // Keywords commonly emitted by Pydantic v2 and zod-to-json-schema.
    let pydantic_zod_keywords = [
        "type",
        "properties",
        "required",
        "additionalProperties",
        "items",
        "prefixItems",
        "minItems",
        "maxItems",
        "uniqueItems",
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "multipleOf",
        "minLength",
        "maxLength",
        "pattern",
        "format",
        "enum",
        "const",
        "anyOf",
        "allOf",
        "oneOf",
        "not",
        "description",
        "title",
        "default",
        "$ref",
        "$defs",
    ];

    for kw in &pydantic_zod_keywords {
        let in_keyword_map = profile.keyword_map.get(kw);
        let in_restrictions = profile.restrictions.contains_key(kw);
        assert!(
            in_keyword_map.is_some() || in_restrictions,
            "keyword '{}' missing from OpenAI profile (not in keyword_map or restrictions)",
            kw
        );
        if let Some(sev) = in_keyword_map {
            assert_ne!(
                *sev,
                Severity::Unknown,
                "keyword '{}' has unknown severity in OpenAI profile",
                kw
            );
        }
    }
}

#[test]
fn openai_profile_restrictions_present() {
    let bytes = schemalint_profiles::OPENAI_SO_2026_04_30.as_bytes();
    let profile = load(bytes).unwrap();

    assert!(profile.restrictions.contains_key("format"));
    assert!(profile.restrictions.contains_key("additionalProperties"));

    let format_restriction = profile.restrictions.get("format").unwrap();
    assert_eq!(format_restriction.allowed_values.len(), 9);

    let ap_restriction = profile.restrictions.get("additionalProperties").unwrap();
    assert_eq!(ap_restriction.allowed_values.len(), 1);
    assert_eq!(ap_restriction.allowed_values[0], serde_json::json!(false));
}
