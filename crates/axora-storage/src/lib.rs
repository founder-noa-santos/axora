//! AXORA Storage Layer
//!
//! Provides persistent storage using SQLite for agents, tasks, and messages.
//! Uses migrations for schema management and provides a clean API for
//! data access.

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod db;
pub mod store;

pub use db::{Database, DatabaseConfig};
pub use store::{AgentStore, TaskStore, MessageStore};

use thiserror::Error;

/// Storage-related errors
#[derive(Error, Debug)]
pub enum StorageError {
    /// Database error
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    
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

/// Result type for storage operations
pub type Result<T> = std::result::Result<T, StorageError>;
