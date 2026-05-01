use serde_json::json;

use schemalint::ir::{parse, NodeKind, ParseError};

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

#[test]
fn parse_simple_object_schema() {
    let value = json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" }
        },
        "required": ["name"]
    });

    let (arena, root) = parse(value).unwrap();
    assert_eq!(arena.len(), 1);
    let node = &arena[root];
    assert_eq!(node.kind, NodeKind::Object);
    assert!(node.annotations.r#type.as_ref().unwrap().is_string());
    assert!(node.annotations.properties.is_some());
    assert!(node.annotations.required.is_some());
    assert!(node.annotations.items.is_none());
}

#[test]
fn parse_nested_schema_with_ref() {
    let value = json!({
        "type": "object",
        "properties": {
            "item": { "$ref": "#/$defs/Item" }
        },
        "$defs": {
            "Item": { "type": "string" }
        }
    });

    let (arena, root) = parse(value).unwrap();
    assert_eq!(arena.len(), 1);
    let node = &arena[root];
    assert_eq!(node.kind, NodeKind::Object);
    assert!(node.annotations.r#ref.is_none());
    // $ref is inside the properties value, not at root
    let props = node.annotations.properties.as_ref().unwrap();
    assert!(props.get("item").unwrap().get("$ref").is_some());
}

#[test]
fn parse_schema_with_all_keywords() {
    let raw = r##"{
        "type": "object",
        "properties": {},
        "required": [],
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
        "$defs": {},
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

    let (arena, root) = parse(value).unwrap();
    let node = &arena[root];
    assert_eq!(node.kind, NodeKind::Object);
    assert!(node.annotations.r#type.is_some());
    assert!(node.annotations.properties.is_some());
    assert!(node.annotations.required.is_some());
    assert!(node.annotations.additional_properties.is_some());
    assert!(node.annotations.items.is_some());
    assert!(node.annotations.prefix_items.is_some());
    assert!(node.annotations.min_items.is_some());
    assert!(node.annotations.max_items.is_some());
    assert!(node.annotations.unique_items.is_some());
    assert!(node.annotations.contains.is_some());
    assert!(node.annotations.minimum.is_some());
    assert!(node.annotations.maximum.is_some());
    assert!(node.annotations.exclusive_minimum.is_some());
    assert!(node.annotations.exclusive_maximum.is_some());
    assert!(node.annotations.multiple_of.is_some());
    assert!(node.annotations.min_length.is_some());
    assert!(node.annotations.max_length.is_some());
    assert!(node.annotations.pattern.is_some());
    assert!(node.annotations.format.is_some());
    assert!(node.annotations.enum_values.is_some());
    assert!(node.annotations.const_value.is_some());
    assert!(node.annotations.pattern_properties.is_some());
    assert!(node.annotations.unevaluated_properties.is_some());
    assert!(node.annotations.property_names.is_some());
    assert!(node.annotations.min_properties.is_some());
    assert!(node.annotations.max_properties.is_some());
    assert!(node.annotations.description.is_some());
    assert!(node.annotations.title.is_some());
    assert!(node.annotations.default.is_some());
    assert!(node.annotations.discriminator.is_some());
    assert!(node.annotations.r#ref.is_some());
    assert!(node.annotations.defs.is_some());
    assert!(node.annotations.definitions.is_some());
    assert!(node.annotations.any_of.is_some());
    assert!(node.annotations.all_of.is_some());
    assert!(node.annotations.one_of.is_some());
    assert!(node.annotations.not.is_some());
    assert!(node.annotations.if_schema.is_some());
    assert!(node.annotations.then_schema.is_some());
    assert!(node.annotations.else_schema.is_some());
    assert!(node.annotations.dependent_required.is_some());
    assert!(node.annotations.dependent_schemas.is_some());
    assert!(node.unknown.is_empty());
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn parse_unknown_keywords() {
    let value = json!({
        "type": "string",
        "x-custom": 42,
        "x-annotation": "hello"
    });

    let (arena, root) = parse(value).unwrap();
    let node = &arena[root];
    assert_eq!(node.unknown.len(), 2);
    assert_eq!(node.unknown.get("x-custom").unwrap(), &json!(42));
    assert_eq!(node.unknown.get("x-annotation").unwrap(), &json!("hello"));
    assert!(node.annotations.r#type.is_some());
}

#[test]
fn parse_boolean_true_schema() {
    let (arena, root) = parse(json!(true)).unwrap();
    assert_eq!(arena.len(), 1);
    assert_eq!(arena[root].kind, NodeKind::Any);
    assert!(arena[root].annotations.r#type.is_none());
}

#[test]
fn parse_boolean_false_schema() {
    let (arena, root) = parse(json!(false)).unwrap();
    assert_eq!(arena.len(), 1);
    assert_eq!(arena[root].kind, NodeKind::Not);
    assert!(arena[root].annotations.r#type.is_none());
}

#[test]
fn parse_empty_object() {
    let (arena, root) = parse(json!({})).unwrap();
    assert_eq!(arena.len(), 1);
    assert_eq!(arena[root].kind, NodeKind::Object);
    assert!(arena[root].annotations.r#type.is_none());
    assert!(arena[root].unknown.is_empty());
}

// ---------------------------------------------------------------------------
// Error paths
// ---------------------------------------------------------------------------

#[test]
fn parse_null_rejected() {
    let err = parse(json!(null)).unwrap_err();
    assert!(matches!(err, ParseError::InvalidRootType(ref s) if s == "null"));
}

#[test]
fn parse_array_rejected() {
    let err = parse(json!([])).unwrap_err();
    assert!(matches!(err, ParseError::InvalidRootType(ref s) if s == "array"));
}

#[test]
fn parse_string_rejected() {
    let err = parse(json!("hello")).unwrap_err();
    assert!(matches!(err, ParseError::InvalidRootType(ref s) if s == "string"));
}

#[test]
fn parse_number_rejected() {
    let err = parse(json!(42)).unwrap_err();
    assert!(matches!(err, ParseError::InvalidRootType(ref s) if s == "number"));
}

// ---------------------------------------------------------------------------
// Integration / structural
// ---------------------------------------------------------------------------

#[test]
fn parse_duplicate_keys_last_value_wins() {
    // serde_json parses duplicate keys by taking the last value.
    // This test documents that behavior as a known limitation.
    let raw = r#"{"type": "string", "type": "number"}"#;
    let value: serde_json::Value = serde_json::from_str(raw).unwrap();
    let (arena, root) = parse(value).unwrap();
    let node = &arena[root];
    assert_eq!(node.annotations.r#type.as_ref().unwrap(), &json!("number"));
}

#[test]
fn node_size_is_reasonable() {
    use std::mem::size_of;
    let node_size = size_of::<schemalint::ir::Node>();
    // With 40 Option<Value> fields, Annotations is large but should be
    // under ~2 KiB. This is acceptable for arena allocation in Phase 1.
    assert!(
        node_size < 4096,
        "Node size {} bytes is unexpectedly large",
        node_size
    );
}
