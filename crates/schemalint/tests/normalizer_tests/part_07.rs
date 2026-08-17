// ---------------------------------------------------------------------------
// U1: draft-07 tuple `items: [A, B]` normalizes into one node per member.
// ---------------------------------------------------------------------------

#[test]
fn normalize_tuple_items_array_creates_one_child_per_member() {
    // This is the exact shape zod-to-json-schema (Zod 3) emits for
    // `z.tuple([z.string(), z.number()])`.
    let schema = json!({
        "type": "array",
        "items": [
            { "type": "string" },
            { "type": "number" }
        ]
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.children.len(), 2);

    let first = &norm.arena[root.children[0]];
    assert_eq!(first.json_pointer, "/items/0");
    assert_eq!(first.annotations.r#type, Some(serde_json::json!("string")));

    let second = &norm.arena[root.children[1]];
    assert_eq!(second.json_pointer, "/items/1");
    assert_eq!(second.annotations.r#type, Some(serde_json::json!("number")));
}

#[test]
fn normalize_tuple_items_member_is_walked() {
    // A tuple member that is itself an object schema must have its own
    // nested keywords expanded, not left as an opaque raw value.
    let schema = json!({
        "type": "array",
        "items": [
            { "type": "string" },
            {
                "type": "object",
                "properties": {
                    "count": { "type": "number" }
                }
            }
        ]
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    let second = &norm.arena[root.children[1]];
    assert_eq!(second.json_pointer, "/items/1");
    assert_eq!(second.children.len(), 1);
    let nested = &norm.arena[second.children[0]];
    assert_eq!(nested.json_pointer, "/items/1/properties/count");
}

#[test]
fn normalize_items_single_object_schema_still_one_child_at_items() {
    // No regression: a plain (non-tuple) `items` schema still produces a
    // single child at `/items`.
    let schema = json!({
        "type": "array",
        "items": {
            "type": "string"
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.children.len(), 1);
    assert_eq!(norm.arena[root.children[0]].json_pointer, "/items");
}

#[test]
fn normalize_items_boolean_still_normalizes() {
    let schema = json!({
        "type": "array",
        "items": true
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.children.len(), 1);
    assert_eq!(norm.arena[root.children[0]].json_pointer, "/items");
}

#[test]
fn normalize_items_wrong_type_names_owning_pointer_not_root() {
    let schema = json!({
        "type": "object",
        "properties": {
            "list": {
                "type": "array",
                "items": 5
            }
        }
    });

    let err = normalize(schema).unwrap_err();
    let message = match err {
        NormalizeError::ParseError(ref s) => s.clone(),
        other => panic!("expected ParseError, got {:?}", other),
    };
    assert!(
        message.contains("/properties/list/items"),
        "expected message to name the owning pointer /properties/list/items, got: {}",
        message
    );
}

#[test]
fn normalize_ref_to_tuple_member_resolves() {
    // U2 interaction: a $ref into a tuple member's index must resolve.
    let schema = json!({
        "type": "object",
        "properties": {
            "x": {
                "type": "array",
                "items": [
                    { "type": "string" },
                    { "type": "number" }
                ]
            },
            "y": { "$ref": "#/properties/x/items/0" }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    let x_id = root
        .children
        .iter()
        .copied()
        .find(|&id| norm.arena[id].json_pointer == "/properties/x")
        .unwrap();
    let x_item_0 = norm.arena[x_id]
        .children
        .iter()
        .copied()
        .find(|&id| norm.arena[id].json_pointer == "/properties/x/items/0")
        .unwrap();
    let y_id = root
        .children
        .iter()
        .copied()
        .find(|&id| norm.arena[id].json_pointer == "/properties/y")
        .unwrap();
    assert_eq!(norm.arena[y_id].ref_target, Some(x_item_0));
}
