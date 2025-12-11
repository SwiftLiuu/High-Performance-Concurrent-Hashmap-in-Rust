
use std::cell::UnsafeCell;
use std::collections::hash_map::RandomState;
use std::collections::{HashSet, VecDeque};
use std::hash::{BuildHasher, Hash};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::bucket::{Bucket, BucketContainer};
use crate::config::*;
use crate::error::{CuckooError, Result};
use crate::hash::*;
use crate::lock::{
    lock_ind_for, AllLocksWriteGuard, Spinlock, TwoBucketGuard, TwoBucketWriteGuard,
};

/// Result of a single insert attempt
enum InsertResult<V> {
    Success(Option<V>), // Inserted (with old value if key existed)
    Duplicate(V),       // Key existed, value updated
    NeedCuckoo,         // Both buckets full, need cuckoo displacement
    #[allow(dead_code)]
    ResizeNeeded, // Cuckoo failed, need resize
}

/// Result of an optimistic lock-free read attempt.
/// Used by the version-counter based optimistic concurrency control.
enum OptimisticReadResult<T> {
    /// Successfully read data, no concurrent modification detected
    Found(T),
    /// Key not found, no concurrent modification detected
    NotFound,
    /// Concurrent modification detected, caller should retry
    Retry,
}

/// BFS slot info for path search
#[derive(Clone, Copy, Default)]
struct BSlot {
    bucket: usize,
    pathcode: u16, // Encodes slot choices as base-SLOT_PER_BUCKET number
    depth: i8,
}

/// Record for cuckoo path
#[derive(Clone, Copy, Default)]
struct CuckooRecord {
    bucket: usize,
    slot: usize,
    hv: Option<HashValue>,
    version: u64,
    is_empty: bool,
}

/// A concurrent hash map using cuckoo hashing.
///
/// This implementation provides O(1) expected time for lookups and
/// amortized O(1) for insertions, with support for concurrent access
/// from multiple threads.
pub struct CuckooHashMap<K, V, S = RandomState> {
    /// Current bucket storage
    /// SAFETY: Access requires holding appropriate lock(s)
    buckets: UnsafeCell<BucketContainer<K, V>>,

    /// Old buckets during lazy rehash
    /// SAFETY: Access requires holding lock being rehashed
    old_buckets: UnsafeCell<Option<BucketContainer<K, V>>>,

    /// Striped locks array
    pub(crate) locks: Vec<Spinlock>,

    /// Hash function builder
    pub(crate) hash_builder: S,

    /// Counter incremented on each resize
    /// Used to detect concurrent resizes
    resize_counter: AtomicUsize,

    /// Number of locks remaining to rehash
    /// When this reaches 0, old_buckets can be deallocated
    num_remaining_lazy_rehash_locks: AtomicUsize,

    /// Minimum load factor for automatic expansion
    minimum_load_factor: AtomicF64,

    /// Maximum hashpower (NO_MAXIMUM_HASHPOWER = unlimited)
    maximum_hashpower: AtomicUsize,
}