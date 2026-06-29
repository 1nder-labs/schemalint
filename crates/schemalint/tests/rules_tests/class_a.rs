use super::*;

#[test]
fn class_a_forbid_allof() {
    let profile = load_test_profile(
        r##"
name = "test"
version = "1.0"
allOf = "forbid"

[structural]
require_object_root = false
"##,
    );
    let schema = normalize_schema(serde_json::json!({
        "allOf": [{"type": "string"}]
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "TEST-K-allOf");
    assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Error);
    assert!(diagnostics[0].message.contains("allOf"));
}

#[test]
fn class_a_warn_uniqueitems() {
    let profile = load_test_profile(
        r##"
name = "test"
version = "1.0"
uniqueItems = "warn"

[structural]
require_object_root = false
"##,
    );
    let schema = normalize_schema(serde_json::json!({
        "uniqueItems": true
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "TEST-K-uniqueItems");
    assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Warning);
}

#[test]
fn class_a_allow_type_no_diagnostic() {
    let profile = load_test_profile(
        r##"
name = "test"
version = "1.0"
type = "allow"

[structural]
require_object_root = false
"##,
    );
    let schema = normalize_schema(serde_json::json!({
        "type": "string"
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);

    assert!(diagnostics.is_empty());
}

#[test]
fn class_a_unknown_no_diagnostic() {
    let profile = load_test_profile(
        r##"
name = "test"
version = "1.0"
contains = "unknown"

[structural]
require_object_root = false
"##,
    );
    let schema = normalize_schema(serde_json::json!({
        "contains": { "type": "string" }
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);

    assert!(diagnostics.is_empty());
}

#[test]
fn class_a_restriction_allowed_value_passes() {
    let profile = load_test_profile(
        r##"
name = "test"
version = "1.0"
format = { kind = "restricted", allowed = ["date-time", "email"] }

[structural]
require_object_root = false
"##,
    );
    let schema = normalize_schema(serde_json::json!({
        "format": "date-time"
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);

    assert!(diagnostics.is_empty());
}

#[test]
fn class_a_restriction_disallowed_value_fails() {
    let profile = load_test_profile(
        r##"
name = "test"
version = "1.0"
format = { kind = "restricted", allowed = ["date-time", "email"] }

[structural]
require_object_root = false
"##,
    );
    let schema = normalize_schema(serde_json::json!({
        "format": "credit-card"
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "TEST-K-format-restricted");
    assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Error);
}

#[test]
fn unknown_keyword_no_class_a_rule() {
    let profile = load_test_profile(
        r##"
name = "test"
version = "1.0"

[structural]
require_object_root = false
"##,
    );
    let schema = normalize_schema(serde_json::json!({
        "x-custom": 42
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);

    assert!(diagnostics.is_empty());
}

#[test]
fn multiple_schemas_in_batch() {
    let profile = load_test_profile(
        r##"
name = "test"
version = "1.0"
allOf = "forbid"

[structural]
require_object_root = false
"##,
    );
    let schema = normalize_schema(serde_json::json!({
        "allOf": [{"type": "string"}]
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);

    assert_eq!(diagnostics.len(), 1);
}
