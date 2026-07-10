use std::sync::{Arc, RwLock};
use std::thread;

use schemalint::cache::{hash_bytes, Cache};
use schemalint::normalize::NormalizedSchema;
use serde_json::json;

fn schema(value: serde_json::Value) -> NormalizedSchema {
    schemalint::normalize::normalize(value).unwrap()
}

fn dummy_schema() -> NormalizedSchema {
    schema(json!({"type": "object", "properties": {"name": {"type": "string"}}}))
}

#[test]
fn cache_insert_get_hit() {
    let mut cache = Cache::new();
    let bytes = br#"{"test":"hit"}"#.to_vec();
    let hash = hash_bytes(&bytes);
    cache.insert(hash, bytes.clone(), dummy_schema());

    assert!(cache.get(hash, &bytes).is_some());
    assert_eq!(cache.len(), 1);
}

#[test]
fn collision_with_same_hash_is_a_miss() {
    let mut cache = Cache::new();
    let original = b"original".to_vec();
    let colliding = b"different bytes";
    cache.insert(42, original.clone(), dummy_schema());

    assert!(cache.get(42, &original).is_some());
    assert!(cache.get(42, colliding).is_none());
}

#[test]
fn overwrite_same_hash_replaces_bytes_and_schema() {
    let mut cache = Cache::new();
    cache.insert(7, b"first".to_vec(), schema(json!({"type": "string"})));
    cache.insert(7, b"second".to_vec(), schema(json!({"type": "integer"})));

    assert!(cache.get(7, b"first").is_none());
    assert!(cache.get(7, b"second").is_some());
    assert_eq!(cache.len(), 1);
}

#[test]
fn cache_evicts_oldest_entry_at_bound() {
    let mut cache = Cache::new();
    for index in 0..=1_000_u64 {
        cache.insert(index, index.to_le_bytes().to_vec(), dummy_schema());
    }

    assert_eq!(cache.len(), 1_000);
    assert!(cache.get(0, &0_u64.to_le_bytes()).is_none());
    assert!(cache.get(1_000, &1_000_u64.to_le_bytes()).is_some());
}

#[test]
fn clear_removes_every_entry() {
    let mut cache = Cache::new();
    cache.insert(1, b"one".to_vec(), dummy_schema());
    cache.insert(2, b"two".to_vec(), dummy_schema());
    cache.clear();

    assert!(cache.is_empty());
    assert!(cache.get(1, b"one").is_none());
}

#[test]
fn shared_cache_supports_concurrent_short_reads() {
    let cache = Arc::new(RwLock::new(Cache::new()));
    cache
        .write()
        .unwrap()
        .insert(9, b"shared".to_vec(), dummy_schema());

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let cache = Arc::clone(&cache);
            thread::spawn(move || cache.read().unwrap().get(9, b"shared").cloned())
        })
        .collect();
    for handle in handles {
        assert!(handle.join().unwrap().is_some());
    }
}

#[test]
fn hash_is_deterministic_and_content_sensitive() {
    assert_eq!(hash_bytes(b"same"), hash_bytes(b"same"));
    assert_ne!(hash_bytes(b"same"), hash_bytes(b"different"));
}
