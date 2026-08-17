// ---------------------------------------------------------------------------
// JSON Pointer escaping (RFC 6901) — user-controlled key segments
// ---------------------------------------------------------------------------
//
// Spec: '~' becomes '~0' first, then '/' becomes '~1'. Numeric-index and
// fixed-literal joins (items, anyOf/N, type/N, ...) need no escaping and are
// covered by the tests above; these tests cover only the joins whose segment
// is a user-controlled key.

#[test]
fn normalize_property_name_with_slash_is_escaped() {
    let schema = json!({
        "type": "object",
        "properties": {
            "a/b": { "type": "string" }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    let child = &norm.arena[root.children[0]];
    assert_eq!(child.json_pointer, "/properties/a~1b");
}

#[test]
fn normalize_property_name_with_tilde_is_escaped() {
    let schema = json!({
        "type": "object",
        "properties": {
            "c~d": { "type": "string" }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    let child = &norm.arena[root.children[0]];
    assert_eq!(child.json_pointer, "/properties/c~0d");
}

#[test]
fn normalize_escape_order_tilde_before_slash() {
    // A name containing both a literal '~' and a literal '/' proves the
    // escape order: '~' -> '~0' must run BEFORE '/' -> '~1'. Reversing the
    // order would re-escape the '~1' just produced by the slash step,
    // corrupting the pointer.
    let schema = json!({
        "type": "object",
        "properties": {
            "m~/n": { "type": "string" }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    let child = &norm.arena[root.children[0]];
    assert_eq!(child.json_pointer, "/properties/m~0~1n");
}

#[test]
fn normalize_property_name_containing_literal_tilde_one_is_not_corrupted() {
    // A name containing the literal two-character substring "~1" must not be
    // read back as an already-escaped slash: only the '~' is escaped.
    let schema = json!({
        "type": "object",
        "properties": {
            "a~1b": { "type": "string" }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    let child = &norm.arena[root.children[0]];
    assert_eq!(child.json_pointer, "/properties/a~01b");
}

#[test]
fn normalize_pattern_properties_key_is_escaped() {
    let schema = json!({
        "type": "object",
        "patternProperties": {
            "^a/b$": { "type": "string" }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    let child = &norm.arena[root.children[0]];
    assert_eq!(child.json_pointer, "/patternProperties/^a~1b$");
}

#[test]
fn normalize_dependent_schemas_key_is_escaped() {
    let schema = json!({
        "type": "object",
        "dependentSchemas": {
            "g/h": { "type": "object" }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    let child = &norm.arena[root.children[0]];
    assert_eq!(child.json_pointer, "/dependentSchemas/g~1h");
}

#[test]
fn normalize_defs_name_with_tilde_is_escaped() {
    let schema = json!({
        "type": "object",
        "$defs": {
            "i~j": { "type": "string" }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    let child = &norm.arena[root.children[0]];
    assert_eq!(child.json_pointer, "/\u{24}defs/i~0j");
}

#[test]
fn normalize_definitions_name_with_slash_is_escaped() {
    let schema = json!({
        "type": "object",
        "definitions": {
            "k/l": { "type": "string" }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    let child = &norm.arena[root.children[0]];
    assert_eq!(child.json_pointer, "/definitions/k~1l");
}

#[test]
fn normalize_numeric_index_joins_stay_unescaped() {
    // Numeric-index and fixed-literal joins are unaffected by the escape:
    // there is no user-controlled key at these sites.
    let schema = json!({
        "type": "object",
        "anyOf": [ { "type": "string" } ],
        "allOf": [ { "type": "string" } ],
        "oneOf": [ { "type": "string" } ],
        "prefixItems": [ { "type": "string" } ]
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    let pointers: Vec<&str> = root
        .children
        .iter()
        .map(|&id| norm.arena[id].json_pointer.as_str())
        .collect();
    assert!(pointers.contains(&"/anyOf/0"));
    assert!(pointers.contains(&"/allOf/0"));
    assert!(pointers.contains(&"/oneOf/0"));
    assert!(pointers.contains(&"/prefixItems/0"));
}
