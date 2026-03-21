# TASK_QUEUE_FORENSIC_AUDIT

## Q1. Persistence & Storage Parity

### Finding

`crates/openakta-agents/src/task_queue.rs` is strictly in-memory.

`crates/openakta-indexing/src/task_queue.rs` was the only queue implementation in this comparison that directly owned SQLite persistence, SQL schema creation, and transactional checkout.

### Evidence: `openakta-agents` queue is memory-only

`/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/task_queue.rs`

```rust
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct TaskQueue {
    scheduler: RwLock<PriorityScheduler>,
    dependency_tracker: RwLock<DependencyTracker>,
    load_balancer: LoadBalancer,
    tasks: DashMap<QueueTaskId, QueuedTask>,
    config: TaskQueueConfig,
}
```

There is:

- no `rusqlite`
- no `Connection`
- no SQL
- no file path
- no transaction
- no disk-backed recovery path

The crate dependency surface confirms this:

`/Users/noasantos/Fluri/openakta/crates/openakta-agents/Cargo.toml`

```toml
dashmap.workspace = true
parking_lot.workspace = true
```

There is no SQLite dependency in `openakta-agents`.

### Evidence: `openakta-indexing` queue was SQLite-backed

Deleted file content from git history:

`git show HEAD~0:crates/openakta-indexing/src/task_queue.rs`

```rust
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct TaskQueue {
    db: Arc<Mutex<Connection>>,
}

pub fn new_in_memory() -> Result<Self> {
    let db = Connection::open(":memory:")?;
    ...
}

pub fn new(path: &Path) -> Result<Self> {
    let db = Connection::open(path)?;
    ...
}
```

It also created and indexed its own queue table:

```rust
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
```

### Additional evidence: coordinator v2 queue integration is also memory-only

`/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2_queue_integration.rs`

```rust
#[derive(Debug, Clone, Default)]
pub struct TaskQueueIntegration {
    mission: Option<String>,
    task_order: Vec<String>,
    tasks: HashMap<String, QueueTaskRecord>,
    dependencies: Vec<Dependency>,
    completed: HashSet<String>,
    in_progress: HashSet<String>,
}
```

The file explicitly states:

```rust
//! `openakta-agents` does not currently depend on `openakta-indexing`, so this file
//! mirrors the relevant queue semantics locally while keeping the API shaped for
//! future replacement with the shared atomic queue.
```

This is direct proof that the current coordinator path is a local in-memory mirror, not the durable shared queue.

### Conclusion for Q1

Storage parity is false.

- `openakta-indexing::TaskQueue` was durable and SQLite-backed.
- `openakta-agents::TaskQueue` is not durable.
- `TaskQueueIntegration` in coordinator v2 is not durable.

Deleting `crates/openakta-indexing/src/task_queue.rs` deleted the only queue implementation here that provided direct SQLite persistence.

---

## Q2. Atomic Checkout & Concurrency Tests

### Finding

`openakta-indexing` had real tests for atomic checkout and concurrent no-duplicate assignment.

`openakta-agents` only tests in-memory scheduling semantics, dependency resolution, and queue accounting. It does not replicate database-level atomic checkout behavior.

### Evidence: atomic checkout tests existed in `openakta-indexing`

Deleted file content from git history:

`git show HEAD~0:crates/openakta-indexing/src/task_queue.rs`

Atomic checkout implementation:

```rust
pub fn checkout_task(&self, agent_id: &str) -> Result<Option<Task>> {
    let mut db = self.db.lock().unwrap();
    let tx = db.transaction()?;

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
        |row| { ... },
    );

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
    ...
}
```

This is conditional transactional checkout.

Test coverage in the deleted indexing queue:

```rust
#[test]
fn test_atomic_checkout() { ... }

#[test]
fn test_single_assignee_model() { ... }

#[test]
fn test_concurrent_checkout_no_race() {
    let queue = std::sync::Arc::new(create_test_queue());
    for i in 0..5 {
        let queue = std::sync::Arc::clone(&queue);
        let agent_id = format!("agent-{}", i);
        let handle = std::thread::spawn(move || queue.checkout_task(&agent_id));
        handles.push(handle);
    }
    ...
    assert_eq!(checked_out.len(), 5);
    assert_eq!(assignees.len(), 5);
}
```

It also tested timeout recovery:

```rust
#[test]
fn test_timed_out_task_release() {
    ...
    let released = queue.release_timed_out_tasks(60).unwrap();
    assert_eq!(released, 1);
}
```

### Evidence: `openakta-agents` tests do not cover DB atomic checkout

`/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/task_queue.rs`

Representative tests:

```rust
#[test]
fn get_next_ready_task_prefers_high_priority_ready_task() { ... }

#[test]
fn dependency_resolution_unblocks_child_after_completion() { ... }

#[test]
fn load_balancing_distributes_ready_tasks_evenly() { ... }

#[test]
fn queue_stats_count_task_states() { ... }
```

These validate:

- priority ordering
- dependency blocking/unblocking
- in-memory queue stats
- load balancing hints

They do not validate:

- SQL transaction behavior
- conditional update semantics
- multi-threaded duplicate checkout prevention against persistent state
- timeout release against persisted `checked_out_at`

### Conclusion for Q2

Atomic checkout test parity is false.

Deleting `openakta-indexing/src/task_queue.rs` removed the only concrete atomic-checkout test suite in this comparison.

---

## Q3. Schema & Migration Dependencies

### Finding

The deleted indexing queue was directly coupled to the indexing migration schema.

The `openakta-agents` queue does not map to any SQL schema at all.

`openakta-storage` has a separate `tasks` table, but it is not schema-compatible with the deleted indexing queue and does not replace its checkout model.

### Evidence: indexing migration matches deleted indexing queue

`/Users/noasantos/Fluri/openakta/crates/openakta-indexing/migrations/0002_task_queue.sql`

```sql
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

CREATE INDEX IF NOT EXISTS idx_tasks_status_priority
ON tasks(status, priority DESC, created_at ASC);

CREATE INDEX IF NOT EXISTS idx_tasks_assignee
ON tasks(assignee_id);

CREATE INDEX IF NOT EXISTS idx_tasks_checked_out
ON tasks(checked_out_at) WHERE status = 'in_progress';
```

Deleted indexing queue struct:

```rust
pub struct Task {
    pub id: String,
    pub description: String,
    pub status: TaskStatus,
    pub assignee_id: Option<String>,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub checked_out_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result: Option<String>,
}
```

This is a direct field-to-column match.

### Evidence: `openakta-storage` schema is different

`/Users/noasantos/Fluri/openakta/crates/openakta-storage/migrations/0001_init.sql`

```sql
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    assignee_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    FOREIGN KEY (assignee_id) REFERENCES agents(id)
);
```

This schema does not contain:

- `priority`
- `checked_out_at`
- `result`
- checkout index ordering
- timeout-release index

And `openakta-storage` maps to a different proto task shape:

`/Users/noasantos/Fluri/openakta/crates/openakta-storage/src/store.rs`

```rust
pub fn create(
    &self,
    title: &str,
    description: &str,
    assignee_id: Option<&str>,
) -> Result<Task> {
    self.conn.execute(
        "INSERT INTO tasks (id, title, description, status, assignee_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        ...
    )?;
}
```

This is not the same queue contract.

### Evidence: `openakta-agents` queue has no schema coupling at all

`/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/task_queue.rs`

```rust
pub struct QueuedTask {
    pub task_id: QueueTaskId,
    pub task: Task,
    pub priority: u8,
    pub dependencies: Vec<QueueTaskId>,
    pub status: TaskQueueStatus,
    pub added_at: Instant,
    pub critical_path_length: usize,
}
```

These fields map to in-process scheduling state, not SQL persistence.

### Conclusion for Q3

Schema coupling is exact for `openakta-indexing` and absent for `openakta-agents`.

The deleted file was the implementation that matched the queue migration.

---

## Q4. The "Orphaned but Intentional" Hypothesis

### Finding

The deleted indexing queue was not a fully dead relic.

It was an unwired but intentional persistence-layer implementation:

- SQLite-backed
- schema-matched to `crates/openakta-indexing/migrations/0002_task_queue.sql`
- transaction-based
- tested for atomic checkout and single-assignee safety

Meanwhile, `openakta-agents` currently contains:

- an in-memory scheduler queue
- an in-memory coordinator queue facade
- no database coupling
- an explicit comment that the local queue mirror exists while waiting for future replacement by a shared atomic queue

### Evidence of intended future integration

`/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2_queue_integration.rs`

```rust
//! `openakta-agents` does not currently depend on `openakta-indexing`, so this file
//! mirrors the relevant queue semantics locally while keeping the API shaped for
//! future replacement with the shared atomic queue.
```

This sentence resolves the ambiguity.

The local `openakta-agents` implementation was not the true durable replacement.
It was a temporary mirror.

---

## Definitive Verdict

The deletion of `/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/task_queue.rs` was not safe.

This is the mathematically certain ground truth from the code:

1. `openakta-agents/src/task_queue.rs` is in-memory only.
2. `openakta-agents/src/coordinator/v2_queue_integration.rs` is also in-memory only.
3. The deleted indexing queue was the only implementation here with:
   - SQLite persistence
   - transactional checkout
   - conditional update for single-assignee atomicity
   - timeout-based release
   - direct mapping to the queue migration schema
4. The deleted indexing queue also carried the only concrete atomic-checkout concurrency tests in this comparison.
5. `openakta-storage` does not replace it because its schema and API are different.

Therefore:

- the deleted indexing queue was not “fully superseded by `openakta-agents`”
- it was an unwired but intentional persistence layer
- deleting it destroyed the only SQLite-backed task queue implementation in the repository

## Final Architectural Classification

`crates/openakta-indexing/src/task_queue.rs` was:

- not the active coordinator runtime path
- not a dead V1 relic
- an unfinished or currently unintegrated persistence subsystem

`crates/openakta-agents/src/task_queue.rs` is:

- an in-memory orchestration queue
- not a durable replacement

## Required Corrective Action

The indexing queue should be restored unless there is an explicit executive decision to abandon durable queue persistence entirely.

If the product goal is durable atomic checkout, then the correct purge target is not the indexing queue itself, but the duplication between:

- the deleted SQLite-backed queue in `openakta-indexing`
- the temporary mirrored in-memory queue facade in `openakta-agents`

The correct end-state would be:

- one canonical persistent queue implementation
- one coordinator integration layer calling it
- one test suite proving atomic checkout behavior end-to-end
