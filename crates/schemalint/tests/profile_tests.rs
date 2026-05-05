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
            external_refs: false,
        }
    );
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
        // "oneOf" intentionally omitted — OpenAI docs are silent, so severity is Unknown (R10).
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

#[test]
fn openai_profile_corrections() {
    let bytes = schemalint_profiles::OPENAI_SO_2026_04_30.as_bytes();
    let profile = load(bytes).unwrap();

    assert_eq!(profile.code_prefix, "OAI");
    assert_eq!(profile.structural.max_object_depth, 10);
    assert_eq!(profile.keyword_map.get("oneOf"), Some(&Severity::Unknown));
}

// ---------------------------------------------------------------------------
// Integration: Anthropic built-in profile
// ---------------------------------------------------------------------------

#[test]
fn anthropic_profile_loads() {
    let bytes = schemalint_profiles::ANTHROPIC_SO_2026_04_30.as_bytes();
    let profile = load(bytes).unwrap();
    assert_eq!(profile.name, "anthropic.so.2026-04-30");
    assert_eq!(profile.version, "2026-04-30");
}

#[test]
fn anthropic_profile_values() {
    let bytes = schemalint_profiles::ANTHROPIC_SO_2026_04_30.as_bytes();
    let profile = load(bytes).unwrap();

    assert_eq!(profile.code_prefix, "ANT");
    assert_eq!(profile.keyword_map.get("minimum"), Some(&Severity::Forbid));
    assert_eq!(profile.keyword_map.get("allOf"), Some(&Severity::Allow));
    assert_eq!(profile.structural.require_object_root, false);
    assert_eq!(profile.structural.require_all_properties_in_required, false);
    assert_eq!(profile.structural.require_additional_properties_false, true);
    assert_eq!(profile.structural.external_refs, true);
}

#[test]
fn anthropic_profile_restrictions_present() {
    let bytes = schemalint_profiles::ANTHROPIC_SO_2026_04_30.as_bytes();
    let profile = load(bytes).unwrap();

    assert!(profile.restrictions.contains_key("additionalProperties"));
    assert!(profile.restrictions.contains_key("format"));
    assert!(profile.restrictions.contains_key("minItems"));

    let ap_restriction = profile.restrictions.get("additionalProperties").unwrap();
    assert_eq!(ap_restriction.allowed_values.len(), 1);
    assert_eq!(ap_restriction.allowed_values[0], serde_json::json!(false));

    let format_restriction = profile.restrictions.get("format").unwrap();
    assert_eq!(format_restriction.allowed_values.len(), 10);

    let min_items_restriction = profile.restrictions.get("minItems").unwrap();
    assert_eq!(min_items_restriction.allowed_values.len(), 2);
    assert_eq!(
        min_items_restriction.allowed_values[0],
        serde_json::json!(0)
    );
    assert_eq!(
        min_items_restriction.allowed_values[1],
        serde_json::json!(1)
    );
}

#[test]
fn default_code_prefix_from_name() {
    let toml = r#"
name = "custom-profile.toml"
version = "1.0"
type = "allow"

[structural]
require_object_root = false
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert_eq!(profile.code_prefix, "CUSTOM-PROFILE");
}

#[test]
fn explicit_code_prefix_overrides_default() {
    let toml = r#"
name = "my.profile.name"
version = "1.0"
code_prefix = "XYZ"
type = "allow"

[structural]
require_object_root = false
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert_eq!(profile.code_prefix, "XYZ");
}

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
        matches!(err, ProfileError::InvalidSeverity(ref s) if s.contains("unknown keyword")),
        "expected InvalidSeverity with 'unknown keyword', got {:?}",
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
        matches!(err, ProfileError::InvalidSeverity(_)),
        "expected InvalidSeverity for integer keyword value, got {:?}",
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
        matches!(err, ProfileError::InvalidSeverity(_)),
        "expected InvalidSeverity for array keyword value, got {:?}",
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
fn restrictions_not_an_array_is_silently_ignored() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false

restrictions = "not an array of tables"
"#;

    // The parser skips "restrictions" in the keyword loop (it's in the continue list),
    // and table.get("restrictions") returns a String (not Array), so it's silently ignored.
    let profile = load(toml.as_bytes()).unwrap();
    assert!(profile.restrictions.is_empty());
}

// ---------------------------------------------------------------------------
// Severity coverage — exercise all five severity values
// ---------------------------------------------------------------------------

#[test]
fn explicit_unknown_severity() {
    let toml = r#"
name = "test"
version = "1.0"
type = "unknown"

[structural]
require_object_root = false
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert_eq!(profile.keyword_map.get("type"), Some(&Severity::Unknown));
}

#[test]
fn explicit_strip_severity() {
    let toml = r#"
name = "test"
version = "1.0"
pattern = "strip"

[structural]
require_object_root = false
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert_eq!(profile.keyword_map.get("pattern"), Some(&Severity::Strip));
}

#[test]
fn all_severities_explicit() {
    let toml = r#"
name = "test"
version = "1.0"
type = "allow"
allOf = "warn"
uniqueItems = "strip"
oneOf = "forbid"
anyOf = "unknown"

[structural]
require_object_root = false
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert_eq!(profile.keyword_map.get("type"), Some(&Severity::Allow));
    assert_eq!(profile.keyword_map.get("allOf"), Some(&Severity::Warn));
    assert_eq!(
        profile.keyword_map.get("uniqueItems"),
        Some(&Severity::Strip)
    );
    assert_eq!(profile.keyword_map.get("oneOf"), Some(&Severity::Forbid));
    assert_eq!(profile.keyword_map.get("anyOf"), Some(&Severity::Unknown));
}

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
// Minimal profiles
// ---------------------------------------------------------------------------

#[test]
fn minimal_valid_profile() {
    let toml = r#"
name = "minimal"

[structural]
require_object_root = false
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert_eq!(profile.name, "minimal");
    assert_eq!(profile.version, "unknown");
    assert!(profile.keyword_map.is_empty());
    assert!(profile.restrictions.is_empty());
    assert_eq!(
        profile.structural,
        StructuralLimits {
            require_object_root: false,
            ..StructuralLimits::default()
        }
    );
}

#[test]
fn profile_version_defaults_to_unknown() {
    let toml = r#"
name = "noversion"

[structural]
require_object_root = false
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert_eq!(profile.version, "unknown");
}

#[test]
fn empty_structural_section() {
    let toml = r#"
name = "test"

[structural]
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert_eq!(profile.structural, StructuralLimits::default());
}

// ---------------------------------------------------------------------------
// Restrictions — edge cases
// ---------------------------------------------------------------------------

#[test]
fn restrictions_with_mixed_types() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false

[[restrictions]]
keyword = "format"
allowed = ["date-time", "email", 42, true]
"#;

    let profile = load(toml.as_bytes()).unwrap();
    let r = profile.restrictions.get("format").unwrap();
    assert_eq!(r.allowed_values.len(), 4);
    assert_eq!(r.allowed_values[0], serde_json::json!("date-time"));
    assert_eq!(r.allowed_values[1], serde_json::json!("email"));
    assert_eq!(r.allowed_values[2], serde_json::json!(42));
    assert_eq!(r.allowed_values[3], serde_json::json!(true));
}

#[test]
fn restrictions_nested_array() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false

[[restrictions]]
keyword = "enum"
allowed = [["a", "b"], ["c"]]
"#;

    let profile = load(toml.as_bytes()).unwrap();
    let r = profile.restrictions.get("enum").unwrap();
    assert_eq!(r.allowed_values.len(), 2);
    assert!(r.allowed_values[0].is_array());
    assert!(r.allowed_values[1].is_array());
}

#[test]
fn restrictions_inline_and_array_of_tables_combined() {
    let toml = r#"
name = "test"
version = "1.0"
format = { kind = "restricted", allowed = ["date-time"] }

[structural]
require_object_root = false

[[restrictions]]
keyword = "additionalProperties"
allowed = [false]
"#;

    let profile = load(toml.as_bytes()).unwrap();
    assert!(profile.restrictions.contains_key("format"));
    assert!(profile.restrictions.contains_key("additionalProperties"));
    assert_eq!(
        profile
            .restrictions
            .get("format")
            .unwrap()
            .allowed_values
            .len(),
        1
    );
    assert_eq!(
        profile
            .restrictions
            .get("additionalProperties")
            .unwrap()
            .allowed_values
            .len(),
        1
    );
}

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

// ---------------------------------------------------------------------------
// Multiple restrictions with same keyword in [[restrictions]]
// ---------------------------------------------------------------------------

#[test]
fn restrictions_array_multiple_entries_same_keyword() {
    let toml = r#"
name = "test"
version = "1.0"

[structural]
require_object_root = false

[[restrictions]]
keyword = "minItems"
allowed = [0]

[[restrictions]]
keyword = "minItems"
allowed = [1]
"#;

    // Last one wins (HashMap overwrite).
    let profile = load(toml.as_bytes()).unwrap();
    let r = profile.restrictions.get("minItems").unwrap();
    assert_eq!(r.allowed_values.len(), 1);
    assert_eq!(r.allowed_values[0], serde_json::json!(1));
}
