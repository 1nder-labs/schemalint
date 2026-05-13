#[test]
fn normalize_external_http_ref() {
    let schema: serde_json::Value = serde_json::from_str(
        r##"{
        "type": "object",
        "properties": {
            "x": { "$ref": "https://example.com/schema.json" }
        }
    }"##,
    )
    .unwrap();

    // External refs are left unresolved — not an error
    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];

    let ref_child = root
        .children
        .iter()
        .find(|&&id| norm.arena[id].annotations.r#ref.is_some())
        .copied()
        .unwrap();

    // External refs are not resolved
    assert!(norm.arena[ref_child].ref_target.is_none());
}

// ---------------------------------------------------------------------------
// Schema with $schema keyword (dialect detection)
// ---------------------------------------------------------------------------

#[test]
fn normalize_schema_with_dollar_schema_draft2020() {
    let schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "properties": {
            "foo": { "type": "string" }
        }
    });

    let norm = normalize(schema).unwrap();
    assert_eq!(
        norm.dialect,
        schemalint::normalize::dialect::Dialect::Draft2020_12
    );
}

#[test]
fn normalize_schema_with_dollar_schema_draft07() {
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object"
    });

    let norm = normalize(schema).unwrap();
    assert_eq!(
        norm.dialect,
        schemalint::normalize::dialect::Dialect::Draft07
    );
}

#[test]
fn normalize_schema_with_dollar_schema_draft2019() {
    let schema = json!({
        "$schema": "https://json-schema.org/draft/2019-09/schema",
        "type": "object"
    });

    let norm = normalize(schema).unwrap();
    assert_eq!(
        norm.dialect,
        schemalint::normalize::dialect::Dialect::Draft2019_09
    );
}

#[test]
fn normalize_schema_prefix_items_draft2020_heuristic() {
    let schema = json!({
        "type": "array",
        "prefixItems": [
            { "type": "string" }
        ]
    });

    let norm = normalize(schema).unwrap();
    // No $schema, but prefixItems implies Draft 2020-12
    assert_eq!(
        norm.dialect,
        schemalint::normalize::dialect::Dialect::Draft2020_12
    );
}

#[test]
fn normalize_schema_no_dialect_hints() {
    let schema = json!({
        "type": "object"
    });

    let norm = normalize(schema).unwrap();
    assert_eq!(
        norm.dialect,
        schemalint::normalize::dialect::Dialect::Unknown
    );
}

// ---------------------------------------------------------------------------
// Complex nested compositions
// ---------------------------------------------------------------------------

#[test]
fn normalize_nested_any_of_in_all_of() {
    let schema = json!({
        "allOf": [
            { "type": "object" },
            {
                "anyOf": [
                    { "type": "string" },
                    { "type": "integer" }
                ]
            }
        ]
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.children.len(), 2);
    // Second allOf entry should have anyOf children
    let any_of_parent = &norm.arena[root.children[1]];
    assert_eq!(any_of_parent.children.len(), 2);
}

#[test]
fn normalize_one_of_with_refs() {
    let schema: serde_json::Value = serde_json::from_str(
        r##"{
        "$defs": {
            "Str": { "type": "string" },
            "Int": { "type": "integer" }
        },
        "oneOf": [
            { "$ref": "#/$defs/Str" },
            { "$ref": "#/$defs/Int" }
        ]
    }"##,
    )
    .unwrap();

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    // Children: 2 from oneOf ($ref Str, Int) + 2 from $defs (Str, Int) = 4
    assert_eq!(root.children.len(), 4);
    let ref_count = root
        .children
        .iter()
        .filter(|&&id| {
            norm.arena[id].annotations.r#ref.is_some() && norm.arena[id].ref_target.is_some()
        })
        .count();
    assert_eq!(ref_count, 2, "both oneOf $refs should be resolved");
}

// ---------------------------------------------------------------------------
// Pattern properties & additionalProperties schema (non-boolean)
// ---------------------------------------------------------------------------

#[test]
fn normalize_pattern_properties_expands_children() {
    let schema = json!({
        "type": "object",
        "patternProperties": {
            "^S_": { "type": "string" },
            "^I_": { "type": "integer" }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.children.len(), 2);
}

#[test]
fn normalize_additional_properties_schema_expands() {
    let schema = json!({
        "type": "object",
        "additionalProperties": {
            "type": "string",
            "minLength": 1
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.children.len(), 1);
    let child = &norm.arena[root.children[0]];
    assert_eq!(child.json_pointer, "/additionalProperties");
    assert_eq!(child.annotations.r#type, Some(serde_json::json!("string")));
}

#[test]
fn normalize_additional_properties_false_no_expand() {
    let schema = json!({
        "type": "object",
        "additionalProperties": false
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    // Boolean false should not create a child
    assert!(root.children.is_empty());
}

// ---------------------------------------------------------------------------
// dependentSchemas
// ---------------------------------------------------------------------------

#[test]
fn normalize_dependent_schemas_expands_children() {
    let schema = json!({
        "type": "object",
        "dependentSchemas": {
            "credit_card": {
                "properties": {
                    "billing_address": { "type": "string" }
                },
                "required": ["billing_address"]
            }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.children.len(), 1);
    let child = &norm.arena[root.children[0]];
    assert!(child.json_pointer.starts_with("/dependentSchemas/"));
}

// ---------------------------------------------------------------------------
// contains
// ---------------------------------------------------------------------------

#[test]
fn normalize_contains_expands_child() {
    let schema = json!({
        "type": "array",
        "contains": {
            "type": "string",
            "minLength": 3
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.children.len(), 1);
    let child = &norm.arena[root.children[0]];
    assert_eq!(child.json_pointer, "/contains");
}

// ---------------------------------------------------------------------------
// unevaluatedProperties (schema, not boolean)
// ---------------------------------------------------------------------------

#[test]
fn normalize_unevaluated_properties_schema_expands() {
    let schema = json!({
        "type": "object",
        "unevaluatedProperties": {
            "type": "string"
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.children.len(), 1);
    let child = &norm.arena[root.children[0]];
    assert_eq!(child.json_pointer, "/unevaluatedProperties");
}

#[test]
fn normalize_unevaluated_properties_false_no_expand() {
    let schema = json!({
        "type": "object",
        "unevaluatedProperties": false
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert!(root.children.is_empty());
}

// ---------------------------------------------------------------------------
// propertyNames
// ---------------------------------------------------------------------------

#[test]
fn normalize_property_names_expands_child() {
    let schema = json!({
        "type": "object",
        "propertyNames": {
            "pattern": "^[a-z]+$"
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.children.len(), 1);
    let child = &norm.arena[root.children[0]];
    assert_eq!(child.json_pointer, "/propertyNames");
}

// ---------------------------------------------------------------------------
// prefixItems
// ---------------------------------------------------------------------------
