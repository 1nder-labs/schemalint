use std::collections::HashMap;

use crate::normalize::NormalizedSchema;

/// In-memory content-hash cache for normalized schemas.
///
/// The cache is cleared between CLI invocations in Phase 1.
/// Disk persistence is deferred to Phase 2 server mode.
#[derive(Debug, Default)]
pub struct Cache {
    inner: HashMap<[u8; 32], NormalizedSchema>,
}

impl Cache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, hash: &[u8; 32]) -> Option<&NormalizedSchema> {
        self.inner.get(hash)
    }

    pub fn insert(&mut self, hash: [u8; 32], schema: NormalizedSchema) {
        self.inner.insert(hash, schema);
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}
