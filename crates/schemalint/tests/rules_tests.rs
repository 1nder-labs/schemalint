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
