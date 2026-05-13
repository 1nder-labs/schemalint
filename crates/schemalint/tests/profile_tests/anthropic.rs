use super::*;

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
    assert_eq!(profile.structural.max_optional_properties, 24);
    assert_eq!(profile.structural.max_union_properties, 16);
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
