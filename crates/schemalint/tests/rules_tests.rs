use schemalint::normalize::normalize;
use schemalint::profile::load;
use schemalint::rules::registry::{DiagnosticSeverity, Rule, RuleSet, RULES};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn load_test_profile(toml: &str) -> schemalint::profile::Profile {
    load(toml.as_bytes()).unwrap()
}

fn normalize_schema(value: serde_json::Value) -> schemalint::normalize::NormalizedSchema {
    normalize(value).unwrap()
}

// ---------------------------------------------------------------------------
// Class A keyword rule tests
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Class A restriction rule tests
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

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
    // RuleSet operates on a single arena. Batch aggregation is done by the
    // caller (CLI in U7). Here we just verify rules work per-schema.
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

// ---------------------------------------------------------------------------
// Semantic rules
// ---------------------------------------------------------------------------

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
        "anyOf": [{ "type": "object" }, { "type": "object" }]
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

// ---------------------------------------------------------------------------
// linkme auto-registration
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct TestRule;

impl Rule for TestRule {
    fn check(
        &self,
        _node: schemalint::ir::NodeId,
        _arena: &schemalint::ir::Arena,
        _profile: &schemalint::profile::Profile,
    ) -> Vec<schemalint::rules::Diagnostic> {
        Vec::new()
    }
}

#[linkme::distributed_slice(schemalint::rules::RULES)]
static TEST_RULE: &dyn Rule = &TestRule;

#[test]
fn linkme_auto_registration_works() {
    let found = RULES.iter().any(|&r| {
        // We can't easily identify the rule by name, but we can verify
        // that at least one rule exists (TEST_RULE) by checking the
        // slice is non-empty.
        std::ptr::eq(r, TEST_RULE)
    });
    assert!(found, "TEST_RULE should be auto-registered via linkme");
}
