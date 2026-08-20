// ---------------------------------------------------------------------------
// U7: a subschema nested inside an unrecognized keyword reaches the arena.
// Before this fix, `expand_children` only read the typed `Annotations`
// fields, never `node.unknown`, so a forbidden keyword hidden under
// `additionalItems`, `unevaluatedItems`, `contentSchema`, or a draft-07
// `dependencies` entry was completely invisible to every rule.
//
// Per KTD12, the walk is a named four-keyword allowlist, not a blanket walk
// of `node.unknown`. That map also holds vendor extensions such as
// `x-ui-hints`, which are object-shaped but are not schemas; those must
// never reach the arena or count toward any budget.
// ---------------------------------------------------------------------------

fn openai_diagnostics(schema: serde_json::Value) -> Vec<schemalint::rules::Diagnostic> {
    let norm = normalize(schema).unwrap();
    let openai = profile::load(OPENAI_SO_2026_04_30.as_bytes()).unwrap();
    let rules = RuleSet::from_profile(&openai).unwrap();
    rules.check_all(&norm.arena, &openai)
}

#[test]
fn normalize_forbidden_keyword_inside_unevaluated_items_is_reported() {
    let schema = json!({
        "type": "object",
        "properties": {
            "list": {
                "type": "array",
                "unevaluatedItems": {
                    "oneOf": [{ "type": "string" }, { "type": "integer" }]
                }
            }
        },
        "additionalProperties": false,
        "required": ["list"]
    });

    let diagnostics = openai_diagnostics(schema);
    let hit = diagnostics
        .iter()
        .find(|d| d.pointer == "/properties/list/unevaluatedItems" && d.code == "OAI-K-oneOf");
    assert!(
        hit.is_some(),
        "expected a oneOf diagnostic under unevaluatedItems, got: {:?}",
        diagnostics
    );
}

#[test]
fn normalize_forbidden_keyword_inside_additional_items_is_reported() {
    let schema = json!({
        "type": "object",
        "properties": {
            "list": {
                "type": "array",
                "items": [{ "type": "string" }],
                "additionalItems": {
                    "oneOf": [{ "type": "string" }, { "type": "integer" }]
                }
            }
        },
        "additionalProperties": false,
        "required": ["list"]
    });

    let diagnostics = openai_diagnostics(schema);
    let hit = diagnostics
        .iter()
        .find(|d| d.pointer == "/properties/list/additionalItems" && d.code == "OAI-K-oneOf");
    assert!(
        hit.is_some(),
        "expected a oneOf diagnostic under additionalItems, got: {:?}",
        diagnostics
    );
}

#[test]
fn normalize_forbidden_keyword_inside_content_schema_is_reported() {
    let schema = json!({
        "type": "object",
        "properties": {
            "blob": {
                "type": "string",
                "contentSchema": {
                    "oneOf": [{ "type": "string" }, { "type": "integer" }]
                }
            }
        },
        "additionalProperties": false,
        "required": ["blob"]
    });

    let diagnostics = openai_diagnostics(schema);
    let hit = diagnostics
        .iter()
        .find(|d| d.pointer == "/properties/blob/contentSchema" && d.code == "OAI-K-oneOf");
    assert!(
        hit.is_some(),
        "expected a oneOf diagnostic under contentSchema, got: {:?}",
        diagnostics
    );
}

#[test]
fn normalize_forbidden_keyword_inside_dependencies_entry_is_reported() {
    let schema = json!({
        "type": "object",
        "properties": {
            "a": { "type": "string" }
        },
        "dependencies": {
            "a": {
                "oneOf": [{ "type": "string" }, { "type": "integer" }]
            }
        },
        "additionalProperties": false
    });

    let diagnostics = openai_diagnostics(schema);
    let hit = diagnostics
        .iter()
        .find(|d| d.pointer == "/dependencies/a" && d.code == "OAI-K-oneOf");
    assert!(
        hit.is_some(),
        "expected a oneOf diagnostic under dependencies/a, got: {:?}",
        diagnostics
    );
}

#[test]
fn normalize_dependencies_property_name_array_adds_no_node() {
    // The other draft-07 `dependencies` shape: the value is an array of
    // property names, not a schema. It must not be allocated as a node.
    let schema = json!({
        "type": "object",
        "properties": {
            "a": { "type": "string" },
            "b": { "type": "string" }
        },
        "dependencies": {
            "a": ["b"]
        }
    });

    let norm = normalize(schema).unwrap();
    let dependencies_nodes = norm
        .arena
        .iter()
        .filter(|(_, n)| n.json_pointer.starts_with("/dependencies"))
        .count();
    assert_eq!(
        dependencies_nodes, 0,
        "a property-name array under dependencies must not become a node"
    );
}

#[test]
fn normalize_inert_examples_annotation_adds_no_node() {
    let schema = json!({
        "type": "object",
        "properties": {
            "a": { "type": "string" }
        },
        "examples": ["one", "two", "three"]
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    // Only the one typed property child, nothing from `examples`.
    assert_eq!(root.children.len(), 1);
    assert!(norm
        .arena
        .iter()
        .all(|(_, n)| !n.json_pointer.contains("examples")));
}

#[test]
fn normalize_comment_string_adds_no_node() {
    let schema = json!({
        "type": "object",
        "properties": {
            "a": { "type": "string" }
        },
        "$comment": "internal note for reviewers"
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(root.children.len(), 1);
}

#[test]
fn normalize_vendor_extension_adds_no_node_and_no_budget_contribution() {
    // KTD12's guard case: `x-ui-hints` is object-shaped but is not a
    // schema. It must not be allocated as a node, and its nested
    // properties must not count toward any arena-wide budget.
    let schema = json!({
        "type": "object",
        "properties": {
            "a": { "type": "string" }
        },
        "x-ui-hints": {
            "widget": "text",
            "layout": {
                "columns": 2,
                "rows": 3
            }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    assert_eq!(
        root.children.len(),
        1,
        "x-ui-hints must not be allocated as a node"
    );
    assert!(norm
        .arena
        .iter()
        .all(|(_, n)| !n.json_pointer.contains("x-ui-hints")));

    // A budget so tight that even one property inside `x-ui-hints.layout`
    // would trip it if it were visible. `a` is the only property that may
    // count.
    let tiny_profile = profile::load(
        br#"
        name = "tiny.test"
        code_prefix = "TINY"

        [structural]
        max_total_properties = 1
        "#,
    )
    .unwrap();
    let rules = RuleSet::from_profile(&tiny_profile).unwrap();
    let diagnostics = rules.check_all(&norm.arena, &tiny_profile);
    let budget_hit = diagnostics
        .iter()
        .find(|d| d.code == "TINY-S-max-total-properties");
    assert!(
        budget_hit.is_none(),
        "x-ui-hints content must not contribute to max_total_properties, got: {:?}",
        diagnostics
    );
}

#[test]
fn normalize_deep_content_behind_unevaluated_items_trips_depth_budget() {
    // Cross-talk case: the visible tree is compliant and unchanged. The
    // depth diagnostic comes only from content newly visible behind
    // `unevaluatedItems`.
    let schema = json!({
        "type": "object",
        "properties": {
            "safe": { "type": "string" }
        },
        "unevaluatedItems": {
            "type": "object",
            "properties": {
                "deep": { "type": "string" }
            }
        }
    });

    let norm = normalize(schema).unwrap();

    // root=0, properties/safe=1, unevaluatedItems=1, its properties/deep=2.
    let tiny_profile = profile::load(
        br#"
        name = "tiny.test"
        code_prefix = "TINY"

        [structural]
        max_object_depth = 1
        "#,
    )
    .unwrap();
    let rules = RuleSet::from_profile(&tiny_profile).unwrap();
    let diagnostics = rules.check_all(&norm.arena, &tiny_profile);

    let depth_hit = diagnostics
        .iter()
        .find(|d| d.code == "TINY-S-max-depth" && d.pointer == "/unevaluatedItems/properties/deep");
    assert!(
        depth_hit.is_some(),
        "expected a max-depth diagnostic from content behind unevaluatedItems, got: {:?}",
        diagnostics
    );

    let visible_hit = diagnostics
        .iter()
        .find(|d| d.code == "TINY-S-max-depth" && d.pointer == "/properties/safe");
    assert!(
        visible_hit.is_none(),
        "the compliant visible tree must not gain a depth diagnostic, got: {:?}",
        diagnostics
    );
}
