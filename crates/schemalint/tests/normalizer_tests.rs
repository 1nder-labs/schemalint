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
    let schema: serde_json::Value = serde_json::from_str(r##"{
        "type": "object",
        "properties": {
            "item": { "$ref": "#/$defs/Item" }
        },
        "$defs": {
            "Item": { "type": "string" }
        }
    }"##).unwrap();

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];

    // Find the $ref node among children.
    let ref_node_id = root.children.iter().find(|&&id| {
        norm.arena[id].annotations.r#ref.is_some()
    }).copied().expect("$ref child not found");

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
        let next = node.children.iter().find(|&&id| {
            norm.arena[id].json_pointer.ends_with("/next")
        });
        match next {
            Some(&id) => current = id,
            None => break,
        }
    }
    assert!(max_depth > 10, "expected depth > 10, got {}", max_depth);
}

#[test]
fn normalize_cyclic_ref() {
    let schema: serde_json::Value = serde_json::from_str(r##"{
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
    }"##).unwrap();

    let norm = normalize(schema).unwrap();

    let a_id = *norm.defs.get("A").unwrap();
    let b_id = *norm.defs.get("B").unwrap();

    assert!(norm.arena[a_id].is_cyclic, "A should be marked cyclic");
    assert!(norm.arena[b_id].is_cyclic, "B should be marked cyclic");
}

#[test]
fn normalize_self_referential_ref() {
    let schema: serde_json::Value = serde_json::from_str(r##"{
        "$defs": {
            "Self": {
                "type": "object",
                "properties": {
                    "self": { "$ref": "#/$defs/Self" }
                }
            }
        },
        "$ref": "#/$defs/Self"
    }"##).unwrap();

    let norm = normalize(schema).unwrap();
    let self_id = *norm.defs.get("Self").unwrap();
    assert!(norm.arena[self_id].is_cyclic, "Self should be marked cyclic");
}

// ---------------------------------------------------------------------------
// Error paths
// ---------------------------------------------------------------------------

#[test]
fn normalize_unresolved_internal_ref() {
    let schema: serde_json::Value = serde_json::from_str(r##"{
        "type": "object",
        "properties": {
            "item": { "$ref": "#/$defs/Missing" }
        }
    }"##).unwrap();

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
