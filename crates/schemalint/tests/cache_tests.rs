use std::fs;
use std::sync::Arc;
use std::thread;

use schemalint::cache::{hash_bytes, Cache, DiskCache};
use schemalint::normalize::NormalizedSchema;
use serde_json::json;

fn make_dummy_schema() -> NormalizedSchema {
    schemalint::normalize::normalize(json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" }
        }
    }))
    .unwrap()
}

// ---------------------------------------------------------------------------
// In-memory Cache tests
// ---------------------------------------------------------------------------

#[test]
fn cache_insert_get_hit() {
    let mut cache = Cache::new();
    let schema = make_dummy_schema();
    let bytes = serde_json::to_vec(&json!({"test": "hit"})).unwrap();
    let hash = hash_bytes(&bytes);
    cache.insert(hash, bytes.clone(), schema.clone());
    let cached = cache.get(hash, &bytes);
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().root_id, schema.root_id);
    assert_eq!(cached.unwrap().arena.len(), schema.arena.len());
}

#[test]
fn cache_miss_different_content() {
    let mut cache = Cache::new();
    let schema = make_dummy_schema();
    let bytes = serde_json::to_vec(&json!({"content": "a"})).unwrap();
    let hash = hash_bytes(&bytes);
    cache.insert(hash, bytes, schema);
    let other_bytes = b"completely different content";
    let other_hash = hash_bytes(other_bytes);
    assert!(cache.get(other_hash, other_bytes).is_none());
}

#[test]
fn cache_miss_empty() {
    let cache = Cache::new();
    let bytes = b"nothing inserted";
    let hash = hash_bytes(bytes);
    assert!(cache.get(hash, bytes).is_none());
}

#[test]
fn cache_clear_empties_all_entries() {
    let mut cache = Cache::new();
    let bytes1 = serde_json::to_vec(&json!({"a": 1})).unwrap();
    let bytes2 = serde_json::to_vec(&json!({"b": 2})).unwrap();
    let h1 = hash_bytes(&bytes1);
    let h2 = hash_bytes(&bytes2);
    cache.insert(h1, bytes1.clone(), make_dummy_schema());
    cache.insert(h2, bytes2.clone(), make_dummy_schema());
    cache.clear();
    assert!(cache.get(h1, &bytes1).is_none());
    assert!(cache.get(h2, &bytes2).is_none());
}

#[test]
fn cache_multiple_inserts() {
    let mut cache = Cache::new();
    let s1 = make_dummy_schema();
    let s2 = schemalint::normalize::normalize(json!({"type": "integer"})).unwrap();
    let b1 = b"schema1";
    let b2 = b"schema2";
    let h1 = hash_bytes(b1);
    let h2 = hash_bytes(b2);
    cache.insert(h1, b1.to_vec(), s1.clone());
    cache.insert(h2, b2.to_vec(), s2.clone());
    let got1 = cache.get(h1, b1).unwrap();
    let got2 = cache.get(h2, b2).unwrap();
    assert_eq!(got1.arena.len(), s1.arena.len());
    assert_eq!(got2.arena.len(), s2.arena.len());
}

#[test]
fn cache_overwrite_same_hash() {
    let mut cache = Cache::new();
    let s1 = make_dummy_schema();
    let s2 = schemalint::normalize::normalize(json!({"type": "boolean"})).unwrap();
    let bytes = b"same hash different schema";
    let hash = hash_bytes(bytes);
    cache.insert(hash, bytes.to_vec(), s1);
    cache.insert(hash, bytes.to_vec(), s2.clone());
    let cached = cache.get(hash, bytes).unwrap();
    assert_eq!(cached.arena.len(), s2.arena.len());
}

// ---------------------------------------------------------------------------
// DiskCache tests
// ---------------------------------------------------------------------------

/// Resolve the PID-isolated subdirectory that `with_cache_dir` uses.
fn pid_isolated(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join(format!("pid-{}", std::process::id()))
}

#[test]
fn disk_cache_roundtrip_write_read() {
    let dir = tempfile::tempdir().unwrap();
    let cache = DiskCache::with_cache_dir(dir.path().to_path_buf());
    let schema = make_dummy_schema();
    let bytes = serde_json::to_vec(&json!({"disk": "roundtrip"})).unwrap();
    let hash = hash_bytes(&bytes);
    cache.insert(hash, bytes.clone(), schema.clone());
    let cached = cache.get(hash, &bytes).unwrap();
    assert_eq!(cached.root_id, schema.root_id);
    assert_eq!(cached.arena.len(), schema.arena.len());
}

#[test]
fn disk_cache_truncated_file_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = serde_json::to_vec(&json!({"trunc": "test"})).unwrap();
    let hash = hash_bytes(&bytes);

    // First DiskCache: insert the schema normally.
    {
        let cache = DiskCache::with_cache_dir(dir.path().to_path_buf());
        let schema = make_dummy_schema();
        cache.insert(hash, bytes.clone(), schema);
    }

    // Corrupt the file on disk inside the PID-isolated subdirectory.
    let file_path = pid_isolated(dir.path()).join(format!("{:016x}.bin", hash));
    fs::write(&file_path, &[0x01, 0x02]).unwrap();

    // New DiskCache: in-memory is empty, must fall back to disk.
    let fresh_cache = DiskCache::with_cache_dir(dir.path().to_path_buf());
    assert!(fresh_cache.get(hash, &bytes).is_none());
}

#[test]
fn disk_cache_invalid_version_header_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = serde_json::to_vec(&json!({"ver": "invalid"})).unwrap();
    let hash = hash_bytes(&bytes);

    // First DiskCache: insert the schema normally.
    {
        let cache = DiskCache::with_cache_dir(dir.path().to_path_buf());
        let schema = make_dummy_schema();
        cache.insert(hash, bytes.clone(), schema);
    }

    // Corrupt: overwrite with wrong version (99) and no body inside PID-isolated dir.
    let file_path = pid_isolated(dir.path()).join(format!("{:016x}.bin", hash));
    let wrong_version: u32 = 99;
    fs::write(&file_path, &wrong_version.to_le_bytes()).unwrap();

    // New DiskCache: in-memory is empty, must fall back to disk.
    let fresh_cache = DiskCache::with_cache_dir(dir.path().to_path_buf());
    assert!(fresh_cache.get(hash, &bytes).is_none());
}

#[test]
fn disk_cache_nonexistent_entry_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let cache = DiskCache::with_cache_dir(dir.path().to_path_buf());
    let bytes = b"this file does not exist on disk";
    let hash = hash_bytes(bytes);
    assert!(cache.get(hash, bytes).is_none());
}

#[test]
fn disk_cache_empty_cache_dir_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let cache = DiskCache::with_cache_dir(dir.path().to_path_buf());
    let bytes = b"";
    let hash = 0xdead_beef;
    assert!(cache.get(hash, bytes).is_none());
}

#[test]
fn disk_cache_second_get_hits_memory() {
    let dir = tempfile::tempdir().unwrap();
    let cache = DiskCache::with_cache_dir(dir.path().to_path_buf());
    let schema = make_dummy_schema();
    let bytes = serde_json::to_vec(&json!({"mem": "cache"})).unwrap();
    let hash = hash_bytes(&bytes);
    cache.insert(hash, bytes.clone(), schema.clone());
    // First get — reads from disk, populates memory
    let _first = cache.get(hash, &bytes).unwrap();
    // Delete the file on disk from the PID-isolated subdirectory
    let file_path = pid_isolated(dir.path()).join(format!("{:016x}.bin", hash));
    fs::remove_file(&file_path).unwrap();
    // Second get — should hit in-memory, still return Some
    let second = cache.get(hash, &bytes);
    assert!(second.is_some());
}

#[test]
fn multithreaded_insert_read() {
    let dir = tempfile::tempdir().unwrap();
    let cache = Arc::new(DiskCache::with_cache_dir(dir.path().to_path_buf()));
    let schema = make_dummy_schema();
    let bytes = serde_json::to_vec(&json!({"mt": "shared"})).unwrap();
    let hash = hash_bytes(&bytes);

    cache.insert(hash, bytes.clone(), schema.clone());

    let mut handles = vec![];
    for _ in 0..8 {
        let cache = Arc::clone(&cache);
        let thread_bytes = bytes.clone();
        handles.push(thread::spawn(move || {
            let cached = cache.get(hash, &thread_bytes);
            assert!(cached.is_some());
            let c = cached.unwrap();
            assert_eq!(c.root_id, schema.root_id);
            assert!(c.arena.len() > 0);
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
}

// ---------------------------------------------------------------------------
// Hash tests
// ---------------------------------------------------------------------------

#[test]
fn hash_bytes_different_content_different_hash() {
    let h1 = hash_bytes(b"hello world");
    let h2 = hash_bytes(b"hello worle");
    assert_ne!(h1, h2);
}

#[test]
fn hash_bytes_same_content_same_hash() {
    let a = b"identical content for hashing";
    assert_eq!(hash_bytes(a), hash_bytes(a));
}

#[test]
fn hash_bytes_empty_input() {
    let h = hash_bytes(b"");
    // Should not panic; hash should be deterministic
    assert_eq!(h, hash_bytes(b""));
}

#[test]
fn hash_bytes_single_byte() {
    let h = hash_bytes(&[0x42]);
    assert_eq!(h, hash_bytes(&[0x42]));
    assert_ne!(h, hash_bytes(&[0x43]));
}

#[test]
fn hash_bytes_large_input() {
    let large = vec![0u8; 10_000];
    let h = hash_bytes(&large);
    assert_eq!(h, hash_bytes(&large));
}
