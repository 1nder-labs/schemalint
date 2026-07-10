use schemalint::profiles::{ANTHROPIC_TRUTH, OPENAI_TRUTH};
use schemalint_conformance::{
    evaluate, evaluate_provider, evaluate_structural_truth, parse_truth, InfrastructureFailureKind,
    LiveRefreshState,
};

#[test]
fn openai_truth_parses() {
    let truth = parse_truth(OPENAI_TRUTH).expect("openai.truth.toml should parse");
    assert_eq!(truth.provider.name, "openai");
    assert!(!truth.keywords.is_empty(), "should have keywords");
}

#[test]
fn anthropic_truth_parses() {
    let truth = parse_truth(ANTHROPIC_TRUTH).expect("anthropic.truth.toml should parse");
    assert_eq!(truth.provider.name, "anthropic");
    assert!(!truth.keywords.is_empty(), "should have keywords");
}

#[test]
fn openai_truth_known_reject() {
    let truth = parse_truth(OPENAI_TRUTH).unwrap();
    let schema = serde_json::json!({
        "type": "object",
        "allOf": [{"properties": {"x": {"type": "string"}}}],
        "properties": {}
    });
    let result = evaluate(&truth, &schema);
    assert!(
        result.is_rejected(),
        "allOf should be rejected by OpenAI truth"
    );
}

#[test]
fn openai_truth_known_accept() {
    let truth = parse_truth(OPENAI_TRUTH).unwrap();
    let schema = serde_json::json!({
        "type": "object",
        "properties": {"name": {"type": "string"}},
        "required": ["name"],
        "additionalProperties": false
    });
    let result = evaluate(&truth, &schema);
    assert!(
        result.is_accepted(),
        "clean schema should be accepted by OpenAI truth"
    );
}

#[test]
fn anthropic_truth_known_reject() {
    let truth = parse_truth(ANTHROPIC_TRUTH).unwrap();
    let schema = serde_json::json!({
        "type": "string",
        "minLength": 5
    });
    let result = evaluate(&truth, &schema);
    assert!(
        result.is_rejected(),
        "minLength should be rejected by Anthropic truth"
    );
}

#[test]
fn anthropic_truth_known_accept() {
    let truth = parse_truth(ANTHROPIC_TRUTH).unwrap();
    let schema = serde_json::json!({
        "type": "object",
        "properties": {"name": {"type": "string"}},
        "additionalProperties": false
    });
    let result = evaluate(&truth, &schema);
    assert!(
        result.is_accepted(),
        "clean schema should be accepted by Anthropic truth"
    );
}

#[test]
fn evaluate_strip_with_expected_transformed() {
    use schemalint_conformance::TruthResult;
    let truth = parse_truth(
        r#"
[provider]
name = "test"
version = "1.0"
behavior = "strict"

[[keywords]]
name = "type"
behavior = "accept"
test_schema = '''
{ "type": "object", "properties": {} }
'''

[[keywords]]
name = "description"
behavior = "strip"
test_schema = '''
{ "type": "object", "description": "original", "properties": {} }
'''
expected_transformed = '''
"replaced description"
'''
"#,
    )
    .unwrap();
    let schema = serde_json::json!({
        "type": "object",
        "description": "original",
        "properties": {}
    });
    let result = evaluate(&truth, &schema);
    assert!(result.is_accepted());
    if let TruthResult::Accepted { transformed } = &result {
        let obj = transformed.as_object().unwrap();
        assert_eq!(
            obj.get("description"),
            Some(&serde_json::Value::String(
                "replaced description".to_string()
            ))
        );
    } else {
        panic!("expected accepted result");
    }
}

#[test]
fn every_keyword_has_test_schema() {
    let truth = parse_truth(OPENAI_TRUTH).unwrap();
    for kw in &truth.keywords {
        // Test schema must be valid JSON.
        serde_json::from_str::<serde_json::Value>(&kw.test_schema)
            .unwrap_or_else(|e| panic!("keyword '{}': invalid test_schema: {e}", kw.name));
    }
}

#[test]
fn truth_keywords_cover_profile_keywords() {
    use std::collections::HashSet;

    let truth = parse_truth(OPENAI_TRUTH).unwrap();
    let truth_keywords: HashSet<&str> = truth.keywords.iter().map(|k| k.name.as_str()).collect();

    // Every keyword in the profile should have a truth entry.
    let profile_keywords: &[&str] = &[
        "type",
        "properties",
        "required",
        "additionalProperties",
        "items",
        "prefixItems",
        "minItems",
        "maxItems",
        "uniqueItems",
        "contains",
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "multipleOf",
        "minLength",
        "maxLength",
        "pattern",
        "format",
        "enum",
        "const",
        "patternProperties",
        "unevaluatedProperties",
        "propertyNames",
        "minProperties",
        "maxProperties",
        "description",
        "title",
        "default",
        "discriminator",
        "$ref",
        "$defs",
        "definitions",
        "anyOf",
        "allOf",
        "oneOf",
        "not",
        "if",
        "then",
        "else",
        "dependentRequired",
        "dependentSchemas",
    ];

    for kw in profile_keywords {
        assert!(
            truth_keywords.contains(kw),
            "OpenAI truth file missing keyword: {kw}"
        );
    }
}

#[test]
fn every_structural_truth_case_matches_production_rules() {
    for source in [OPENAI_TRUTH, ANTHROPIC_TRUTH] {
        let truth = parse_truth(source).unwrap();
        let outcomes = evaluate_structural_truth(&truth).unwrap();
        assert!(!outcomes.is_empty());
        for outcome in outcomes {
            assert!(outcome.matches(), "structural truth drift: {outcome:?}");
        }
    }
}

#[test]
fn known_provider_evaluation_uses_production_structural_rules() {
    let truth = parse_truth(OPENAI_TRUTH).unwrap();
    let invalid_root = serde_json::json!({ "type": "array", "items": { "type": "string" } });
    let result = evaluate_provider(&truth, &invalid_root).unwrap();
    assert!(result.is_rejected());
}

#[test]
fn live_refresh_states_keep_infrastructure_and_lint_incompleteness_distinct() {
    let states = [
        LiveRefreshState::ProviderAccepted,
        LiveRefreshState::ProviderRejected {
            message: "schema rejected".into(),
        },
        LiveRefreshState::InfrastructureFailure {
            kind: InfrastructureFailureKind::Authentication,
            message: "bad credential".into(),
        },
        LiveRefreshState::IncompleteLintEvaluation {
            message: "normalization failed".into(),
        },
    ];
    let encoded: Vec<_> = states
        .iter()
        .map(|state| serde_json::to_value(state).unwrap()["state"].clone())
        .collect();
    assert_eq!(
        encoded,
        vec![
            "provider_accepted",
            "provider_rejected",
            "infrastructure_failure",
            "incomplete_lint_evaluation"
        ]
    );
}
