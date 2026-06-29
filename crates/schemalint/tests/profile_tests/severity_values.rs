use super::*;

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
