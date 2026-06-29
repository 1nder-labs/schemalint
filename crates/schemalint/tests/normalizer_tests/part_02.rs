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
