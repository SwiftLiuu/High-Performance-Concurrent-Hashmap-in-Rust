
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

// SAFETY: All access to internal data is protected by locks
unsafe impl<K: Send, V: Send, S: Send> Send for CuckooHashMap<K, V, S> {}
unsafe impl<K: Send + Sync, V: Send + Sync, S: Sync> Sync for CuckooHashMap<K, V, S> {}

impl<K, V> CuckooHashMap<K, V, RandomState>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    /// Creates an empty CuckooHashMap.
    pub fn new() -> Self {
        Self::with_capacity_and_hasher(DEFAULT_CAPACITY, RandomState::new())
    }

    /// Creates an empty CuckooHashMap with specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, RandomState::new())
    }
}

impl<K, V, S> CuckooHashMap<K, V, S>
where
    K: Hash + Eq + Clone,
    V: Clone,
    S: BuildHasher,
{
    /// Creates an empty CuckooHashMap with custom hasher.
    pub fn with_hasher(hash_builder: S) -> Self {
        Self::with_capacity_and_hasher(DEFAULT_CAPACITY, hash_builder)
    }

    /// Creates an empty CuckooHashMap with capacity and custom hasher.
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        let hp = reserve_calc(capacity.max(1));
        let num_buckets = hashsize(hp);
        let num_locks = num_buckets.min(MAX_NUM_LOCKS);

        let mut locks = Vec::with_capacity(num_locks);
        for _ in 0..num_locks {
            locks.push(Spinlock::new());
        }

        Self {
            buckets: UnsafeCell::new(BucketContainer::new(hp)),
            old_buckets: UnsafeCell::new(None),
            locks,
            hash_builder,
            resize_counter: AtomicUsize::new(0),
            num_remaining_lazy_rehash_locks: AtomicUsize::new(0),
            minimum_load_factor: AtomicF64::new(DEFAULT_MINIMUM_LOAD_FACTOR),
            maximum_hashpower: AtomicUsize::new(NO_MAXIMUM_HASHPOWER),
        }
    }

    // ========== Helper Methods ==========

    /// Map bucket index to lock index
    #[inline]
    fn lock_ind(&self, bucket_ind: usize) -> usize {
        lock_ind_for(bucket_ind, self.locks.len())
    }

    // ========== Capacity Methods ==========

    /// Returns the number of elements in the map.
    pub fn len(&self) -> usize {
        let mut count: i64 = 0;
        for lock in &self.locks {
            count += lock.elem_counter();
        }
        count.max(0) as usize
    }

    /// Returns true if the map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the current capacity.
    pub fn capacity(&self) -> usize {
        unsafe { (*self.buckets.get()).capacity() }
    }

    /// Returns the current load factor (len / capacity).
    pub fn load_factor(&self) -> f64 {
        let cap = self.capacity();
        if cap == 0 {
            0.0
        } else {
            self.len() as f64 / cap as f64
        }
    }

    /// Returns the current hashpower.
    pub fn hashpower(&self) -> usize {
        unsafe { (*self.buckets.get()).hashpower() }
    }

    // ========== Lookup Methods ==========

    /// Returns a clone of the value corresponding to the key.
    ///
    /// This method uses optimistic lock-free reads with version counters
    /// as described in the paper "Algorithmic Improvements for Fast Concurrent
    /// Cuckoo Hashing" (EuroSys 2014). It first attempts a lock-free read,
    /// and only falls back to locked read if a concurrent modification is detected.
    pub fn get(&self, key: &K) -> Option<V> {
        let hv = hash_key(key, &self.hash_builder);

        for _ in 0..3 {
            match self.try_get_optimistic(key, hv) {
                OptimisticReadResult::Found(value) => return Some(value),
                OptimisticReadResult::NotFound => return None,
                OptimisticReadResult::Retry => continue,
            }
        }

        self.get_locked(key, hv)
    }

    /// Optimistic lock-free read attempt.
    /// Returns the result of trying to read without locks.
    fn try_get_optimistic(&self, key: &K, hv: HashValue) -> OptimisticReadResult<V> {
        let hp = self.hashpower();
        let resize_counter = self.resize_counter.load(Ordering::Acquire);
        let i1 = index_hash(hp, hv.hash);
        let i2 = alt_index(hp, hv.partial, i1);

        let l1 = self.lock_ind(i1);
        let l2 = self.lock_ind(i2);

        let v1_before = self.locks[l1].read_version();
        let v2_before = if l1 != l2 {
            self.locks[l2].read_version()
        } else {
            v1_before
        };

        if self.locks[l1].is_version_locked(v1_before)
            || (l1 != l2 && self.locks[l2].is_version_locked(v2_before))
        {
            return OptimisticReadResult::Retry;
        }

        if !self.locks[l1].is_migrated() || (l1 != l2 && !self.locks[l2].is_migrated()) {
            return OptimisticReadResult::Retry;
        }

        let result = unsafe {
            let buckets = &*self.buckets.get();

            let mut found_value: Option<V> = None;
            for slot in 0..SLOT_PER_BUCKET {
                let bucket = buckets.get(i1);
                if bucket.occupied(slot) {
                    if bucket.partial(slot) == Some(hv.partial) {
                        if bucket.key(slot) == Some(key) {
                            found_value = bucket.value(slot).cloned();
                            break;
                        }
                    }
                }
            }

            if found_value.is_none() {
                for slot in 0..SLOT_PER_BUCKET {
                    let bucket = buckets.get(i2);
                    if bucket.occupied(slot) {
                        if bucket.partial(slot) == Some(hv.partial) {
                            if bucket.key(slot) == Some(key) {
                                found_value = bucket.value(slot).cloned();
                                break;
                            }
                        }
                    }
                }
            }

            found_value
        };

        let v1_after = self.locks[l1].read_version();
        let v2_after = if l1 != l2 {
            self.locks[l2].read_version()
        } else {
            v1_after
        };

        if self.resize_counter.load(Ordering::Acquire) != resize_counter {
            return OptimisticReadResult::Retry;
        }

        if v1_before != v1_after || v2_before != v2_after {
            return OptimisticReadResult::Retry;
        }

        match result {
            Some(value) => OptimisticReadResult::Found(value),
            None => OptimisticReadResult::NotFound,
        }
    }

    /// Locked read - fallback when optimistic read fails.
    fn get_locked(&self, key: &K, hv: HashValue) -> Option<V> {
        let guard = self.snapshot_and_lock_two(hv);

        unsafe {
            if let Some((bucket_idx, slot)) = self.cuckoo_find(key, hv.partial, guard.i1, guard.i2)
            {
                let bucket = (*self.buckets.get()).get(bucket_idx);
                bucket.value(slot).cloned()
            } else {
                None
            }
        }
    }

    /// Returns true if the map contains the key.
    ///
    /// Uses optimistic lock-free reads with fallback to locked reads.
    pub fn contains_key(&self, key: &K) -> bool {
        let hv = hash_key(key, &self.hash_builder);

        for _ in 0..3 {
            match self.try_contains_key_optimistic(key, hv) {
                OptimisticReadResult::Found(()) => return true,
                OptimisticReadResult::NotFound => return false,
                OptimisticReadResult::Retry => continue,
            }
        }

        self.contains_key_locked(key, hv)
    }

    /// Optimistic lock-free contains_key attempt.
    fn try_contains_key_optimistic(&self, key: &K, hv: HashValue) -> OptimisticReadResult<()> {
        let hp = self.hashpower();
        let resize_counter = self.resize_counter.load(Ordering::Acquire);
        let i1 = index_hash(hp, hv.hash);
        let i2 = alt_index(hp, hv.partial, i1);

        let l1 = self.lock_ind(i1);
        let l2 = self.lock_ind(i2);

        let v1_before = self.locks[l1].read_version();
        let v2_before = if l1 != l2 {
            self.locks[l2].read_version()
        } else {
            v1_before
        };

        if self.locks[l1].is_version_locked(v1_before)
            || (l1 != l2 && self.locks[l2].is_version_locked(v2_before))
        {
            return OptimisticReadResult::Retry;
        }

        if !self.locks[l1].is_migrated() || (l1 != l2 && !self.locks[l2].is_migrated()) {
            return OptimisticReadResult::Retry;
        }

        let found = unsafe {
            let buckets = &*self.buckets.get();

            let mut found = false;
            for slot in 0..SLOT_PER_BUCKET {
                let bucket = buckets.get(i1);
                if bucket.occupied(slot) {
                    if bucket.partial(slot) == Some(hv.partial) {
                        if bucket.key(slot) == Some(key) {
                            found = true;
                            break;
                        }
                    }
                }
            }

            if !found {
                for slot in 0..SLOT_PER_BUCKET {
                    let bucket = buckets.get(i2);
                    if bucket.occupied(slot) {
                        if bucket.partial(slot) == Some(hv.partial) {
                            if bucket.key(slot) == Some(key) {
                                found = true;
                                break;
                            }
                        }
                    }
                }
            }

            found
        };

        let v1_after = self.locks[l1].read_version();
        let v2_after = if l1 != l2 {
            self.locks[l2].read_version()
        } else {
            v1_after
        };

        if self.resize_counter.load(Ordering::Acquire) != resize_counter {
            return OptimisticReadResult::Retry;
        }

        if v1_before != v1_after || v2_before != v2_after {
            return OptimisticReadResult::Retry;
        }

        if found {
            OptimisticReadResult::Found(())
        } else {
            OptimisticReadResult::NotFound
        }
    }

    /// Locked contains_key - fallback when optimistic read fails.
    fn contains_key_locked(&self, key: &K, hv: HashValue) -> bool {
        let guard = self.snapshot_and_lock_two(hv);
        unsafe { self.cuckoo_find(key, hv.partial, guard.i1, guard.i2).is_some() }
    }

    /// Returns a clone of the key-value pair if present.
    #[allow(dead_code)]
    pub fn get_key_value(&self, key: &K) -> Option<(K, V)> {
        let hv = hash_key(key, &self.hash_builder);
        let guard = self.snapshot_and_lock_two(hv);

        unsafe {
            if let Some((bucket_idx, slot)) = self.cuckoo_find(key, hv.partial, guard.i1, guard.i2)
            {
                let bucket = (*self.buckets.get()).get(bucket_idx);
                let k = bucket.key(slot)?.clone();
                let v = bucket.value(slot)?.clone();
                Some((k, v))
            } else {
                None
            }
        }
    }