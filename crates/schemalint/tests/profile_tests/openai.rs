use super::*;

// ---------------------------------------------------------------------------
// Integration: OpenAI built-in profile
// ---------------------------------------------------------------------------

#[test]
fn openai_profile_loads() {
    let bytes = schemalint::profiles::OPENAI_SO_2026_04_30.as_bytes();
    let profile = load(bytes).unwrap();
    assert_eq!(profile.name, "openai.so.2026-04-30");
    assert_eq!(profile.version, "2026-04-30");
}

#[test]
fn openai_profile_has_zero_unknown_for_pydantic_zod_keywords() {
    let bytes = schemalint::profiles::OPENAI_SO_2026_04_30.as_bytes();
    let profile = load(bytes).unwrap();

    // Keywords commonly emitted by Pydantic v2 and zod-to-json-schema.
    let pydantic_zod_keywords = [
        "type",
        "properties",
        "required",
        "additionalProperties",
        "items",
        "prefixItems",
        "minItems",
        "maxItems",
        "uniqueItems",
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
        "anyOf",
        "allOf",
        "oneOf",
        "not",
        "description",
        "title",
        "default",
        "$ref",
        "$defs",
    ];

    for kw in &pydantic_zod_keywords {
        let in_keyword_map = profile.keyword_map.get(kw);
        let in_restrictions = profile.restrictions.contains_key(kw);
        assert!(
            in_keyword_map.is_some() || in_restrictions,
            "keyword '{}' missing from OpenAI profile (not in keyword_map or restrictions)",
            kw
        );
        if let Some(sev) = in_keyword_map {
            assert_ne!(
                *sev,
                Severity::Unknown,
                "keyword '{}' has unknown severity in OpenAI profile",
                kw
            );
        }
    }
}

#[test]
fn openai_profile_restrictions_present() {
    let bytes = schemalint::profiles::OPENAI_SO_2026_04_30.as_bytes();
    let profile = load(bytes).unwrap();

    assert!(profile.restrictions.contains_key("format"));
    assert!(profile.restrictions.contains_key("additionalProperties"));

    let format_restriction = profile.restrictions.get("format").unwrap();
    assert_eq!(format_restriction.allowed_values.len(), 9);

    let ap_restriction = profile.restrictions.get("additionalProperties").unwrap();
    assert_eq!(ap_restriction.allowed_values.len(), 1);
    assert_eq!(ap_restriction.allowed_values[0], serde_json::json!(false));
}

#[test]
fn openai_profile_corrections() {
    let bytes = schemalint::profiles::OPENAI_SO_2026_04_30.as_bytes();
    let profile = load(bytes).unwrap();

    assert_eq!(profile.code_prefix, "OAI");
    assert_eq!(profile.structural.max_object_depth, 10);
    assert_eq!(profile.keyword_map.get("oneOf"), Some(&Severity::Forbid));
    assert_eq!(
        profile.keyword_map.get("patternProperties"),
        Some(&Severity::Allow)
    );
}

/// P2 guard: OpenAI profile must NOT enable AllOfWithRefRule.
/// The flag defaults to false and the OpenAI profile does not set it.
#[test]
fn openai_profile_does_not_forbid_allof_with_ref() {
    let bytes = schemalint::profiles::OPENAI_SO_2026_04_30.as_bytes();
    let profile = load(bytes).unwrap();

    assert_eq!(
        profile.structural.forbid_allof_with_ref, false,
        "OpenAI profile must not enable forbid_allof_with_ref"
    );
}
