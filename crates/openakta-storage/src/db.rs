//! Database connection and configuration

use rusqlite::Connection;
use tracing::{debug, info};

use crate::Result;

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
        info!("Running database migrations");

        conn.execute_batch(include_str!("../migrations/0001_init.sql"))?;
        conn.execute_batch(include_str!("../migrations/0002_memory_domains.sql"))?;
        conn.execute_batch(include_str!("../migrations/0003_runtime_seeds.sql"))?;

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
}
