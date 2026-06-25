use std::collections::VecDeque;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use rustc_hash::FxHasher;
use std::hash::Hasher;

use crate::normalize::NormalizedSchema;

/// Monotonic counter used to disambiguate concurrent temp-file names within
/// the same process. Combined with the PID it guarantees uniqueness across
/// threads without relying on wall-clock time.
static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

const CACHE_VERSION: u32 = 2;
const MAX_MEMORY_ENTRIES: usize = 1000;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CacheEntry {
    schema: NormalizedSchema,
    original_bytes: Vec<u8>,
}

/// In-memory content-hash cache for normalized schemas.
///
/// The cache is cleared between CLI invocations in Phase 1.
/// Disk persistence is deferred to Phase 2 server mode.
#[derive(Debug, Default)]
pub struct Cache {
    inner: std::collections::HashMap<u64, CacheEntry>,
    order: VecDeque<u64>,
}

impl Cache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up a normalized schema by its content hash, verifying the
    /// stored raw bytes match the provided bytes to prevent cache poisoning
    /// via hash collisions.
    pub fn get(&self, hash: u64, bytes: &[u8]) -> Option<&NormalizedSchema> {
        let entry = self.inner.get(&hash)?;
        if entry.original_bytes == bytes {
            Some(&entry.schema)
        } else {
            None
        }
    }

    pub fn insert(&mut self, hash: u64, bytes: Vec<u8>, schema: NormalizedSchema) {
        let is_new = !self.inner.contains_key(&hash);
        if is_new && self.inner.len() >= MAX_MEMORY_ENTRIES {
            if let Some(oldest) = self.order.pop_front() {
                self.inner.remove(&oldest);
            }
        }
        self.inner.insert(
            hash,
            CacheEntry {
                schema,
                original_bytes: bytes,
            },
        );
        if is_new {
            self.order.push_back(hash);
        }
    }

    pub fn clear(&mut self) {
        self.inner.clear();
        self.order.clear();
    }
}

/// Persistent disk-backed cache extending the in-memory cache.
///
/// Each cache entry is stored as a separate file under the system cache
/// directory (e.g. `~/.cache/schemalint/`). A 4-byte version header is
/// prepended to every file for future migration. If the version does not
/// match, the entry is treated as a miss and overwritten.
#[derive(Debug)]
pub struct DiskCache {
    memory: RwLock<Cache>,
    cache_dir: Option<PathBuf>,
}

impl Default for DiskCache {
    fn default() -> Self {
        Self::new()
    }
}

impl DiskCache {
    pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
        // Isolate by PID to prevent concurrent process corruption.
        // Callers needing shared caches should use file-level advisory locks.
        let isolated = cache_dir.join(format!("pid-{}", std::process::id()));
        if let Err(e) = fs::create_dir_all(&isolated) {
            eprintln!(
                "warning: failed to create cache directory '{}': {}",
                isolated.display(),
                e
            );
            return Self {
                memory: RwLock::new(Cache::new()),
                cache_dir: None,
            };
        }
        Self {
            memory: RwLock::new(Cache::new()),
            cache_dir: Some(isolated),
        }
    }

    pub fn new() -> Self {
        let cache_dir =
            dirs::cache_dir().map(|d| d.join(format!("schemalint-{}", std::process::id())));
        if let Some(ref dir) = cache_dir {
            if let Err(e) = fs::create_dir_all(dir) {
                eprintln!(
                    "warning: failed to create cache directory '{}': {}",
                    dir.display(),
                    e
                );
            }
        }
        Self {
            memory: RwLock::new(Cache::new()),
            cache_dir,
        }
    }

    /// Look up a normalized schema by its content hash.
    ///
    /// Checks the in-memory cache first, then falls back to the on-disk
    /// cache. Disk entries are deserialized and inserted into memory on
    /// a successful read.  The stored raw bytes are verified against
    /// `bytes` on every hit to prevent cache poisoning via hash
    /// collisions.
    pub fn get(&self, hash: u64, bytes: &[u8]) -> Option<NormalizedSchema> {
        // In-memory hit
        {
            let memory = self.memory.read().unwrap();
            if let Some(cached) = memory.get(hash, bytes) {
                return Some(cached.clone());
            }
        }

        // Disk fallback
        let dir = self.cache_dir.as_ref()?;
        let path = dir.join(format!("{:016x}.bin", hash));
        let mut file = fs::File::open(&path).ok()?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).ok()?;
        if buf.len() < 4 {
            return None;
        }
        let version = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        if version != CACHE_VERSION {
            return None;
        }
        let entry: CacheEntry = serde_json::from_slice(&buf[4..]).ok()?;
        if entry.original_bytes != bytes {
            return None;
        }

        // Populate memory cache for future lookups
        self.memory.write().unwrap().insert(
            hash,
            entry.original_bytes.clone(),
            entry.schema.clone(),
        );
        Some(entry.schema)
    }

    /// Insert a normalized schema into both the in-memory and on-disk caches.
    ///
    /// Disk writes are atomic: the payload is written to a uniquely-named
    /// temporary file in the same directory and then renamed into place.
    /// `fs::rename` is atomic on POSIX filesystems (same device), so a
    /// concurrent reader will see either the old complete entry or the new
    /// complete entry — never a partial write.  On any error the temp file is
    /// cleaned up and the failure is reported as a non-fatal warning (identical
    /// to the previous behaviour).
    pub fn insert(&self, hash: u64, bytes: Vec<u8>, schema: NormalizedSchema) {
        // Memory
        self.memory
            .write()
            .unwrap()
            .insert(hash, bytes.clone(), schema.clone());

        // Disk — atomic write via temp file + rename
        if let Some(ref dir) = self.cache_dir {
            let final_path = dir.join(format!("{:016x}.bin", hash));
            let counter = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let tmp_name = format!("{:016x}.bin.tmp.{}.{}", hash, std::process::id(), counter);
            let tmp_path = dir.join(&tmp_name);

            let mut buf = Vec::new();
            buf.extend_from_slice(&CACHE_VERSION.to_le_bytes());
            let entry = CacheEntry {
                schema,
                original_bytes: bytes,
            };
            match serde_json::to_vec(&entry) {
                Ok(serialized) => {
                    buf.extend_from_slice(&serialized);
                    // Write to temp file first.
                    if let Err(e) = fs::write(&tmp_path, &buf) {
                        eprintln!(
                            "warning: failed to write cache temp file '{}': {}",
                            tmp_path.display(),
                            e
                        );
                        // Attempt cleanup of any partially-written temp file.
                        let _ = fs::remove_file(&tmp_path);
                    } else {
                        // Atomically publish the entry.
                        if let Err(e) = fs::rename(&tmp_path, &final_path) {
                            eprintln!(
                                "warning: failed to rename cache file '{}' -> '{}': {}",
                                tmp_path.display(),
                                final_path.display(),
                                e
                            );
                            // Clean up orphaned temp file so it doesn't
                            // accumulate or confuse evict_if_needed.
                            let _ = fs::remove_file(&tmp_path);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("warning: failed to serialize schema for cache: {}", e);
                }
            }
            self.evict_if_needed(dir);
        }
    }

    /// Returns the resolved cache directory path, if one was configured.
    ///
    /// Exposed for testing so tests can inspect directory contents without
    /// re-deriving the PID-namespaced path.
    #[cfg(test)]
    pub(crate) fn cache_dir(&self) -> Option<&PathBuf> {
        self.cache_dir.as_ref()
    }

    fn evict_if_needed(&self, dir: &PathBuf) {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                eprintln!(
                    "warning: failed to read cache directory '{}': {}",
                    dir.display(),
                    e
                );
                return;
            }
        };
        let mut files: Vec<(fs::DirEntry, std::time::SystemTime)> = Vec::new();
        for entry in entries.filter_map(|e| e.ok()) {
            let Ok(meta) = entry.metadata() else { continue };
            let Ok(mtime) = meta.modified() else { continue };
            files.push((entry, mtime));
        }
        if files.len() > 1000 {
            files.sort_by_key(|a| a.1);
            let to_remove = files.len() - 1000;
            for (entry, _) in files.into_iter().take(to_remove) {
                if let Err(e) = fs::remove_file(entry.path()) {
                    eprintln!("warning: failed to remove stale cache file: {}", e);
                }
            }
        }
    }
}

/// Compute a fast hash of raw JSON bytes for cache keys.
pub fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = FxHasher::default();
    hasher.write(bytes);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Build a minimal valid `NormalizedSchema` without duplicating the
    /// normalizer internals.  `true` is a valid boolean JSON Schema that
    /// passes through the full pipeline.
    fn make_schema() -> NormalizedSchema {
        crate::normalize::normalize(serde_json::Value::Bool(true))
            .expect("normalizing `true` must succeed")
    }

    /// Serialise schema bytes the same way the production code does so that
    /// hash_bytes produces a stable key we can use in tests.
    fn schema_bytes(val: &serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(val).expect("serialization of test value must succeed")
    }

    // -----------------------------------------------------------------------
    // 1. Insert → get round-trip via disk
    // -----------------------------------------------------------------------

    /// Verify that a schema written by one `DiskCache` instance can be
    /// retrieved by a *second* instance pointing at the same directory.
    ///
    /// Using a fresh instance guarantees the warm-memory path is bypassed and
    /// the actual disk read is exercised.
    #[test]
    fn test_disk_round_trip() {
        let tmp = TempDir::new().unwrap();
        let raw = serde_json::json!({"type": "string"});
        let bytes = schema_bytes(&raw);
        let hash = hash_bytes(&bytes);
        let schema = make_schema();

        // Insert via the first instance.
        let cache1 = DiskCache::with_cache_dir(tmp.path().to_path_buf());
        cache1.insert(hash, bytes.clone(), schema);

        // Retrieve via a second instance — memory cache is cold.
        let cache2 = DiskCache::with_cache_dir(tmp.path().to_path_buf());
        let result = cache2.get(hash, &bytes);
        assert!(
            result.is_some(),
            "disk round-trip: expected a cache hit on the second DiskCache instance"
        );
    }

    /// A collision-prevention check: the same hash with *different* bytes must
    /// not return a hit (the entry stores the original bytes and verifies them).
    #[test]
    fn test_disk_round_trip_collision_rejected() {
        let tmp = TempDir::new().unwrap();
        let raw = serde_json::json!({"type": "string"});
        let bytes = schema_bytes(&raw);
        let hash = hash_bytes(&bytes);
        let schema = make_schema();

        let cache1 = DiskCache::with_cache_dir(tmp.path().to_path_buf());
        cache1.insert(hash, bytes.clone(), schema);

        let different_bytes = schema_bytes(&serde_json::json!({"type": "integer"}));
        let cache2 = DiskCache::with_cache_dir(tmp.path().to_path_buf());
        let result = cache2.get(hash, &different_bytes);
        assert!(
            result.is_none(),
            "collision guard: bytes mismatch must result in a cache miss"
        );
    }

    // -----------------------------------------------------------------------
    // 2. Atomic rename leaves no .tmp files behind
    // -----------------------------------------------------------------------

    /// After a successful insert the cache directory must contain exactly the
    /// final `.bin` file; no `.tmp` artefacts should remain.
    #[test]
    fn test_atomic_write_no_tmp_files_remain() {
        let tmp = TempDir::new().unwrap();
        let raw = serde_json::json!({"type": "object"});
        let bytes = schema_bytes(&raw);
        let hash = hash_bytes(&bytes);
        let schema = make_schema();

        let cache = DiskCache::with_cache_dir(tmp.path().to_path_buf());
        cache.insert(hash, bytes, schema);

        // The cache dir is PID-namespaced; use the accessor to get the real path.
        let cache_dir = cache.cache_dir().expect("cache dir must be set");
        let entries: Vec<_> = fs::read_dir(cache_dir)
            .expect("read_dir must succeed")
            .filter_map(|e| e.ok())
            .collect();

        let tmp_files: Vec<_> = entries
            .iter()
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
            .collect();

        assert!(
            tmp_files.is_empty(),
            "expected no leftover .tmp files after successful insert, found: {:?}",
            tmp_files.iter().map(|e| e.file_name()).collect::<Vec<_>>()
        );

        // Also assert the final file was written.
        let bin_files: Vec<_> = entries
            .iter()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".bin"))
            .collect();
        assert_eq!(
            bin_files.len(),
            1,
            "expected exactly one .bin cache file, found {:?}",
            bin_files.iter().map(|e| e.file_name()).collect::<Vec<_>>()
        );
    }

    // -----------------------------------------------------------------------
    // 3. Eviction trims the disk cache to ≤ 1000 entries
    // -----------------------------------------------------------------------

    /// Insert 1 001 distinct entries so `evict_if_needed` runs and must trim
    /// the directory back to exactly 1 000 files.
    ///
    /// Uniqueness of each entry is ensured by embedding the index into both
    /// the bytes (so `hash_bytes` produces a different key) and the raw JSON
    /// value used to look it up.  No wall-clock calls are made; the ordering
    /// used by eviction is filesystem mtime which is non-deterministic at
    /// millisecond granularity, so we only assert the *count* post-eviction.
    #[test]
    fn test_eviction_trims_to_limit() {
        let tmp = TempDir::new().unwrap();
        // Use a *shared* base dir; with_cache_dir appends `pid-<pid>` so all
        // inserts land in the same subdirectory.
        let cache = DiskCache::with_cache_dir(tmp.path().to_path_buf());
        let cache_dir = cache.cache_dir().expect("cache dir must be set").clone();

        let total = 1_001usize;
        for i in 0..total {
            // Embed the index so every entry has a distinct hash.
            let raw = serde_json::json!({"__test_index": i});
            let bytes = schema_bytes(&raw);
            let hash = hash_bytes(&bytes);
            let schema = make_schema();
            // Insert directly; each call triggers evict_if_needed at the end,
            // which is correct — we want to exercise the trim path.
            cache.insert(hash, bytes, schema);
        }

        let file_count = fs::read_dir(&cache_dir)
            .expect("read_dir must succeed")
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".bin"))
            .count();

        assert_eq!(
            file_count, 1_000,
            "eviction must trim disk cache to exactly 1 000 entries, found {}",
            file_count
        );
    }

    // -----------------------------------------------------------------------
    // 4. In-memory Cache — direct get/insert/eviction paths
    // -----------------------------------------------------------------------

    /// `Cache::get` on an empty cache must return `None`.
    #[test]
    fn test_memory_cache_get_miss_absent_hash() {
        let cache = Cache::new();
        let bytes = b"hello";
        let result = cache.get(0xdeadbeef, bytes);
        assert!(result.is_none(), "get on absent hash must return None");
    }

    /// `Cache::get` with the correct hash but mismatched bytes must return
    /// `None` (collision guard in the in-memory layer).
    #[test]
    fn test_memory_cache_get_miss_bytes_mismatch() {
        let mut cache = Cache::new();
        let bytes_a = b"schema-a".to_vec();
        let hash = hash_bytes(&bytes_a);
        cache.insert(hash, bytes_a, make_schema());

        // Same hash, different bytes — must be rejected.
        let result = cache.get(hash, b"schema-b");
        assert!(
            result.is_none(),
            "bytes mismatch must result in None even when hash matches"
        );
    }

    /// `Cache::get` returns the schema when hash AND bytes both match.
    #[test]
    fn test_memory_cache_get_hit() {
        let mut cache = Cache::new();
        let bytes = b"schema-x".to_vec();
        let hash = hash_bytes(&bytes);
        cache.insert(hash, bytes.clone(), make_schema());

        let result = cache.get(hash, &bytes);
        assert!(result.is_some(), "get with matching hash+bytes must hit");
    }

    /// Re-inserting the same hash must update the stored schema and must not
    /// grow the `order` deque (dedup of is_new check).
    #[test]
    fn test_memory_cache_reinsertion_does_not_grow_order() {
        let mut cache = Cache::new();
        let bytes = b"same".to_vec();
        let hash = hash_bytes(&bytes);
        cache.insert(hash, bytes.clone(), make_schema());
        let order_len_after_first = cache.order.len();
        // Insert again with the same hash.
        cache.insert(hash, bytes.clone(), make_schema());
        assert_eq!(
            cache.order.len(),
            order_len_after_first,
            "re-inserting an existing hash must not append to the order deque"
        );
        // The entry must still be reachable.
        assert!(cache.get(hash, &bytes).is_some());
    }

    /// When MAX_MEMORY_ENTRIES + 1 entries are inserted the LRU entry (first
    /// inserted) must be evicted from `inner` AND removed from `order`.
    #[test]
    fn test_memory_cache_lru_eviction() {
        let mut cache = Cache::new();

        // Insert the sentinel entry first.
        let sentinel_bytes = b"sentinel".to_vec();
        let sentinel_hash = hash_bytes(&sentinel_bytes);
        cache.insert(sentinel_hash, sentinel_bytes.clone(), make_schema());

        // Fill up to the limit (one slot is already taken by the sentinel).
        for i in 0..MAX_MEMORY_ENTRIES {
            let bytes = format!("entry-{}", i).into_bytes();
            let hash = hash_bytes(&bytes);
            cache.insert(hash, bytes, make_schema());
        }

        // The in-memory map now has MAX_MEMORY_ENTRIES + 1 entries, but the
        // last insert must have triggered eviction, removing the sentinel.
        assert!(
            cache.get(sentinel_hash, &sentinel_bytes).is_none(),
            "sentinel entry must have been evicted after MAX_MEMORY_ENTRIES overflow"
        );
        // Total size must be capped at the limit.
        assert_eq!(
            cache.inner.len(),
            MAX_MEMORY_ENTRIES,
            "inner map size must equal MAX_MEMORY_ENTRIES after eviction"
        );
    }

    /// `Cache::clear` must remove all entries and allow no further hits.
    #[test]
    fn test_memory_cache_clear() {
        let mut cache = Cache::new();
        let bytes = b"to-be-cleared".to_vec();
        let hash = hash_bytes(&bytes);
        cache.insert(hash, bytes.clone(), make_schema());
        assert!(
            cache.get(hash, &bytes).is_some(),
            "pre-clear: must be a hit"
        );

        cache.clear();

        assert!(
            cache.get(hash, &bytes).is_none(),
            "post-clear: must be a miss"
        );
        assert!(
            cache.inner.is_empty(),
            "inner map must be empty after clear"
        );
        assert!(
            cache.order.is_empty(),
            "order deque must be empty after clear"
        );
    }

    // -----------------------------------------------------------------------
    // 5. with_cache_dir — graceful degradation when the path is unusable
    // -----------------------------------------------------------------------

    /// If `with_cache_dir` receives a path that cannot be created as a
    /// directory (because the parent is actually a regular file), it must
    /// return a `DiskCache` whose `cache_dir()` is `None` — the memory-only
    /// fallback.
    #[test]
    fn test_with_cache_dir_fallback_when_path_is_file() {
        let tmp = TempDir::new().unwrap();
        // Create a regular FILE where we want to use as a base directory.
        let file_path = tmp.path().join("not_a_dir");
        fs::write(&file_path, b"I am a file").expect("write must succeed");

        // Pass the file as the cache dir — `with_cache_dir` will attempt
        // `create_dir_all(file_path/pid-<pid>)` which must fail because the
        // path component `file_path` is a file, not a directory.
        let cache = DiskCache::with_cache_dir(file_path);
        assert!(
            cache.cache_dir().is_none(),
            "cache_dir must be None when directory creation fails"
        );

        // The fallback cache must still be usable in memory-only mode.
        let bytes = b"mem-only".to_vec();
        let hash = hash_bytes(&bytes);
        cache.insert(hash, bytes.clone(), make_schema());
        assert!(
            cache.get(hash, &bytes).is_some(),
            "memory-only fallback must still return hits"
        );
    }

    // -----------------------------------------------------------------------
    // 6. Disk read — CACHE_VERSION mismatch treated as miss
    // -----------------------------------------------------------------------

    /// Write a cache file with a wrong version prefix (99) into the cache
    /// directory and assert that `DiskCache::get` treats it as a miss.
    #[test]
    fn test_disk_get_version_mismatch_is_miss() {
        let tmp = TempDir::new().unwrap();
        let cache = DiskCache::with_cache_dir(tmp.path().to_path_buf());
        let cache_dir = cache.cache_dir().expect("cache dir must be set").clone();

        let raw = serde_json::json!({"type": "boolean"});
        let bytes = schema_bytes(&raw);
        let hash = hash_bytes(&bytes);

        // Write the file manually with a wrong version (99 instead of CACHE_VERSION).
        let bad_version: u32 = 99;
        let mut buf = bad_version.to_le_bytes().to_vec();
        let schema = make_schema();
        let entry = CacheEntry {
            schema,
            original_bytes: bytes.clone(),
        };
        buf.extend_from_slice(&serde_json::to_vec(&entry).unwrap());
        let path = cache_dir.join(format!("{:016x}.bin", hash));
        fs::write(&path, &buf).expect("manual write must succeed");

        // A fresh DiskCache pointing at the same base dir must see a miss.
        let cache2 = DiskCache::with_cache_dir(tmp.path().to_path_buf());
        let result = cache2.get(hash, &bytes);
        assert!(
            result.is_none(),
            "a disk entry with a wrong CACHE_VERSION must be treated as a miss"
        );
    }

    // -----------------------------------------------------------------------
    // 7. Disk read — file shorter than 4 bytes treated as miss
    // -----------------------------------------------------------------------

    /// Write a 3-byte file into the cache directory and assert that `get`
    /// returns `None` (the `buf.len() < 4` guard at line 159).
    #[test]
    fn test_disk_get_truncated_file_is_miss() {
        let tmp = TempDir::new().unwrap();
        let cache = DiskCache::with_cache_dir(tmp.path().to_path_buf());
        let cache_dir = cache.cache_dir().expect("cache dir must be set").clone();

        let bytes = b"truncated-file-test".to_vec();
        let hash = hash_bytes(&bytes);
        let path = cache_dir.join(format!("{:016x}.bin", hash));
        // Only 3 bytes — too short to hold the 4-byte version prefix.
        fs::write(&path, &[0x01u8, 0x02, 0x03]).expect("write must succeed");

        let cache2 = DiskCache::with_cache_dir(tmp.path().to_path_buf());
        let result = cache2.get(hash, &bytes);
        assert!(
            result.is_none(),
            "a file shorter than 4 bytes must be treated as a miss"
        );
    }

    // -----------------------------------------------------------------------
    // 8. Disk read — correct version but invalid JSON payload treated as miss
    // -----------------------------------------------------------------------

    /// Write a file with the correct 4-byte version prefix followed by garbage
    /// bytes that are not valid JSON. `serde_json::from_slice` must fail and
    /// `get` must return `None`.
    #[test]
    fn test_disk_get_corrupt_json_is_miss() {
        let tmp = TempDir::new().unwrap();
        let cache = DiskCache::with_cache_dir(tmp.path().to_path_buf());
        let cache_dir = cache.cache_dir().expect("cache dir must be set").clone();

        let bytes = b"corrupt-json-test".to_vec();
        let hash = hash_bytes(&bytes);
        let path = cache_dir.join(format!("{:016x}.bin", hash));

        let mut buf = CACHE_VERSION.to_le_bytes().to_vec();
        buf.extend_from_slice(b"not valid json !!!");
        fs::write(&path, &buf).expect("write must succeed");

        let cache2 = DiskCache::with_cache_dir(tmp.path().to_path_buf());
        let result = cache2.get(hash, &bytes);
        assert!(
            result.is_none(),
            "a disk entry with corrupt JSON must be treated as a miss"
        );
    }

    // -----------------------------------------------------------------------
    // 9. Disk get on a hash that was never inserted → miss
    // -----------------------------------------------------------------------

    /// A fresh `DiskCache` has no file on disk for the requested hash;
    /// `File::open` returns `Err` and `get` returns `None`.
    #[test]
    fn test_disk_get_absent_entry_is_miss() {
        let tmp = TempDir::new().unwrap();
        let cache = DiskCache::with_cache_dir(tmp.path().to_path_buf());

        let bytes = b"never-written".to_vec();
        let hash = hash_bytes(&bytes);

        let result = cache.get(hash, &bytes);
        assert!(
            result.is_none(),
            "a hash that was never inserted must be a miss"
        );
    }

    // -----------------------------------------------------------------------
    // 10. rename failure path — pre-existing directory at the target path
    // -----------------------------------------------------------------------

    /// Create a directory at the location that `insert` would use for the
    /// final `.bin` file. The atomic rename (tmp → final) must fail, the
    /// warning must be non-fatal, and the `.tmp.` file must be cleaned up.
    #[test]
    fn test_disk_insert_rename_failure_cleans_up_tmp() {
        let tmp = TempDir::new().unwrap();
        let cache = DiskCache::with_cache_dir(tmp.path().to_path_buf());
        let cache_dir = cache.cache_dir().expect("cache dir must be set").clone();

        let raw = serde_json::json!({"type": "null"});
        let bytes = schema_bytes(&raw);
        let hash = hash_bytes(&bytes);

        // Pre-create a directory at the exact path `insert` would rename into.
        let final_path = cache_dir.join(format!("{:016x}.bin", hash));
        fs::create_dir_all(&final_path).expect("creating dir as rename target must succeed");

        // `insert` must not panic; the rename will fail (target is a dir),
        // and the temp file must be cleaned up.
        cache.insert(hash, bytes, make_schema());

        // No orphaned .tmp. files must remain.
        let tmp_count = fs::read_dir(&cache_dir)
            .expect("read_dir must succeed")
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
            .count();
        assert_eq!(
            tmp_count, 0,
            "rename failure must not leave orphaned .tmp files"
        );

        // The pre-existing directory must still be present (we did not remove it).
        assert!(
            final_path.is_dir(),
            "pre-existing directory at final path must still exist"
        );
    }
}
