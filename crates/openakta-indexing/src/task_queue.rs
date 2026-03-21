//! Atomic Checkout Semantics
//!
//! This module implements production-grade task queue management:
//! - Atomic checkout (prevents race conditions)
//! - Single-assignee model (one task, one agent)
//! - Paperclip pattern (adapted for SQLite with rusqlite)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     TaskQueue                               │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Atomic Checkout            │  Single-Asssignee Model       │
//! │  - Transaction-based        │  - One task, one agent        │
//! │  - Conditional update       │  - assignee_id tracking       │
//! │  - No race conditions       │  - checked_out_at timestamp   │
//! │                               │                             │
//! │  Task Lifecycle               │  Priority Ordering           │
//! │  - Pending → InProgress     │  - DESC priority              │
//! │  - InProgress → Completed   │  - ASC created_at (FIFO)      │
//! │  - InProgress → Failed      │  - Efficient indexing         │
//! │  - InProgress → Pending     │                              │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use openakta_indexing::task_queue::{TaskQueue, TaskStatus, Task};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let queue = TaskQueue::new_in_memory()?;
//!
//! // Enqueue a task
//! queue.enqueue(&Task::new("task-1", "Test task", 10))?;
//!
//! // Agent atomically checks out a task
//! if let Some(task) = queue.checkout_task("agent-1")? {
//!     println!("Processing task: {}", task.id);
//!     
//!     // Complete the task
//!     queue.complete_task(&task.id, true, "Result")?;
//! }
//! # Ok(())
//! # }
//! ```

use chrono::{DateTime, Utc};
use rusqlite::{params, types::Type, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Task queue error types
#[derive(Error, Debug)]
pub enum TaskQueueError {
    /// Database error
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Task not found
    #[error("task not found: {0}")]
    TaskNotFound(String),

    /// Task already assigned
    #[error("task already assigned to {0}")]
    TaskAlreadyAssigned(String),

    /// Invalid status transition
    #[error("invalid status transition: {from} → {to}")]
    InvalidStatusTransition { from: TaskStatus, to: TaskStatus },
}

/// Result type for task queue operations
pub type Result<T> = std::result::Result<T, TaskQueueError>;

/// Task status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is waiting to be processed
    Pending,
    /// Task is being processed by an agent
    InProgress,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
}

impl TaskStatus {
    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
        }
    }
}

impl FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "pending" => Ok(TaskStatus::Pending),
            "in_progress" => Ok(TaskStatus::InProgress),
            "completed" => Ok(TaskStatus::Completed),
            "failed" => Ok(TaskStatus::Failed),
            _ => Err(format!("unknown task status: {s}")),
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Task entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier
    pub id: String,

    /// Task description
    pub description: String,

    /// Current status
    pub status: TaskStatus,

    /// Agent ID assigned to this task
    pub assignee_id: Option<String>,

    /// Priority (higher = more urgent)
    pub priority: i32,

    /// When task was created
    pub created_at: DateTime<Utc>,

    /// When task was checked out
    pub checked_out_at: Option<DateTime<Utc>>,

    /// When task was completed
    pub completed_at: Option<DateTime<Utc>>,

    /// Task result (JSON)
    pub result: Option<String>,
}

impl Task {
    /// Creates a new pending task
    pub fn new(id: &str, description: &str, priority: i32) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
            status: TaskStatus::Pending,
            assignee_id: None,
            priority,
            created_at: Utc::now(),
            checked_out_at: None,
            completed_at: None,
            result: None,
        }
    }

    /// Checks if task is assigned
    pub fn is_assigned(&self) -> bool {
        self.assignee_id.is_some()
    }

    /// Checks if task is pending
    pub fn is_pending(&self) -> bool {
        self.status == TaskStatus::Pending
    }

    /// Checks if task is in progress
    pub fn is_in_progress(&self) -> bool {
        self.status == TaskStatus::InProgress
    }

    /// Checks if task is completed
    pub fn is_completed(&self) -> bool {
        self.status == TaskStatus::Completed
    }

    /// Checks if task is failed
    pub fn is_failed(&self) -> bool {
        self.status == TaskStatus::Failed
    }
}

/// Task result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Whether task succeeded
    pub success: bool,

    /// Result content
    pub content: String,

    /// Error message (if failed)
    pub error: Option<String>,
}

impl TaskResult {
    /// Creates a successful result
    pub fn success(content: &str) -> Self {
        Self {
            success: true,
            content: content.to_string(),
            error: None,
        }
    }

    /// Creates a failed result
    pub fn failure(error: &str) -> Self {
        Self {
            success: false,
            content: String::new(),
            error: Some(error.to_string()),
        }
    }
}

/// Task queue with atomic checkout
///
/// Implements the Paperclip pattern for atomic task assignment:
/// - Uses transactions for atomic checkout
/// - Single-assignee model (one task, one agent)
/// - Priority ordering (DESC priority, ASC created_at)
pub struct TaskQueue {
    db: Arc<Mutex<Connection>>,
}

impl TaskQueue {
    /// Creates a new in-memory task queue
    pub fn new_in_memory() -> Result<Self> {
        let db = Connection::open(":memory:")?;
        let queue = Self {
            db: Arc::new(Mutex::new(db)),
        };
        queue.init()?;
        Ok(queue)
    }

    /// Creates a new task queue with a file-backed database
    pub fn new(path: &Path) -> Result<Self> {
        let db = Connection::open(path)?;
        let queue = Self {
            db: Arc::new(Mutex::new(db)),
        };
        queue.init()?;
        Ok(queue)
    }

    /// Initialize the database schema
    pub fn init(&self) -> Result<()> {
        let db = self.db.lock().unwrap();
        db.execute(
            r#"
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
            )
            "#,
            [],
        )?;

        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_tasks_status_priority 
             ON tasks(status, priority DESC, created_at ASC)",
            [],
        )?;

        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_tasks_assignee 
             ON tasks(assignee_id)",
            [],
        )?;

        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_tasks_checked_out 
             ON tasks(checked_out_at) WHERE status = 'in_progress'",
            [],
        )?;

        Ok(())
    }

    /// Enqueue a new task
    pub fn enqueue(&self, task: &Task) -> Result<()> {
        let db = self.db.lock().unwrap();
        db.execute(
            r#"
            INSERT INTO tasks (id, description, status, assignee_id, priority, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                task.id,
                task.description,
                task.status.as_str(),
                task.assignee_id,
                task.priority,
                task.created_at.to_rfc3339()
            ],
        )?;

        Ok(())
    }

    /// Atomic checkout (prevents duplicate execution)
    ///
    /// Uses a transaction with conditional update to ensure:
    /// - Only one agent can checkout a task
    /// - No race conditions
    /// - Single-assignee model enforced
    pub fn checkout_task(&self, agent_id: &str) -> Result<Option<Task>> {
        let mut db = self.db.lock().unwrap();
        let tx = db.transaction()?;

        // Find a pending task (priority DESC, created_at ASC for FIFO within priority)
        let task = tx.query_row(
            r#"
            SELECT id, description, status, assignee_id, priority, created_at, 
                   checked_out_at, completed_at, result
            FROM tasks 
            WHERE status = 'pending' 
            ORDER BY priority DESC, created_at ASC 
            LIMIT 1
            "#,
            [],
            |row| {
                Ok(Task {
                    id: row.get(0)?,
                    description: row.get(1)?,
                    status: row
                        .get::<_, String>(2)?
                        .parse::<TaskStatus>()
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(2, Type::Text, e.into())
                        })?,
                    assignee_id: row.get(3)?,
                    priority: row.get(4)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .unwrap()
                        .with_timezone(&Utc),
                    checked_out_at: row.get::<_, Option<String>>(6)?.map(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .unwrap()
                            .with_timezone(&Utc)
                    }),
                    completed_at: row.get::<_, Option<String>>(7)?.map(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .unwrap()
                            .with_timezone(&Utc)
                    }),
                    result: row.get(8)?,
                })
            },
        );

        match task {
            Ok(task) => {
                // Atomically update the task status and assignee
                let rows_affected = tx.execute(
                    r#"
                    UPDATE tasks 
                    SET status = 'in_progress', 
                        assignee_id = ?1, 
                        checked_out_at = ?2
                    WHERE id = ?3 AND status = 'pending'
                    "#,
                    params![agent_id, Utc::now().to_rfc3339(), task.id],
                )?;

                if rows_affected > 0 {
                    // Successfully checked out
                    tx.commit()?;
                    drop(db);

                    // Return the updated task
                    let updated_task = self.get_task(&task.id)?;
                    Ok(updated_task)
                } else {
                    // Task was already taken by another agent (race condition)
                    tx.rollback()?;
                    drop(db);
                    // Retry once
                    self.checkout_task(agent_id)
                }
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // No pending tasks
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Atomic checkout with timeout
    ///
    /// Releases tasks that have been checked out for too long
    pub fn checkout_task_with_timeout(
        &self,
        agent_id: &str,
        timeout_secs: i64,
    ) -> Result<Option<Task>> {
        // First, release any timed-out tasks
        self.release_timed_out_tasks(timeout_secs)?;

        // Then checkout a task
        self.checkout_task(agent_id)
    }

    /// Complete a task (release checkout)
    pub fn complete_task(&self, task_id: &str, success: bool, result_content: &str) -> Result<()> {
        let status = if success {
            TaskStatus::Completed
        } else {
            TaskStatus::Failed
        };

        let task_result = if success {
            TaskResult::success(result_content)
        } else {
            TaskResult::failure(result_content)
        };

        let result_json = serde_json::to_string(&task_result)?;

        let db = self.db.lock().unwrap();
        db.execute(
            r#"
            UPDATE tasks 
            SET status = ?1, 
                completed_at = ?2, 
                result = ?3
            WHERE id = ?4
            "#,
            params![
                status.as_str(),
                Utc::now().to_rfc3339(),
                result_json,
                task_id
            ],
        )?;

        Ok(())
    }

    /// Release a task (timeout or error)
    ///
    /// Returns the task to pending status so another agent can pick it up
    pub fn release_task(&self, task_id: &str) -> Result<()> {
        let db = self.db.lock().unwrap();
        db.execute(
            r#"
            UPDATE tasks 
            SET status = 'pending', 
                assignee_id = NULL, 
                checked_out_at = NULL
            WHERE id = ?1
            "#,
            params![task_id],
        )?;

        Ok(())
    }

    /// Release timed-out tasks
    ///
    /// Tasks checked out for longer than timeout_secs are released
    pub fn release_timed_out_tasks(&self, timeout_secs: i64) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::seconds(timeout_secs);

        let db = self.db.lock().unwrap();
        let rows_affected = db.execute(
            r#"
            UPDATE tasks 
            SET status = 'pending', 
                assignee_id = NULL, 
                checked_out_at = NULL
            WHERE status = 'in_progress' 
              AND checked_out_at < ?1
            "#,
            params![cutoff.to_rfc3339()],
        )?;

        Ok(rows_affected)
    }

    /// Get a task by ID
    pub fn get_task(&self, task_id: &str) -> Result<Option<Task>> {
        let db = self.db.lock().unwrap();
        let task = db.query_row(
            "SELECT id, description, status, assignee_id, priority, created_at, 
                    checked_out_at, completed_at, result
             FROM tasks WHERE id = ?1",
            params![task_id],
            |row| {
                Ok(Task {
                    id: row.get(0)?,
                    description: row.get(1)?,
                    status: row
                        .get::<_, String>(2)?
                        .parse::<TaskStatus>()
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(2, Type::Text, e.into())
                        })?,
                    assignee_id: row.get(3)?,
                    priority: row.get(4)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .unwrap()
                        .with_timezone(&Utc),
                    checked_out_at: row.get::<_, Option<String>>(6)?.map(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .unwrap()
                            .with_timezone(&Utc)
                    }),
                    completed_at: row.get::<_, Option<String>>(7)?.map(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .unwrap()
                            .with_timezone(&Utc)
                    }),
                    result: row.get(8)?,
                })
            },
        );

        match task {
            Ok(task) => Ok(Some(task)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get tasks by assignee
    pub fn get_tasks_by_assignee(&self, agent_id: &str) -> Result<Vec<Task>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT id, description, status, assignee_id, priority, created_at, 
                    checked_out_at, completed_at, result
             FROM tasks WHERE assignee_id = ?1 
             ORDER BY priority DESC, created_at ASC",
        )?;

        let tasks = stmt.query_map(params![agent_id], |row| {
            Ok(Task {
                id: row.get(0)?,
                description: row.get(1)?,
                status: row
                    .get::<_, String>(2)?
                    .parse::<TaskStatus>()
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(2, Type::Text, e.into())
                    })?,
                assignee_id: row.get(3)?,
                priority: row.get(4)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap()
                    .with_timezone(&Utc),
                checked_out_at: row.get::<_, Option<String>>(6)?.map(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .unwrap()
                        .with_timezone(&Utc)
                }),
                completed_at: row.get::<_, Option<String>>(7)?.map(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .unwrap()
                        .with_timezone(&Utc)
                }),
                result: row.get(8)?,
            })
        })?;

        let mut result = Vec::new();
        for task in tasks {
            result.push(task?);
        }

        Ok(result)
    }

    /// Get pending task count
    pub fn pending_count(&self) -> Result<usize> {
        let db = self.db.lock().unwrap();
        let count: i64 = db.query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'pending'",
            [],
            |row| row.get(0),
        )?;

        Ok(count as usize)
    }

    /// Get in-progress task count
    pub fn in_progress_count(&self) -> Result<usize> {
        let db = self.db.lock().unwrap();
        let count: i64 = db.query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'in_progress'",
            [],
            |row| row.get(0),
        )?;

        Ok(count as usize)
    }

    /// Get completed task count
    pub fn completed_count(&self) -> Result<usize> {
        let db = self.db.lock().unwrap();
        let count: i64 = db.query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'completed'",
            [],
            |row| row.get(0),
        )?;

        Ok(count as usize)
    }

    /// Get all tasks
    pub fn get_all_tasks(&self) -> Result<Vec<Task>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT id, description, status, assignee_id, priority, created_at, 
                    checked_out_at, completed_at, result
             FROM tasks ORDER BY status, priority DESC, created_at ASC",
        )?;

        let tasks = stmt.query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                description: row.get(1)?,
                status: row
                    .get::<_, String>(2)?
                    .parse::<TaskStatus>()
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(2, Type::Text, e.into())
                    })?,
                assignee_id: row.get(3)?,
                priority: row.get(4)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap()
                    .with_timezone(&Utc),
                checked_out_at: row.get::<_, Option<String>>(6)?.map(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .unwrap()
                        .with_timezone(&Utc)
                }),
                completed_at: row.get::<_, Option<String>>(7)?.map(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .unwrap()
                        .with_timezone(&Utc)
                }),
                result: row.get(8)?,
            })
        })?;

        let mut result = Vec::new();
        for task in tasks {
            result.push(task?);
        }

        Ok(result)
    }

    /// Delete a task
    pub fn delete_task(&self, task_id: &str) -> Result<()> {
        let db = self.db.lock().unwrap();
        db.execute("DELETE FROM tasks WHERE id = ?1", params![task_id])?;
        Ok(())
    }

    /// Clear all tasks
    pub fn clear_all(&self) -> Result<usize> {
        let db = self.db.lock().unwrap();
        let rows_affected = db.execute("DELETE FROM tasks", [])?;
        Ok(rows_affected)
    }

    /// Get queue statistics
    pub fn get_stats(&self) -> Result<TaskQueueStats> {
        let pending = self.pending_count()?;
        let in_progress = self.in_progress_count()?;
        let completed = self.completed_count()?;

        Ok(TaskQueueStats {
            pending,
            in_progress,
            completed,
            total: pending + in_progress + completed,
        })
    }
}

/// Task queue statistics
#[derive(Debug, Clone)]
pub struct TaskQueueStats {
    /// Number of pending tasks
    pub pending: usize,

    /// Number of in-progress tasks
    pub in_progress: usize,

    /// Number of completed tasks
    pub completed: usize,

    /// Total number of tasks
    pub total: usize,
}

impl TaskQueueStats {
    /// Get completion rate
    pub fn completion_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.completed as f64 / self.total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_queue() -> TaskQueue {
        TaskQueue::new_in_memory().unwrap()
    }

    #[test]
    fn test_atomic_checkout() {
        let queue = create_test_queue();

        // Enqueue a task
        let task = Task::new("task-1", "Test task", 10);
        queue.enqueue(&task).unwrap();

        // Checkout task
        let checked_out = queue.checkout_task("agent-1").unwrap();

        assert!(checked_out.is_some());
        let task = checked_out.unwrap();
        assert_eq!(task.status, TaskStatus::InProgress);
        assert_eq!(task.assignee_id, Some("agent-1".to_string()));
        assert!(task.checked_out_at.is_some());
    }

    #[test]
    fn test_single_assignee_model() {
        let queue = create_test_queue();

        // Enqueue a task
        let task = Task::new("task-1", "Test task", 10);
        queue.enqueue(&task).unwrap();

        // First agent checks out
        let result1 = queue.checkout_task("agent-1").unwrap();
        assert!(result1.is_some());

        // Second agent tries to checkout (should get nothing since only one task)
        let result2 = queue.checkout_task("agent-2").unwrap();
        assert!(result2.is_none());

        // Verify task is assigned to agent-1
        let task = queue.get_task("task-1").unwrap().unwrap();
        assert_eq!(task.assignee_id, Some("agent-1".to_string()));
    }

    #[test]
    fn test_concurrent_checkout_no_race() {
        let queue = std::sync::Arc::new(create_test_queue());

        // Enqueue 5 tasks
        for i in 0..5 {
            let task = Task::new(&format!("task-{}", i), "Test task", 10);
            queue.enqueue(&task).unwrap();
        }

        // Simulate concurrent checkouts
        let mut handles = Vec::new();
        for i in 0..5 {
            let queue = std::sync::Arc::clone(&queue);
            let agent_id = format!("agent-{}", i);
            let handle = std::thread::spawn(move || queue.checkout_task(&agent_id));
            handles.push(handle);
        }

        // Collect results
        let mut results = Vec::new();
        for handle in handles {
            let result = handle.join().unwrap().unwrap();
            results.push(result);
        }

        // All 5 tasks should be checked out (no duplicates)
        let checked_out: Vec<_> = results.into_iter().flatten().collect();
        assert_eq!(checked_out.len(), 5);

        // Each task should have unique assignee
        let assignees: std::collections::HashSet<_> = checked_out
            .iter()
            .filter_map(|t| t.assignee_id.clone())
            .collect();
        assert_eq!(assignees.len(), 5);
    }

    #[test]
    fn test_task_completion() {
        let queue = create_test_queue();

        // Enqueue and checkout
        let task = Task::new("task-1", "Test task", 10);
        queue.enqueue(&task).unwrap();
        queue.checkout_task("agent-1").unwrap();

        // Complete task
        queue.complete_task("task-1", true, "Success!").unwrap();

        // Verify completion
        let task = queue.get_task("task-1").unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.completed_at.is_some());
        assert!(task.result.is_some());

        let result: TaskResult = serde_json::from_str(&task.result.unwrap()).unwrap();
        assert!(result.success);
        assert_eq!(result.content, "Success!");
    }

    #[test]
    fn test_task_release_timeout() {
        let queue = create_test_queue();

        // Enqueue and checkout
        let task = Task::new("task-1", "Test task", 10);
        queue.enqueue(&task).unwrap();
        queue.checkout_task("agent-1").unwrap();

        // Release task
        queue.release_task("task-1").unwrap();

        // Verify release
        let task = queue.get_task("task-1").unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.assignee_id.is_none());
        assert!(task.checked_out_at.is_none());

        // Another agent can now checkout
        let result = queue.checkout_task("agent-2").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().assignee_id, Some("agent-2".to_string()));
    }

    #[test]
    fn test_priority_ordering() {
        let queue = create_test_queue();

        // Enqueue tasks with different priorities
        queue.enqueue(&Task::new("low", "Low priority", 1)).unwrap();
        queue
            .enqueue(&Task::new("high", "High priority", 100))
            .unwrap();
        queue
            .enqueue(&Task::new("medium", "Medium priority", 50))
            .unwrap();

        // Checkout should return highest priority first
        let task1 = queue.checkout_task("agent-1").unwrap().unwrap();
        assert_eq!(task1.id, "high");

        let task2 = queue.checkout_task("agent-2").unwrap().unwrap();
        assert_eq!(task2.id, "medium");

        let task3 = queue.checkout_task("agent-3").unwrap().unwrap();
        assert_eq!(task3.id, "low");
    }

    #[test]
    fn test_no_pending_tasks() {
        let queue = create_test_queue();

        // No tasks enqueued
        let result = queue.checkout_task("agent-1").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_queue_integration() {
        let queue = create_test_queue();

        // Enqueue multiple tasks
        for i in 0..10 {
            let task = Task::new(&format!("task-{}", i), "Test task", i);
            queue.enqueue(&task).unwrap();
        }

        // Check stats
        let stats = queue.get_stats().unwrap();
        assert_eq!(stats.pending, 10);
        assert_eq!(stats.in_progress, 0);
        assert_eq!(stats.completed, 0);

        // Process all tasks
        for i in 0..10 {
            let agent_id = format!("agent-{}", i);
            if let Some(task) = queue.checkout_task(&agent_id).unwrap() {
                queue.complete_task(&task.id, true, "Done").unwrap();
            }
        }

        // Check final stats
        let stats = queue.get_stats().unwrap();
        assert_eq!(stats.pending, 0);
        assert_eq!(stats.in_progress, 0);
        assert_eq!(stats.completed, 10);
        assert_eq!(stats.completion_rate(), 1.0);
    }

    #[test]
    fn test_task_failure() {
        let queue = create_test_queue();

        // Enqueue and checkout
        let task = Task::new("task-1", "Test task", 10);
        queue.enqueue(&task).unwrap();
        queue.checkout_task("agent-1").unwrap();

        // Fail task
        queue
            .complete_task("task-1", false, "Error occurred")
            .unwrap();

        // Verify failure
        let task = queue.get_task("task-1").unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Failed);

        let result: TaskResult = serde_json::from_str(&task.result.unwrap()).unwrap();
        assert!(!result.success);
        assert_eq!(result.error, Some("Error occurred".to_string()));
    }

    #[test]
    fn test_get_tasks_by_assignee() {
        let queue = create_test_queue();

        // Enqueue and checkout multiple tasks for same agent
        for i in 0..5 {
            let task = Task::new(&format!("task-{}", i), "Test task", 10);
            queue.enqueue(&task).unwrap();
        }

        for _ in 0..3 {
            queue.checkout_task("agent-1").unwrap();
        }

        // Get tasks by assignee
        let tasks = queue.get_tasks_by_assignee("agent-1").unwrap();
        assert_eq!(tasks.len(), 3);

        // All should be assigned to agent-1
        for task in &tasks {
            assert_eq!(task.assignee_id, Some("agent-1".to_string()));
        }
    }

    #[test]
    fn test_timed_out_task_release() {
        let queue = create_test_queue();

        // Enqueue and checkout
        let task = Task::new("task-1", "Test task", 10);
        queue.enqueue(&task).unwrap();
        queue.checkout_task("agent-1").unwrap();

        // Manually set checked_out_at to past (simulate timeout)
        let past = Utc::now() - chrono::Duration::seconds(100);
        let db = queue.db.lock().unwrap();
        db.execute(
            "UPDATE tasks SET checked_out_at = ? WHERE id = ?",
            params![past.to_rfc3339(), "task-1"],
        )
        .unwrap();
        drop(db);

        // Release timed out tasks (timeout = 60 seconds)
        let released = queue.release_timed_out_tasks(60).unwrap();
        assert_eq!(released, 1);

        // Task should be back to pending
        let task = queue.get_task("task-1").unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.assignee_id.is_none());
    }

    #[test]
    fn test_clear_all() {
        let queue = create_test_queue();

        // Enqueue tasks
        for i in 0..10 {
            let task = Task::new(&format!("task-{}", i), "Test task", 10);
            queue.enqueue(&task).unwrap();
        }

        // Clear all
        let deleted = queue.clear_all().unwrap();
        assert_eq!(deleted, 10);

        // Verify empty
        let stats = queue.get_stats().unwrap();
        assert_eq!(stats.total, 0);
    }
}
