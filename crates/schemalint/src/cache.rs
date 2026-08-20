use std::collections::{HashMap, VecDeque};
use std::hash::Hasher;

use rustc_hash::FxHasher;

use crate::normalize::NormalizedSchema;

const MAX_MEMORY_ENTRIES: usize = 1_000;

#[derive(Debug, Clone)]
struct CacheEntry {
    schema: NormalizedSchema,
    original_bytes: Vec<u8>,
}

/// Bounded in-memory cache for normalized schemas.
///
/// `FxHasher` is intentionally fast rather than collision resistant, so every
/// hit also compares the original bytes. A hash collision therefore becomes a
/// cache miss instead of returning the wrong normalized schema.
#[derive(Debug, Default)]
pub struct Cache {
    entries: HashMap<u64, CacheEntry>,
    insertion_order: VecDeque<u64>,
}

impl Cache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, hash: u64, bytes: &[u8]) -> Option<&NormalizedSchema> {
        let entry = self.entries.get(&hash)?;
        (entry.original_bytes == bytes).then_some(&entry.schema)
    }

    pub fn insert(&mut self, hash: u64, bytes: Vec<u8>, schema: NormalizedSchema) {
        let is_new = !self.entries.contains_key(&hash);
        if is_new && self.entries.len() >= MAX_MEMORY_ENTRIES {
            if let Some(oldest) = self.insertion_order.pop_front() {
                self.entries.remove(&oldest);
            }
        }
        self.entries.insert(
            hash,
            CacheEntry {
                schema,
                original_bytes: bytes,
            },
        );
        if is_new {
            self.insertion_order.push_back(hash);
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.insertion_order.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = FxHasher::default();
    hasher.write(bytes);
    hasher.finish()
}
