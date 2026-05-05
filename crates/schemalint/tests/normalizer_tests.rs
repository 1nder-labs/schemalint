use serde_json::json;

use schemalint::ir::NodeKind;
use schemalint::normalize::{normalize, NormalizeError};

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

#[test]
fn normalize_simple_object_schema() {
    let schema = json!({
        "type": "object",
        "properties": {
            "foo": { "type": "string" }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.kind, NodeKind::Object);
    assert_eq!(root.depth, 0);
    assert_eq!(root.json_pointer, "");
    assert_eq!(root.children.len(), 1);

    let prop = &norm.arena[root.children[0]];
    assert_eq!(prop.depth, 1);
    assert_eq!(prop.json_pointer, "/properties/foo");
    assert_eq!(prop.parent, Some(norm.root_id));
}

#[test]
fn normalize_schema_with_internal_ref() {
    let schema: serde_json::Value = serde_json::from_str(
        r##"{
        "type": "object",
        "properties": {
            "item": { "$ref": "#/$defs/Item" }
        },
        "$defs": {
            "Item": { "type": "string" }
        }
    }"##,
    )
    .unwrap();

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];

    // Find the $ref node among children.
    let ref_node_id = root
        .children
        .iter()
        .find(|&&id| norm.arena[id].annotations.r#ref.is_some())
        .copied()
        .expect("$ref child not found");

    let ref_node = &norm.arena[ref_node_id];
    assert!(ref_node.ref_target.is_some());
    let target_id = ref_node.ref_target.unwrap();
    assert_eq!(norm.arena[target_id].json_pointer, "/\u{24}defs/Item");
}

#[test]
fn normalize_type_array_desugaring() {
    let schema = json!({
        "type": ["string", "null"]
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.kind, NodeKind::AnyOf);
    assert_eq!(root.children.len(), 2);

    let child0 = &norm.arena[root.children[0]];
    assert_eq!(child0.json_pointer, "/type/0");
    assert_eq!(child0.annotations.r#type, Some(json!("string")));

    let child1 = &norm.arena[root.children[1]];
    assert_eq!(child1.json_pointer, "/type/1");
    assert_eq!(child1.annotations.r#type, Some(json!("null")));
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn normalize_empty_object() {
    let norm = normalize(json!({})).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.kind, NodeKind::Object);
    assert_eq!(root.depth, 0);
    assert!(root.children.is_empty());
}

#[test]
fn normalize_boolean_true() {
    let norm = normalize(json!(true)).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.kind, NodeKind::Any);
    assert_eq!(root.depth, 0);
}

#[test]
fn normalize_deeply_nested() {
    let mut obj = json!({ "type": "string" });
    for _ in 0..15 {
        obj = json!({
            "type": "object",
            "properties": {
                "next": obj
            }
        });
    }

    let norm = normalize(obj).unwrap();
    // Walk down the tree to find the deepest node.
    let mut current = norm.root_id;
    let mut max_depth = 0;
    loop {
        let node = &norm.arena[current];
        max_depth = max_depth.max(node.depth);
        let next = node
            .children
            .iter()
            .find(|&&id| norm.arena[id].json_pointer.ends_with("/next"));
        match next {
            Some(&id) => current = id,
            None => break,
        }
    }
    assert!(max_depth > 10, "expected depth > 10, got {}", max_depth);
}

#[test]
fn normalize_cyclic_ref() {
    let schema: serde_json::Value = serde_json::from_str(
        r##"{
        "$defs": {
            "A": {
                "type": "object",
                "properties": {
                    "b": { "$ref": "#/$defs/B" }
                }
            },
            "B": {
                "type": "object",
                "properties": {
                    "a": { "$ref": "#/$defs/A" }
                }
            }
        },
        "$ref": "#/$defs/A"
    }"##,
    )
    .unwrap();

    let norm = normalize(schema).unwrap();

    let a_id = *norm.defs.get("A").unwrap();
    let b_id = *norm.defs.get("B").unwrap();

    assert!(norm.arena[a_id].is_cyclic, "A should be marked cyclic");
    assert!(norm.arena[b_id].is_cyclic, "B should be marked cyclic");
}

#[test]
fn normalize_self_referential_ref() {
    let schema: serde_json::Value = serde_json::from_str(
        r##"{
        "$defs": {
            "Self": {
                "type": "object",
                "properties": {
                    "self": { "$ref": "#/$defs/Self" }
                }
            }
        },
        "$ref": "#/$defs/Self"
    }"##,
    )
    .unwrap();

    let norm = normalize(schema).unwrap();
    let self_id = *norm.defs.get("Self").unwrap();
    assert!(
        norm.arena[self_id].is_cyclic,
        "Self should be marked cyclic"
    );
}

// ---------------------------------------------------------------------------
// Error paths
// ---------------------------------------------------------------------------

#[test]
fn normalize_unresolved_internal_ref() {
    let schema: serde_json::Value = serde_json::from_str(
        r##"{
        "type": "object",
        "properties": {
            "item": { "$ref": "#/$defs/Missing" }
        }
    }"##,
    )
    .unwrap();

    let err = normalize(schema).unwrap_err();
    assert!(
        matches!(err, NormalizeError::UnresolvedRef(ref s) if s == "#/$defs/Missing"),
        "expected UnresolvedRef, got {:?}",
        err
    );
}

// ---------------------------------------------------------------------------
// Integration / structural
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// $ref chain depth tests
// ---------------------------------------------------------------------------

#[test]
fn normalize_ref_chain_depth_10() {
    let schema: serde_json::Value = serde_json::from_str(
        r##"{
        "$defs": {
            "A0": { "$ref": "#/$defs/A1" },
            "A1": { "$ref": "#/$defs/A2" },
            "A2": { "$ref": "#/$defs/A3" },
            "A3": { "$ref": "#/$defs/A4" },
            "A4": { "$ref": "#/$defs/A5" },
            "A5": { "$ref": "#/$defs/A6" },
            "A6": { "$ref": "#/$defs/A7" },
            "A7": { "$ref": "#/$defs/A8" },
            "A8": { "$ref": "#/$defs/A9" },
            "A9": { "type": "string" }
        },
        "$ref": "#/$defs/A0"
    }"##,
    )
    .unwrap();

    let norm = normalize(schema).unwrap();
    // Follow the chain from root
    let root_id = norm.root_id;
    let mut current_id = root_id;
    for _ in 0..10 {
        let node = &norm.arena[current_id];
        assert!(node.ref_target.is_some(), "expected ref_target at depth");
        current_id = node.ref_target.unwrap();
    }
    // The 10th target should be the leaf with type: string
    assert_eq!(
        norm.arena[current_id].annotations.r#type,
        Some(serde_json::json!("string"))
    );
}

#[test]
fn normalize_ref_to_definitions_nonexistent() {
    let schema: serde_json::Value = serde_json::from_str(
        r##"{
        "type": "object",
        "properties": {
            "item": { "$ref": "#/definitions/DoesNotExist" }
        }
    }"##,
    )
    .unwrap();

    let err = normalize(schema).unwrap_err();
    assert!(
        matches!(err, NormalizeError::UnresolvedRef(ref s) if s == "#/definitions/DoesNotExist"),
        "expected UnresolvedRef for nonexistent definition, got {:?}",
        err
    );
}

#[test]
fn normalize_ref_to_nonexistent_defs() {
    let schema: serde_json::Value = serde_json::from_str(
        r##"{
        "$ref": "#/$defs/Ghost"
    }"##,
    )
    .unwrap();

    let err = normalize(schema).unwrap_err();
    assert!(
        matches!(err, NormalizeError::UnresolvedRef(ref s) if s == "#/$defs/Ghost"),
        "expected UnresolvedRef for nonexistent $defs, got {:?}",
        err
    );
}

// ---------------------------------------------------------------------------
// Boolean schemas in compound keywords
// ---------------------------------------------------------------------------

#[test]
fn normalize_boolean_true_in_all_of() {
    let schema = json!({
        "allOf": [true, { "type": "string" }]
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.children.len(), 2);
    let child0 = &norm.arena[root.children[0]];
    assert_eq!(child0.kind, schemalint::ir::NodeKind::Any);
}

#[test]
fn normalize_boolean_false_in_all_of() {
    let schema = json!({
        "allOf": [false, { "type": "string" }]
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.children.len(), 2);
    let child0 = &norm.arena[root.children[0]];
    assert_eq!(child0.kind, schemalint::ir::NodeKind::Not);
}

#[test]
fn normalize_boolean_true_in_any_of() {
    let schema = json!({
        "anyOf": [true, { "type": "string" }]
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.children.len(), 2);
    let child0 = &norm.arena[root.children[0]];
    assert_eq!(child0.kind, schemalint::ir::NodeKind::Any);
}

#[test]
fn normalize_not_true() {
    let schema = json!({
        "not": true
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.children.len(), 1);
    let child = &norm.arena[root.children[0]];
    assert_eq!(child.kind, schemalint::ir::NodeKind::Any);
    assert_eq!(child.json_pointer, "/not");
}

#[test]
fn normalize_not_false() {
    let schema = json!({
        "not": false
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.children.len(), 1);
    let child = &norm.arena[root.children[0]];
    assert_eq!(child.kind, schemalint::ir::NodeKind::Not);
}

// ---------------------------------------------------------------------------
// if/then/else + $ref
// ---------------------------------------------------------------------------

#[test]
fn normalize_if_then_else_with_refs() {
    let schema: serde_json::Value = serde_json::from_str(
        r##"{
        "type": "object",
        "$defs": {
            "Cond": { "type": "string" },
            "Then": { "type": "integer" },
            "Else": { "type": "boolean" }
        },
        "if": { "$ref": "#/$defs/Cond" },
        "then": { "$ref": "#/$defs/Then" },
        "else": { "$ref": "#/$defs/Else" }
    }"##,
    )
    .unwrap();

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];

    // Find the if/then/else children
    let if_child = root
        .children
        .iter()
        .find(|&&id| norm.arena[id].json_pointer == "/if")
        .unwrap();
    let then_child = root
        .children
        .iter()
        .find(|&&id| norm.arena[id].json_pointer == "/then")
        .unwrap();
    let else_child = root
        .children
        .iter()
        .find(|&&id| norm.arena[id].json_pointer == "/else")
        .unwrap();

    assert!(norm.arena[*if_child].ref_target.is_some());
    assert!(norm.arena[*then_child].ref_target.is_some());
    assert!(norm.arena[*else_child].ref_target.is_some());

    // Verify targets resolve to correct defs
    let if_target = norm.arena[*if_child].ref_target.unwrap();
    let then_target = norm.arena[*then_child].ref_target.unwrap();
    let else_target = norm.arena[*else_child].ref_target.unwrap();

    assert_eq!(
        norm.arena[if_target].annotations.r#type,
        Some(serde_json::json!("string"))
    );
    assert_eq!(
        norm.arena[then_target].annotations.r#type,
        Some(serde_json::json!("integer"))
    );
    assert_eq!(
        norm.arena[else_target].annotations.r#type,
        Some(serde_json::json!("boolean"))
    );
}

// ---------------------------------------------------------------------------
// type array desugaring with additional keywords
// ---------------------------------------------------------------------------

#[test]
fn normalize_type_array_with_properties() {
    let schema = json!({
        "type": ["object", "null"],
        "properties": {
            "foo": { "type": "string" }
        },
        "required": ["foo"]
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.kind, schemalint::ir::NodeKind::AnyOf);
    // Children: 1 from properties (foo) + 2 from type array desugaring = 3
    assert_eq!(root.children.len(), 3);

    // The two desugared type variant children should exist
    let type_children: Vec<_> = root
        .children
        .iter()
        .filter(|&&id| norm.arena[id].json_pointer.starts_with("/type/"))
        .collect();
    assert_eq!(type_children.len(), 2);
    // First variant should be "object"
    assert_eq!(
        norm.arena[*type_children[0]].annotations.r#type,
        Some(serde_json::json!("object"))
    );

    // properties key should still be on root
    assert!(root.annotations.properties.is_some());
}

#[test]
fn normalize_type_array_single_element_not_desugared() {
    let schema = json!({
        "type": ["string"]
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    // Single-element type array should NOT be desugared
    assert_eq!(root.kind, schemalint::ir::NodeKind::Object);
    assert_eq!(root.children.len(), 0);
}

#[test]
fn normalize_type_array_three_elements() {
    let schema = json!({
        "type": ["string", "integer", "null"]
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.kind, schemalint::ir::NodeKind::AnyOf);
    assert_eq!(root.children.len(), 3);
}

// ---------------------------------------------------------------------------
// allOf with multiple $refs
// ---------------------------------------------------------------------------

#[test]
fn normalize_all_of_with_multiple_refs() {
    let schema: serde_json::Value = serde_json::from_str(
        r##"{
        "type": "object",
        "$defs": {
            "A": { "type": "string", "minLength": 1 },
            "B": { "type": "integer", "minimum": 0 },
            "C": { "type": "boolean" }
        },
        "allOf": [
            { "$ref": "#/$defs/A" },
            { "$ref": "#/$defs/B" },
            { "$ref": "#/$defs/C" }
        ]
    }"##,
    )
    .unwrap();

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    // Children: 3 from allOf ($ref A, B, C) + 3 from $defs (A, B, C) = 6
    assert_eq!(root.children.len(), 6);

    // Find allOf children (they have $ref annotations) and verify resolution
    let ref_count = root
        .children
        .iter()
        .filter(|&&id| {
            norm.arena[id].annotations.r#ref.is_some() && norm.arena[id].ref_target.is_some()
        })
        .count();
    assert_eq!(ref_count, 3, "all three allOf $refs should be resolved");
}

// ---------------------------------------------------------------------------
// Draft 7 definitions (not $defs)
// ---------------------------------------------------------------------------

#[test]
fn normalize_definitions_draft7_ref() {
    let schema: serde_json::Value = serde_json::from_str(
        r##"{
        "type": "object",
        "properties": {
            "item": { "$ref": "#/definitions/Item" }
        },
        "definitions": {
            "Item": { "type": "string" }
        }
    }"##,
    )
    .unwrap();

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    let ref_node_id = root
        .children
        .iter()
        .find(|&&id| norm.arena[id].annotations.r#ref.is_some())
        .copied()
        .expect("$ref child not found");

    assert!(norm.arena[ref_node_id].ref_target.is_some());
    let target_id = norm.arena[ref_node_id].ref_target.unwrap();
    assert_eq!(norm.arena[target_id].json_pointer, "/definitions/Item");
}

// ---------------------------------------------------------------------------
// $defs shadowing definitions (draft 2020-12 priority)
// ---------------------------------------------------------------------------

#[test]
fn normalize_defs_shadow_definitions() {
    let schema: serde_json::Value = serde_json::from_str(
        r##"{
        "$defs": {
            "Same": { "type": "string" }
        },
        "definitions": {
            "Same": { "type": "integer" }
        },
        "$ref": "#/$defs/Same"
    }"##,
    )
    .unwrap();

    let norm = normalize(schema).unwrap();
    // $defs should contain "Same" pointing to the string-typed schema
    let defs_same_id = norm.defs.get("Same").unwrap();
    assert_eq!(
        norm.arena[*defs_same_id].annotations.r#type,
        Some(serde_json::json!("string"))
    );
}

// ---------------------------------------------------------------------------
// Multiple $refs to same def
// ---------------------------------------------------------------------------

#[test]
fn normalize_multiple_refs_to_same_def() {
    let schema: serde_json::Value = serde_json::from_str(
        r##"{
        "type": "object",
        "properties": {
            "a": { "$ref": "#/$defs/Shared" },
            "b": { "$ref": "#/$defs/Shared" }
        },
        "$defs": {
            "Shared": { "type": "string" }
        }
    }"##,
    )
    .unwrap();

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];

    let shared_id = norm.defs.get("Shared").unwrap();
    for child_id in &root.children {
        let child = &norm.arena[*child_id];
        if child.annotations.r#ref.is_some() {
            assert_eq!(child.ref_target, Some(*shared_id));
        }
    }
}

// ---------------------------------------------------------------------------
// External ref (http/https) is left unresolved
// ---------------------------------------------------------------------------

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

#[test]
fn normalize_prefix_items_expands_children() {
    let schema = json!({
        "type": "array",
        "prefixItems": [
            { "type": "string" },
            { "type": "integer" }
        ]
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.children.len(), 2);
}

// ---------------------------------------------------------------------------
// Error: invalid root type (number, string, null, array)
// ---------------------------------------------------------------------------

#[test]
fn normalize_string_root_errors() {
    let err = normalize(serde_json::json!("hello")).unwrap_err();
    assert!(
        matches!(err, NormalizeError::ParseError(_)),
        "expected ParseError for string root, got {:?}",
        err
    );
}

#[test]
fn normalize_null_root_errors() {
    let err = normalize(serde_json::json!(null)).unwrap_err();
    assert!(
        matches!(err, NormalizeError::ParseError(_)),
        "expected ParseError for null root, got {:?}",
        err
    );
}

#[test]
fn normalize_array_root_errors() {
    let err = normalize(serde_json::json!([1, 2, 3])).unwrap_err();
    assert!(
        matches!(err, NormalizeError::ParseError(_)),
        "expected ParseError for array root, got {:?}",
        err
    );
}

#[test]
fn normalize_number_root_errors() {
    let err = normalize(serde_json::json!(42)).unwrap_err();
    assert!(
        matches!(err, NormalizeError::ParseError(_)),
        "expected ParseError for number root, got {:?}",
        err
    );
}

// ---------------------------------------------------------------------------
// 100+ $refs — stress test
// ---------------------------------------------------------------------------

#[test]
fn normalize_many_refs_completes() {
    // Build a schema with 100+ refs dynamically
    let mut defs = serde_json::Map::new();
    for i in 0..110 {
        defs.insert(
            format!("R{}", i),
            serde_json::json!({ "type": "string", "minLength": 1 }),
        );
    }

    let mut properties = serde_json::Map::new();
    for i in 0..110 {
        properties.insert(
            format!("field{}", i),
            serde_json::json!({ "$ref": format!("#/$defs/R{}", i) }),
        );
    }

    let schema = serde_json::json!({
        "type": "object",
        "$defs": defs,
        "properties": properties
    });

    let norm = normalize(schema).unwrap();
    assert!(
        norm.arena.len() > 220,
        "expected > 220 nodes, got {}",
        norm.arena.len()
    );
}

// ---------------------------------------------------------------------------
// Boolean false root schema
// ---------------------------------------------------------------------------

#[test]
fn normalize_boolean_false_root() {
    let norm = normalize(serde_json::json!(false)).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.kind, NodeKind::Not);
    assert_eq!(root.depth, 0);
    assert!(root.children.is_empty());
}

// ---------------------------------------------------------------------------
// items (single schema)
// ---------------------------------------------------------------------------

#[test]
fn normalize_items_schema_expands() {
    let schema = json!({
        "type": "array",
        "items": {
            "type": "string",
            "maxLength": 100
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.children.len(), 1);
    let child = &norm.arena[root.children[0]];
    assert_eq!(child.json_pointer, "/items");
    assert_eq!(child.annotations.r#type, Some(serde_json::json!("string")));
}

#[test]
fn normalize_all_keywords_present() {
    let raw = r##"{
        "type": "object",
        "properties": { "a": {} },
        "required": ["a"],
        "additionalProperties": false,
        "items": {},
        "prefixItems": [],
        "minItems": 0,
        "maxItems": 10,
        "uniqueItems": true,
        "contains": {},
        "minimum": 0,
        "maximum": 100,
        "exclusiveMinimum": 0,
        "exclusiveMaximum": 100,
        "multipleOf": 2,
        "minLength": 1,
        "maxLength": 255,
        "pattern": "^foo$",
        "format": "email",
        "enum": ["a", "b"],
        "const": "x",
        "patternProperties": {},
        "unevaluatedProperties": false,
        "propertyNames": {},
        "minProperties": 0,
        "maxProperties": 10,
        "description": "desc",
        "title": "title",
        "default": null,
        "discriminator": {},
        "$ref": "#/$defs/X",
        "$defs": { "X": {} },
        "definitions": {},
        "anyOf": [],
        "allOf": [],
        "oneOf": [],
        "not": {},
        "if": {},
        "then": {},
        "else": {},
        "dependentRequired": {},
        "dependentSchemas": {}
    }"##;
    let value: serde_json::Value = serde_json::from_str(raw).unwrap();
    let norm = normalize(value).unwrap();
    // Should not panic; all keywords handled during expansion.
    assert!(norm.arena.len() > 1);
}
