use schemalint::normalize::normalize;
use schemalint::profile::load;
use schemalint::rules::registry::RuleSet;

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
    RuleSet::from_profile(profile).check_all(&norm.arena, profile)
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
    assert!(
        diagnostics
            .iter()
            .any(|d| d.code == "TEST-S-max-optional-properties"),
        "expected optional-property budget diagnostic, got {:?}",
        diagnostics
    );
}

#[test]
fn max_union_properties_counts_anyof_and_type_arrays() {
    let profile = profile_with_structural("max_union_properties = 1\n");
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "anyof_value": {
                "anyOf": [
                    { "type": "string" },
                    { "type": "number" }
                ]
            },
            "nullable_value": {
                "type": ["string", "null"]
            }
        },
        "required": ["anyof_value", "nullable_value"],
        "additionalProperties": false
    });

    let diagnostics = lint(schema, &profile);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.code == "TEST-S-max-union-properties"),
        "expected union-property budget diagnostic, got {:?}",
        diagnostics
    );
}
