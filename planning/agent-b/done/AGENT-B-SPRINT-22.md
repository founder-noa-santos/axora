# Agent B — Sprint 22: Atomic Checkout Semantics

**Phase:** 2  
**Sprint:** 22 (Implementation)  
**File:** `crates/axora-indexing/src/task_queue.rs`  
**Priority:** HIGH (prevents duplicate execution)  
**Estimated Tokens:** ~100K output  

---

## 🎯 Task

Implement **Atomic Checkout Semantics** (from Paperclip pattern) for task queue management.

### Context

Competitive analysis provides CRITICAL implementation details:
- **Atomic Checkout** — Prevents race conditions (multiple agents same task)
- **Single-Asssignee Model** — One task, one agent
- **Paperclip Pattern** — Production-validated (PostgreSQL `FOR UPDATE SKIP LOCKED`)

**Your job:** Implement atomic task queue (prevents duplicate execution).

---

## 📋 Deliverables

### 1. Create task_queue.rs

**File:** `crates/axora-indexing/src/task_queue.rs`

**Core Structure:**
```rust
//! Atomic Checkout Semantics
//!
//! This module implements production-grade task queue management:
//! - Atomic checkout (prevents race conditions)
//! - Single-assignee model (one task, one agent)
//! - Paperclip pattern (PostgreSQL `FOR UPDATE SKIP LOCKED`)

use sqlx::{SqlitePool, Transaction, FromRow};
use chrono::{DateTime, Utc};

/// Task queue with atomic checkout
pub struct TaskQueue {
    db: SqlitePool,
}

/// Task entity
#[derive(Debug, Clone, FromRow)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub status: TaskStatus,
    pub assignee_id: Option<String>,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub checked_out_at: Option<DateTime<Utc>>,
}

/// Task status
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl TaskQueue {
    /// Create new task queue
    pub async fn new(db: SqlitePool) -> Self {
        Self { db }
    }
    
    /// Atomic checkout (prevents duplicate execution)
    pub async fn checkout_task(
        &self,
        agent_id: &str,
    ) -> Result<Option<Task>> {
        let mut tx = self.db.begin().await?;
        
        // SELECT FOR UPDATE SKIP LOCKED (atomic checkout)
        let task = sqlx::query_as::<_, Task>(
            "SELECT * FROM tasks 
             WHERE status = 'pending' 
             ORDER BY priority DESC, created_at ASC 
             LIMIT 1 
             FOR UPDATE SKIP LOCKED"
        )
        .fetch_optional(&mut tx)
        .await?;
        
        if let Some(mut task) = task {
            // Atomic update (single-assignee model)
            task.status = TaskStatus::InProgress;
            task.assignee_id = Some(agent_id.to_string());
            task.checked_out_at = Some(Utc::now());
            
            sqlx::query(
                "UPDATE tasks 
                 SET status = ?, assignee_id = ?, checked_out_at = ? 
                 WHERE id = ?"
            )
            .bind(&task.status.to_string())
            .bind(&task.assignee_id)
            .bind(&task.checked_out_at)
            .bind(&task.id)
            .execute(&mut tx)
            .await?;
            
            tx.commit().await?;
            Ok(Some(task))
        } else {
            Ok(None) // No pending tasks
        }
    }
    
    /// Complete task (release checkout)
    pub async fn complete_task(
        &self,
        task_id: &str,
        result: TaskResult,
    ) -> Result<()> {
        let status = if result.success {
            TaskStatus::Completed
        } else {
            TaskStatus::Failed
        };
        
        sqlx::query(
            "UPDATE tasks 
             SET status = ?, completed_at = ?, result = ? 
             WHERE id = ?"
        )
        .bind(&status.to_string())
        .bind(&Utc::now())
        .bind(&serde_json::to_string(&result)?)
        .bind(task_id)
        .execute(&self.db)
        .await?;
        
        Ok(())
    }
    
    /// Release task (timeout or error)
    pub async fn release_task(
        &self,
        task_id: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE tasks 
             SET status = 'pending', assignee_id = NULL, checked_out_at = NULL 
             WHERE id = ?"
        )
        .bind(task_id)
        .execute(&self.db)
        .await?;
        
        Ok(())
    }
}
```

---

### 2. Create Database Migration

**File:** `crates/axora-indexing/migrations/0002_task_queue.sql`

```sql
-- Task queue table
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    description TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    assignee_id TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    checked_out_at TEXT,
    completed_at TEXT,
    result TEXT
);

-- Index for efficient checkout queries
CREATE INDEX IF NOT EXISTS idx_tasks_status_priority 
ON tasks(status, priority DESC, created_at ASC);

-- Index for assignee lookup
CREATE INDEX IF NOT EXISTS idx_tasks_assignee 
ON tasks(assignee_id);
```

---

### 3. Integrate with Coordinator

**File:** `crates/axora-agents/src/coordinator.rs` (UPDATE)

```rust
// Add to existing Coordinator
impl Coordinator {
    /// Dispatch tasks via atomic queue (prevents duplicates)
    pub async fn dispatch_via_queue(
        &self,
        queue: &TaskQueue,
        agent_id: &str,
    ) -> Result<Option<TaskResult>> {
        // Atomic checkout (single-assignee model)
        if let Some(task) = queue.checkout_task(agent_id).await? {
            // Execute task
            let result = self.execute_task(&task).await?;
            
            // Complete task (release checkout)
            queue.complete_task(&task.id, result.clone()).await?;
            
            Ok(Some(result))
        } else {
            Ok(None) // No pending tasks
        }
    }
}
```

---

## 📁 File Boundaries

**Create:**
- `crates/axora-indexing/src/task_queue.rs` (NEW)
- `crates/axora-indexing/migrations/0002_task_queue.sql` (NEW)

**Update:**
- `crates/axora-indexing/src/lib.rs` (add module export)
- `crates/axora-agents/src/coordinator.rs` (integrate queue)

**DO NOT Edit:**
- `crates/axora-cache/` (Agent B's other work)
- `crates/axora-docs/` (Agent A's domain)

---

## 🧪 Tests Required

```rust
#[test]
fn test_atomic_checkout() { }

#[test]
fn test_single_assignee_model() { }

#[test]
fn test_concurrent_checkout_no_race() { }

#[test]
fn test_task_completion() { }

#[test]
fn test_task_release_timeout() { }

#[test]
fn test_priority_ordering() { }

#[test]
fn test_no_pending_tasks() { }

#[test]
fn test_queue_integration() { }
```

---

## ✅ Success Criteria

- [ ] `task_queue.rs` created (atomic checkout)
- [ ] Database migration created
- [ ] Atomic checkout works (no race conditions)
- [ ] Single-assignee model enforced
- [ ] Task completion works
- [ ] Task release works (timeout/error)
- [ ] Priority ordering works
- [ ] 8+ tests passing
- [ ] Concurrent checkout test passes (no duplicates)

---

## 🔗 References

- [`PHASE-2-INTEGRATION-COMPETITIVE-ANALYSIS.md`](../shared/PHASE-2-INTEGRATION-COMPETITIVE-ANALYSIS.md) — Competitive analysis
- Research document — Paperclip pattern spec

---

**Start AFTER Sprint 21 (Sliding-Window Semaphores) is complete.**

**Priority: HIGH — prevents duplicate execution in production.**

**Dependencies:**
- Sprint 21 (recommended but not required)

**Blocks:**
- None (infrastructure improvement)
