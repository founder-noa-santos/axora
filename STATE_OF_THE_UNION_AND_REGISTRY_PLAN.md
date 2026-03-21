# STATE_OF_THE_UNION_AND_REGISTRY_PLAN

## 1. Current Architectural State (Post-Restoration)

### 1.1 Runtime Entry Path

The factual runtime bootstrap path today is:

- `/Users/noasantos/Fluri/openakta/crates/openakta-core/src/bootstrap.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs`

The active execution flow is:

1. `RuntimeBootstrap::new` loads and merges config via:
   - `load_project_config`
   - `load_workspace_overlay`
   - `merge_config_layers`
   - `resolve_secrets`
   - `build_provider_bundle`
   - `build_model_registry_snapshot`
2. `RuntimeBootstrap::new` initializes:
   - SQLite app database through `openakta_storage::Database`
   - memory services
   - doc sync service
   - embedded MCP server
   - runtime blackboard
3. `RuntimeBootstrap::run_mission` rebuilds:
   - `ProviderRuntimeBundle`
   - `ModelRegistrySnapshot`
   - `ProviderRegistry`
4. `RuntimeBootstrap::run_mission` constructs `CoordinatorConfig`
5. `Coordinator::new(...)` is called
6. `Coordinator::new_with_provider_registry(...)` replaces the temporary internal registry with the built runtime registry
7. `Coordinator::execute_mission(...)` runs decomposition, queueing, dispatch, retrieval, model execution, and blackboard publication

### 1.2 Shared State Path

The factual shared-state runtime today is:

- `CoordinatorV2` uses `RuntimeBlackboard`
- `RuntimeBlackboard` wraps OPENAKTA Cache Blackboard v2

Active path:

- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/blackboard_runtime.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/blackboard.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/blackboard/v2.rs`

Concrete binding:

```rust
// crates/openakta-agents/src/coordinator/v2.rs
pub type BlackboardV2 = Mutex<RuntimeBlackboard>;
```

```rust
// crates/openakta-agents/src/blackboard_runtime.rs
use openakta_cache::{BlackboardV2 as SharedStateBlackboard, BlackboardV2Error};

pub struct RuntimeBlackboard {
    state: SharedStateBlackboard,
    access_control: HashMap<String, Vec<String>>,
    version_tx: watch::Sender<u64>,
}
```

This means the active runtime is:

- local-first
- single-process
- event-driven through local watch/pubsub semantics
- backed by Blackboard v2

The restored `crates/openakta-cache/src/blackboard.rs` remains important because it is the module root and export surface for the subsystem, even though the active runtime data path uses its `v2` child module.

### 1.3 Task Queue Path

There are two queue realities today:

#### Active coordinator queue

- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2_queue_integration.rs`

This is the queue the coordinator actually uses right now.

It is in-memory and dependency-aware:

```rust
pub struct TaskQueueIntegration {
    mission: Option<String>,
    task_order: Vec<String>,
    tasks: HashMap<String, QueueTaskRecord>,
    dependencies: Vec<Dependency>,
    completed: HashSet<String>,
    in_progress: HashSet<String>,
}
```

#### Restored durable queue

- `/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/task_queue.rs`

This is the durable SQLite-backed queue with atomic checkout and timeout release:

```rust
pub struct TaskQueue {
    db: Arc<Mutex<Connection>>,
}
```

Current state of truth:

- coordinator runtime currently uses the in-memory queue facade
- the durable SQLite queue exists and is restored
- the system has not yet unified coordinator v2 onto the durable queue

So the baseline is stable, but not yet architecturally unified.

### 1.4 Provider Execution Path

The active multi-provider execution path today is:

- config DTOs in `/Users/noasantos/Fluri/openakta/crates/openakta-core/src/config.rs`
- config resolution in `/Users/noasantos/Fluri/openakta/crates/openakta-core/src/config_resolve.rs`
- transport/runtime types in `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider_transport.rs`
- transport registry in `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider_registry.rs`
- routing in `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/routing/mod.rs`
- coordinator execution in `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs`

The live design already supports:

- multiple provider instances
- per-instance `base_url`
- per-instance secrets
- per-instance default model
- local vs cloud lanes
- instance-keyed transport maps

Key live types:

- `ProviderInstancesConfig`
- `ProviderInstanceConfig`
- `ResolvedProviderInstance`
- `ProviderRuntimeBundle`
- `ProviderRegistry`
- `CloudModelRef`
- `LocalModelRef`
- `ModelRegistrySnapshot`

### 1.5 Model Registry Path

The live model-registry path today is:

- DTO storage in `CoreConfig.registry_models`
- builder in `build_model_registry_snapshot(...)`
- runtime types in `provider_transport.rs`
- helper functions in `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/model_registry/mod.rs`

Current factual state:

- builtin catalog exists
- TOML extension merge exists
- remote fetch helper exists
- remote registry is not yet wired into bootstrap
- routing is not yet truly registry-first

This is the main unfinished seam for the original mission.

---

## 2. V1/V2 Coexistence Matrix

| File / Module | Current Role | Integrated With V2 Core? | Keep / Refactor | Reason |
|---|---|---|---|---|
| `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs` | Active core orchestrator | Yes | Keep | This is the current coordinator runtime. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator.rs` | Old shim file that was replaced by `coordinator/mod.rs` | Yes, via re-export only | Keep deleted | This was not a separate runtime; it was only an export shim. Replacing it with `coordinator/mod.rs` is stable. No need to restore. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/mod.rs` | Current module root re-exporting v2 | Yes | Keep | Correct crate module root. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/blackboard.rs` | Blackboard subsystem root and export surface | Yes | Keep | Must remain because it is the architectural attachment point for Blackboard v2 and shared exports. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-cache/src/blackboard/v2.rs` | Active runtime shared-state engine | Yes | Keep | This is the actual local-first shared-state implementation in use. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/blackboard_runtime.rs` | Adapter from coordinator/runtime flows to Blackboard v2 | Yes | Keep | This is the correct local-first runtime facade. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-indexing/src/task_queue.rs` | Durable SQLite queue with atomic checkout | Not yet | Keep, refactor integration | Restored. It is not on the active coordinator path yet, but it owns persistence and concurrency invariants. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2_queue_integration.rs` | In-memory temporary queue mirror used by coordinator v2 | Partially | Refactor | It is stable enough for runtime, but explicitly temporary and should be replaced by the durable queue adapter rather than deleted blindly. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/task_queue.rs` | In-memory scheduler/load-balancer queue | Not used by coordinator v2 as persistence | Keep for now, clarify scope | This is a stable in-memory queue utility. It should not be deleted merely because the durable queue exists. It only needs refactoring if product policy requires one canonical queue abstraction. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider_transport.rs` | Active provider transport/runtime type system | Yes | Keep | This is the right injection point for multi-provider runtime resolution. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider_registry.rs` | Active execution-lane registry | Yes | Keep, extend | This should become more registry-authoritative, not replaced. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/routing/mod.rs` | Active routing logic | Yes | Keep, refactor | Stable path, but still heuristic/config-first rather than truly model-registry-first. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/model_registry/mod.rs` | Registry parsing/merge helper layer | Partially | Keep, complete | Good foundation, but remote registry and authoritative routing are incomplete. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/executor.rs` | Older parallel execution subsystem | Not clearly tied to coordinator v2 | Keep for now | No proof yet that it conflicts with v2. Do not delete without a dedicated replacement decision. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/state_machine.rs` | Older orchestration state model | Not clearly tied to coordinator v2 | Keep for now | Version mismatch alone is not enough reason to migrate or delete. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/heartbeat.rs` | Agent lifecycle helper | Not clearly tied to coordinator v2 | Keep for now | Not proven conflicting. |
| `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/memory.rs` | Standalone in-process memory store | Not part of current v2 runtime | Keep deleted unless architecture demands it | No active runtime integration evidence yet. This is not a forced v1/v2 issue; it is simply not on the current baseline path. |

### Coexistence Rule Going Forward

Modules stay if they satisfy any of these:

- they are on the active runtime path
- they own a storage schema
- they own concurrency invariants
- they are the module root or export surface for a core subsystem
- they provide stable functionality that does not conflict with the v2 core

Modules only become refactor targets if they:

- actively duplicate authority with the v2 core
- block required capabilities for the v2 runtime
- force conflicting semantics
- create operational ambiguity around persistence, routing, or shared state

---

## 3. Multi-Provider Implementation Plan

### 3.1 Objective

Resume the original mission by making OPENAKTA’s provider/model layer dynamically configurable for:

- multiple provider instances
- custom URLs
- file-backed or inline API keys
- model metadata such as context window and output limits
- per-model preferred instance routing

The implementation must build on the current stabilized v2 architecture, not rewrite it.

### 3.2 Current Injection Points

The exact integration seams already exist.

#### Config declaration

`/Users/noasantos/Fluri/openakta/crates/openakta-core/src/config.rs`

Current fields:

```rust
pub providers: ProviderInstancesConfig,
pub provider_runtime: ProviderRuntimeConfig,
pub remote_registry: Option<RemoteRegistryConfig>,
pub registry_models: Vec<TomlModelRegistryEntry>,
pub fallback_policy: FallbackPolicy,
pub routing_enabled: bool,
pub provider_context_use_ratio: f32,
pub provider_context_margin_tokens: u32,
pub provider_retrieval_share: f32,
```

This is already the correct config surface.

#### Config resolution

`/Users/noasantos/Fluri/openakta/crates/openakta-core/src/config_resolve.rs`

Current functions:

```rust
resolve_secrets(...)
build_provider_bundle(...)
build_model_registry_snapshot(...)
```

These are the correct places to resolve:

- URLs
- secrets
- runtime bundle
- model registry snapshot

#### Transport/runtime types

`/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider_transport.rs`

Current runtime types already support the target model:

- `ProviderInstanceId`
- `ProviderProfileId`
- `ProviderInstanceConfig`
- `ResolvedProviderInstance`
- `ProviderRuntimeBundle`
- `ModelRegistryEntry`
- `ModelRegistrySnapshot`
- `ModelRoutingHint`
- `CloudModelRef`
- `LocalModelRef`

This file is the correct place for:

- provider profile to wire-shape mapping
- HTTP transport construction
- per-instance base URL + API key handling

#### Transport registry

`/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider_registry.rs`

Current role:

- holds cloud transports by instance id
- holds local transports by instance id
- holds default lane refs
- holds bundle and registry snapshot

This is the correct place to make model-registry metadata more authoritative.

#### Routing

`/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/routing/mod.rs`

Current role:

- chooses cloud vs local lane
- honors explicit hint instance
- orders instances by configured priority
- derives telemetry provider kind from instance profile

This is the primary place that must change from heuristic/config-first to registry-aware routing.

### 3.3 Exact Build Path For Dynamic Model Registry

#### Phase A: Complete bootstrap model-registry assembly

File:

- `/Users/noasantos/Fluri/openakta/crates/openakta-core/src/config_resolve.rs`

Current gap:

- `build_model_registry_snapshot(...)` only merges builtin + TOML extensions
- `remote_registry` is ignored

Required change:

1. update `build_model_registry_snapshot(...)` to accept optional remote bytes or fetch result
2. call:
   - `builtin_catalog()`
   - `parse_remote_json(...)`
   - `apply_toml_extensions(...)`
   - `merge_layers(...)`
3. set truthful provenance instead of hard-coded placeholder versions

Target shape:

```rust
pub async fn build_model_registry_snapshot(core: &CoreConfig) -> anyhow::Result<ModelRegistrySnapshot>
```

or keep the current pure helper and move remote fetch into bootstrap:

```rust
let remote = if let Some(remote_cfg) = &config.remote_registry {
    Some(fetch_remote(...).await?)
} else {
    None
};
let registry = build_model_registry_snapshot(&config, remote.as_deref())?;
```

#### Phase B: Make provider bundle + registry the only runtime selection inputs

Files:

- `/Users/noasantos/Fluri/openakta/crates/openakta-core/src/bootstrap.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider_registry.rs`

Current state:

- bootstrap already builds `ProviderRuntimeBundle`
- bootstrap already builds `ProviderRegistry`
- bootstrap already injects both into `CoordinatorConfig`

Required refinement:

- remove the redundant `Coordinator::new(...)` then `new_with_provider_registry(...)` double construction
- construct the provider registry once and pass it directly into coordinator construction

This does not change architecture, it just tightens the stabilized path.

#### Phase C: Make routing registry-aware

File:

- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/routing/mod.rs`

Current gap:

- router uses:
  - explicit hints
  - local-vs-cloud heuristics
  - configured instance priority
- router does not consult `registry.model_registry.models[model].preferred_instance`
- router does not use model metadata to constrain lane choice or budgets

Required change:

1. add lookup by target model in `ProviderRegistry.model_registry`
2. use precedence:
   - explicit `ModelRoutingHint.instance`
   - registry `preferred_instance`
   - configured `model_instance_priority`
   - default lane fallback
3. when preferred instance is selected:
   - derive `telemetry_kind` from the chosen instance
   - preserve local/cloud lane distinction by checking membership in `cloud` or `local`

This preserves the existing routing module and upgrades it instead of replacing it.

#### Phase D: Make token budgeting registry-driven

Files:

- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/token_budget.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs`

Current state:

- budget derivation helper exists
- config already carries:
  - `context_use_ratio`
  - `context_margin_tokens`
  - `retrieval_share`
- coordinator already calls `derive_effective_budget`

Required change:

- ensure the selected model’s `ModelRegistryEntry` is always used when building:
  - retrieval budget
  - `max_output_tokens`
  - prompt packing constraints

This turns model metadata into real runtime behavior rather than advisory metadata.

#### Phase E: Keep `ProviderKind` coarse and derived only

Files:

- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider_transport.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider.rs`

Rule:

- `ProviderKind` remains valid for request-shape selection and telemetry
- transport selection must continue to be based on instance/profile, not global provider kind

Current code already mostly follows this:

```rust
impl ProviderProfileId {
    pub fn provider_kind(self) -> ProviderKind { ... }
}
```

This is the correct coexistence pattern. Do not “v2 rewrite” `ProviderKind`; simply keep it derived.

### 3.4 Exact Files To Modify For Implementation Phase

Primary files:

- `/Users/noasantos/Fluri/openakta/crates/openakta-core/src/config.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-core/src/config_resolve.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-core/src/bootstrap.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider_transport.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/model_registry/mod.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/provider_registry.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/routing/mod.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/token_budget.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-agents/src/coordinator/v2.rs`

Secondary verification paths:

- `/Users/noasantos/Fluri/openakta/crates/openakta-cli/src/main.rs`
- `/Users/noasantos/Fluri/openakta/crates/openakta-daemon/src/main.rs`

### 3.5 Safe Implementation Order

1. complete `build_model_registry_snapshot(...)` so registry assembly is real
2. wire remote registry into bootstrap
3. make `routing/mod.rs` consult registry metadata
4. make token budgeting consistently use selected model metadata
5. simplify coordinator/provider-registry construction to remove duplicate initialization
6. add tests for:
   - preferred instance wins
   - explicit instance hint wins
   - priority list fallback
   - unknown model default behavior
   - remote + TOML merge order

### 3.6 Frozen Baseline Before Implementation

Before the implementation phase begins, the safe architectural baseline is:

- keep Blackboard subsystem as restored
- keep durable SQLite task queue as restored
- do not delete additional “legacy-looking” files without proving they do not own schemas, concurrency invariants, or subsystem roots
- treat stable V1/V2 coexistence as acceptable unless a real conflict exists

That gives a stable foundation for the actual mission:

- dynamic multi-provider config
- dynamic model metadata registry
- instance-aware routing
- metadata-clamped budgeting
