use schemalint::normalize::normalize;
use schemalint::profile::load;
use schemalint::rules::registry::{DiagnosticSeverity, RuleSet};

fn profile_with_structural(toml: &str) -> schemalint::profile::Profile {
    let base = r##"
name = "test"
version = "1.0"

[structural]
"##;
    load((base.to_string() + toml).as_bytes()).unwrap()
}

fn lint(
    schema: serde_json::Value,
    profile: &schemalint::profile::Profile,
) -> Vec<schemalint::rules::Diagnostic> {
    let norm = normalize(schema).unwrap();
    let ruleset = RuleSet::from_profile(profile);
    ruleset.check_all(&norm.arena, profile)
}

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

#[test]
fn structural_clean_schema() {
    let profile = profile_with_structural(
        r##"
require_object_root = true
require_additional_properties_false = true
require_all_properties_in_required = false
max_object_depth = 10
max_total_properties = 5000
max_total_enum_values = 1000
max_string_length_total = 120000
external_refs = true
"##,
    );
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" }
        },
        "required": ["name"],
        "additionalProperties": false
    });
    let diagnostics = lint(schema, &profile);
    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics, got {:?}",
        diagnostics
    );
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn structural_root_not_object() {
    let profile = profile_with_structural("require_object_root = true\n");
    let schema = serde_json::json!({ "type": "string" });
    let diagnostics = lint(schema, &profile);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "OAI-S-object-root");
    assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Error);
}

#[test]
fn structural_additional_properties_true() {
    let profile = profile_with_structural("require_additional_properties_false = true\n");
    let schema = serde_json::json!({
        "type": "object",
        "additionalProperties": true
    });
    let diagnostics = lint(schema, &profile);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "OAI-S-additional-properties-false");
}

#[test]
fn structural_missing_required_property() {
    let profile = profile_with_structural("require_all_properties_in_required = true\n");
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "a": { "type": "string" },
            "b": { "type": "string" }
        },
        "required": ["a"]
    });
    let diagnostics = lint(schema, &profile);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "OAI-S-all-properties-required");
    assert!(diagnostics[0].message.contains("b"));
}

#[test]
fn structural_max_depth_exceeded() {
    let profile = profile_with_structural("max_object_depth = 3\n");
    let mut schema = serde_json::json!({ "type": "string" });
    for _ in 0..5 {
        schema = serde_json::json!({
            "type": "object",
            "properties": {
                "next": schema
            },
            "additionalProperties": false
        });
    }
    let diagnostics = lint(schema, &profile);
    let depth_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "OAI-S-max-depth")
        .collect();
    assert!(
        !depth_errors.is_empty(),
        "expected depth errors, got {:?}",
        diagnostics
    );
}

#[test]
fn structural_external_ref() {
    let profile = profile_with_structural("external_refs = true\n");
    let schema = serde_json::json!({
        "$ref": "https://example.com/schema.json"
    });
    let diagnostics = lint(schema, &profile);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "OAI-S-external-refs");
}

// ---------------------------------------------------------------------------
// Global budget rules
// ---------------------------------------------------------------------------

#[test]
fn structural_max_total_properties_exceeded() {
    let profile = profile_with_structural("max_total_properties = 2\n");
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "a": { "type": "string" },
            "b": { "type": "string" },
            "c": { "type": "string" }
        },
        "additionalProperties": false
    });
    let diagnostics = lint(schema, &profile);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "OAI-S-max-total-properties")
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "expected one global property error, got {:?}",
        diagnostics
    );
}

#[test]
fn structural_max_enum_values_exceeded() {
    let profile = profile_with_structural("max_total_enum_values = 2\n");
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "status": {
                "type": "string",
                "enum": ["a", "b", "c"]
            }
        },
        "additionalProperties": false
    });
    let diagnostics = lint(schema, &profile);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "OAI-S-max-enum-values")
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "expected one global enum error, got {:?}",
        diagnostics
    );
}

#[test]
fn structural_string_length_exceeded() {
    let profile = profile_with_structural("max_string_length_total = 5\n");
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "abcdef": {
                "type": "string",
                "enum": ["x"]
            }
        },
        "additionalProperties": false
    });
    let diagnostics = lint(schema, &profile);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "OAI-S-string-length-budget")
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "expected one global string length error, got {:?}",
        diagnostics
    );
}

// ---------------------------------------------------------------------------
// Integration: Class A + Class B together
// ---------------------------------------------------------------------------

#[test]
fn structural_and_class_a_together() {
    let profile = load(
        r##"
name = "test"
version = "1.0"
allOf = "forbid"

[structural]
require_object_root = true
require_additional_properties_false = true
require_all_properties_in_required = false
max_object_depth = 10
max_total_properties = 5000
max_total_enum_values = 1000
max_string_length_total = 120000
external_refs = true
"##
        .as_bytes(),
    )
    .unwrap();

    let schema = serde_json::json!({
        "type": "object",
        "allOf": [{"type": "string"}],
        "additionalProperties": true
    });
    let diagnostics = lint(schema, &profile);

    let has_class_a = diagnostics.iter().any(|d| d.code == "OAI-K-allOf");
    let has_structural = diagnostics
        .iter()
        .any(|d| d.code == "OAI-S-additional-properties-false");
    assert!(has_class_a, "expected Class A diagnostic");
    assert!(has_structural, "expected structural diagnostic");
}
