//! Cache errors

use thiserror::Error;

/// Cache operation errors
#[derive(Error, Debug)]
pub enum CacheError {
    /// Cache miss
    #[error("cache miss")]
    Miss,

    /// Serialization error
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Database error
    #[error("database error: {0}")]
    Database(String),
}
