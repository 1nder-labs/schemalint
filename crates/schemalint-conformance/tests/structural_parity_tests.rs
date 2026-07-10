use std::collections::BTreeSet;

use schemalint::profile::{load, StructuralLimits, StructuralRuleId};
use schemalint::profiles::{
    ANTHROPIC_SO_2026_04_30, ANTHROPIC_TRUTH, OPENAI_SO_2026_04_30, OPENAI_TRUTH,
};
use schemalint_conformance::{
    evaluate_structural_truth, parse_truth, KeywordBehavior, StructuralTest,
};
use serde_json::{json, Map, Value};

fn providers() -> [(&'static str, &'static str); 2] {
    [
        (OPENAI_SO_2026_04_30, OPENAI_TRUTH),
        (ANTHROPIC_SO_2026_04_30, ANTHROPIC_TRUTH),
    ]
}

#[test]
fn truth_structural_limits_exactly_match_enabled_production_rules() {
    for (profile_source, truth_source) in providers() {
        let profile = load(profile_source.as_bytes()).unwrap();
        let truth = parse_truth(truth_source).unwrap();
        let declared: BTreeSet<_> = truth
            .structural_tests
            .iter()
            .map(|case| StructuralRuleId::from_name(&case.limit_name).unwrap())
            .collect();
        assert_eq!(
            declared.len(),
            truth.structural_tests.len(),
            "{} truth contains duplicate structural mappings",
            profile.name
        );
        assert_eq!(
            declared,
            profile.structural.enabled_rule_ids().into_iter().collect(),
            "{} truth/profile structural parity drift",
            profile.name
        );
    }
}

#[test]
fn every_enabled_structural_rule_accepts_boundary_and_rejects_overage() {
    for (profile_source, truth_source) in providers() {
        let profile = load(profile_source.as_bytes()).unwrap();
        let enabled = profile.structural.enabled_rule_ids();
        let mut truth = parse_truth(truth_source).unwrap();
        truth.structural_tests = enabled
            .iter()
            .flat_map(|rule| boundary_cases(*rule, &profile.structural))
            .collect();

        let outcomes = evaluate_structural_truth(&truth).unwrap();
        assert_eq!(outcomes.len(), enabled.len() * 2, "{}", profile.name);
        for outcome in outcomes {
            assert!(outcome.matches(), "structural boundary drift: {outcome:?}");
        }
    }
}

fn boundary_cases(rule: StructuralRuleId, limits: &StructuralLimits) -> Vec<StructuralTest> {
    match rule {
        StructuralRuleId::ObjectRoot => pair(
            rule,
            json!({"type": "object", "properties": {"x": {"type": "string"}}}),
            json!({"type": "array", "items": {"type": "string"}}),
            "/",
        ),
        StructuralRuleId::AdditionalPropertiesFalse => pair(
            rule,
            object_schema(true, true),
            object_schema(false, true),
            "/",
        ),
        StructuralRuleId::AllPropertiesRequired => pair(
            rule,
            object_schema(true, true),
            object_schema(true, false),
            "/",
        ),
        StructuralRuleId::ArrayItems => pair(
            rule,
            json!({"type": "array", "items": {"type": "string"}}),
            json!({"type": "array"}),
            "/",
        ),
        StructuralRuleId::RootAnyOf => pair(
            rule,
            object_schema(true, true),
            json!({"type": "object", "anyOf": [{"type": "object"}]}),
            "/",
        ),
        StructuralRuleId::RootEnum => pair(
            rule,
            object_schema(true, true),
            json!({"type": "string", "enum": ["x"]}),
            "/",
        ),
        StructuralRuleId::EmptyObject => pair(
            rule,
            object_schema(true, true),
            json!({"type": "object", "properties": {}, "additionalProperties": false}),
            "/",
        ),
        StructuralRuleId::MaxDepth => {
            let limit = limits.max_object_depth as usize;
            pair(
                rule,
                nested_schema(limit),
                nested_schema(limit + 1),
                &nested_pointer(limit + 1),
            )
        }
        StructuralRuleId::MaxTotalProperties => pair(
            rule,
            properties_schema(limits.max_total_properties as usize, false),
            properties_schema(limits.max_total_properties as usize + 1, false),
            "/",
        ),
        StructuralRuleId::MaxTotalEnumValues => pair(
            rule,
            enum_schema(limits.max_total_enum_values as usize, 1),
            enum_schema(limits.max_total_enum_values as usize + 1, 1),
            "/",
        ),
        StructuralRuleId::StringLengthBudget => pair(
            rule,
            json!({"const": "x".repeat(limits.max_string_length_total as usize)}),
            json!({"const": "x".repeat(limits.max_string_length_total as usize + 1)}),
            "/",
        ),
        StructuralRuleId::EnumStringLengthBudget => {
            let threshold = limits.enum_string_length_threshold as usize;
            let limit = limits.max_enum_string_length as usize;
            pair(
                rule,
                enum_schema(threshold, limit / threshold + 1),
                enum_schema(threshold + 1, limit / (threshold + 1) + 1),
                "/enum",
            )
        }
        StructuralRuleId::MaxOptionalProperties => pair(
            rule,
            properties_schema(limits.max_optional_properties as usize, false),
            properties_schema(limits.max_optional_properties as usize + 1, false),
            "/",
        ),
        StructuralRuleId::MaxUnionProperties => pair(
            rule,
            properties_schema(limits.max_union_properties as usize, true),
            properties_schema(limits.max_union_properties as usize + 1, true),
            "/",
        ),
        StructuralRuleId::ExternalRefs => pair(
            rule,
            json!({"$ref": "#/$defs/X", "$defs": {"X": {"type": "string"}}}),
            json!({"$ref": "https://example.com/schema.json"}),
            "/",
        ),
        StructuralRuleId::AllOfWithRef => pair(
            rule,
            json!({"type": "object", "allOf": [{"type": "object"}]}),
            json!({"type": "object", "allOf": [{"$ref": "#/$defs/X"}], "$defs": {"X": {"type": "object"}}}),
            "/",
        ),
    }
}

fn pair(
    rule: StructuralRuleId,
    accept: Value,
    reject: Value,
    reject_path: &str,
) -> Vec<StructuralTest> {
    vec![
        structural_case(rule, "accept", accept, KeywordBehavior::Accept, None),
        structural_case(
            rule,
            "reject",
            reject,
            KeywordBehavior::Reject,
            Some(reject_path),
        ),
    ]
}

fn structural_case(
    rule: StructuralRuleId,
    suffix: &str,
    schema: Value,
    expected_behavior: KeywordBehavior,
    expected_error_path: Option<&str>,
) -> StructuralTest {
    StructuralTest {
        limit_name: format!("{}__{suffix}", rule.name()),
        test_schema: schema.to_string(),
        expected_behavior,
        expected_error_path: expected_error_path.map(str::to_owned),
    }
}

fn object_schema(additional_properties: bool, required: bool) -> Value {
    let mut schema = json!({"type": "object", "properties": {"x": {"type": "string"}}});
    if additional_properties {
        schema["additionalProperties"] = Value::Bool(false);
    }
    if required {
        schema["required"] = json!(["x"]);
    }
    schema
}

fn properties_schema(count: usize, union: bool) -> Value {
    let properties: Map<_, _> = (0..count)
        .map(|index| {
            let schema = if union {
                json!({"anyOf": [{"type": "string"}, {"type": "null"}]})
            } else {
                json!({"type": "string"})
            };
            (format!("p{index}"), schema)
        })
        .collect();
    json!({"type": "object", "properties": properties, "additionalProperties": false})
}

fn enum_schema(count: usize, value_length: usize) -> Value {
    let values: Vec<_> = (0..count)
        .map(|index| format!("{index:04}{}", "x".repeat(value_length)))
        .collect();
    json!({"type": "string", "enum": values})
}

fn nested_schema(depth: usize) -> Value {
    let mut schema = json!({"type": "string"});
    for index in (0..depth).rev() {
        let name = format!("p{index}");
        let mut properties = Map::new();
        properties.insert(name.clone(), schema);
        schema = json!({
            "type": "object",
            "properties": properties,
            "required": [name],
            "additionalProperties": false
        });
    }
    schema
}

fn nested_pointer(depth: usize) -> String {
    (0..depth)
        .map(|index| format!("/properties/p{index}"))
        .collect()
}
