# Agent C — Sprint 27: Episodic Memory Store

**Phase:** 2  
**Sprint:** 27 (Memory Architecture)  
**File:** `crates/axora-memory/src/episodic_store.rs`  
**Priority:** HIGH (chronological experience logging)  
**Estimated Tokens:** ~100K output  

---

## 🎯 Task

Implement **Episodic Memory Store** using SQLite for chronological experience logging.

### Context

R-14 research provides CRITICAL implementation details:
- **Episodic Memory** — Conversations, terminal I/O, success/failure states
- **SQLite Storage** — Time-series chronological logging
- **Time-Bound Retrieval** — Query by time ranges, trajectory extraction
- **Integration:** All agent actions logged chronologically

**Your job:** Implement episodic memory store (chronological experience ledger).

---

## 📋 Deliverables

### 1. Create episodic_store.rs

**File:** `crates/axora-memory/src/episodic_store.rs`

**Core Structure:**
```rust
//! Episodic Memory Store
//!
//! This module implements episodic memory storage:
//! - SQLite for chronological time-series logging
//! - Time-bound retrieval (query by time ranges)
//! - Trajectory extraction for consolidation

use sqlx::{SqlitePool, FromRow};
use chrono::{DateTime, Utc};

/// Episodic memory entity
#[derive(Debug, Clone, FromRow)]
pub struct EpisodicMemory {
    pub id: String,
    pub session_id: String,
    pub turn_number: i32,
    pub content: String,
    pub memory_type: MemoryType,
    pub success: Option<bool>,
    pub created_at: DateTime<Utc>,
}

/// Memory type
#[derive(Debug, Clone)]
pub enum MemoryType {
    ConversationTurn,
    TerminalInput,
    TerminalOutput,
    ToolExecution,
    SuccessState,
    FailureState,
}

/// Episodic memory store
pub struct EpisodicStore {
    db: SqlitePool,
}

impl EpisodicStore {
    /// Create new episodic store
    pub async fn new(db: SqlitePool) -> Result<Self> {
        // Run migrations
        sqlx::migrate!("./migrations/episodic").run(&db).await?;
        
        Ok(Self { db })
    }
    
    /// Log episodic memory
    pub async fn log(&self, memory: EpisodicMemory) -> Result<()> {
        sqlx::query(
            "INSERT INTO episodic_memories 
             (id, session_id, turn_number, content, memory_type, success, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&memory.id)
        .bind(&memory.session_id)
        .bind(&memory.turn_number)
        .bind(&memory.content)
        .bind(&memory.memory_type.to_string())
        .bind(&memory.success)
        .bind(&memory.created_at)
        .execute(&self.db)
        .await?;
        
        Ok(())
    }
    
    /// Retrieve memories by time range
    pub async fn retrieve_by_time_range(
        &self,
        session_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<EpisodicMemory>> {
        let memories = sqlx::query_as::<_, EpisodicMemory>(
            "SELECT * FROM episodic_memories 
             WHERE session_id = ? AND created_at BETWEEN ? AND ?
             ORDER BY turn_number ASC"
        )
        .bind(session_id)
        .bind(&start)
        .bind(&end)
        .fetch_all(&self.db)
        .await?;
        
        Ok(memories)
    }
    
    /// Retrieve trajectory (for consolidation)
    pub async fn retrieve_trajectory(
        &self,
        session_id: &str,
    ) -> Result<Vec<EpisodicMemory>> {
        let memories = sqlx::query_as::<_, EpisodicMemory>(
            "SELECT * FROM episodic_memories 
             WHERE session_id = ?
             ORDER BY turn_number ASC"
        )
        .bind(session_id)
        .fetch_all(&self.db)
        .await?;
        
        Ok(memories)
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
        let memory = EpisodicMemory {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            turn_number,
            content: format!("Action: {}\nOutput: {}", action, output),
            memory_type: MemoryType::ToolExecution,
            success: Some(success),
            created_at: Utc::now(),
        };
        
        self.log(memory).await
    }
}
```

---

### 2. Create Database Migration

**File:** `crates/axora-memory/migrations/episodic/0001_episodic_memories.sql`

```sql
-- Episodic memories table
CREATE TABLE IF NOT EXISTS episodic_memories (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    turn_number INTEGER NOT NULL,
    content TEXT NOT NULL,
    memory_type TEXT NOT NULL,
    success BOOLEAN,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for time-range queries
CREATE INDEX IF NOT EXISTS idx_episodic_session_time 
ON episodic_memories(session_id, created_at);

-- Index for trajectory extraction
CREATE INDEX IF NOT EXISTS idx_episodic_session_turn 
ON episodic_memories(session_id, turn_number);
```

---

### 3. Integrate with ReAct Loops

**File:** `crates/axora-agents/src/react.rs` (UPDATE)

```rust
// Add to existing DualThreadReactAgent
impl DualThreadReactAgent {
    /// Execute ReAct loop with episodic logging
    pub async fn execute_with_logging(
        &mut self,
        task: &Task,
        episodic_store: &EpisodicStore,
    ) -> Result<TaskResult> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let mut turn_number = 0;
        
        loop {
            // Generate thought + action
            let (thought, action) = self.llm_plan(&task).await?;
            
            // Log thought (conversation turn)
            episodic_store.log(EpisodicMemory {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: session_id.clone(),
                turn_number,
                content: thought,
                memory_type: MemoryType::ConversationTurn,
                success: None,
                created_at: Utc::now(),
            }).await?;
            
            // Execute action
            let observation = self.tools.execute(&action).await?;
            
            // Log action output (terminal output)
            episodic_store.log(EpisodicMemory {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: session_id.clone(),
                turn_number,
                content: observation.result.clone(),
                memory_type: MemoryType::TerminalOutput,
                success: Some(observation.success),
                created_at: Utc::now(),
            }).await?;
            
            turn_number += 1;
            
            // Check if task complete
            if self.is_complete(&observation)? {
                return Ok(TaskResult {
                    success: true,
                    output: observation.result,
                });
            }
            
            // Check max cycles
            if turn_number > 12 {
                return Err(Error::MaxCyclesExceeded);
            }
        }
    }
}
```

---

## 📁 File Boundaries

**Create:**
- `crates/axora-memory/src/episodic_store.rs` (NEW)
- `crates/axora-memory/migrations/episodic/0001_episodic_memories.sql` (NEW)

**Update:**
- `crates/axora-agents/src/react.rs` (integrate episodic logging)

**DO NOT Edit:**
- `crates/axora-cache/` (Agent B's domain)
- `crates/axora-indexing/` (Agent B's domain)
- `crates/axora-docs/` (Agent A's other work)

---

## 🧪 Tests Required

```rust
#[test]
fn test_episodic_memory_log() { }

#[test]
fn test_retrieve_by_time_range() { }

#[test]
fn test_retrieve_trajectory() { }

#[test]
fn test_log_action() { }

#[test]
fn test_session_isolation() { }

#[test]
fn test_turn_ordering() { }

#[test]
fn test_memory_type_filtering() { }

#[test]
fn test_react_loop_integration() { }
```

---

## ✅ Success Criteria

- [ ] `episodic_store.rs` created (SQLite time-series)
- [ ] Database migration created
- [ ] Log episodic memories works
- [ ] Time-range retrieval works
- [ ] Trajectory extraction works (for consolidation)
- [ ] ReAct loop integration works
- [ ] 8+ tests passing
- [ ] Performance: <50ms for logging, <100ms for retrieval

---

## 🔗 References

- [`PHASE-2-INTEGRATION-MEMORY-ARCHITECTURE.md`](../shared/PHASE-2-INTEGRATION-MEMORY-ARCHITECTURE.md) — Memory architecture
- Research document — R-14 Episodic Memory spec

---

**⚠️ DEPENDENCY:** This sprint requires **A-26 (Semantic Memory)** to be complete first.

**Start AFTER Agent A completes Sprint 26.**

**Priority: HIGH — chronological experience logging for consolidation.**

**Dependencies:**
- A-26 (Semantic Memory Store) — must complete first

**Blocks:**
- C-29 (Consolidation Pipeline — needs trajectory extraction)
