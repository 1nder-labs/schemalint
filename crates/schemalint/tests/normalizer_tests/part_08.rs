// ---------------------------------------------------------------------------
// U3: a `definitions` entry shadowed by a same-named `$defs` entry is still
// allocated as a node and linted. `$defs` keeps winning the `defs` lookup
// map used for `$ref` resolution — allocation and resolution precedence are
// separate concerns.
// ---------------------------------------------------------------------------

use schemalint::profile;
use schemalint::profiles::OPENAI_SO_2026_04_30;
use schemalint::rules::RuleSet;

#[test]
fn normalize_shadowed_definitions_entry_is_linted() {
    // `oneOf` is forbidden by the OpenAI profile. Before this fix, a
    // colliding `definitions/D` was skipped entirely during `build_defs`,
    // so this `oneOf` was never allocated and never reported: a false
    // green, while the provider still saw that subtree.
    let schema = json!({
        "type": "object",
        "properties": {
            "item": { "$ref": "#/$defs/D" }
        },
        "additionalProperties": false,
        "required": ["item"],
        "$defs": {
            "D": { "type": "string" }
        },
        "definitions": {
            "D": { "oneOf": [{ "type": "string" }, { "type": "integer" }] }
        }
    });

    let norm = normalize(schema).unwrap();
    let openai = profile::load(OPENAI_SO_2026_04_30.as_bytes()).unwrap();
    let rules = RuleSet::from_profile(&openai).unwrap();
    let diagnostics = rules.check_all(&norm.arena, &openai);

    let shadowed_hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.pointer == "/definitions/D")
        .collect();
    assert_eq!(
        shadowed_hits.len(),
        1,
        "expected exactly one diagnostic at /definitions/D, got: {:?}",
        diagnostics
    );
    assert_eq!(shadowed_hits[0].code, "OAI-K-oneOf");
}

#[test]
fn normalize_ref_to_defs_still_resolves_to_defs_node_when_shadowed() {
    let schema = json!({
        "$defs": {
            "D": { "type": "string" }
        },
        "definitions": {
            "D": { "type": "integer" }
        },
        "$ref": "#/$defs/D"
    });

    let norm = normalize(schema).unwrap();

    // `defs` map: $defs wins the lookup used for $ref resolution.
    let defs_id = *norm.defs.get("D").unwrap();
    assert_eq!(norm.arena[defs_id].json_pointer, "/$defs/D");
    assert_eq!(
        norm.arena[defs_id].annotations.r#type,
        Some(serde_json::json!("string"))
    );

    // The root's own $ref resolves to that same $defs node.
    assert_eq!(norm.arena[norm.root_id].ref_target, Some(defs_id));

    // The shadowed `definitions/D` node still exists, at its own pointer,
    // with its own (different) content.
    let shadowed_id = norm
        .arena
        .iter()
        .find(|(_, n)| n.json_pointer == "/definitions/D")
        .map(|(id, _)| id)
        .expect("shadowed /definitions/D node must be allocated");
    assert_eq!(
        norm.arena[shadowed_id].annotations.r#type,
        Some(serde_json::json!("integer"))
    );
    assert_ne!(shadowed_id, defs_id);
}

#[test]
fn normalize_ref_to_definitions_pointer_resolves_to_definitions_node_when_shadowed() {
    // `#/definitions/D` is the literal pointer of the `definitions` node, so
    // it must resolve there — not fall back to the `$defs` node via the
    // `defs` map, now that the `definitions` node is allocated at its own
    // pointer and indexed by `resolve_refs`.
    let schema = json!({
        "type": "object",
        "properties": {
            "viaDefs": { "$ref": "#/$defs/D" },
            "viaDefinitions": { "$ref": "#/definitions/D" }
        },
        "$defs": {
            "D": { "type": "string" }
        },
        "definitions": {
            "D": { "type": "integer" }
        }
    });

    let norm = normalize(schema).unwrap();
    let root = &norm.arena[norm.root_id];
    let via_defs_id = root
        .children
        .iter()
        .copied()
        .find(|&id| norm.arena[id].json_pointer == "/properties/viaDefs")
        .unwrap();
    let via_definitions_id = root
        .children
        .iter()
        .copied()
        .find(|&id| norm.arena[id].json_pointer == "/properties/viaDefinitions")
        .unwrap();

    let defs_target = norm.arena[via_defs_id].ref_target.unwrap();
    let definitions_target = norm.arena[via_definitions_id].ref_target.unwrap();

    assert_eq!(norm.arena[defs_target].json_pointer, "/$defs/D");
    assert_eq!(
        norm.arena[defs_target].annotations.r#type,
        Some(serde_json::json!("string"))
    );

    assert_eq!(norm.arena[definitions_target].json_pointer, "/definitions/D");
    assert_eq!(
        norm.arena[definitions_target].annotations.r#type,
        Some(serde_json::json!("integer"))
    );

    assert_ne!(defs_target, definitions_target);
}

#[test]
fn normalize_definitions_without_collision_behaves_as_before() {
    // No `$defs` entry with the same name — no collision, no change in
    // behavior. One node, one `defs` map entry, at `/definitions/Item`.
    let schema = json!({
        "type": "object",
        "properties": {
            "item": { "$ref": "#/definitions/Item" }
        },
        "definitions": {
            "Item": { "type": "string" }
        }
    });

    let norm = normalize(schema).unwrap();
    assert_eq!(norm.defs.len(), 1);
    let item_id = *norm.defs.get("Item").unwrap();
    assert_eq!(norm.arena[item_id].json_pointer, "/definitions/Item");

    let definitions_nodes = norm
        .arena
        .iter()
        .filter(|(_, n)| n.json_pointer == "/definitions/Item")
        .count();
    assert_eq!(
        definitions_nodes, 1,
        "no collision must allocate exactly one node for the definitions entry"
    );
}

#[test]
fn normalize_shadowed_definitions_entry_raises_node_count_and_trips_a_budget() {
    // Node counts rise by the shadowed subtree: the shadowed
    // `definitions/D` carries its own `properties`, which the budget rules
    // walk across the *whole* arena (not just what's reachable via $ref).
    // Exercise that interaction with a profile whose `max_total_properties`
    // sits right at the edge, so the newly visible content is what tips it
    // over.
    let schema = json!({
        "type": "object",
        "properties": {
            "item": { "$ref": "#/$defs/D" }
        },
        "$defs": {
            "D": { "type": "string" }
        },
        "definitions": {
            "D": {
                "type": "object",
                "properties": {
                    "a": { "type": "string" },
                    "b": { "type": "string" }
                }
            }
        }
    });

    let norm = normalize(schema).unwrap();

    // Two properties on the shadowed node itself, plus the one root
    // property ("item") that is always allocated.
    let tiny_profile = profile::load(
        br#"
        name = "tiny.test"
        code_prefix = "TINY"

        [structural]
        max_total_properties = 2
        "#,
    )
    .unwrap();
    let rules = RuleSet::from_profile(&tiny_profile).unwrap();
    let diagnostics = rules.check_all(&norm.arena, &tiny_profile);

    let budget_hit = diagnostics
        .iter()
        .find(|d| d.code == "TINY-S-max-total-properties");
    assert!(
        budget_hit.is_some(),
        "expected the shadowed definitions/D properties to trip max_total_properties, got: {:?}",
        diagnostics
    );
}
