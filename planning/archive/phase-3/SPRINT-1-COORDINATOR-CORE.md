# Phase 3 Sprint C1: Coordinator Core Structure

**Agent:** C (Implementation Specialist — Coordinator Core)  
**Sprint:** C1  
**Priority:** CRITICAL  
**Estimated:** 8 hours  
**Subagents:** ENABLED (GPT-5.4)

---

## 🎯 Mission

Implement Coordinator Agent core structure with worker registry, task queue integration, and basic dispatch mechanism.

**Context:** Phase 2 has no coordinator (user manages everything). Phase 3 needs Coordinator that autonomously manages workers and tasks.

**Difficulty:** ⚠️ **HIGH** — Foundation for all Phase 3, must be solid

---

## 📋 Subagents Assignment

**You are Lead Agent. Delegate to 3 subagents:**

### Subagent 1: Coordinator Struct + Worker Registry
**Task:** Implement Coordinator core struct with worker management
**File:** `crates/axora-agents/src/coordinator/v2_core.rs`
**Deliverables:**
- `Coordinator` struct (main orchestrator)
- `worker_registry` (track all workers)
- `get_available_worker()` returns idle worker
- `assign_task(worker_id, task)` assigns task
- 5+ tests

### Subagent 2: Task Queue Integration
**Task:** Integrate task queue with coordinator
**File:** `crates/axora-agents/src/coordinator/v2_queue_integration.rs`
**Deliverables:**
- `TaskQueueIntegration` struct
- `load_tasks(mission)` loads decomposed tasks
- `get_next_dispatchable_task()` returns ready task
- `mark_task_complete(task_id)` updates queue
- 5+ tests

### Subagent 3: Basic Dispatch Mechanism
**Task:** Implement basic task dispatch loop
**File:** `crates/axora-agents/src/coordinator/v2_dispatcher.rs`
**Deliverables:**
- `Dispatcher` struct
- `dispatch_loop()` main dispatch loop
- `monitor_workers()` checks worker status
- `handle_completions()` processes completed tasks
- 5+ tests

---

## 🏗️ Lead Agent Responsibilities

**You must:**

1. **Coordinate 3 Subagents:**
   - Assign tasks to all 3 subagents
   - Review core + queue integration + dispatcher
   - Resolve integration issues

2. **Integrate Components:**
   - Create `crates/axora-agents/src/coordinator/v2.rs` (main module)
   - Combine all 3 subagent components
   - Export unified `Coordinator` struct

3. **Implement Basic Mission Execution:**
   - `execute_mission(mission)` high-level method
   - Decompose → Dispatch → Monitor → Merge (simplified)
   - Return `MissionResult`

4. **Write Integration Tests:**
   - Test coordinator creation
   - Test task dispatch (single task)
   - Test worker assignment
   - Test mission execution (end-to-end)

5. **Update Documentation:**
   - Add module to `crates/axora-agents/src/lib.rs`
   - Add coordinator examples

---

## 📐 Technical Spec

### Coordinator Interface

```rust
pub struct Coordinator {
    worker_registry: WorkerRegistry,
    task_queue: TaskQueue,
    dispatcher: Dispatcher,
    blackboard: Arc<BlackboardV2>,
    config: CoordinatorConfig,
}

pub struct CoordinatorConfig {
    pub max_workers: usize,         // Default: 10
    pub dispatch_interval: Duration, // Default: 1s
    pub enable_monitoring: bool,     // Default: true
}

pub struct WorkerRegistry {
    workers: DashMap<WorkerId, WorkerInfo>,
}

pub struct WorkerInfo {
    pub id: WorkerId,
    pub status: WorkerStatus,
    pub current_task: Option<TaskId>,
    pub last_heartbeat: Instant,
}

pub enum WorkerStatus {
    Idle,
    Busy,
    Unhealthy,
    Failed,
}

impl Coordinator {
    pub fn new(config: CoordinatorConfig, blackboard: Arc<BlackboardV2>) -> Result<Self>;
    
    pub async fn execute_mission(&mut self, mission: &str) -> Result<MissionResult>;
    
    pub fn get_available_worker(&self) -> Option<WorkerId>;
    
    pub fn assign_task(&mut self, worker_id: WorkerId, task: Task) -> Result<()>;
    
    pub fn get_mission_status(&self) -> MissionStatus;
}

pub struct MissionResult {
    pub mission_id: String,
    pub success: bool,
    pub output: String,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub duration: Duration,
}

pub struct MissionStatus {
    pub mission_id: String,
    pub progress: f32,  // 0-100%
    pub eta: Option<Duration>,
    pub active_workers: usize,
    pub completed_tasks: usize,
    pub total_tasks: usize,
}
```

### Basic Dispatch Loop

```
1. Get next dispatchable task from queue
2. Get available worker
3. Assign task to worker
4. Wait for completion (or timeout)
5. Mark task complete
6. Repeat until all tasks done
7. Merge results
8. Return MissionResult
```

---

## ✅ Success Criteria

**Sprint is done when:**

- [ ] 3 subagents complete their tasks
- [ ] Lead agent integrates all components
- [ ] `Coordinator` compiles and works
- [ ] 15+ tests passing (5 per subagent + 5 integration)
- [ ] Basic mission execution works (single task)
- [ ] Worker registry tracks all workers
- [ ] Task dispatch works (no crashes)
- [ ] Documentation updated

---

## 🔗 Dependencies

**None** — Can start immediately (FIRST Phase 3 sprint)

**Blocks:**
- Sprint C2 (Decomposition needs coordinator)
- Sprint A1 (Context compacting needs coordinator context)
- Sprint B1 (Worker pool integrates with coordinator)

---

## 📝 Notes for GPT-5.4

**Subagent Pattern:**
```
Lead Agent:
  ├─ Subagent 1: Coordinator Core (parallel)
  ├─ Subagent 2: Queue Integration (parallel)
  └─ Subagent 3: Dispatcher (parallel)
  ↓
Lead Agent: Integration + Mission Execution + Tests
```

**Critical Path:**
- This is FIRST Phase 3 sprint
- All other sprints depend on Coordinator working
- Must be solid foundation (refactor later if needed)

**Difficulty: HIGH**
- 3 subagents to coordinate
- Foundation for all Phase 3
- Must integrate with Phase 2 components
- Async execution (tokio)

**Review Checklist:**
- [ ] Coordinator compiles
- [ ] Worker registry works (add/remove workers)
- [ ] Task dispatch works (assign + execute)
- [ ] Basic mission execution works
- [ ] No memory leaks (workers cleaned up)

---

**START NOW. This is CRITICAL PATH for entire Phase 3.**
