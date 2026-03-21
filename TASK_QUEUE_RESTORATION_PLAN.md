# TASK_QUEUE_RESTORATION_PLAN

## 1. Restoration Commands

The durable queue restoration is:

```bash
git checkout -- /Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/task_queue.rs
```

The crate surface restoration is:

```bash
# restore pub mod + pub use in crates/openakta-indexing/src/lib.rs
```

If the queue had also been removed from the index in a future pass, the full restore command would be:

```bash
git checkout -- /Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/task_queue.rs
git checkout -- /Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/lib.rs
```

Atomic checkout tests are restored with the file itself because they are embedded in:

- `/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/task_queue.rs`

Key restored tests:

- `test_atomic_checkout`
- `test_single_assignee_model`
- `test_concurrent_checkout_no_race`
- `test_task_release_timeout`
- `test_timed_out_task_release`
- `test_queue_integration`

## 2. Restoration Executed

Restored:

- `/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/task_queue.rs`
- `pub mod task_queue;` in `/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/lib.rs`
- `pub use task_queue::{Task, TaskQueue, TaskQueueError, TaskQueueStats, TaskResult, TaskStatus};` in `/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/lib.rs`

## 3. Immediate Verification

Run:

```bash
cargo check -p openakta-indexing -p openakta-agents -p openakta-core -p openakta-daemon
cargo test -p openakta-indexing task_queue --quiet
```

## 4. Canonical Durable Queue Refactor

Target architecture:

- canonical queue implementation: `/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/task_queue.rs`
- coordinator runtime integration: `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2_queue_integration.rs`
- no in-memory mirror as source of truth

### Step 1: Move coordinator queue integration behind a trait

In `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2_queue_integration.rs`, replace the local `HashMap`/`HashSet` state store with a trait boundary:

```rust
pub trait MissionQueue {
    fn load_tasks(&mut self, mission: &DecomposedMission) -> Result<usize>;
    fn get_next_dispatchable_task(&mut self) -> Result<Option<Task>>;
    fn mark_task_complete(&mut self, task_id: &str) -> Result<()>;
    fn total_tasks(&self) -> usize;
    fn completed_tasks(&self) -> usize;
    fn get_task(&self, task_id: &str) -> Option<Task>;
    fn is_complete(&self) -> bool;
}
```

Purpose:

- stop hard-coding a mirror queue into coordinator v2
- make the durable queue injectable

### Step 2: Add an adapter over `openakta_indexing::TaskQueue`

Create a new adapter in `openakta-agents`, for example:

- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2_sqlite_queue.rs`

Responsibilities:

- map `DecomposedMission` tasks into durable queue rows
- map coordinator `Task` into durable queue `Task`
- call:
  - `enqueue`
  - `checkout_task`
  - `complete_task`
  - `get_task`
  - `get_stats`

This adapter becomes the concrete implementation of `MissionQueue`.

### Step 3: Make `openakta-agents` depend on `openakta-indexing`

Add the crate dependency in:

- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/Cargo.toml`

This removes the current anti-pattern explicitly documented in:

- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2_queue_integration.rs`

Current comment:

```rust
//! `openakta-agents` does not currently depend on `openakta-indexing`, so this file
//! mirrors the relevant queue semantics locally while keeping the API shaped for
//! future replacement with the shared atomic queue.
```

That “future replacement” must now happen.

### Step 4: Persist decomposition into the durable queue

Current in-memory loading:

```rust
self.task_queue.load_tasks(&decomposed)?;
```

Required replacement:

- on mission start, convert decomposed mission tasks into durable queue rows
- write task dependency metadata into durable storage as well

This requires extending the durable queue schema. The current indexing queue persists:

- `id`
- `description`
- `status`
- `assignee_id`
- `priority`
- `created_at`
- `checked_out_at`
- `completed_at`
- `result`

To replace the in-memory coordinator mirror correctly, add durable dependency support:

- a `task_dependencies` table keyed by task id
- or a serialized dependency field if simplicity is preferred initially

Without this, coordinator v2 cannot preserve dependency-aware dispatch semantics across restarts.

### Step 5: Replace local reservation logic with atomic checkout

Current mirror behavior in `v2_queue_integration.rs`:

```rust
record.task.status = TaskStatus::InProgress;
self.in_progress.insert(next_id.clone());
```

Required replacement:

- coordinator calls durable `checkout_task(agent_id)`
- queue row transitions from `pending` to `in_progress` inside a transaction
- durable queue becomes the only reservation authority

### Step 6: Replace local completion bookkeeping with durable completion

Current mirror behavior:

```rust
record.task.status = TaskStatus::Completed;
self.in_progress.remove(task_id);
self.completed.insert(task_id.to_string());
```

Required replacement:

- call durable `complete_task(task_id, success, result_content)`
- query queue stats from the DB-backed queue
- compute mission completion from durable state, not local sets

### Step 7: Delete the in-memory mirror after parity is reached

Only after durable parity exists, delete the local state from:

- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2_queue_integration.rs`

Delete:

- `task_order: Vec<String>`
- `tasks: HashMap<String, QueueTaskRecord>`
- `dependencies: Vec<Dependency>`
- `completed: HashSet<String>`
- `in_progress: HashSet<String>`

At end-state, `v2_queue_integration.rs` should be an adapter or facade over the canonical SQLite queue, not its own queue engine.

## 5. Anti-Pattern to Purge

The anti-pattern is not the durable queue.

The anti-pattern is:

- a real SQLite-backed queue in `openakta-indexing`
- plus a shadow in-memory queue in `openakta-agents`
- plus coordinator logic running against the shadow instead of the durable source of truth

The purge target is the mirror, not the persistence layer.

## 6. Final Required End-State

There must be exactly one canonical queue authority:

- durable
- SQLite-backed
- atomic checkout
- timeout release
- dependency-aware
- used directly by coordinator v2

That authority should be:

- `/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/task_queue.rs`
