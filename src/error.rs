//! Error types for cuckoo hash map operations.

use std::error::Error;
use std::fmt;

/// Errors that can occur during cuckoo hash map operations
#[derive(Debug, Clone, PartialEq)]
pub enum CuckooError {
    /// Load factor is below minimum threshold during automatic expansion.
    /// This usually indicates a poor hash function or adversarial input.
    LoadFactorTooLow { load_factor: f64, minimum: f64 },

    /// Expansion would exceed the configured maximum hashpower.
    MaximumHashpowerExceeded {
        current: usize,
        requested: usize,
        maximum: usize,
    },

    /// Table is completely full and cannot be expanded.
    /// This should be rare with proper configuration.
    TableFull,
}

impl fmt::Display for CuckooError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CuckooError::LoadFactorTooLow {
                load_factor,
                minimum,
            } => {
                write!(
                    f,
                    "load factor {:.4} is below minimum threshold {:.4}",
                    load_factor, minimum
                )
            }
            CuckooError::MaximumHashpowerExceeded {
                current,
                requested,
                maximum,
            } => {
                write!(
                    f,
                    "cannot expand from hashpower {} to {} (maximum: {})",
                    current, requested, maximum
                )
            }
            CuckooError::TableFull => {
                write!(f, "table is full and cannot be expanded")
            }
        }
    }
}

impl Error for CuckooError {}

/// Result type alias for cuckoo hash map operations
pub type Result<T> = std::result::Result<T, CuckooError>;
