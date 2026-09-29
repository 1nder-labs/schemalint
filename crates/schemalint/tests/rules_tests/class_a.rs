use super::*;
use schemalint::profile::{Keyword, Restriction};
use schemalint::rules::registry::RuleSetError;

#[test]
fn conflicting_typed_profile_is_rejected_without_panicking() {
    let mut profile = load_test_profile(
        r##"
name = "test"
version = "1.0"
type = "forbid"

[structural]
require_object_root = false
"##,
    );
    profile.restrictions.insert(
        Keyword::Type,
        Restriction {
            allowed_values: vec![serde_json::json!("object")],
        },
    );

    assert!(matches!(
        RuleSet::from_profile(&profile),
        Err(RuleSetError::ConflictingKeyword(Keyword::Type))
    ));
}

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
    let ruleset = RuleSet::from_profile(&profile).unwrap();
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
    let ruleset = RuleSet::from_profile(&profile).unwrap();
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
    let ruleset = RuleSet::from_profile(&profile).unwrap();
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
    let ruleset = RuleSet::from_profile(&profile).unwrap();
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
    let ruleset = RuleSet::from_profile(&profile).unwrap();
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
    let ruleset = RuleSet::from_profile(&profile).unwrap();
    let diagnostics = ruleset.check_all(&schema.arena, &profile);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "TEST-K-format-restricted");
    assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Error);
}

#[test]
fn unknown_keyword_no_class_a_rule() {
    // No Class A rule fires for a keyword the profile never mentions — the
    // profile's `keyword_map` has no entry for it, so `RuleSet::from_profile`
    // never constructs a `KeywordRule` for it. Class B's unknown-keyword rule
    // (U8) does fire, since that rule reads `Node::unknown` directly rather
    // than the profile's keyword map; that is covered separately in
    // class_b.rs.
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
    let ruleset = RuleSet::from_profile(&profile).unwrap();
    let diagnostics = ruleset.check_all(&schema.arena, &profile);

    let class_a_hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.contains("-K-"))
        .collect();
    assert!(class_a_hits.is_empty());
}

// ---------------------------------------------------------------------------
// U1: a draft-07 tuple member is walked like any other nested schema, so a
// forbidden keyword inside it is still reported at its own pointer.
// ---------------------------------------------------------------------------

#[test]
fn class_a_forbid_allof_inside_tuple_member() {
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
        "type": "array",
        "items": [
            { "type": "string" },
            { "allOf": [{ "type": "number" }] }
        ]
    }));
    let ruleset = RuleSet::from_profile(&profile).unwrap();
    let diagnostics = ruleset.check_all(&schema.arena, &profile);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "TEST-K-allOf");
    assert_eq!(diagnostics[0].pointer, "/items/1");
}

fn hint_for(profile_toml: &str, schema: serde_json::Value) -> String {
    let profile = load_test_profile(profile_toml);
    let schema = normalize_schema(schema);
    let ruleset = RuleSet::from_profile(&profile).unwrap();
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    diagnostics[0].hint.clone().unwrap()
}

#[test]
fn one_of_hint_suggests_any_of_only_when_profile_allows_it() {
    let schema = serde_json::json!({ "oneOf": [{ "type": "string" }] });
    let with_any_of = hint_for(
        "name = \"test\"\nversion = \"1.0\"\noneOf = \"forbid\"\nanyOf = \"allow\"\n[structural]\nrequire_object_root = false\n",
        schema.clone(),
    );
    assert!(with_any_of.contains("replace 'oneOf' with 'anyOf'"));

    let without_any_of = hint_for(
        "name = \"test\"\nversion = \"1.0\"\noneOf = \"forbid\"\nanyOf = \"forbid\"\n[structural]\nrequire_object_root = false\n",
        schema,
    );
    assert!(!without_any_of.contains("anyOf"));
}
