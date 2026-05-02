use schemalint_conformance::{evaluate, parse_truth};
use schemalint_profiles::{ANTHROPIC_TRUTH, OPENAI_TRUTH};

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
