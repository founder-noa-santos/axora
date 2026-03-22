//! Database connection and configuration

use rusqlite::Connection;
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
            path: "openakta.db".to_string(),
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
        let mut conn = self.connect()?;
        self.migrate(&mut conn)?;
        info!("Database initialized at: {}", self.config.path);
        Ok(conn)
    }

    /// Connect to the database
    pub fn connect(&self) -> Result<Connection> {
        let conn = Connection::open(&self.config.path).map_err(|e| StorageError::Database {
            path: self.config.path.clone(),
            source: e,
        })?;

        if self.config.wal_mode {
            conn.execute_batch("PRAGMA journal_mode = WAL;").map_err(|e| {
                StorageError::Database {
                    path: self.config.path.clone(),
                    source: e,
                }
            })?;
            debug!("WAL mode enabled");
        }

        conn.busy_timeout(std::time::Duration::from_millis(
            self.config.busy_timeout_ms,
        ))
        .map_err(|e| StorageError::Database {
            path: self.config.path.clone(),
            source: e,
        })?;

        Ok(conn)
    }

    /// Run embedded migrations
    pub fn migrate(&self, conn: &mut Connection) -> Result<()> {
        info!("Running database migrations");

        conn.execute_batch(include_str!("../migrations/0001_init.sql"))
            .map_err(|e| StorageError::Database {
                path: self.config.path.clone(),
                source: e,
            })?;
        conn.execute_batch(include_str!("../migrations/0002_memory_domains.sql"))
            .map_err(|e| StorageError::Database {
                path: self.config.path.clone(),
                source: e,
            })?;
        conn.execute_batch(include_str!("../migrations/0003_runtime_seeds.sql"))
            .map_err(|e| StorageError::Database {
                path: self.config.path.clone(),
                source: e,
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_config_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.path, "openakta.db");
        assert!(config.wal_mode);
        assert_eq!(config.busy_timeout_ms, 5000);
    }

    #[test]
    fn test_migrations_create_memory_tables() {
        let db = Database::new(DatabaseConfig {
            path: ":memory:".to_string(),
            ..Default::default()
        });

        let conn = db.init().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='episodic_events'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_database_error_includes_path() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a temp file and write garbage to simulate corruption
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"this is not a valid sqlite database").unwrap();
        temp_file.flush().unwrap();

        let db = Database::new(DatabaseConfig {
            path: temp_file.path().to_string_lossy().to_string(),
            ..Default::default()
        });

        let result = db.connect();
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_str = err.to_string();

        // Verify the error includes the path
        assert!(
            err_str.contains(temp_file.path().to_string_lossy().as_ref()),
            "Error should contain database path, got: {}",
            err_str
        );

        // Verify it's a Database error variant
        assert!(err_str.contains("database error at"));
    }
}
