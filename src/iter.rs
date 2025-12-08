//! Iterator implementations for CuckooHashMap.

use crate::bucket::BucketContainer;
use crate::config::SLOT_PER_BUCKET;
use crate::lock::AllLocksGuard;
use crate::map::CuckooHashMap;
use std::hash::{BuildHasher, Hash};

/// An iterator over the key-value pairs of a CuckooHashMap.
///
/// Note: This locks the entire table for the duration of iteration,
/// blocking all other operations.
pub struct Iter<'a, K, V, S> {
    buckets: &'a BucketContainer<K, V>,
    _guard: AllLocksGuard<'a>,
    bucket_idx: usize,
    slot_idx: usize,
    _marker: std::marker::PhantomData<S>,
}

impl<'a, K: Clone, V: Clone, S> Iterator for Iter<'a, K, V, S> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        while self.bucket_idx < self.buckets.size() {
            while self.slot_idx < SLOT_PER_BUCKET {
                let bucket = self.buckets.get(self.bucket_idx);
                let slot = self.slot_idx;
                self.slot_idx += 1;

                if bucket.occupied(slot) {
                    let key = bucket.key(slot)?.clone();
                    let value = bucket.value(slot)?.clone();
                    return Some((key, value));
                }
            }
            self.bucket_idx += 1;
            self.slot_idx = 0;
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.buckets.capacity()))
    }
}

/// An iterator over the keys of a CuckooHashMap.
pub struct Keys<'a, K, V, S> {
    inner: Iter<'a, K, V, S>,
}

impl<'a, K: Clone, V: Clone, S> Iterator for Keys<'a, K, V, S> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, _)| k)
    }
}

/// An iterator over the values of a CuckooHashMap.
pub struct Values<'a, K, V, S> {
    inner: Iter<'a, K, V, S>,
}

impl<'a, K: Clone, V: Clone, S> Iterator for Values<'a, K, V, S> {
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
    }
}

/// An owning iterator over the key-value pairs of a CuckooHashMap.
pub struct IntoIter<K, V, S> {
    map: CuckooHashMap<K, V, S>,
    bucket_idx: usize,
    slot_idx: usize,
}

impl<K: Hash + Eq + Clone, V: Clone, S: BuildHasher> Iterator for IntoIter<K, V, S> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let buckets = self.map.buckets_ref();
            let size = buckets.size();

            while self.bucket_idx < size {
                while self.slot_idx < SLOT_PER_BUCKET {
                    let bucket = buckets.get(self.bucket_idx);
                    let slot = self.slot_idx;
                    self.slot_idx += 1;

                    if bucket.occupied(slot) {
                        let key = bucket.key(slot)?.clone();
                        let value = bucket.value(slot)?.clone();
                        return Some((key, value));
                    }
                }
                self.bucket_idx += 1;
                self.slot_idx = 0;
            }
        }
        None
    }
}

impl<K, V, S> CuckooHashMap<K, V, S>
where
    K: Hash + Eq + Clone,
    V: Clone,
    S: BuildHasher,
{
    /// Returns an iterator over the key-value pairs.
    ///
    /// **Warning**: This locks the entire table during iteration.
    pub fn iter(&self) -> Iter<'_, K, V, S> {
        let guard = AllLocksGuard::new(&self.locks);

        for i in 0..self.locks.len() {
            self.maybe_rehash_lock(i);
        }

        Iter {
            buckets: unsafe { self.buckets_ref() },
            _guard: guard,
            bucket_idx: 0,
            slot_idx: 0,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns an iterator over the keys.
    pub fn keys(&self) -> Keys<'_, K, V, S> {
        Keys { inner: self.iter() }
    }

    /// Returns an iterator over the values.
    pub fn values(&self) -> Values<'_, K, V, S> {
        Values { inner: self.iter() }
    }
}

impl<K, V, S> IntoIterator for CuckooHashMap<K, V, S>
where
    K: Hash + Eq + Clone,
    V: Clone,
    S: BuildHasher,
{
    type Item = (K, V);
    type IntoIter = IntoIter<K, V, S>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            map: self,
            bucket_idx: 0,
            slot_idx: 0,
        }
    }
}
