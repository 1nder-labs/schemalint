use std::collections::BTreeSet;

use schemalint::profile::{load, StructuralLimits};
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
            .map(|case| base_name(&case.limit_name))
            .collect();
        assert_eq!(
            declared.len(),
            truth.structural_tests.len(),
            "{} truth contains duplicate structural mappings",
            profile.name
        );
        assert_eq!(
            declared,
            enabled_limits(&profile.structural),
            "{} truth/profile structural parity drift",
            profile.name
        );
    }
}

#[test]
fn every_enabled_structural_rule_accepts_boundary_and_rejects_overage() {
    for (profile_source, truth_source) in providers() {
        let profile = load(profile_source.as_bytes()).unwrap();
        let enabled = enabled_limits(&profile.structural);
        let mut truth = parse_truth(truth_source).unwrap();
        truth.structural_tests = enabled
            .iter()
            .flat_map(|name| boundary_cases(name, &profile.structural))
            .collect();

        let outcomes = evaluate_structural_truth(&truth).unwrap();
        assert_eq!(outcomes.len(), enabled.len() * 2, "{}", profile.name);
        for outcome in outcomes {
            assert!(outcome.matches(), "structural boundary drift: {outcome:?}");
        }
    }
}

fn enabled_limits(limits: &StructuralLimits) -> BTreeSet<&'static str> {
    let StructuralLimits {
        require_object_root,
        require_additional_properties_false,
        require_all_properties_in_required,
        require_array_items,
        forbid_root_any_of,
        forbid_root_enum,
        forbid_empty_object,
        max_object_depth,
        max_total_properties,
        max_total_enum_values,
        max_string_length_total,
        enum_string_length_threshold,
        max_enum_string_length,
        max_optional_properties,
        max_union_properties,
        external_refs,
        forbid_allof_with_ref,
    } = limits;
    let configured = [
        (*require_object_root, "require_object_root"),
        (
            *require_additional_properties_false,
            "require_additional_properties_false",
        ),
        (
            *require_all_properties_in_required,
            "require_all_properties_in_required",
        ),
        (*require_array_items, "require_array_items"),
        (*forbid_root_any_of, "forbid_root_any_of"),
        (*forbid_root_enum, "forbid_root_enum"),
        (*forbid_empty_object, "forbid_empty_object"),
        (*max_object_depth > 0, "max_object_depth"),
        (*max_total_properties > 0, "max_total_properties"),
        (*max_total_enum_values > 0, "max_total_enum_values"),
        (*max_string_length_total > 0, "max_string_length_total"),
        (
            *enum_string_length_threshold > 0 && *max_enum_string_length > 0,
            "enum_string_length_budget",
        ),
        (*max_optional_properties > 0, "max_optional_properties"),
        (*max_union_properties > 0, "max_union_properties"),
        (*external_refs, "external_refs"),
        (*forbid_allof_with_ref, "forbid_allof_with_ref"),
    ];
    configured
        .into_iter()
        .filter_map(|(enabled, name)| enabled.then_some(name))
        .collect()
}

fn boundary_cases(name: &str, limits: &StructuralLimits) -> Vec<StructuralTest> {
    match name {
        "require_object_root" => pair(
            name,
            json!({"type": "object", "properties": {"x": {"type": "string"}}}),
            json!({"type": "array", "items": {"type": "string"}}),
            "/",
        ),
        "require_additional_properties_false" => pair(
            name,
            object_schema(true, true),
            object_schema(false, true),
            "/",
        ),
        "require_all_properties_in_required" => pair(
            name,
            object_schema(true, true),
            object_schema(true, false),
            "/",
        ),
        "require_array_items" => pair(
            name,
            json!({"type": "array", "items": {"type": "string"}}),
            json!({"type": "array"}),
            "/",
        ),
        "forbid_root_any_of" => pair(
            name,
            object_schema(true, true),
            json!({"type": "object", "anyOf": [{"type": "object"}]}),
            "/",
        ),
        "forbid_root_enum" => pair(
            name,
            object_schema(true, true),
            json!({"type": "string", "enum": ["x"]}),
            "/",
        ),
        "forbid_empty_object" => pair(
            name,
            object_schema(true, true),
            json!({"type": "object", "properties": {}, "additionalProperties": false}),
            "/",
        ),
        "max_object_depth" => {
            let limit = limits.max_object_depth as usize;
            pair(
                name,
                nested_schema(limit),
                nested_schema(limit + 1),
                &nested_pointer(limit + 1),
            )
        }
        "max_total_properties" => pair(
            name,
            properties_schema(limits.max_total_properties as usize, false),
            properties_schema(limits.max_total_properties as usize + 1, false),
            "/",
        ),
        "max_total_enum_values" => pair(
            name,
            enum_schema(limits.max_total_enum_values as usize, 1),
            enum_schema(limits.max_total_enum_values as usize + 1, 1),
            "/",
        ),
        "max_string_length_total" => pair(
            name,
            json!({"const": "x".repeat(limits.max_string_length_total as usize)}),
            json!({"const": "x".repeat(limits.max_string_length_total as usize + 1)}),
            "/",
        ),
        "enum_string_length_budget" => {
            let threshold = limits.enum_string_length_threshold as usize;
            let limit = limits.max_enum_string_length as usize;
            pair(
                name,
                enum_schema(threshold, limit / threshold + 1),
                enum_schema(threshold + 1, limit / (threshold + 1) + 1),
                "/enum",
            )
        }
        "max_optional_properties" => pair(
            name,
            properties_schema(limits.max_optional_properties as usize, false),
            properties_schema(limits.max_optional_properties as usize + 1, false),
            "/",
        ),
        "max_union_properties" => pair(
            name,
            properties_schema(limits.max_union_properties as usize, true),
            properties_schema(limits.max_union_properties as usize + 1, true),
            "/",
        ),
        "external_refs" => pair(
            name,
            json!({"$ref": "#/$defs/X", "$defs": {"X": {"type": "string"}}}),
            json!({"$ref": "https://example.com/schema.json"}),
            "/",
        ),
        "forbid_allof_with_ref" => pair(
            name,
            json!({"type": "object", "allOf": [{"type": "object"}]}),
            json!({"type": "object", "allOf": [{"$ref": "#/$defs/X"}], "$defs": {"X": {"type": "object"}}}),
            "/",
        ),
        other => panic!("unmapped enabled structural limit: {other}"),
    }
}

fn pair(name: &str, accept: Value, reject: Value, reject_path: &str) -> Vec<StructuralTest> {
    vec![
        structural_case(name, "accept", accept, KeywordBehavior::Accept, None),
        structural_case(
            name,
            "reject",
            reject,
            KeywordBehavior::Reject,
            Some(reject_path),
        ),
    ]
}

fn structural_case(
    name: &str,
    suffix: &str,
    schema: Value,
    expected_behavior: KeywordBehavior,
    expected_error_path: Option<&str>,
) -> StructuralTest {
    StructuralTest {
        limit_name: format!("{name}__{suffix}"),
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

fn base_name(name: &str) -> &str {
    name.split("__").next().unwrap_or(name)
}
