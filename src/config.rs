//! Configuration constants for the cuckoo hash map.

/// Number of slots per bucket (fixed for cache efficiency)
pub const SLOT_PER_BUCKET: usize = 4;

/// Default initial capacity (2^16 * 4 = 262,144 elements)
pub const DEFAULT_CAPACITY: usize = (1 << 16) * SLOT_PER_BUCKET;

/// Minimum load factor before expansion throws error
pub const DEFAULT_MINIMUM_LOAD_FACTOR: f64 = 0.05;

/// Maximum BFS path length for cuckoo displacement
pub const MAX_BFS_PATH_LEN: usize = 5;

/// Maximum number of locks 
pub const MAX_NUM_LOCKS: usize = 2048;

/// MurmurHash constant for alternate index calculation
pub const MURMUR_CONST: usize = 0xc6a4a7935bd1e995;

/// Sentinel value: no limit on hashpower
pub const NO_MAXIMUM_HASHPOWER: usize = usize::MAX;

/// BFS queue size: 2 * sum(SLOT_PER_BUCKET^k for k in 0..MAX_BFS_PATH_LEN)
/// For SLOT_PER_BUCKET=4, MAX_BFS_PATH_LEN=5: 2 * 341 = 682
pub const MAX_CUCKOO_COUNT: usize =
    2 * ((const_pow(SLOT_PER_BUCKET, MAX_BFS_PATH_LEN) - 1) / (SLOT_PER_BUCKET - 1));

/// Compile-time power function
pub const fn const_pow(base: usize, exp: usize) -> usize {
    if exp == 0 {
        1
    } else {
        base * const_pow(base, exp - 1)
    }
}
