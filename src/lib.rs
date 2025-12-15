//! A concurrent hash map using cuckoo hashing.
//!
//! This crate provides `CuckooHashMap`, a high-performance concurrent hash map
//! that uses cuckoo hashing for O(1) lookups and amortized O(1) insertions.
//!
//! # Example
//!
//! ```
//! use cuckoo_hashmap::CuckooHashMap;
//!
//! let map = CuckooHashMap::new();
//! map.insert("key", "value").unwrap();
//! assert_eq!(map.get(&"key"), Some("value"));
//! ```

mod bucket;
mod config;
mod error;
mod hash;
mod iter;
mod lock;
mod map;

pub use config::{
    DEFAULT_CAPACITY, DEFAULT_MINIMUM_LOAD_FACTOR, MAX_BFS_PATH_LEN, SLOT_PER_BUCKET,
};
pub use error::{CuckooError, Result};
pub use iter::{IntoIter, Iter, Keys, Values};
pub use map::CuckooHashMap;

// Additional trait implementations
use std::fmt;
use std::hash::{BuildHasher, Hash};
use std::iter::FromIterator;

impl<K, V, S> fmt::Debug for CuckooHashMap<K, V, S>
where
    K: Hash + Eq + Clone + fmt::Debug,
    V: Clone + fmt::Debug,
    S: BuildHasher,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K, V, S> Clone for CuckooHashMap<K, V, S>
where
    K: Hash + Eq + Clone,
    V: Clone,
    S: BuildHasher + Clone,
{
    fn clone(&self) -> Self {
        let new_map = Self::with_hasher(self.hash_builder.clone());
        for (k, v) in self.iter() {
            let _ = new_map.insert(k, v);
        }
        new_map
    }
}

impl<K, V, S> FromIterator<(K, V)> for CuckooHashMap<K, V, S>
where
    K: Hash + Eq + Clone,
    V: Clone,
    S: BuildHasher + Default,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let map = Self::default();
        for (k, v) in iter {
            let _ = map.insert(k, v);
        }
        map
    }
}

impl<K, V, S> Extend<(K, V)> for CuckooHashMap<K, V, S>
where
    K: Hash + Eq + Clone,
    V: Clone,
    S: BuildHasher,
{
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            let _ = self.insert(k, v);
        }
    }
}
