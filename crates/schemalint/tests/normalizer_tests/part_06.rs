// ---------------------------------------------------------------------------
// U2: internal $ref resolves to any node in the document, not only a
// $defs/definitions entry.
// ---------------------------------------------------------------------------

#[test]
fn normalize_ref_to_sibling_property_resolves() {
    // This is the exact shape zod-to-json-schema (Zod 3) emits when one
    // sub-schema is reused across two locations.
    let schema = json!({
        "type": "object",
        "properties": {
            "a": { "type": "string" },
            "b": { "$ref": "#/properties/a" }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    let a_id = root.children[0];
    let b_id = root.children[1];
    assert_eq!(norm.arena[b_id].ref_target, Some(a_id));
}

#[test]
fn normalize_ref_to_nested_defs_path_resolves() {
    let schema = json!({
        "$defs": {
            "A": {
                "type": "object",
                "properties": {
                    "b": { "type": "string" }
                }
            }
        },
        "type": "object",
        "properties": {
            "item": { "$ref": "#/$defs/A/properties/b" }
        }
    });

    let norm = normalize(schema).unwrap();
    let a_id = *norm.defs.get("A").unwrap();
    let b_id = norm.arena[a_id].children[0];
    let root = &norm.arena[norm.root_id];
    let item_id = root
        .children
        .iter()
        .copied()
        .find(|&id| norm.arena[id].json_pointer == "/properties/item")
        .unwrap();
    assert_eq!(norm.arena[item_id].ref_target, Some(b_id));
}

#[test]
fn normalize_ref_to_array_element_resolves() {
    let schema = json!({
        "anyOf": [
            { "type": "string" },
            { "type": "number" }
        ],
        "properties": {
            "item": { "$ref": "#/anyOf/0" }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    let any_of_0 = root
        .children
        .iter()
        .copied()
        .find(|&id| norm.arena[id].json_pointer == "/anyOf/0")
        .unwrap();
    let item_id = root
        .children
        .iter()
        .copied()
        .find(|&id| norm.arena[id].json_pointer == "/properties/item")
        .unwrap();
    assert_eq!(norm.arena[item_id].ref_target, Some(any_of_0));
}

#[test]
fn normalize_ref_with_escaped_segment_resolves_to_literal_property() {
    // "#/properties/a~1b" must resolve to the property literally named "a/b".
    let schema = json!({
        "type": "object",
        "properties": {
            "a/b": { "type": "string" },
            "c": { "$ref": "#/properties/a~1b" }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    let ab_id = root
        .children
        .iter()
        .copied()
        .find(|&id| norm.arena[id].json_pointer == "/properties/a~1b")
        .unwrap();
    let c_id = root
        .children
        .iter()
        .copied()
        .find(|&id| norm.arena[id].json_pointer == "/properties/c")
        .unwrap();
    assert_eq!(norm.arena[c_id].ref_target, Some(ab_id));
}

#[test]
fn normalize_ref_to_nowhere_is_still_fatal() {
    let schema = json!({
        "type": "object",
        "properties": {
            "item": { "$ref": "#/properties/does_not_exist" }
        }
    });

    let err = normalize(schema).unwrap_err();
    assert!(
        matches!(err, NormalizeError::UnresolvedRef(ref s) if s == "#/properties/does_not_exist"),
        "expected UnresolvedRef, got {:?}",
        err
    );
}

#[test]
fn normalize_external_ref_is_not_an_error() {
    let schema = json!({
        "type": "object",
        "properties": {
            "item": { "$ref": "http://example.com/schema.json" }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    let item_id = root.children[0];
    assert_eq!(norm.arena[item_id].ref_target, None);
}

#[test]
fn normalize_arbitrary_self_loop_is_cyclic_and_terminates() {
    // A ref that resolves through an arbitrary (non-$defs) pointer back to
    // one of its own ancestors must still be caught by Tarjan SCC.
    let schema = json!({
        "type": "object",
        "properties": {
            "node": {
                "type": "object",
                "properties": {
                    "child": { "$ref": "#/properties/node" }
                }
            }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    let node_id = root.children[0];
    assert!(
        norm.arena[node_id].is_cyclic,
        "node should be marked cyclic"
    );
}
