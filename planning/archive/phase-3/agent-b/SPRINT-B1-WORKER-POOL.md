# Phase 3 Sprint B1: Worker Agent Pool

**Agent:** B (Storage + Context Specialist — HARDEST TASKS)  
**Sprint:** B1  
**Priority:** CRITICAL  
**Estimated:** 8 hours  
**Subagents:** ENABLED (GPT-5.4)

---

## 🎯 Mission

Implement dynamic worker agent pool with lifecycle management, health monitoring, and auto-restart on failure.

**Context:** Phase 2 agents are static (manually managed). Phase 3 needs dynamic worker pool that spawns, monitors, and restarts automatically.

**Difficulty:** ⚠️ **HIGH** — Concurrent state management, failure handling, resource limits

---

## 📋 Subagents Assignment

**You are Lead Agent. Delegate to 4 subagents:**

### Subagent 1: Lifecycle Manager
**Task:** Implement worker lifecycle (spawn, terminate, restart)
**File:** `crates/openakta-agents/src/worker_pool/lifecycle.rs`
**Deliverables:**
- `WorkerLifecycle` enum (Spawning, Ready, Busy, Failed, Terminated)
- `spawn_worker()` creates new worker
- `terminate_worker(id)` gracefully shuts down
- `restart_worker(id)` recreates failed worker
- 5+ tests

### Subagent 2: Health Monitor
**Task:** Implement health monitoring with heartbeat
**File:** `crates/openakta-agents/src/worker_pool/health_monitor.rs`
**Deliverables:**
- `HealthMonitor` struct
- `check_health(worker_id)` returns status
- `heartbeat_interval()` (every 30 seconds)
- `mark_unhealthy(worker_id)` after 3 missed heartbeats
- 5+ tests

### Subagent 3: Worker Spawner
**Task:** Implement dynamic spawning with resource limits
**File:** `crates/openakta-agents/src/worker_pool/spawner.rs`
**Deliverables:**
- `WorkerSpawner` struct
- `spawn_if_capacity()` respects max_workers limit
- `get_available_capacity()` returns available slots
- Auto-scale based on queue depth
- 5+ tests

### Subagent 4: Task Dispatcher
**Task:** Implement task dispatch to workers
**File:** `crates/openakta-agents/src/worker_pool/dispatcher.rs`
**Deliverables:**
- `TaskDispatcher` struct
- `dispatch(task_id, worker_id)` sends task
- `get_result(task_id)` retrieves result
- `retry_failed(task_id)` requeues failed tasks
- 5+ tests

---

## 🏗️ Lead Agent Responsibilities

**You must:**

1. **Coordinate 4 Subagents:**
   - Assign tasks to all 4 subagents
   - Review lifecycle + health + spawner + dispatcher
   - Resolve conflicts (e.g., lifecycle vs health state)

2. **Integrate Components:**
   - Create `crates/openakta-agents/src/worker_pool.rs` (main module)
   - Combine all 4 components
   - Export unified `WorkerPool` struct

3. **Implement Concurrency Control:**
   - Use `DashMap` for concurrent worker access
   - Use `RwLock` for state mutations
   - Prevent race conditions (spawn vs terminate)

4. **Write Integration Tests:**
   - Test concurrent spawning (100 workers)
   - Test health monitoring (simulate failures)
   - Test auto-restart (failed workers recover)
   - Test resource limits (max_workers enforced)

5. **Update Documentation:**
   - Add module to `crates/openakta-agents/src/lib.rs`
   - Add worker pool examples

---

## 📐 Technical Spec

### Worker Pool Interface

```rust
pub struct WorkerPool {
    workers: DashMap<WorkerId, Worker>,
    lifecycle: WorkerLifecycleManager,
    health_monitor: HealthMonitor,
    spawner: WorkerSpawner,
    dispatcher: TaskDispatcher,
    config: WorkerPoolConfig,
}

pub struct Worker {
    id: WorkerId,
    agent: Agent,
    status: WorkerStatus,
    current_task: Option<TaskId>,
    last_heartbeat: Instant,
    health_score: f32,
}

pub enum WorkerStatus {
    Idle,
    Busy(TaskId),
    Unhealthy { reason: String },
    Failed { error: String },
    Terminated,
}

pub struct WorkerPoolConfig {
    pub min_workers: usize,        // Default: 2
    pub max_workers: usize,        // Default: 10
    pub health_check_interval: Duration,  // Default: 30s
    pub unhealthy_threshold: usize, // Default: 3 missed heartbeats
    pub auto_restart: bool,         // Default: true
}

impl WorkerPool {
    pub fn new(config: WorkerPoolConfig) -> Result<Self>;
    
    pub fn get_available_worker(&self) -> Option<WorkerId>;
    
    pub fn dispatch_task(&mut self, worker_id: WorkerId, task: Task) -> Result<()>;
    
    pub fn get_task_status(&mut self, worker_id: &WorkerId) -> Result<TaskStatus>;
    
    pub fn health_check(&mut self) -> Result<Vec<WorkerId>>; // Returns unhealthy workers
    
    pub fn get_pool_stats(&self) -> PoolStats;
}

pub struct PoolStats {
    pub total_workers: usize,
    pub idle_workers: usize,
    pub busy_workers: usize,
    pub unhealthy_workers: usize,
    pub failed_workers: usize,
}
```

### Health Check Algorithm

```
1. Iterate all workers
2. For each worker, check last_heartbeat
3. If time_since(last_heartbeat) > threshold:
   - Decrement health_score
   - If health_score == 0, mark as unhealthy
4. If unhealthy and auto_restart:
   - Terminate worker
   - Spawn new worker
5. Return list of unhealthy workers
```

### Auto-Scaling Algorithm

```
1. Get queue_depth (pending tasks)
2. Get available_workers (idle count)
3. If queue_depth > available_workers * 2:
   - Spawn new worker (if below max)
4. If available_workers > queue_depth * 2:
   - Terminate excess worker (if above min)
```

---

## ✅ Success Criteria

**Sprint is done when:**

- [ ] 4 subagents complete their tasks
- [ ] Lead agent integrates all components
- [ ] `WorkerPool` compiles and works
- [ ] 20+ tests passing (5 per subagent + 10 integration)
- [ ] Concurrent spawning works (100 workers)
- [ ] Health monitoring detects failures
- [ ] Auto-restart recovers failed workers
- [ ] Resource limits enforced (min/max workers)
- [ ] Documentation updated

---

## 🔗 Dependencies

**None** — Can start immediately

**Blocks:**
- Sprint B2 (Task Queue needs worker pool)
- Sprint C1 (Coordinator needs worker pool)

---

## 📝 Notes for GPT-5.4

**Subagent Pattern:**
```
Lead Agent:
  ├─ Subagent 1: Lifecycle (parallel)
  ├─ Subagent 2: Health Monitor (parallel)
  ├─ Subagent 3: Spawner (parallel)
  └─ Subagent 4: Dispatcher (parallel)
  ↓
Lead Agent: Integration + Concurrency + Tests
```

**Concurrency Concerns:**
- Use `DashMap` for worker storage
- Use `RwLock` for state mutations
- Use `mpsc` channels for health notifications
- Test with 100+ concurrent operations

**Difficulty: HIGH**
- 4 subagents to coordinate (most complex so far)
- Concurrent state management (race conditions)
- Failure handling (auto-restart logic)
- Resource limits (min/max workers)

**Review Checklist:**
- [ ] No race conditions in lifecycle
- [ ] Health checks run every 30s
- [ ] Auto-restart works (failed → new worker)
- [ ] Max workers enforced (no over-spawning)
- [ ] Memory leaks (terminated workers cleaned up)

---

**Start NOW. This is CRITICAL path for Phase 3.**
