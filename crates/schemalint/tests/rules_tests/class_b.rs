use super::*;
use schemalint::rules::metadata::RuleCategory;

// ---------------------------------------------------------------------------
// Profile helpers
// ---------------------------------------------------------------------------

fn array_items_profile() -> schemalint::profile::Profile {
    load_test_profile(
        r##"
name = "test"
version = "1.0"

[structural]
require_object_root = false
require_array_items = true
"##,
    )
}

fn root_anyof_profile() -> schemalint::profile::Profile {
    load_test_profile(
        r##"
name = "test"
version = "1.0"

[structural]
require_object_root = false
forbid_root_any_of = true
"##,
    )
}

fn root_enum_profile() -> schemalint::profile::Profile {
    load_test_profile(
        r##"
name = "test"
version = "1.0"

[structural]
require_object_root = false
forbid_root_enum = true
"##,
    )
}

fn object_root_profile() -> schemalint::profile::Profile {
    load_test_profile(
        r##"
name = "test"
version = "1.0"

[structural]
require_object_root = true
"##,
    )
}

fn additional_properties_false_profile() -> schemalint::profile::Profile {
    load_test_profile(
        r##"
name = "test"
version = "1.0"

[structural]
require_object_root = false
require_additional_properties_false = true
"##,
    )
}

fn all_properties_required_profile() -> schemalint::profile::Profile {
    load_test_profile(
        r##"
name = "test"
version = "1.0"

[structural]
require_object_root = false
require_all_properties_in_required = true
"##,
    )
}

fn allof_with_ref_profile() -> schemalint::profile::Profile {
    load_test_profile(
        r##"
name = "test"
version = "1.0"
allOf = "allow"

[structural]
require_object_root = false
forbid_allof_with_ref = true
"##,
    )
}

fn external_refs_profile() -> schemalint::profile::Profile {
    load_test_profile(
        r##"
name = "test"
version = "1.0"

[structural]
require_object_root = false
external_refs = true
"##,
    )
}

/// Profile with every Class B flag on, for testing metadata coverage.
fn all_class_b_profile() -> schemalint::profile::Profile {
    load_test_profile(
        r##"
name = "test"
version = "1.0"
allOf = "allow"

[structural]
require_object_root = true
require_additional_properties_false = true
require_all_properties_in_required = true
require_array_items = true
forbid_root_any_of = true
forbid_root_enum = true
external_refs = true
forbid_allof_with_ref = true
"##,
    )
}

// ===========================================================================
// ArrayItemsRule
// ===========================================================================

#[test]
fn array_items_rule_fires_when_array_without_items() {
    let profile = array_items_profile();
    let schema = normalize_schema(serde_json::json!({ "type": "array" }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-array-items")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected exactly one array-items diagnostic, got {:?}",
        diagnostics
    );
    assert_eq!(
        hits[0].severity,
        DiagnosticSeverity::Error,
        "array-items must be Error severity"
    );
}

#[test]
fn array_items_rule_not_fire_when_array_has_items() {
    let profile = array_items_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "array",
        "items": { "type": "string" }
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-array-items")
        .collect();
    assert!(
        hits.is_empty(),
        "expected no array-items diagnostic when items is declared, got {:?}",
        diagnostics
    );
}

#[test]
fn array_items_rule_not_fire_for_non_array_type() {
    let profile = array_items_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": { "x": { "type": "string" } },
        "additionalProperties": false
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-array-items")
        .collect();
    assert!(
        hits.is_empty(),
        "expected no array-items diagnostic for object type, got {:?}",
        diagnostics
    );
}

#[test]
fn array_items_rule_fires_for_nested_array_without_items() {
    let profile = array_items_profile();
    // Nested array property without items should also trigger the rule.
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "tags": { "type": "array" }
        },
        "additionalProperties": false
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-array-items")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected one array-items diagnostic for nested array without items, got {:?}",
        diagnostics
    );
    assert_eq!(hits[0].severity, DiagnosticSeverity::Error);
}

// ===========================================================================
// RootAnyOfRule
// ===========================================================================

#[test]
fn root_anyof_rule_fires_at_root() {
    let profile = root_anyof_profile();
    let schema = normalize_schema(serde_json::json!({
        "anyOf": [
            { "type": "string" },
            { "type": "number" }
        ]
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-root-anyof")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected exactly one root-anyof diagnostic, got {:?}",
        diagnostics
    );
    assert_eq!(
        hits[0].severity,
        DiagnosticSeverity::Error,
        "root-anyof must be Error severity"
    );
}

#[test]
fn root_anyof_rule_not_fire_when_no_anyof_at_root() {
    let profile = root_anyof_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": { "x": { "type": "string" } },
        "additionalProperties": false
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-root-anyof")
        .collect();
    assert!(
        hits.is_empty(),
        "expected no root-anyof diagnostic for object-root schema, got {:?}",
        diagnostics
    );
}

#[test]
fn root_anyof_rule_not_fire_when_anyof_nested_not_root() {
    let profile = root_anyof_profile();
    // anyOf under a property (parent.is_some()) must NOT trigger RootAnyOfRule.
    // Use string/number branches so AnyOfObjectsHint doesn't interfere.
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "value": {
                "anyOf": [
                    { "type": "string" },
                    { "type": "number" }
                ]
            }
        },
        "additionalProperties": false
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-root-anyof")
        .collect();
    assert!(
        hits.is_empty(),
        "root-anyof must not fire for nested anyOf (parent.is_some()), got {:?}",
        diagnostics
    );
}

#[test]
fn root_anyof_rule_not_fire_when_flag_unset() {
    // With forbid_root_any_of = false (default), no RootAnyOfRule is generated.
    let profile = load_test_profile(
        r##"
name = "test"
version = "1.0"

[structural]
require_object_root = false
"##,
    );
    let schema = normalize_schema(serde_json::json!({
        "anyOf": [{ "type": "string" }, { "type": "number" }]
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-root-anyof")
        .collect();
    assert!(
        hits.is_empty(),
        "root-anyof must not fire when forbid_root_any_of is false, got {:?}",
        diagnostics
    );
}

// ===========================================================================
// RootEnumRule
// ===========================================================================

#[test]
fn root_enum_rule_fires_at_root() {
    let profile = root_enum_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "string",
        "enum": ["yes", "no"]
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-root-enum")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected exactly one root-enum diagnostic, got {:?}",
        diagnostics
    );
    assert_eq!(
        hits[0].severity,
        DiagnosticSeverity::Error,
        "root-enum must be Error severity"
    );
}

#[test]
fn root_enum_rule_not_fire_when_no_enum_at_root() {
    let profile = root_enum_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": { "x": { "type": "string" } },
        "additionalProperties": false
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-root-enum")
        .collect();
    assert!(
        hits.is_empty(),
        "expected no root-enum diagnostic for object-root schema, got {:?}",
        diagnostics
    );
}

#[test]
fn root_enum_rule_not_fire_when_enum_nested_not_root() {
    let profile = root_enum_profile();
    // enum under a property (parent.is_some()) must NOT trigger RootEnumRule.
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "status": {
                "type": "string",
                "enum": ["active", "inactive"]
            }
        },
        "additionalProperties": false
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-root-enum")
        .collect();
    assert!(
        hits.is_empty(),
        "root-enum must not fire for nested enum (parent.is_some()), got {:?}",
        diagnostics
    );
}

#[test]
fn root_enum_rule_not_fire_when_flag_unset() {
    let profile = load_test_profile(
        r##"
name = "test"
version = "1.0"

[structural]
require_object_root = false
"##,
    );
    let schema = normalize_schema(serde_json::json!({
        "type": "string",
        "enum": ["a", "b"]
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-root-enum")
        .collect();
    assert!(
        hits.is_empty(),
        "root-enum must not fire when forbid_root_enum is false, got {:?}",
        diagnostics
    );
}

// ===========================================================================
// ObjectRootRule
// ===========================================================================

#[test]
fn object_root_rule_fires_for_string_root() {
    let profile = object_root_profile();
    let schema = normalize_schema(serde_json::json!({ "type": "string" }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-object-root")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected object-root diagnostic for string root, got {:?}",
        diagnostics
    );
    assert_eq!(hits[0].severity, DiagnosticSeverity::Error);
}

#[test]
fn object_root_rule_fires_for_array_root() {
    let profile = object_root_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "array",
        "items": { "type": "string" }
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-object-root")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected object-root diagnostic for array root, got {:?}",
        diagnostics
    );
    assert_eq!(hits[0].severity, DiagnosticSeverity::Error);
}

#[test]
fn object_root_rule_not_fire_for_object_root() {
    let profile = object_root_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": { "x": { "type": "string" } },
        "additionalProperties": false
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-object-root")
        .collect();
    assert!(
        hits.is_empty(),
        "expected no object-root diagnostic for object root, got {:?}",
        diagnostics
    );
}

#[test]
fn object_root_rule_not_fire_for_child_non_object_nodes() {
    // Non-object sub-nodes must not trigger ObjectRootRule (parent.is_some() guard).
    let profile = object_root_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "count": { "type": "integer" }
        },
        "additionalProperties": false
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-object-root")
        .collect();
    assert!(
        hits.is_empty(),
        "object-root must not fire for non-object child nodes, got {:?}",
        diagnostics
    );
}

// ===========================================================================
// AdditionalPropertiesFalseRule
// ===========================================================================

#[test]
fn additional_properties_false_rule_fires_when_missing() {
    let profile = additional_properties_false_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": { "x": { "type": "string" } }
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-additional-properties-false")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected additional-properties-false diagnostic when AP missing, got {:?}",
        diagnostics
    );
    assert_eq!(hits[0].severity, DiagnosticSeverity::Error);
}

#[test]
fn additional_properties_false_rule_fires_when_ap_is_true() {
    let profile = additional_properties_false_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": { "x": { "type": "string" } },
        "additionalProperties": true
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-additional-properties-false")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected additional-properties-false diagnostic when AP is true, got {:?}",
        diagnostics
    );
    assert_eq!(hits[0].severity, DiagnosticSeverity::Error);
}

#[test]
fn additional_properties_false_rule_fires_when_ap_is_object_schema() {
    let profile = additional_properties_false_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": { "x": { "type": "string" } },
        "additionalProperties": { "type": "string" }
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-additional-properties-false")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected additional-properties-false diagnostic when AP is an object schema, got {:?}",
        diagnostics
    );
    assert_eq!(hits[0].severity, DiagnosticSeverity::Error);
}

#[test]
fn additional_properties_false_rule_not_fire_when_ap_is_false() {
    let profile = additional_properties_false_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": { "x": { "type": "string" } },
        "additionalProperties": false
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-additional-properties-false")
        .collect();
    assert!(
        hits.is_empty(),
        "expected no additional-properties-false diagnostic when AP is false, got {:?}",
        diagnostics
    );
}

#[test]
fn additional_properties_false_rule_not_fire_for_non_object() {
    let profile = additional_properties_false_profile();
    let schema = normalize_schema(serde_json::json!({ "type": "string" }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-additional-properties-false")
        .collect();
    assert!(
        hits.is_empty(),
        "expected no additional-properties-false diagnostic for non-object, got {:?}",
        diagnostics
    );
}

// ===========================================================================
// AllPropertiesRequiredRule
// ===========================================================================

#[test]
fn all_properties_required_rule_fires_with_missing_singular() {
    // 1 property not in required — exercises singular "property" branch in message.
    let profile = all_properties_required_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "age": { "type": "integer" }
        },
        "required": ["name"],
        "additionalProperties": false
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-all-properties-required")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected one all-properties-required diagnostic, got {:?}",
        diagnostics
    );
    assert_eq!(hits[0].severity, DiagnosticSeverity::Error);
    // Singular form "property" must appear in message.
    assert!(
        hits[0].message.contains("property"),
        "expected 'property' in singular message, got: {}",
        hits[0].message
    );
}

#[test]
fn all_properties_required_rule_fires_with_missing_plural() {
    // 2 properties missing — exercises plural "properties" branch.
    let profile = all_properties_required_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "a": { "type": "string" },
            "b": { "type": "string" },
            "c": { "type": "string" }
        },
        "required": ["a"],
        "additionalProperties": false
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-all-properties-required")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected one all-properties-required diagnostic for two missing, got {:?}",
        diagnostics
    );
    assert!(
        hits[0].message.contains("properties"),
        "expected 'properties' in plural message, got: {}",
        hits[0].message
    );
}

#[test]
fn all_properties_required_rule_fires_more_than_eight_missing() {
    // 10 properties total, only "a" in required → 9 missing.
    // The ">8 total missing" hint branch fires when missing.len() > 8.
    let profile = all_properties_required_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "a": { "type": "string" },
            "b": { "type": "string" },
            "c": { "type": "string" },
            "d": { "type": "string" },
            "e": { "type": "string" },
            "f": { "type": "string" },
            "g": { "type": "string" },
            "h": { "type": "string" },
            "ii": { "type": "string" },
            "jj": { "type": "string" }
        },
        "required": ["a"],
        "additionalProperties": false
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-all-properties-required")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected one all-properties-required diagnostic for 9 missing, got {:?}",
        diagnostics
    );
    // The ">8 total missing" hint branch is exercised when 9 props are missing.
    let hint = hits[0].hint.as_deref().unwrap_or("");
    assert!(
        hint.contains("total missing"),
        "expected '>8 missing' hint branch, got: {:?}",
        hint
    );
}

#[test]
fn all_properties_required_rule_not_fire_when_all_required() {
    let profile = all_properties_required_profile();
    let schema = normalize_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "age": { "type": "integer" }
        },
        "required": ["name", "age"],
        "additionalProperties": false
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-all-properties-required")
        .collect();
    assert!(
        hits.is_empty(),
        "expected no all-properties-required diagnostic when all properties listed, got {:?}",
        diagnostics
    );
}

#[test]
fn all_properties_required_rule_not_fire_for_non_object() {
    let profile = all_properties_required_profile();
    let schema = normalize_schema(serde_json::json!({ "type": "string" }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-all-properties-required")
        .collect();
    assert!(
        hits.is_empty(),
        "expected no all-properties-required diagnostic for non-object, got {:?}",
        diagnostics
    );
}

// ===========================================================================
// AllOfWithRefRule (refs.rs) — behavior + nested $ref via contains_ref recursion
// ===========================================================================

#[test]
fn allof_with_ref_rule_fires_direct_ref_in_branch() {
    let profile = allof_with_ref_profile();
    let schema = normalize_schema(serde_json::json!({
        "$defs": { "Base": { "type": "object" } },
        "allOf": [{ "$ref": "#/$defs/Base" }]
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-allof-with-ref")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected allof-with-ref diagnostic for direct $ref in allOf branch, got {:?}",
        diagnostics
    );
    assert_eq!(hits[0].severity, DiagnosticSeverity::Error);
}

#[test]
fn allof_with_ref_rule_fires_nested_ref_in_branch() {
    // Exercises the contains_ref recursion: $ref is nested inside a property
    // value within the allOf branch (map.values().any(contains_ref) path).
    let profile = allof_with_ref_profile();
    let schema = normalize_schema(serde_json::json!({
        "$defs": { "X": { "type": "string" } },
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "x": { "$ref": "#/$defs/X" }
                }
            }
        ]
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-allof-with-ref")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected allof-with-ref diagnostic for nested $ref inside allOf branch, got {:?}",
        diagnostics
    );
    assert_eq!(hits[0].severity, DiagnosticSeverity::Error);
}

#[test]
fn allof_with_ref_rule_not_fire_when_no_ref_in_branches() {
    let profile = allof_with_ref_profile();
    let schema = normalize_schema(serde_json::json!({
        "allOf": [{ "type": "string" }, { "type": "number" }]
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-allof-with-ref")
        .collect();
    assert!(
        hits.is_empty(),
        "expected no allof-with-ref diagnostic when branches have no $ref, got {:?}",
        diagnostics
    );
}

#[test]
fn allof_with_ref_rule_not_fire_when_flag_unset() {
    // When forbid_allof_with_ref = false (default), no AllOfWithRefRule is generated.
    let profile = load_test_profile(
        r##"
name = "test"
version = "1.0"
allOf = "allow"

[structural]
require_object_root = false
"##,
    );
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
        "allof-with-ref must not fire when forbid_allof_with_ref = false, got {:?}",
        diagnostics
    );
}

// ===========================================================================
// ExternalRefsRule — http(s) external ref (distinct from semantic.rs coverage)
// ===========================================================================

#[test]
fn external_refs_rule_fires_for_http_ref() {
    let profile = external_refs_profile();
    let schema = normalize_schema(serde_json::json!({
        "$ref": "https://example.com/schema.json"
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-external-refs")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected external-refs diagnostic for https:// ref, got {:?}",
        diagnostics
    );
    assert_eq!(hits[0].severity, DiagnosticSeverity::Error);
}

#[test]
fn external_refs_rule_not_fire_when_flag_unset() {
    let profile = load_test_profile(
        r##"
name = "test"
version = "1.0"

[structural]
require_object_root = false
"##,
    );
    let schema = normalize_schema(serde_json::json!({
        "$ref": "https://example.com/schema.json"
    }));
    let ruleset = RuleSet::from_profile(&profile);
    let diagnostics = ruleset.check_all(&schema.arena, &profile);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "TEST-S-external-refs")
        .collect();
    assert!(
        hits.is_empty(),
        "external-refs must not fire when external_refs = false, got {:?}",
        diagnostics
    );
}

// ===========================================================================
// Class B rule metadata() coverage — all flags on, find by name
// ===========================================================================

#[test]
fn class_b_array_items_metadata_fields() {
    let profile = all_class_b_profile();
    let ruleset = RuleSet::from_profile(&profile);
    let meta = ruleset
        .dynamic_rules()
        .filter_map(|r| r.metadata())
        .find(|m| m.name == "array-items")
        .expect("ArrayItemsRule must return metadata");

    assert_eq!(meta.code, "{prefix}-S-array-items");
    assert_eq!(meta.category.as_str(), "structural");
    assert!(!meta.description.is_empty());
    assert!(!meta.rationale.is_empty());
    assert!(!meta.bad_example.is_empty());
    assert!(!meta.good_example.is_empty());
    assert_eq!(
        meta.profile.as_deref(),
        Some("test"),
        "ArrayItemsRule must be a profile-gated rule"
    );
}

#[test]
fn class_b_root_anyof_metadata_fields() {
    let profile = all_class_b_profile();
    let ruleset = RuleSet::from_profile(&profile);
    let meta = ruleset
        .dynamic_rules()
        .filter_map(|r| r.metadata())
        .find(|m| m.name == "root-anyof")
        .expect("RootAnyOfRule must return metadata");

    assert_eq!(meta.code, "{prefix}-S-root-anyof");
    assert_eq!(meta.category.as_str(), "structural");
    assert!(!meta.description.is_empty());
    assert_eq!(meta.profile.as_deref(), Some("test"));
}

#[test]
fn class_b_root_enum_metadata_fields() {
    let profile = all_class_b_profile();
    let ruleset = RuleSet::from_profile(&profile);
    let meta = ruleset
        .dynamic_rules()
        .filter_map(|r| r.metadata())
        .find(|m| m.name == "root-enum")
        .expect("RootEnumRule must return metadata");

    assert_eq!(meta.code, "{prefix}-S-root-enum");
    assert_eq!(meta.category.as_str(), "structural");
    assert!(!meta.description.is_empty());
    assert_eq!(meta.profile.as_deref(), Some("test"));
}

#[test]
fn class_b_additional_properties_false_metadata_fields() {
    let profile = all_class_b_profile();
    let ruleset = RuleSet::from_profile(&profile);
    let meta = ruleset
        .dynamic_rules()
        .filter_map(|r| r.metadata())
        .find(|m| m.name == "additional-properties-false")
        .expect("AdditionalPropertiesFalseRule must return metadata");

    assert_eq!(meta.code, "{prefix}-S-additional-properties-false");
    assert_eq!(meta.category.as_str(), "structural");
    assert!(!meta.description.is_empty());
    assert_eq!(meta.profile.as_deref(), Some("test"));
}

#[test]
fn class_b_all_properties_required_metadata_fields() {
    let profile = all_class_b_profile();
    let ruleset = RuleSet::from_profile(&profile);
    let meta = ruleset
        .dynamic_rules()
        .filter_map(|r| r.metadata())
        .find(|m| m.name == "all-properties-required")
        .expect("AllPropertiesRequiredRule must return metadata");

    assert_eq!(meta.code, "{prefix}-S-all-properties-required");
    assert_eq!(meta.category.as_str(), "structural");
    assert!(!meta.description.is_empty());
    assert_eq!(meta.profile.as_deref(), Some("test"));
}

#[test]
fn class_b_external_refs_metadata_fields() {
    let profile = all_class_b_profile();
    let ruleset = RuleSet::from_profile(&profile);
    let meta = ruleset
        .dynamic_rules()
        .filter_map(|r| r.metadata())
        .find(|m| m.name == "external-refs")
        .expect("ExternalRefsRule must return metadata");

    assert_eq!(meta.code, "{prefix}-S-external-refs");
    assert_eq!(meta.category.as_str(), "structural");
    assert!(!meta.description.is_empty());
    assert_eq!(meta.profile.as_deref(), Some("test"));
}

#[test]
fn class_b_allof_with_ref_metadata_fields() {
    let profile = all_class_b_profile();
    let ruleset = RuleSet::from_profile(&profile);
    let meta = ruleset
        .dynamic_rules()
        .filter_map(|r| r.metadata())
        .find(|m| m.name == "allof-with-ref")
        .expect("AllOfWithRefRule must return metadata");

    assert_eq!(meta.code, "{prefix}-S-allof-with-ref");
    assert_eq!(meta.category.as_str(), "structural");
    assert!(!meta.description.is_empty());
    assert_eq!(meta.profile.as_deref(), Some("test"));
}

// ===========================================================================
// RuleCategory::as_str() — covers all four variants
// ===========================================================================

#[test]
fn rule_category_as_str_all_variants() {
    assert_eq!(RuleCategory::Keyword.as_str(), "keyword");
    assert_eq!(RuleCategory::Restriction.as_str(), "restriction");
    assert_eq!(RuleCategory::Structural.as_str(), "structural");
    assert_eq!(RuleCategory::Semantic.as_str(), "semantic");
}

#[test]
fn rule_category_equality() {
    // Confirm PartialEq holds for all variants — used in doc generation grouping.
    assert_eq!(RuleCategory::Keyword, RuleCategory::Keyword);
    assert_eq!(RuleCategory::Structural, RuleCategory::Structural);
    assert_ne!(RuleCategory::Keyword, RuleCategory::Structural);
    assert_ne!(RuleCategory::Semantic, RuleCategory::Restriction);
}
