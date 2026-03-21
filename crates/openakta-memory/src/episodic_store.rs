//! Episodic Memory Store
//!
//! This module implements episodic memory storage for OPENAKTA agents:
//! - **SQLite** for chronological time-series logging
//! - **Time-bound retrieval** (query by time ranges)
//! - **Trajectory extraction** for consolidation
//!
//! # Example
//!
//! ```rust,no_run
//! use openakta_memory::{EpisodicStore, EpisodicMemory, MemoryType};
//! use chrono::Utc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create episodic store
//!     let store = EpisodicStore::new_in_memory().await?;
//!
//!     // Log episodic memory
//!     let memory = EpisodicMemory {
//!         id: uuid::Uuid::new_v4().to_string(),
//!         session_id: "session-001".to_string(),
//!         turn_number: 1,
//!         content: "User asked about authentication".to_string(),
//!         memory_type: MemoryType::ConversationTurn.to_string().to_string(),
//!         success: None,
//!         created_at: Utc::now(),
//!     };
//!
//!     store.log(memory).await?;
//!
//!     // Retrieve trajectory
//!     let trajectory = store.retrieve_trajectory("session-001").await?;
//!
//!     Ok(())
//! }
//! ```

use crate::lifecycle::MemoryTrait;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use uuid::Uuid;

/// Episodic memory errors
#[derive(Error, Debug)]
pub enum EpisodicError {
    /// SQLite database error
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// Memory not found
    #[error("memory not found: {0}")]
    NotFound(String),

    /// Invalid memory type
    #[error("invalid memory type: {0}")]
    InvalidMemoryType(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for episodic memory operations
pub type Result<T> = std::result::Result<T, EpisodicError>;

/// Memory type for episodic logging
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// Conversation turn (thought, reasoning)
    ConversationTurn,
    /// Terminal input (user command)
    TerminalInput,
    /// Terminal output (command output)
    TerminalOutput,
    /// Tool execution (action + result)
    ToolExecution,
    /// Success state (task completed)
    SuccessState,
    /// Failure state (task failed)
    FailureState,
}

impl MemoryType {
    /// Convert to string for database storage
    pub fn to_string(&self) -> &'static str {
        match self {
            MemoryType::ConversationTurn => "conversation_turn",
            MemoryType::TerminalInput => "terminal_input",
            MemoryType::TerminalOutput => "terminal_output",
            MemoryType::ToolExecution => "tool_execution",
            MemoryType::SuccessState => "success_state",
            MemoryType::FailureState => "failure_state",
        }
    }
}

impl FromStr for MemoryType {
    type Err = EpisodicError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "conversation_turn" => Ok(MemoryType::ConversationTurn),
            "terminal_input" => Ok(MemoryType::TerminalInput),
            "terminal_output" => Ok(MemoryType::TerminalOutput),
            "tool_execution" => Ok(MemoryType::ToolExecution),
            "success_state" => Ok(MemoryType::SuccessState),
            "failure_state" => Ok(MemoryType::FailureState),
            _ => Err(EpisodicError::InvalidMemoryType(s.to_string())),
        }
    }
}

/// Episodic memory entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicMemory {
    /// Unique identifier
    pub id: String,

    /// Session identifier (groups related memories)
    pub session_id: String,

    /// Turn number within session (for ordering)
    pub turn_number: i32,

    /// Memory content (text)
    pub content: String,

    /// Type of memory
    pub memory_type: String,

    /// Success flag (for tool execution, success/failure states)
    pub success: Option<bool>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl EpisodicMemory {
    /// Create new episodic memory
    pub fn new(
        session_id: &str,
        turn_number: i32,
        content: &str,
        memory_type: MemoryType,
        success: Option<bool>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            turn_number,
            content: content.to_string(),
            memory_type: memory_type.to_string().to_string(),
            success,
            created_at: Utc::now(),
        }
    }

    /// Get memory type as enum
    pub fn get_memory_type(&self) -> Result<MemoryType> {
        self.memory_type.parse()
    }

    /// Create conversation turn memory
    pub fn conversation_turn(session_id: &str, turn_number: i32, content: &str) -> Self {
        Self::new(
            session_id,
            turn_number,
            content,
            MemoryType::ConversationTurn,
            None,
        )
    }

    /// Create terminal output memory
    pub fn terminal_output(session_id: &str, turn_number: i32, output: &str) -> Self {
        Self::new(
            session_id,
            turn_number,
            output,
            MemoryType::TerminalOutput,
            None,
        )
    }

    /// Create tool execution memory
    pub fn tool_execution(
        session_id: &str,
        turn_number: i32,
        action: &str,
        output: &str,
        success: bool,
    ) -> Self {
        Self::new(
            session_id,
            turn_number,
            &format!("Action: {}\nOutput: {}", action, output),
            MemoryType::ToolExecution,
            Some(success),
        )
    }

    /// Create success state memory
    pub fn success_state(session_id: &str, turn_number: i32, message: &str) -> Self {
        Self::new(
            session_id,
            turn_number,
            message,
            MemoryType::SuccessState,
            Some(true),
        )
    }

    /// Create failure state memory
    pub fn failure_state(session_id: &str, turn_number: i32, error: &str) -> Self {
        Self::new(
            session_id,
            turn_number,
            error,
            MemoryType::FailureState,
            Some(false),
        )
    }
}

impl MemoryTrait for EpisodicMemory {
    fn id(&self) -> &str {
        &self.id
    }

    fn created_at(&self) -> u64 {
        self.created_at.timestamp().max(0) as u64
    }

    fn updated_at(&self) -> u64 {
        self.created_at()
    }

    fn retrieval_count(&self) -> u32 {
        0
    }

    fn importance(&self) -> f32 {
        match self.success {
            Some(true) => 0.9,
            Some(false) => 0.4,
            None => 0.6,
        }
    }
}

/// Configuration for episodic store
#[derive(Debug, Clone)]
pub struct EpisodicStoreConfig {
    /// Database path (use ":memory:" for in-memory)
    pub database_path: String,
}

impl Default for EpisodicStoreConfig {
    fn default() -> Self {
        Self {
            database_path: ":memory:".to_string(),
        }
    }
}

impl EpisodicStoreConfig {
    /// Create config for persistent storage
    pub fn persistent(path: &str) -> Self {
        Self {
            database_path: path.to_string(),
        }
    }

    /// Create config for in-memory storage (testing)
    pub fn in_memory() -> Self {
        Self {
            database_path: ":memory:".to_string(),
        }
    }
}

/// Internal connection wrapper for thread-safe access
struct DbConnection {
    conn: Connection,
}

/// Episodic memory store (SQLite-based)
pub struct EpisodicStore {
    db: Arc<RwLock<DbConnection>>,
    config: EpisodicStoreConfig,
}

impl EpisodicStore {
    /// Create new episodic store with config
    pub async fn new(config: EpisodicStoreConfig) -> Result<Self> {
        let conn = if config.database_path == ":memory:" {
            Connection::open_in_memory()?
        } else {
            // Ensure directory exists
            if let Some(parent) = Path::new(&config.database_path).parent() {
                std::fs::create_dir_all(parent)?;
            }
            Connection::open(&config.database_path)?
        };

        #[allow(clippy::arc_with_non_send_sync)]
        let store = Self {
            db: Arc::new(RwLock::new(DbConnection { conn })),
            config,
        };

        // Run migrations
        store.run_migrations()?;

        Ok(store)
    }

    /// Create new episodic store with default config (in-memory)
    pub async fn new_in_memory() -> Result<Self> {
        Self::new(EpisodicStoreConfig::in_memory()).await
    }

    /// Create new episodic store with persistent storage
    pub async fn new_persistent(path: &str) -> Result<Self> {
        Self::new(EpisodicStoreConfig::persistent(path)).await
    }

    /// Run database migrations
    fn run_migrations(&self) -> Result<()> {
        let db = self.db.read().unwrap();

        // Episodic memories table
        db.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS episodic_memories (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                turn_number INTEGER NOT NULL,
                content TEXT NOT NULL,
                memory_type TEXT NOT NULL,
                success BOOLEAN,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
            [],
        )?;

        // Index for time-range queries
        db.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_episodic_session_time ON episodic_memories(session_id, created_at)",
            [],
        )?;

        // Index for trajectory extraction
        db.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_episodic_session_turn ON episodic_memories(session_id, turn_number)",
            [],
        )?;

        Ok(())
    }

    /// Log episodic memory
    pub async fn log(&self, memory: EpisodicMemory) -> Result<()> {
        let db = self.db.write().unwrap();

        db.conn.execute(
            r#"
            INSERT INTO episodic_memories 
            (id, session_id, turn_number, content, memory_type, success, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                memory.id,
                memory.session_id,
                memory.turn_number,
                memory.content,
                memory.memory_type,
                memory.success,
                memory.created_at.to_rfc3339()
            ],
        )?;

        Ok(())
    }

    /// Retrieve memories by time range
    pub async fn retrieve_by_time_range(
        &self,
        session_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<EpisodicMemory>> {
        let db = self.db.read().unwrap();

        let mut stmt = db.conn.prepare(
            r#"
            SELECT id, session_id, turn_number, content, memory_type, success, created_at
            FROM episodic_memories 
            WHERE session_id = ? AND created_at BETWEEN ? AND ?
            ORDER BY turn_number ASC
            "#,
        )?;

        let memories = stmt
            .query_map(
                params![session_id, start.to_rfc3339(), end.to_rfc3339()],
                |row| {
                    Ok(EpisodicMemory {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        turn_number: row.get(2)?,
                        content: row.get(3)?,
                        memory_type: row.get(4)?,
                        success: row.get(5)?,
                        created_at: row
                            .get::<_, String>(6)?
                            .parse()
                            .unwrap_or_else(|_| Utc::now()),
                    })
                },
            )?
            .filter_map(|r| r.ok())
            .collect();

        Ok(memories)
    }

    /// Retrieve trajectory (all memories for a session, ordered by turn)
    pub async fn retrieve_trajectory(&self, session_id: &str) -> Result<Vec<EpisodicMemory>> {
        let db = self.db.read().unwrap();

        let mut stmt = db.conn.prepare(
            r#"
            SELECT id, session_id, turn_number, content, memory_type, success, created_at
            FROM episodic_memories 
            WHERE session_id = ?
            ORDER BY turn_number ASC
            "#,
        )?;

        let memories = stmt
            .query_map(params![session_id], |row| {
                Ok(EpisodicMemory {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    turn_number: row.get(2)?,
                    content: row.get(3)?,
                    memory_type: row.get(4)?,
                    success: row.get(5)?,
                    created_at: row
                        .get::<_, String>(6)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(memories)
    }

    /// Retrieve memories by type
    pub async fn retrieve_by_type(
        &self,
        session_id: &str,
        memory_type: MemoryType,
    ) -> Result<Vec<EpisodicMemory>> {
        let db = self.db.read().unwrap();

        let mut stmt = db.conn.prepare(
            r#"
            SELECT id, session_id, turn_number, content, memory_type, success, created_at
            FROM episodic_memories 
            WHERE session_id = ? AND memory_type = ?
            ORDER BY turn_number ASC
            "#,
        )?;

        let memories = stmt
            .query_map(params![session_id, memory_type.to_string()], |row| {
                Ok(EpisodicMemory {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    turn_number: row.get(2)?,
                    content: row.get(3)?,
                    memory_type: row.get(4)?,
                    success: row.get(5)?,
                    created_at: row
                        .get::<_, String>(6)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(memories)
    }

    /// Retrieve single memory by ID
    pub async fn retrieve_by_id(&self, id: &str) -> Result<EpisodicMemory> {
        let db = self.db.read().unwrap();

        let mut stmt = db.conn.prepare(
            "SELECT id, session_id, turn_number, content, memory_type, success, created_at FROM episodic_memories WHERE id = ?",
        )?;

        let memory = stmt
            .query_row(params![id], |row| {
                Ok(EpisodicMemory {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    turn_number: row.get(2)?,
                    content: row.get(3)?,
                    memory_type: row.get(4)?,
                    success: row.get(5)?,
                    created_at: row
                        .get::<_, String>(6)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .optional()?;

        memory.ok_or_else(|| EpisodicError::NotFound(id.to_string()))
    }

    /// Retrieve all episodic memories.
    pub async fn list_all(&self) -> Result<Vec<EpisodicMemory>> {
        let db = self.db.read().unwrap();
        let mut stmt = db.conn.prepare(
            "SELECT id, session_id, turn_number, content, memory_type, success, created_at FROM episodic_memories ORDER BY created_at ASC",
        )?;
        let memories = stmt
            .query_map([], |row| {
                Ok(EpisodicMemory {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    turn_number: row.get(2)?,
                    content: row.get(3)?,
                    memory_type: row.get(4)?,
                    success: row.get(5)?,
                    created_at: row
                        .get::<_, String>(6)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .filter_map(|row| row.ok())
            .collect();
        Ok(memories)
    }

    /// List distinct session ids in chronological order.
    pub async fn list_session_ids(&self) -> Result<Vec<String>> {
        let db = self.db.read().unwrap();
        let mut stmt = db.conn.prepare(
            r#"
            SELECT session_id
            FROM episodic_memories
            GROUP BY session_id
            ORDER BY MIN(created_at) ASC
            "#,
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut session_ids = Vec::new();
        for row in rows {
            session_ids.push(row?);
        }
        Ok(session_ids)
    }

    /// Delete an episodic memory by ID.
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let db = self.db.write().unwrap();
        let affected = db
            .conn
            .execute("DELETE FROM episodic_memories WHERE id = ?", params![id])?;
        Ok(affected > 0)
    }

    /// Log agent action (convenience method)
    pub async fn log_action(
        &self,
        session_id: &str,
        turn_number: i32,
        action: &str,
        output: &str,
        success: bool,
    ) -> Result<()> {
        let memory =
            EpisodicMemory::tool_execution(session_id, turn_number, action, output, success);
        self.log(memory).await
    }

    /// Log conversation turn (convenience method)
    pub async fn log_conversation(
        &self,
        session_id: &str,
        turn_number: i32,
        content: &str,
    ) -> Result<()> {
        let memory = EpisodicMemory::conversation_turn(session_id, turn_number, content);
        self.log(memory).await
    }

    /// Log success state (convenience method)
    pub async fn log_success(
        &self,
        session_id: &str,
        turn_number: i32,
        message: &str,
    ) -> Result<()> {
        let memory = EpisodicMemory::success_state(session_id, turn_number, message);
        self.log(memory).await
    }

    /// Log failure state (convenience method)
    pub async fn log_failure(&self, session_id: &str, turn_number: i32, error: &str) -> Result<()> {
        let memory = EpisodicMemory::failure_state(session_id, turn_number, error);
        self.log(memory).await
    }

    /// Get session statistics
    pub async fn get_session_stats(&self, session_id: &str) -> Result<SessionStats> {
        let db = self.db.read().unwrap();

        let total: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM episodic_memories WHERE session_id = ?",
            params![session_id],
            |r: &rusqlite::Row| r.get(0),
        )?;

        let successful: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM episodic_memories WHERE session_id = ? AND success = 1",
            params![session_id],
            |r: &rusqlite::Row| r.get(0),
        )?;

        let failed: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM episodic_memories WHERE session_id = ? AND success = 0",
            params![session_id],
            |r: &rusqlite::Row| r.get(0),
        )?;

        let first_turn: Option<i32> = db
            .conn
            .query_row(
                "SELECT MIN(turn_number) FROM episodic_memories WHERE session_id = ?",
                params![session_id],
                |r: &rusqlite::Row| r.get(0),
            )
            .optional()?;

        let last_turn: Option<i32> = db
            .conn
            .query_row(
                "SELECT MAX(turn_number) FROM episodic_memories WHERE session_id = ?",
                params![session_id],
                |r: &rusqlite::Row| r.get(0),
            )
            .optional()?;

        Ok(SessionStats {
            total_memories: total as usize,
            successful_actions: successful as usize,
            failed_actions: failed as usize,
            first_turn: first_turn.unwrap_or(0),
            last_turn: last_turn.unwrap_or(0),
        })
    }

    /// Delete session (for cleanup)
    pub async fn delete_session(&self, session_id: &str) -> Result<usize> {
        let db = self.db.write().unwrap();

        let rows = db.conn.execute(
            "DELETE FROM episodic_memories WHERE session_id = ?",
            params![session_id],
        )?;

        Ok(rows)
    }

    /// Get config
    pub fn config(&self) -> &EpisodicStoreConfig {
        &self.config
    }
}

/// Session statistics
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub total_memories: usize,
    pub successful_actions: usize,
    pub failed_actions: usize,
    pub first_turn: i32,
    pub last_turn: i32,
}

impl SessionStats {
    /// Get success rate
    pub fn success_rate(&self) -> f32 {
        let total = self.successful_actions + self.failed_actions;
        if total == 0 {
            return 1.0;
        }
        self.successful_actions as f32 / total as f32
    }

    /// Get total turns
    pub fn total_turns(&self) -> i32 {
        self.last_turn - self.first_turn + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;

    #[tokio::test]
    async fn test_episodic_memory_log() {
        let store = EpisodicStore::new_in_memory().await.unwrap();

        let memory = EpisodicMemory::new(
            "session-001",
            1,
            "Test content",
            MemoryType::ConversationTurn,
            None,
        );

        store.log(memory).await.unwrap();

        // Verify by retrieving
        let trajectory = store.retrieve_trajectory("session-001").await.unwrap();
        assert_eq!(trajectory.len(), 1);
        assert_eq!(trajectory[0].content, "Test content");
    }

    #[tokio::test]
    async fn test_retrieve_by_time_range() {
        let store = EpisodicStore::new_in_memory().await.unwrap();

        let now = Utc::now();

        // Log memories at different times
        let mut memory1 = EpisodicMemory::conversation_turn("session-001", 1, "First");
        memory1.created_at = now;
        store.log(memory1).await.unwrap();

        let mut memory2 = EpisodicMemory::conversation_turn("session-001", 2, "Second");
        memory2.created_at = now + ChronoDuration::seconds(10);
        store.log(memory2).await.unwrap();

        let mut memory3 = EpisodicMemory::conversation_turn("session-001", 3, "Third");
        memory3.created_at = now + ChronoDuration::seconds(20);
        store.log(memory3).await.unwrap();

        // Retrieve by time range (should get first two)
        let start = now;
        let end = now + ChronoDuration::seconds(15);
        let memories = store
            .retrieve_by_time_range("session-001", start, end)
            .await
            .unwrap();

        assert_eq!(memories.len(), 2);
        assert_eq!(memories[0].content, "First");
        assert_eq!(memories[1].content, "Second");
    }

    #[tokio::test]
    async fn test_retrieve_trajectory() {
        let store = EpisodicStore::new_in_memory().await.unwrap();

        // Log multiple turns
        store
            .log_conversation("session-001", 1, "Turn 1")
            .await
            .unwrap();
        store
            .log_conversation("session-001", 2, "Turn 2")
            .await
            .unwrap();
        store
            .log_conversation("session-001", 3, "Turn 3")
            .await
            .unwrap();

        // Retrieve trajectory
        let trajectory = store.retrieve_trajectory("session-001").await.unwrap();

        assert_eq!(trajectory.len(), 3);
        assert_eq!(trajectory[0].turn_number, 1);
        assert_eq!(trajectory[1].turn_number, 2);
        assert_eq!(trajectory[2].turn_number, 3);
    }

    #[tokio::test]
    async fn test_log_action() {
        let store = EpisodicStore::new_in_memory().await.unwrap();

        store
            .log_action("session-001", 1, "run_command", "Output here", true)
            .await
            .unwrap();

        let trajectory = store.retrieve_trajectory("session-001").await.unwrap();
        assert_eq!(trajectory.len(), 1);
        assert!(trajectory[0].content.contains("Action:"));
        assert!(trajectory[0].content.contains("Output:"));
        assert_eq!(trajectory[0].success, Some(true));
    }

    #[tokio::test]
    async fn test_session_isolation() {
        let store = EpisodicStore::new_in_memory().await.unwrap();

        // Log to different sessions
        store
            .log_conversation("session-A", 1, "Session A")
            .await
            .unwrap();
        store
            .log_conversation("session-B", 1, "Session B")
            .await
            .unwrap();
        store
            .log_conversation("session-A", 2, "Session A again")
            .await
            .unwrap();

        // Retrieve each session
        let session_a = store.retrieve_trajectory("session-A").await.unwrap();
        let session_b = store.retrieve_trajectory("session-B").await.unwrap();

        assert_eq!(session_a.len(), 2);
        assert_eq!(session_b.len(), 1);
        assert!(session_a.iter().all(|m| m.session_id == "session-A"));
        assert!(session_b.iter().all(|m| m.session_id == "session-B"));
    }

    #[tokio::test]
    async fn test_turn_ordering() {
        let store = EpisodicStore::new_in_memory().await.unwrap();

        // Log out of order
        store
            .log_conversation("session-001", 5, "Turn 5")
            .await
            .unwrap();
        store
            .log_conversation("session-001", 2, "Turn 2")
            .await
            .unwrap();
        store
            .log_conversation("session-001", 8, "Turn 8")
            .await
            .unwrap();
        store
            .log_conversation("session-001", 1, "Turn 1")
            .await
            .unwrap();

        // Should be retrieved in order
        let trajectory = store.retrieve_trajectory("session-001").await.unwrap();

        assert_eq!(trajectory.len(), 4);
        assert_eq!(trajectory[0].turn_number, 1);
        assert_eq!(trajectory[1].turn_number, 2);
        assert_eq!(trajectory[2].turn_number, 5);
        assert_eq!(trajectory[3].turn_number, 8);
    }

    #[tokio::test]
    async fn test_memory_type_filtering() {
        let store = EpisodicStore::new_in_memory().await.unwrap();

        // Log different types
        store
            .log_conversation("session-001", 1, "Conversation")
            .await
            .unwrap();
        store
            .log_action("session-001", 2, "cmd", "output", true)
            .await
            .unwrap();
        store
            .log_success("session-001", 3, "Success!")
            .await
            .unwrap();
        store.log_failure("session-001", 4, "Error!").await.unwrap();

        // Filter by type
        let conversations = store
            .retrieve_by_type("session-001", MemoryType::ConversationTurn)
            .await
            .unwrap();
        let tool_executions = store
            .retrieve_by_type("session-001", MemoryType::ToolExecution)
            .await
            .unwrap();
        let successes = store
            .retrieve_by_type("session-001", MemoryType::SuccessState)
            .await
            .unwrap();
        let failures = store
            .retrieve_by_type("session-001", MemoryType::FailureState)
            .await
            .unwrap();

        assert_eq!(conversations.len(), 1);
        assert_eq!(tool_executions.len(), 1);
        assert_eq!(successes.len(), 1);
        assert_eq!(failures.len(), 1);
    }

    #[tokio::test]
    async fn test_session_stats() {
        let store = EpisodicStore::new_in_memory().await.unwrap();

        // Log various memories
        store
            .log_conversation("session-001", 1, "Start")
            .await
            .unwrap();
        store
            .log_action("session-001", 2, "cmd1", "out1", true)
            .await
            .unwrap();
        store
            .log_action("session-001", 3, "cmd2", "out2", false)
            .await
            .unwrap();
        store
            .log_action("session-001", 4, "cmd3", "out3", true)
            .await
            .unwrap();
        store.log_success("session-001", 5, "Done").await.unwrap();

        let stats = store.get_session_stats("session-001").await.unwrap();

        assert_eq!(stats.total_memories, 5);
        assert_eq!(stats.successful_actions, 3); // cmd1, cmd3, and success state
        assert_eq!(stats.failed_actions, 1); // cmd2
        assert_eq!(stats.first_turn, 1);
        assert_eq!(stats.last_turn, 5);
        assert!((stats.success_rate() - 0.75).abs() < 0.01); // 3/4 = 0.75
    }

    #[tokio::test]
    async fn test_retrieve_by_id() {
        let store = EpisodicStore::new_in_memory().await.unwrap();

        let memory = EpisodicMemory::conversation_turn("session-001", 1, "Test");
        let memory_id = memory.id.clone();
        store.log(memory).await.unwrap();

        let retrieved = store.retrieve_by_id(&memory_id).await.unwrap();
        assert_eq!(retrieved.id, memory_id);
        assert_eq!(retrieved.content, "Test");
    }

    #[tokio::test]
    async fn test_retrieve_not_found() {
        let store = EpisodicStore::new_in_memory().await.unwrap();

        let result = store.retrieve_by_id("non-existent").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), EpisodicError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_delete_session() {
        let store = EpisodicStore::new_in_memory().await.unwrap();

        // Log multiple memories
        store
            .log_conversation("session-001", 1, "Test 1")
            .await
            .unwrap();
        store
            .log_conversation("session-001", 2, "Test 2")
            .await
            .unwrap();
        store
            .log_conversation("session-002", 1, "Other session")
            .await
            .unwrap();

        // Delete session-001
        let deleted = store.delete_session("session-001").await.unwrap();
        assert_eq!(deleted, 2);

        // Verify deletion
        let session_001 = store.retrieve_trajectory("session-001").await.unwrap();
        let session_002 = store.retrieve_trajectory("session-002").await.unwrap();

        assert_eq!(session_001.len(), 0);
        assert_eq!(session_002.len(), 1);
    }

    #[tokio::test]
    async fn test_memory_type_conversion() {
        // Test to_string
        assert_eq!(
            MemoryType::ConversationTurn.to_string(),
            "conversation_turn"
        );
        assert_eq!(MemoryType::TerminalOutput.to_string(), "terminal_output");
        assert_eq!(MemoryType::ToolExecution.to_string(), "tool_execution");
        assert_eq!(MemoryType::SuccessState.to_string(), "success_state");
        assert_eq!(MemoryType::FailureState.to_string(), "failure_state");

        assert_eq!(
            "conversation_turn".parse::<MemoryType>().unwrap(),
            MemoryType::ConversationTurn
        );
        assert_eq!(
            "TERMINAL_OUTPUT".parse::<MemoryType>().unwrap(),
            MemoryType::TerminalOutput
        );
        assert!("invalid_type".parse::<MemoryType>().is_err());
    }

    #[tokio::test]
    async fn test_convenience_methods() {
        let store = EpisodicStore::new_in_memory().await.unwrap();

        // Test all convenience methods
        store
            .log_conversation("session-001", 1, "Thought process")
            .await
            .unwrap();

        store
            .log_action("session-001", 2, "read_file", "content", true)
            .await
            .unwrap();

        store
            .log_success("session-001", 3, "Task completed successfully")
            .await
            .unwrap();

        store
            .log_failure("session-001", 4, "Task failed: timeout")
            .await
            .unwrap();

        let trajectory = store.retrieve_trajectory("session-001").await.unwrap();
        assert_eq!(trajectory.len(), 4);

        let types: Vec<_> = trajectory
            .iter()
            .map(|m| m.get_memory_type().unwrap())
            .collect();

        assert_eq!(types[0], MemoryType::ConversationTurn);
        assert_eq!(types[1], MemoryType::ToolExecution);
        assert_eq!(types[2], MemoryType::SuccessState);
        assert_eq!(types[3], MemoryType::FailureState);
    }

    #[tokio::test]
    async fn test_persistent_storage() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("episodic.db");

        // Create store with persistent storage
        {
            let store = EpisodicStore::new_persistent(db_path.to_str().unwrap())
                .await
                .unwrap();

            store
                .log_conversation("session-001", 1, "Persistent test")
                .await
                .unwrap();
        }

        // Reopen and verify
        {
            let store = EpisodicStore::new_persistent(db_path.to_str().unwrap())
                .await
                .unwrap();

            let trajectory = store.retrieve_trajectory("session-001").await.unwrap();
            assert_eq!(trajectory.len(), 1);
            assert_eq!(trajectory[0].content, "Persistent test");
        }
    }

    #[tokio::test]
    async fn test_session_stats_success_rate() {
        let store = EpisodicStore::new_in_memory().await.unwrap();

        // All successes
        store
            .log_action("session-all-success", 1, "cmd", "out", true)
            .await
            .unwrap();
        store
            .log_action("session-all-success", 2, "cmd", "out", true)
            .await
            .unwrap();

        let stats = store
            .get_session_stats("session-all-success")
            .await
            .unwrap();
        assert!((stats.success_rate() - 1.0).abs() < 0.01);

        // All failures
        store
            .log_action("session-all-fail", 1, "cmd", "out", false)
            .await
            .unwrap();
        store
            .log_action("session-all-fail", 2, "cmd", "out", false)
            .await
            .unwrap();

        let stats = store.get_session_stats("session-all-fail").await.unwrap();
        assert!((stats.success_rate() - 0.0).abs() < 0.01);

        // No actions
        store
            .log_conversation("session-no-actions", 1, "test")
            .await
            .unwrap();

        let stats = store.get_session_stats("session-no-actions").await.unwrap();
        assert!((stats.success_rate() - 1.0).abs() < 0.01); // Default to 1.0
    }
}
