//! Database connection and configuration

use rusqlite::Connection;
use std::path::Path;
use tracing::{debug, info};

use crate::{Result, StorageError};

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Path to the SQLite database file
    pub path: String,
    /// Enable WAL mode for better concurrency
    pub wal_mode: bool,
    /// Busy timeout in milliseconds
    pub busy_timeout_ms: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "axora.db".to_string(),
            wal_mode: true,
            busy_timeout_ms: 5000,
        }
    }
}

/// Database connection manager
pub struct Database {
    config: DatabaseConfig,
}

impl Database {
    /// Create a new database instance
    pub fn new(config: DatabaseConfig) -> Self {
        Self { config }
    }

    /// Initialize the database with migrations
    pub fn init(&self) -> Result<Connection> {
        let conn = self.connect()?;
        info!("Database initialized at: {}", self.config.path);
        Ok(conn)
    }

    /// Connect to the database
    pub fn connect(&self) -> Result<Connection> {
        let conn = Connection::open(&self.config.path)?;

        if self.config.wal_mode {
            conn.execute_batch("PRAGMA journal_mode = WAL;")?;
            debug!("WAL mode enabled");
        }

        conn.busy_timeout(std::time::Duration::from_millis(
            self.config.busy_timeout_ms,
        ))?;

        Ok(conn)
    }

    /// Run embedded migrations
    pub fn migrate(&self, conn: &mut Connection) -> Result<()> {
        // Migrations will be embedded here
        info!("Running database migrations");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_config_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.path, "axora.db");
        assert!(config.wal_mode);
        assert_eq!(config.busy_timeout_ms, 5000);
    }
}
