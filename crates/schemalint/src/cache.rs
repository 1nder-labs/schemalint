use std::collections::VecDeque;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::RwLock;

use rustc_hash::FxHasher;
use std::hash::Hasher;

use crate::normalize::NormalizedSchema;

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
    pub fn insert(&self, hash: u64, bytes: Vec<u8>, schema: NormalizedSchema) {
        // Memory
        self.memory
            .write()
            .unwrap()
            .insert(hash, bytes.clone(), schema.clone());

        // Disk
        if let Some(ref dir) = self.cache_dir {
            let path = dir.join(format!("{:016x}.bin", hash));
            let mut buf = Vec::new();
            buf.extend_from_slice(&CACHE_VERSION.to_le_bytes());
            let entry = CacheEntry {
                schema,
                original_bytes: bytes,
            };
            match serde_json::to_vec(&entry) {
                Ok(serialized) => {
                    buf.extend_from_slice(&serialized);
                    if let Err(e) = fs::write(&path, &buf) {
                        eprintln!(
                            "warning: failed to write cache file '{}': {}",
                            path.display(),
                            e
                        );
                    }
                }
                Err(e) => {
                    eprintln!("warning: failed to serialize schema for cache: {}", e);
                }
            }
            self.evict_if_needed(dir);
        }
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
