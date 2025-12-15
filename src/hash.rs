//! Hash utilities: HashValue, AtomicF64, and index computation functions.

use crate::config::{MURMUR_CONST, SLOT_PER_BUCKET};
use std::hash::{BuildHasher, Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

/// Combined hash value with full hash and 8-bit partial key
#[derive(Clone, Copy, Debug, Default)]
pub struct HashValue {
    pub hash: u64,
    pub partial: u8,
}

/// Atomic f64 wrapper using bit-casting to AtomicU64
/// (Rust std doesn't provide AtomicF64)
#[derive(Debug)]
pub struct AtomicF64(AtomicU64);

impl AtomicF64 {
    /// Create a new AtomicF64 with the given value
    pub fn new(val: f64) -> Self {
        Self(AtomicU64::new(val.to_bits()))
    }

    /// Load the current value with the given memory ordering
    pub fn load(&self, order: Ordering) -> f64 {
        f64::from_bits(self.0.load(order))
    }

    /// Store a new value with the given memory ordering
    pub fn store(&self, val: f64, order: Ordering) {
        self.0.store(val.to_bits(), order)
    }

    /// Swap the current value with a new value and return the old value
    #[allow(dead_code)]
    pub fn swap(&self, val: f64, order: Ordering) -> f64 {
        f64::from_bits(self.0.swap(val.to_bits(), order))
    }
}

impl Clone for AtomicF64 {
    fn clone(&self) -> Self {
        Self::new(self.load(Ordering::Relaxed))
    }
}

/// Compute 8-bit partial key from full hash via XOR-folding.
/// This is independent of hashpower, enabling fast table doubling.
#[inline]
pub fn partial_key(hash: u64) -> u8 {
    let h32 = (hash as u32) ^ ((hash >> 32) as u32);
    let h16 = (h32 as u16) ^ ((h32 >> 16) as u16);
    (h16 as u8) ^ ((h16 >> 8) as u8)
}

/// Number of buckets for given hashpower: 2^hp
#[inline]
pub const fn hashsize(hp: usize) -> usize {
    1usize << hp
}

/// Bitmask for bucket index: 2^hp - 1
#[inline]
pub const fn hashmask(hp: usize) -> usize {
    hashsize(hp) - 1
}

/// Primary bucket index from hash
#[inline]
pub fn index_hash(hp: usize, hash: u64) -> usize {
    (hash as usize) & hashmask(hp)
}

/// Alternate bucket index using partial key.
/// Critical property: alt_index(hp, p, alt_index(hp, p, i)) == i
#[inline]
pub fn alt_index(hp: usize, partial: u8, index: usize) -> usize {
    let nonzero_tag = (partial as usize) + 1;
    (index ^ nonzero_tag.wrapping_mul(MURMUR_CONST)) & hashmask(hp)
}

/// Compute hash value for a key using the provided hasher
#[inline]
pub fn hash_key<K: Hash, S: BuildHasher>(key: &K, build_hasher: &S) -> HashValue {
    let mut hasher = build_hasher.build_hasher();
    key.hash(&mut hasher);
    let hash = hasher.finish();
    HashValue {
        hash,
        partial: partial_key(hash),
    }
}

/// Compute hashpower needed for given capacity
pub fn reserve_calc(n: usize) -> usize {
    let buckets = (n + SLOT_PER_BUCKET - 1) / SLOT_PER_BUCKET;
    let mut hp = 0;
    while (1usize << hp) < buckets {
        hp += 1;
    }
    hp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alt_index_symmetric() {
        for hp in 4..16 {
            for partial in 0..=255u8 {
                for index in 0..hashsize(hp).min(1000) {
                    let alt = alt_index(hp, partial, index);
                    let back = alt_index(hp, partial, alt);
                    assert_eq!(
                        back, index,
                        "hp={}, partial={}, index={}",
                        hp, partial, index
                    );
                }
            }
        }
    }

    #[test]
    fn test_reserve_calc() {
        assert_eq!(reserve_calc(1), 0);
        assert_eq!(reserve_calc(4), 0);
        assert_eq!(reserve_calc(5), 1);
        assert_eq!(reserve_calc(8), 1);
        assert_eq!(reserve_calc(9), 2);
    }
}
