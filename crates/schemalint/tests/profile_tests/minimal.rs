use super::*;

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
