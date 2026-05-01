use rustc_hash::FxHasher;
use std::hash::Hasher;

use crate::normalize::NormalizedSchema;

/// In-memory content-hash cache for normalized schemas.
///
/// The cache is cleared between CLI invocations in Phase 1.
/// Disk persistence is deferred to Phase 2 server mode.
#[derive(Debug, Default)]
pub struct Cache {
    inner: std::collections::HashMap<u64, NormalizedSchema>,
}

impl Cache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, hash: u64) -> Option<&NormalizedSchema> {
        self.inner.get(&hash)
    }

    pub fn insert(&mut self, hash: u64, schema: NormalizedSchema) {
        self.inner.insert(hash, schema);
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

/// Compute a fast hash of raw JSON bytes for cache keys.
pub fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = FxHasher::default();
    hasher.write(bytes);
    hasher.finish()
}
