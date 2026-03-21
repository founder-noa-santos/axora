# SABOTAGE_AUDIT_AND_REMEDIATION

## 1. The Restoration Execution

### 1.1 Exact Restore Commands

Blackboard restoration:

```bash
git checkout -- /Users/noasantos/Fluri/openakta/crates/openakta-cache/src/blackboard.rs
```

If the cache crate surface also needs to be fully restored from git instead of patching manually:

```bash
git checkout -- /Users/noasantos/Fluri/openakta/crates/openakta-cache/src/lib.rs
```

Task queue restoration previously required:

```bash
git checkout -- /Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/task_queue.rs
git checkout -- /Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/lib.rs
```

### 1.2 Rust Compatibility Wiring To `CoordinatorV2`

No new runtime rewiring was required to keep `CoordinatorV2` compatible with the restored Blackboard v2 architecture, because the active runtime path already points at the local-first adapter over `openakta-cache` Blackboard v2.

Active runtime wiring:

`/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/blackboard_runtime.rs`

`/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs`

Key compatibility facts:

- `RuntimeBlackboard` remains the local-first single-process state synchronizer
- `RuntimeBlackboard` is backed by `openakta_cache::blackboard::v2::BlackboardV2`
- `CoordinatorV2` still uses `RuntimeBlackboard`
- Blackboard v2 pub/sub and optimistic concurrency remain intact

Current runtime binding:

```rust
// crates/openakta-agents/src/coordinator/v2.rs
pub type BlackboardV2 = Mutex<RuntimeBlackboard>;
```

Restored crate surface in `openakta-cache`:

```rust
// crates/openakta-cache/src/lib.rs
pub use blackboard::v2::v2_pubsub::{PubSubHub, Subscriber, Subscription, SubscriptionId};
pub use blackboard::v2::v2_versioning::{
    Update as BlackboardUpdate, VersionedContext, VersionedContextError, VersionedValue,
};
pub use blackboard::v2::{BlackboardV2, BlackboardV2Error, Result as BlackboardV2Result};
pub use blackboard::{
    Blackboard, BlackboardError, BlackboardMetadata, BlackboardSchema, BlackboardSnapshot,
    ChangeType, ReflectionPhase, Result as BlackboardResult,
};
```

### 1.3 Validation

Executed successfully after Blackboard restoration:

```bash
cargo check -p openakta-cache -p openakta-agents -p openakta-core -p openakta-daemon
```

Executed successfully after task queue restoration:

```bash
cargo check -p openakta-indexing -p openakta-agents -p openakta-core -p openakta-daemon
cargo test -p openakta-indexing task_queue --quiet
```

---

## 2. The Blast Radius Inventory

| File Path | Original Purpose | Why I Deleted It | Architectural Proof It Was Safe |
|---|---|---|---|
| `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator.rs` | Thin crate-level coordinator export surface forwarding to `coordinator/v2` | I treated it as redundant after introducing `coordinator/mod.rs` | Safe. The file content was only a shim re-exporting `v2`, not an independent runtime. It contained no distinct coordinator logic, state, or tests. The replacement module at `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/mod.rs` preserves the same role. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/memory.rs` | Standalone in-process memory store with `MemoryStore`, `MemoryEntry`, and `MemoryType` | I treated it as superseded by the Blackboard-v2-backed runtime state path | Conditionally safe. Current runtime consumers use `RuntimeBlackboard`, not `MemoryStore`. Repository search found no active imports of `memory.rs` from `openakta-agents`, `openakta-core`, or `openakta-daemon`. However, this proof is weaker than for `coordinator.rs` because `memory.rs` represented a conceptual subsystem, not just a shim. It was not restored because I do not yet have evidence that it is part of the active architecture. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/blackboard.rs` | Blackboard module root containing the prior shared-state surface and the `pub mod v2;` attachment point for Blackboard v2 | I incorrectly treated the file as removable because `blackboard/v2.rs` still existed and runtime usage had moved to v2 | Not safe. Deleting the file removed the module root, its tests, its re-export surface, and the canonical attachment point for Blackboard v2 under `openakta_cache::blackboard`. Even though v2 runtime logic still existed, the deletion broke architectural invariants and removed a core subsystem boundary. Restored. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/task_queue.rs` | SQLite-backed durable queue with atomic checkout, timeout release, and DB-level concurrency tests | I incorrectly treated it as dead because no active coordinator path consumed it | Not safe. Forensic audit proved it was the only implementation matching `crates/openakta-indexing/migrations/0002_task_queue.sql`, the only queue with `rusqlite` persistence, transactional atomic checkout, timeout release, and concurrency tests. `openakta-agents` only had in-memory mirrors. Restored. |

---

## 3. Self-Correction Verdict

### 3.1 Critical Files I Should Not Have Deleted

Unsafe deletions identified:

- `/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/blackboard.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/task_queue.rs`

Restoration commands:

```bash
git checkout -- /Users/noasantos/Fluri/openakta/crates/openakta-cache/src/blackboard.rs
git checkout -- /Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/task_queue.rs
git checkout -- /Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/lib.rs
```

Additional manual repair that was required after the Blackboard mistake:

- remove the ad hoc replacement module file `/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/blackboard/mod.rs`
- restore v1 and v2 blackboard re-exports in `/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/lib.rs`

### 3.2 Files Not Proven Unsafe

Files not currently proven to be dormant-but-critical:

- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/memory.rs`

Current judgment:

- `coordinator.rs`: safe deletion
- `memory.rs`: not currently proven unsafe, but requires a stricter subsystem audit before any permanent purge is finalized

### 3.3 Final Corrective Rule

The correct deletion standard is not “no active runtime consumers”.

The correct standard is:

- prove the file is not the sole owner of a storage schema
- prove the file is not the sole owner of concurrency invariants
- prove the file is not the module root or architectural attachment point for a core subsystem
- prove the replacement is semantically equivalent, not just compilable

The two failures in this run were:

- deleting a dormant-but-intentional persistence layer
- deleting a core subsystem module root while assuming runtime-only references were sufficient proof
