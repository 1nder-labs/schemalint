use super::*;

fn semantic_test_profile() -> schemalint::profile::Profile {
    load_test_profile(
        r##"
name = "test"
version = "1.0"

[structural]
require_object_root = false
"##,
    )
}

fn forbid_empty_object_profile() -> schemalint::profile::Profile {
    load_test_profile(
        r##"
name = "test"
version = "1.0"

[structural]
require_object_root = false
forbid_empty_object = true
"##,
    )
}

fn anthropic_test_profile() -> schemalint::profile::Profile {
    load_test_profile(
        r##"
name = "anthropic.test"
version = "1.0"
code_prefix = "ANT"
allOf = "allow"

[structural]
require_object_root = false
"##,
    )
}

#[test]
fn empty_object_rule_fires() {
    let profile = semantic_test_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "additionalProperties": false
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-empty-object")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected empty-object warning, got {:?}",
        diagnostics
    );
    assert_eq!(hits[0].severity, DiagnosticSeverity::Warning);
}

#[test]
fn empty_object_rule_error_when_forbid_empty_object() {
    // When profile.structural.forbid_empty_object == true the rule must emit
    // DiagnosticSeverity::Error instead of Warning.
    let profile = forbid_empty_object_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "additionalProperties": false
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-empty-object")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected exactly one empty-object diagnostic, got {:?}",
        diagnostics
    );
    assert_eq!(
        hits[0].severity,
        DiagnosticSeverity::Error,
        "expected Error severity when forbid_empty_object = true, got {:?}",
        hits[0].severity
    );
}

#[test]
fn empty_object_rule_fires_with_empty_properties() {
    let profile = semantic_test_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-empty-object")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected empty-object warning, got {:?}",
        diagnostics
    );
}

#[test]
fn empty_object_rule_negative_with_properties() {
    let profile = semantic_test_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": { "x": {} },
        "additionalProperties": false
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-empty-object")
        .collect();
    assert!(
        hits.is_empty(),
        "expected no empty-object warning, got {:?}",
        diagnostics
    );
}

#[test]
fn empty_object_rule_negative_no_ap_false() {
    let profile = semantic_test_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object"
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-empty-object")
        .collect();
    assert!(
        hits.is_empty(),
        "expected no empty-object warning, got {:?}",
        diagnostics
    );
}

#[test]
fn additional_properties_object_rule_fires() {
    let profile = semantic_test_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "additionalProperties": {}
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-additional-properties-object")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected additional-properties-object error, got {:?}",
        diagnostics
    );
    assert_eq!(hits[0].severity, DiagnosticSeverity::Error);
}

#[test]
fn additional_properties_object_rule_negative_bool() {
    let profile = semantic_test_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "additionalProperties": false
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-additional-properties-object")
        .collect();
    assert!(
        hits.is_empty(),
        "expected no additional-properties-object error, got {:?}",
        diagnostics
    );
}

#[test]
fn additional_properties_object_rule_negative_true() {
    let profile = semantic_test_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "additionalProperties": true
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-additional-properties-object")
        .collect();
    assert!(
        hits.is_empty(),
        "expected no additional-properties-object error, got {:?}",
        diagnostics
    );
}

#[test]
fn anyof_objects_hint_fires() {
    let profile = semantic_test_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "value": {
                "anyOf": [{ "type": "object" }, { "type": "object" }]
            }
        }
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-anyof-objects")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected anyof-objects hint, got {:?}",
        diagnostics
    );
    assert_eq!(hits[0].severity, DiagnosticSeverity::Warning);
}

#[test]
fn anyof_objects_hint_negative_mixed() {
    let profile = semantic_test_profile();
    let schema = normalize_schema(serde_json::json!({
        "anyOf": [{ "type": "string" }, { "type": "object" }]
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-anyof-objects")
        .collect();
    assert!(
        hits.is_empty(),
        "expected no anyof-objects hint, got {:?}",
        diagnostics
    );
}

#[test]
fn anyof_objects_hint_negative_empty() {
    let profile = semantic_test_profile();
    let schema = normalize_schema(serde_json::json!({
        "anyOf": []
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-anyof-objects")
        .collect();
    assert!(
        hits.is_empty(),
        "expected no anyof-objects hint, got {:?}",
        diagnostics
    );
}

#[test]
fn allof_with_ref_rule_fires_anthropic() {
    let profile = anthropic_test_profile();
    let schema = normalize_schema(serde_json::json!({
        "$defs": { "X": { "type": "string" } },
        "allOf": [{ "$ref": "#/$defs/X" }]
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "ANT-S-allof-with-ref")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected allof-with-ref error, got {:?}",
        diagnostics
    );
    assert_eq!(hits[0].severity, DiagnosticSeverity::Error);
}

#[test]
fn allof_with_ref_rule_negative_openai() {
    let profile = semantic_test_profile();
    let schema = normalize_schema(serde_json::json!({
        "$defs": { "X": { "type": "string" } },
        "allOf": [{ "$ref": "#/$defs/X" }]
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-allof-with-ref")
        .collect();
    assert!(
        hits.is_empty(),
        "expected no allof-with-ref error for non-ANT profile, got {:?}",
        diagnostics
    );
}

#[test]
fn allof_with_ref_rule_negative_no_ref() {
    let profile = anthropic_test_profile();
    let schema = normalize_schema(serde_json::json!({
        "allOf": [{ "type": "string" }]
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "ANT-S-allof-with-ref")
        .collect();
    assert!(
        hits.is_empty(),
        "expected no allof-with-ref error without $ref, got {:?}",
        diagnostics
    );
}
