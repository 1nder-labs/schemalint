use schemalint::normalize::normalize;
use schemalint::profile::load;
use schemalint::rules::registry::RuleSet;

fn profile_with_structural(structural: &str) -> schemalint::profile::Profile {
    load(
        format!(
            "name = \"openai.test\"\nversion = \"1\"\ncode_prefix = \"OAI\"\n\n[structural]\n{structural}"
        )
        .as_bytes(),
    )
    .unwrap()
}

fn lint(
    schema: serde_json::Value,
    profile: &schemalint::profile::Profile,
) -> Vec<schemalint::rules::Diagnostic> {
    let normalized = normalize(schema).unwrap();
    RuleSet::from_profile(profile)
        .unwrap()
        .check_all(&normalized.arena, profile)
}

fn lint_with_structural(
    schema: serde_json::Value,
    structural: &str,
) -> Vec<schemalint::rules::Diagnostic> {
    lint(schema, &profile_with_structural(structural))
}

#[test]
fn string_budget_counts_unicode_definitions_and_const_by_characters() {
    let schema = serde_json::json!({
        "type": "object",
        "$defs": {
            "déf": { "type": "string", "const": "東京" }
        },
        "properties": {
            "café": { "type": "string", "enum": ["猫"] }
        }
    });

    let at_limit = lint_with_structural(schema.clone(), "max_string_length_total = 10\n");
    assert!(!at_limit
        .iter()
        .any(|diagnostic| diagnostic.code == "OAI-S-string-length-budget"));

    let over_limit = lint_with_structural(schema, "max_string_length_total = 9\n");
    assert!(over_limit
        .iter()
        .any(|diagnostic| diagnostic.code == "OAI-S-string-length-budget"));
}

fn enum_values(count: usize, total_characters: usize) -> Vec<String> {
    let base = total_characters / count;
    let extra = total_characters % count;
    (0..count)
        .map(|index| {
            let length = base + usize::from(index < extra);
            let prefix = format!("{index:04}");
            assert!(length >= prefix.len());
            format!("{prefix}{}", "x".repeat(length - prefix.len()))
        })
        .collect()
}

#[test]
fn conditional_enum_budget_enforces_exact_count_and_character_boundaries() {
    let limits = "enum_string_length_threshold = 250\nmax_enum_string_length = 15000\n";
    let schema = |values: Vec<String>| {
        serde_json::json!({
            "type": "object",
            "properties": { "choice": { "type": "string", "enum": values } }
        })
    };

    let at_count_threshold = lint_with_structural(schema(enum_values(250, 15_001)), limits);
    assert!(!at_count_threshold
        .iter()
        .any(|diagnostic| diagnostic.code == "OAI-S-enum-string-length-budget"));

    let at_character_limit = lint_with_structural(schema(enum_values(251, 15_000)), limits);
    assert!(!at_character_limit
        .iter()
        .any(|diagnostic| diagnostic.code == "OAI-S-enum-string-length-budget"));

    let over_character_limit = lint_with_structural(schema(enum_values(251, 15_001)), limits);
    assert!(over_character_limit
        .iter()
        .any(|diagnostic| diagnostic.code == "OAI-S-enum-string-length-budget"));
}

#[test]
fn max_optional_properties_counts_missing_required_members() {
    let profile = profile_with_structural("max_optional_properties = 1\n");
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "required": { "type": "string" },
            "optional_a": { "type": "string" },
            "optional_b": { "type": "string" }
        },
        "required": ["required"],
        "additionalProperties": false
    });

    let diagnostics = lint(schema, &profile);
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "OAI-S-max-optional-properties"));
}

#[test]
fn max_union_properties_counts_anyof_and_type_arrays() {
    let profile = profile_with_structural("max_union_properties = 1\n");
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "anyof_value": { "anyOf": [{ "type": "string" }, { "type": "number" }] },
            "nullable_value": { "type": ["string", "null"] }
        },
        "required": ["anyof_value", "nullable_value"],
        "additionalProperties": false
    });

    let diagnostics = lint(schema, &profile);
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "OAI-S-max-union-properties"));
}
