//! Bucket and slot storage for key-value pairs.

use crate::config::SLOT_PER_BUCKET;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A single slot containing a key-value pair and partial hash
#[derive(Clone)]
pub struct Slot<K, V> {
    pub key: K,
    pub value: V,
    pub partial: u8,
}

/// A bucket containing SLOT_PER_BUCKET slots.
/// Uses Option for each slot to track occupancy.
pub struct Bucket<K, V> {
    slots: [Option<Slot<K, V>>; SLOT_PER_BUCKET],
}

impl<K, V> Bucket<K, V> {
    /// Create an empty bucket
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }
}

// Manual Default because array of Options doesn't auto-derive for large N
impl<K, V> Default for Bucket<K, V> {
    fn default() -> Self {
        Self {
            slots: [(); SLOT_PER_BUCKET].map(|_| None),
        }
    }
}

impl<K, V> Bucket<K, V> {
    /// Check if slot is occupied
    #[inline]
    pub fn occupied(&self, slot: usize) -> bool {
        self.slots[slot].is_some()
    }

    /// Get partial key for slot (if occupied)
    #[inline]
    pub fn partial(&self, slot: usize) -> Option<u8> {
        self.slots[slot].as_ref().map(|s| s.partial)
    }

    /// Get reference to key (if occupied)
    #[inline]
    pub fn key(&self, slot: usize) -> Option<&K> {
        self.slots[slot].as_ref().map(|s| &s.key)
    }

    /// Get reference to value (if occupied)
    #[inline]
    pub fn value(&self, slot: usize) -> Option<&V> {
        self.slots[slot].as_ref().map(|s| &s.value)
    }

    /// Get mutable reference to value (if occupied)
    #[inline]
    pub fn value_mut(&mut self, slot: usize) -> Option<&mut V> {
        self.slots[slot].as_mut().map(|s| &mut s.value)
    }

    /// Get reference to slot
    #[inline]
    #[allow(dead_code)]
    pub fn slot(&self, slot: usize) -> Option<&Slot<K, V>> {
        self.slots[slot].as_ref()
    }

    /// Set a slot with key-value pair
    #[inline]
    pub fn set(&mut self, slot: usize, partial: u8, key: K, value: V) {
        self.slots[slot] = Some(Slot {
            key,
            value,
            partial,
        });
    }

    /// Erase and return the key-value pair from a slot
    #[inline]
    pub fn erase(&mut self, slot: usize) -> Option<(K, V)> {
        self.slots[slot].take().map(|s| (s.key, s.value))
    }

    /// Find first empty slot, or None if bucket is full
    #[inline]
    pub fn find_empty_slot(&self) -> Option<usize> {
        for i in 0..SLOT_PER_BUCKET {
            if !self.occupied(i) {
                return Some(i);
            }
        }
        None
    }
}

impl<K: Clone, V: Clone> Clone for Bucket<K, V> {
    fn clone(&self) -> Self {
        Self {
            slots: self.slots.clone(),
        }
    }
}

/// Container managing all buckets with atomic hashpower.
/// Size is always 2^hashpower buckets.
pub struct BucketContainer<K, V> {
    buckets: Vec<Bucket<K, V>>,
    hashpower: AtomicUsize,
}

impl<K, V> BucketContainer<K, V> {
    /// Create a new container with 2^hashpower buckets
    pub fn new(hashpower: usize) -> Self {
        let size = 1usize << hashpower;
        let mut buckets = Vec::with_capacity(size);
        for _ in 0..size {
            buckets.push(Bucket::default());
        }
        Self {
            buckets,
            hashpower: AtomicUsize::new(hashpower),
        }
    }

    /// Get current hashpower (atomic load)
    #[inline]
    pub fn hashpower(&self) -> usize {
        self.hashpower.load(Ordering::Acquire)
    }

    /// Number of buckets
    #[inline]
    pub fn size(&self) -> usize {
        self.buckets.len()
    }

    /// Total capacity (buckets * slots per bucket)
    #[inline]
    pub fn capacity(&self) -> usize {
        self.size() * SLOT_PER_BUCKET
    }

    /// Get reference to a bucket
    #[inline]
    pub fn get(&self, index: usize) -> &Bucket<K, V> {
        &self.buckets[index]
    }

    /// Get mutable reference to a bucket
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> &mut Bucket<K, V> {
        &mut self.buckets[index]
    }

    /// Set key-value in specific bucket and slot
    #[inline]
    pub fn set_kv(&mut self, bucket: usize, slot: usize, partial: u8, key: K, value: V) {
        self.buckets[bucket].set(slot, partial, key, value);
    }

    /// Erase key-value from specific bucket and slot
    #[inline]
    pub fn erase_kv(&mut self, bucket: usize, slot: usize) -> Option<(K, V)> {
        self.buckets[bucket].erase(slot)
    }

    /// Clear all buckets
    pub fn clear(&mut self) {
        for bucket in &mut self.buckets {
            for slot in 0..SLOT_PER_BUCKET {
                bucket.erase(slot);
            }
        }
    }
}

impl<K: Clone, V: Clone> Clone for BucketContainer<K, V> {
    fn clone(&self) -> Self {
        Self {
            buckets: self.buckets.clone(),
            hashpower: AtomicUsize::new(self.hashpower()),
        }
    }
}
