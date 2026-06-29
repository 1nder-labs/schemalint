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
