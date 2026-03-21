# CODEBASE_PURGE_EXECUTION_PLAN

## Scope

Repository-wide legacy audit across:

- `crates/openakta-agents`
- `crates/openakta-cache`
- `crates/openakta-indexing`
- `crates/openakta-core`
- `crates/openakta-daemon`

Executive policy applied:

- zero backward compatibility
- delete superseded runtime paths
- no legacy fallbacks
- keep only modules that remain on an active production or exported API path, or where the supposed replacement is still incomplete

## The Hitlist

These deletions are safe and have been executed.

```bash
rm /Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator.rs
rm /Users/noasantos/Fluri/openakta/crates/openakta-agents/src/memory.rs
rm /Users/noasantos/Fluri/openakta/crates/openakta-cache/src/blackboard.rs
rm /Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/task_queue.rs
```

## Required Rust Refactors

### 1. `openakta-agents`: remove legacy coordinator and memory entrypoints

Executed cleanup:

- added `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/mod.rs`
- moved the public coordinator surface to `coordinator/v2.rs`
- introduced `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/blackboard_runtime.rs`
- removed imports and exports that depended on `memory.rs`
- switched runtime blackboard usage to OPENAKTA Cache Blackboard v2

Result:

- `coordinator/v2.rs` is the only coordinator implementation
- `blackboard/v2.rs` is the only runtime shared-state substrate

### 2. `openakta-cache`: remove Blackboard v1 and keep v2-only module surface

Executed cleanup:

- created `/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/blackboard/mod.rs`
- limited the `blackboard` module to:

```rust
pub mod v2;
```

- removed v1 re-exports from `/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/lib.rs`

Deleted public surface:

- `Blackboard`
- `BlackboardError`
- `BlackboardMetadata`
- `BlackboardSchema`
- `BlackboardSnapshot`
- `ChangeType`
- `ReflectionPhase`
- `BlackboardResult`

Kept public surface:

- `BlackboardV2`
- `BlackboardV2Error`
- `BlackboardUpdate`
- versioned context and pubsub types

### 3. `openakta-indexing`: remove orphan task queue implementation

Executed cleanup:

- removed `pub mod task_queue;` from `/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/lib.rs`
- removed task-queue re-exports from `/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/lib.rs`

Deleted public surface:

- `TaskQueue`
- `TaskQueueError`
- `TaskQueueStats`
- `Task`
- `TaskResult`
- `TaskStatus`

Rationale:

- the active task queue lives in `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/task_queue.rs`
- no active production caller in core or daemon depended on the indexing copy

## Validation

Executed successfully:

```bash
cargo check -p openakta-cache -p openakta-indexing -p openakta-agents -p openakta-core -p openakta-daemon
```

Result:

- purge compiles across the affected runtime crates
- removed files were not on the active production path

## The Exceptions

These files were audited and intentionally not deleted.

### `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/executor.rs`

Decision: keep for now.

Strict justification:

- it is still publicly exported from `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/lib.rs`
- it is coupled to `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/state_machine.rs` and `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/heartbeat.rs`
- there is no replacement `executor/v2.rs`
- deleting it now would be a blind API break without a verified successor module

Self-question:

- is it probably legacy? yes
- can it be proven superseded by an active replacement today? not yet
- does v2 coordinator fully cover its exported execution contract? not explicitly

Required precondition before deletion:

- remove its public export from `lib.rs`
- confirm no downstream crate depends on `ConcurrentExecutor`, `ExecutorConfig`, or `ExecutorMissionResult`
- either replace the API with coordinator v2 primitives or formally drop the surface

### `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/state_machine.rs`

Decision: keep for now.

Strict justification:

- still publicly exported from `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/lib.rs`
- directly used by `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/heartbeat.rs`
- deletion requires a coordinated purge of the executor and heartbeat stack

Self-question:

- is it probably an older orchestration model? yes
- is there a typed v2 state-machine replacement already landed? no
- would deletion right now leave a capability gap in exported orchestration state types? yes

Required precondition before deletion:

- replace heartbeat integration with coordinator-v2-native lifecycle state
- remove public re-exports of `StateMachine`, `GlobalState`, `StateTransition`, and `TransitionCondition`

### `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/heartbeat.rs`

Decision: keep for now.

Strict justification:

- it is still part of the executor/state-machine cluster
- it is publicly exported
- there is no landed coordinator-v2-native lifecycle or wake/sleep replacement module

Self-question:

- does the current production path appear to bypass it? mostly yes
- can it be deleted without also deleting executor/state-machine exports? no

Required precondition before deletion:

- either remove the entire executor/state-machine/heartbeat API cluster together
- or provide the replacement lifecycle abstraction first

### `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/graph.rs`

Decision: keep.

Strict justification:

- repository ADRs and planning artifacts still describe graph workflow as active architecture
- the file is still publicly exported as `WorkflowGraph`
- this is not a clear V1/V2 overlap; it is a still-valid architectural primitive

Self-question:

- is it unused by core and daemon right now? mostly
- is that enough to classify it as dead code? no, because the product architecture still names graph execution as active

### `/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/context.rs`

Decision: keep.

Strict justification:

- it is not a clear superseded duplicate of `/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/context_pruning.rs`
- the two modules expose different context-management concerns

Self-question:

- is the naming overlap suspicious? yes
- is there enough evidence that one fully replaces the other? no

### `/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/context_pruning.rs`

Decision: keep.

Strict justification:

- active export surface
- distinct token-budget and pruning behavior
- no direct V2 replacement file exists

## Next Purge Wave

The next candidate purge should be a single coordinated removal of the old agent orchestration cluster:

- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/executor.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/state_machine.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/heartbeat.rs`

That deletion should only happen after:

1. removing all public re-exports from `openakta-agents/src/lib.rs`
2. proving no downstream crate depends on the cluster
3. replacing any remaining lifecycle semantics with coordinator-v2-native types

## Final State After This Purge

Authoritative runtime paths now enforced:

- coordinator: `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs`
- shared state: `/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/blackboard/v2.rs`
- task queue: `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/task_queue.rs`

Deleted overlapping paths:

- legacy coordinator module
- legacy memory module
- blackboard v1
- indexing-local task queue duplicate
