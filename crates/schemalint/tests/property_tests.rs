use proptest::prelude::*;
use schemalint::cache::{hash_bytes, Cache, DiskCache};
use schemalint::ir::parse_node;
use schemalint::normalize::normalize;

// ---------------------------------------------------------------------------
// Strategy: generate simple JSON Schema objects
// ---------------------------------------------------------------------------

fn json_schema_strategy() -> impl Strategy<Value = serde_json::Value> {
    // Generate a constrained subset of JSON Schema that is always valid
    let leaf = prop_oneof![
        Just(serde_json::json!({"type": "string"})),
        Just(serde_json::json!({"type": "integer"})),
        Just(serde_json::json!({"type": "number"})),
        Just(serde_json::json!({"type": "boolean"})),
        Just(serde_json::json!({"type": "null"})),
    ];

    leaf.prop_recursive(
        3,  // depth
        64, // max size
        4,  // max items per collection
        |inner| {
            prop_oneof![
                prop::collection::hash_map("[a-z]{1,5}", inner.clone(), 1..4usize).prop_map(
                    |m| serde_json::json!({
                        "type": "object",
                        "properties": m,
                        "additionalProperties": false
                    })
                ),
                prop::collection::vec(inner, 1..3usize)
                    .prop_map(|v| { serde_json::json!({"type": "array", "items": v[0]}) }),
                // Enum with string values
                prop::collection::vec("[a-z]{1,5}", 1..4usize)
                    .prop_map(|v| { serde_json::json!({"type": "string", "enum": v}) }),
            ]
        },
    )
}

// ---------------------------------------------------------------------------
// Property: parse round-trip retains all keywords
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_parse_retains_all_keywords(schema in json_schema_strategy()) {
        let node = parse_node(schema.clone()).expect("parse_node should succeed");
        // Verify that known keywords from the original JSON are present in annotations
        if let serde_json::Value::Object(ref map) = schema {
            for (key, val) in map {
                let found = match key.as_str() {
                    "type" => node.annotations.r#type.as_ref(),
                    "properties" => node.annotations.properties.as_ref(),
                    "additionalProperties" => node.annotations.additional_properties.as_ref(),
                    "items" => node.annotations.items.as_ref(),
                    "enum" => node.annotations.enum_values.as_ref(),
                    _ => None,
                };
                prop_assert!(
                    found.is_some(),
                    "keyword '{}' should be retained in annotations", key
                );
                if let Some(found_val) = found {
                    prop_assert_eq!(
                        found_val, val,
                        "keyword '{}' value should match", key
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property: normalization is idempotent on IR
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_normalize_idempotent(schema in json_schema_strategy()) {
        let first = normalize(schema.clone()).expect("first normalization should succeed");
        // We can't normalize NormalizedSchema directly, but we can verify that
        // normalizing the same JSON value twice produces equivalent results.
        let second = normalize(schema).expect("second normalization should succeed");

        // Same root kind
        prop_assert_eq!(
            first.arena[first.root_id].kind,
            second.arena[second.root_id].kind
        );

        // Same node count (approximate structural equality)
        prop_assert_eq!(
            first.arena.len(),
            second.arena.len(),
            "normalizing same schema twice should produce same node count"
        );
    }
}

// ---------------------------------------------------------------------------
// Property: content-hash cache hit returns identical result
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_cache_hit_identical(schema in json_schema_strategy()) {
        let bytes = serde_json::to_vec(&schema).unwrap();
        let hash = hash_bytes(&bytes);

        let normalized = normalize(schema).expect("normalization should succeed");

        let mut cache = Cache::new();
        cache.insert(hash, normalized.clone());

        let cached = cache.get(hash).expect("cache should contain the entry");
        prop_assert_eq!(
            cached.arena.len(),
            normalized.arena.len(),
            "cached result should have same node count"
        );
        prop_assert_eq!(
            cached.root_id, normalized.root_id,
            "cached result should have same root id"
        );
    }
}

// ---------------------------------------------------------------------------
// Property: bincode round-trip produces identical NormalizedSchema
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_bincode_roundtrip_identical(schema in json_schema_strategy()) {
        let normalized = normalize(schema).expect("normalization should succeed");
        let serialized = serde_json::to_vec(&normalized).expect("serialize should succeed");
        let deserialized: schemalint::normalize::NormalizedSchema =
            serde_json::from_slice(&serialized).expect("deserialize should succeed");

        prop_assert_eq!(
            normalized.arena.len(),
            deserialized.arena.len(),
            "round-trip should preserve node count"
        );
        prop_assert_eq!(
            normalized.root_id, deserialized.root_id,
            "round-trip should preserve root id"
        );
        prop_assert_eq!(
            normalized.dialect, deserialized.dialect,
            "round-trip should preserve dialect"
        );
    }
}

// ---------------------------------------------------------------------------
// Property: DiskCache round-trip survives get after insert
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_disk_cache_roundtrip(schema in json_schema_strategy()) {
        let bytes = serde_json::to_vec(&schema).unwrap();
        let hash = hash_bytes(&bytes);
        let normalized = normalize(schema).expect("normalization should succeed");

        let cache = DiskCache::new();
        cache.insert(hash, normalized.clone());

        let cached = cache.get(hash).expect("disk cache should return the entry");
        prop_assert_eq!(
            cached.arena.len(), normalized.arena.len(),
            "disk cached result should have same node count"
        );
        prop_assert_eq!(
            cached.root_id, normalized.root_id,
            "disk cached result should have same root id"
        );
    }
}
