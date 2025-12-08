//! Spinlock with metadata and RAII lock guards.
//!
//! This module implements the lock striping mechanism described in the paper
//! "Algorithmic Improvements for Fast Concurrent Cuckoo Hashing" (EuroSys 2014).
//!
//! Key features:
//! - Version counters for optimistic lock-free reads (per-lock versioning)
//! - Cache-line aligned locks to prevent false sharing
//! - Element counters for tracking table size
//! - Migration status for lazy rehashing

use std::hint::spin_loop;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};

/// Cache-line aligned spinlock with per-lock metadata.
///
/// Metadata stored per-lock:
/// - elem_counter: Count of elements under this lock (sum = table size)
/// - is_migrated: Whether buckets under this lock have been rehashed
/// - version: Version counter for optimistic lock-free reads
///
/// The version counter is used for optimistic concurrency control as described
/// in the paper. Readers check the version before and after reading data.
/// If the version changed (odd = write in progress, or different value),
/// the read must be retried.
#[repr(align(64))]
pub struct Spinlock {
    locked: AtomicBool,
    elem_counter: AtomicI64,
    is_migrated: AtomicBool,
    /// Version counter for optimistic reads.
    /// - Even values: No write in progress, data is consistent
    /// - Odd values: Write in progress, readers must retry
    /// Writers increment before and after modification.
    version: AtomicU64,
}

impl Spinlock {
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            elem_counter: AtomicI64::new(0),
            is_migrated: AtomicBool::new(true),
            version: AtomicU64::new(0),
        }
    }

    /// Acquire the lock, spinning until successful
    #[inline]
    pub fn lock(&self) {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            while self.locked.load(Ordering::Relaxed) {
                spin_loop();
            }
        }
    }

    /// Release the lock
    #[inline]
    pub fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }

    /// Try to acquire lock, returns true if successful
    #[inline]
    #[allow(dead_code)]
    pub fn try_lock(&self) -> bool {
        self.locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    /// Get element counter value
    #[inline]
    pub fn elem_counter(&self) -> i64 {
        self.elem_counter.load(Ordering::Relaxed)
    }

    /// Add to element counter (can be negative)
    #[inline]
    pub fn add_elem_counter(&self, delta: i64) {
        self.elem_counter.fetch_add(delta, Ordering::Relaxed);
    }

    /// Check if buckets under this lock are migrated
    #[inline]
    pub fn is_migrated(&self) -> bool {
        self.is_migrated.load(Ordering::Acquire)
    }

    /// Set migration status
    #[inline]
    pub fn set_migrated(&self, migrated: bool) {
        self.is_migrated.store(migrated, Ordering::Release);
    }

    // ========== Version Counter Methods for Optimistic Reads ==========

    /// Read the current version counter.
    /// Used by readers to check for concurrent modifications.
    /// Returns the version value with Acquire ordering to ensure
    /// subsequent reads see data written before this version.
    #[inline]
    pub fn read_version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    /// Check if version indicates a write is in progress (odd value).
    #[inline]
    pub fn is_version_locked(&self, version: u64) -> bool {
        version & 1 == 1
    }

    /// Increment version before starting a write operation.
    /// This makes the version odd, signaling to readers that a write is in progress.
    /// Must be called while holding the spinlock.
    #[inline]
    pub fn increment_version_start(&self) {
        self.version.fetch_add(1, Ordering::Release);
    }

    /// Increment version after completing a write operation.
    /// This makes the version even again, signaling that data is consistent.
    /// Must be called while holding the spinlock.
    #[inline]
    pub fn increment_version_end(&self) {
        self.version.fetch_add(1, Ordering::Release);
    }
}

impl Default for Spinlock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Spinlock {
    fn clone(&self) -> Self {
        Self {
            locked: AtomicBool::new(false),
            elem_counter: AtomicI64::new(self.elem_counter()),
            is_migrated: AtomicBool::new(self.is_migrated()),
            version: AtomicU64::new(0),
        }
    }
}

/// Map bucket index to lock index, given the number of locks.
/// The number of locks is always a power of two.
#[inline]
pub fn lock_ind_for(bucket_ind: usize, num_locks: usize) -> usize {
    bucket_ind & (num_locks - 1)
}

/// RAII guard for two bucket locks (handles same-lock case)
/// Used for read operations - does NOT increment version counters.
pub struct TwoBucketGuard<'a> {
    locks: &'a [Spinlock],
    lock1_idx: usize,
    lock2_idx: Option<usize>, // None if same as lock1
    pub i1: usize,            // First bucket index
    pub i2: usize,            // Second bucket index
}

impl<'a> TwoBucketGuard<'a> {
    /// Create guard by locking two buckets in order (lower index first)
    pub fn new(locks: &'a [Spinlock], i1: usize, i2: usize) -> Self {
        let num_locks = locks.len();
        let l1 = lock_ind_for(i1, num_locks);
        let l2 = lock_ind_for(i2, num_locks);

        let (first, second) = if l1 <= l2 { (l1, l2) } else { (l2, l1) };

        locks[first].lock();
        let lock2_idx = if first != second {
            locks[second].lock();
            Some(second)
        } else {
            None
        };

        Self {
            locks,
            lock1_idx: first,
            lock2_idx,
            i1,
            i2,
        }
    }

    /// Get the first lock index
    #[inline]
    #[allow(dead_code)]
    pub fn lock1_idx(&self) -> usize {
        self.lock1_idx
    }

    /// Get the second lock index (if different from first)
    #[inline]
    #[allow(dead_code)]
    pub fn lock2_idx(&self) -> Option<usize> {
        self.lock2_idx
    }

    /// Manually unlock (usually done via Drop)
    #[allow(dead_code)]
    pub fn unlock(&mut self) {
        if let Some(l2) = self.lock2_idx.take() {
            self.locks[l2].unlock();
        }
        self.locks[self.lock1_idx].unlock();
    }
}

impl<'a> Drop for TwoBucketGuard<'a> {
    fn drop(&mut self) {
        if let Some(l2) = self.lock2_idx {
            self.locks[l2].unlock();
        }
        self.locks[self.lock1_idx].unlock();
    }
}

/// RAII guard for two bucket locks for WRITE operations.
/// Increments version counters on creation (making them odd)
/// and on drop (making them even again).
/// This allows optimistic readers to detect concurrent writes.
pub struct TwoBucketWriteGuard<'a> {
    locks: &'a [Spinlock],
    lock1_idx: usize,
    lock2_idx: Option<usize>, // None if same as lock1
    pub i1: usize,            // First bucket index
    pub i2: usize,            // Second bucket index
}

impl<'a> TwoBucketWriteGuard<'a> {
    /// Create write guard by locking two buckets and incrementing versions.
    /// Order: lock lower index first, then increment versions.
    pub fn new(locks: &'a [Spinlock], i1: usize, i2: usize) -> Self {
        let num_locks = locks.len();
        let l1 = lock_ind_for(i1, num_locks);
        let l2 = lock_ind_for(i2, num_locks);

        let (first, second) = if l1 <= l2 { (l1, l2) } else { (l2, l1) };

        locks[first].lock();
        let lock2_idx = if first != second {
            locks[second].lock();
            Some(second)
        } else {
            None
        };

        locks[first].increment_version_start();
        if let Some(l2) = lock2_idx {
            locks[l2].increment_version_start();
        }

        Self {
            locks,
            lock1_idx: first,
            lock2_idx,
            i1,
            i2,
        }
    }

    /// Get the first lock index
    #[inline]
    #[allow(dead_code)]
    pub fn lock1_idx(&self) -> usize {
        self.lock1_idx
    }

    /// Get the second lock index (if different from first)
    #[inline]
    #[allow(dead_code)]
    pub fn lock2_idx(&self) -> Option<usize> {
        self.lock2_idx
    }
}

impl<'a> Drop for TwoBucketWriteGuard<'a> {
    fn drop(&mut self) {
        self.locks[self.lock1_idx].increment_version_end();
        if let Some(l2) = self.lock2_idx {
            self.locks[l2].increment_version_end();
        }

        if let Some(l2) = self.lock2_idx {
            self.locks[l2].unlock();
        }
        self.locks[self.lock1_idx].unlock();
    }
}

/// RAII guard for all locks (used during iteration - read only)
pub struct AllLocksGuard<'a> {
    locks: &'a [Spinlock],
    num_locked: usize,
}

impl<'a> AllLocksGuard<'a> {
    /// Lock all locks in order
    pub fn new(locks: &'a [Spinlock]) -> Self {
        for lock in locks.iter() {
            lock.lock();
        }
        Self {
            locks,
            num_locked: locks.len(),
        }
    }
}

impl<'a> Drop for AllLocksGuard<'a> {
    fn drop(&mut self) {
        for i in 0..self.num_locked {
            self.locks[i].unlock();
        }
    }
}

/// RAII guard for all locks for WRITE operations (used during resize).
/// Increments all version counters to signal a major modification.
pub struct AllLocksWriteGuard<'a> {
    locks: &'a [Spinlock],
    num_locked: usize,
}

impl<'a> AllLocksWriteGuard<'a> {
    /// Lock all locks in order and increment all versions
    pub fn new(locks: &'a [Spinlock]) -> Self {
        for lock in locks.iter() {
            lock.lock();
        }
        for lock in locks.iter() {
            lock.increment_version_start();
        }
        Self {
            locks,
            num_locked: locks.len(),
        }
    }
}

impl<'a> Drop for AllLocksWriteGuard<'a> {
    fn drop(&mut self) {
        for i in 0..self.num_locked {
            self.locks[i].increment_version_end();
        }
        for i in 0..self.num_locked {
            self.locks[i].unlock();
        }
    }
}

/// Result of trying to lock two buckets with resize detection
#[allow(dead_code)]
pub enum TryLockResult<'a> {
    /// Successfully locked both buckets
    Locked(TwoBucketGuard<'a>),
    /// Resize was detected, caller should retry
    ResizeDetected,
}
