//! OPENAKTA Storage Layer
//!
//! Provides persistent storage using SQLite for agents, tasks, and messages.
//! Uses migrations for schema management and provides a clean API for
//! data access.

pub mod db;
pub mod store;

pub use db::{Database, DatabaseConfig};
pub use store::{AgentStore, MessageStore, TaskStore};

use thiserror::Error;

/// Storage-related errors
#[derive(Error, Debug)]
pub enum StorageError {
    /// Database error with path context
    #[error("database error at {path}: {source}")]
    Database {
        /// Path to the database file
        path: String,
        /// Underlying rusqlite error
        #[source]
        source: rusqlite::Error,
    },

    /// Migration error
    #[error("migration error: {0}")]
    Migration(String),

    /// Serialization error
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Not found
    #[error("not found: {0}")]
    NotFound(String),
}

impl From<rusqlite::Error> for StorageError {
    fn from(err: rusqlite::Error) -> Self {
        // For backward compatibility where path is not available
        StorageError::Database {
            path: "unknown".to_string(),
            source: err,
        }
    }
}

/// Result type for storage operations
pub type Result<T> = std::result::Result<T, StorageError>;
