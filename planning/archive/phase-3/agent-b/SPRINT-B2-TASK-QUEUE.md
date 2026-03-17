# Phase 3 Sprint B2: Task Queue Management

**Agent:** B (Storage + Context Specialist — HARDEST TASKS)  
**Sprint:** B2  
**Priority:** HIGH  
**Estimated:** 8 hours  
**Subagents:** ENABLED (GPT-5.4)

---

## 🎯 Mission

Implement task queue with priority-based scheduling, dependency tracking (DAG), and load balancing.

**Context:** Phase 2 has simple FIFO queue. Phase 3 needs intelligent queue with priorities, dependencies, and critical path calculation.

**Difficulty:** ⚠️ **HIGH** — DAG-based scheduling, dependency resolution, load balancing

---

## 📋 Subagents Assignment

**You are Lead Agent. Delegate to 3 subagents:**

### Subagent 1: Priority Scheduler
**Task:** Implement priority-based task scheduling
**File:** `crates/axora-agents/src/task_queue/priority_scheduler.rs`
**Deliverables:**
- `PriorityScheduler` struct
- `add_task(task, priority)` with priority 0-100
- `get_next_task()` returns highest priority ready task
- `reorder_queue()` adjusts based on priority changes
- 5+ tests

### Subagent 2: Dependency Tracker (DAG)
**Task:** Implement dependency tracking with DAG
**File:** `crates/axora-agents/src/task_queue/dependency_tracker.rs`
**Deliverables:**
- `DependencyTracker` struct
- `add_dependency(task_id, depends_on)` creates edge
- `get_ready_tasks()` returns tasks with no pending dependencies
- `detect_cycles()` prevents circular dependencies
- 5+ tests

### Subagent 3: Load Balancer + Critical Path
**Task:** Implement load balancing and critical path calculation
**File:** `crates/axora-agents/src/task_queue/load_balancer.rs`
**Deliverables:**
- `LoadBalancer` struct
- `calculate_critical_path()` returns longest dependency chain
- `balance_load(workers, tasks)` distributes evenly
- `estimate_completion_time()` based on critical path
- 5+ tests

---

## 🏗️ Lead Agent Responsibilities

**You must:**

1. **Coordinate 3 Subagents:**
   - Assign tasks to all 3 subagents
   - Review scheduler + dependency + load balancer
   - Resolve conflicts (priority vs dependency ordering)

2. **Integrate Components:**
   - Create `crates/axora-agents/src/task_queue.rs` (main module)
   - Combine scheduler + dependency tracker + load balancer
   - Export unified `TaskQueue` struct

3. **Implement Topological Sorting:**
   - Sort tasks by dependencies (DAG)
   - Respect priority within same dependency level
   - Update on task completion

4. **Write Integration Tests:**
   - Test priority ordering (high priority first)
   - Test dependency resolution (no cycles)
   - Test critical path calculation (accurate)
   - Test load balancing (even distribution)

5. **Update Documentation:**
   - Add module to `crates/axora-agents/src/lib.rs`
   - Add task queue examples

---

## 📐 Technical Spec

### Task Queue Interface

```rust
pub struct TaskQueue {
    scheduler: PriorityScheduler,
    dependency_tracker: DependencyTracker,
    load_balancer: LoadBalancer,
    tasks: DashMap<TaskId, QueuedTask>,
    config: TaskQueueConfig,
}

pub struct QueuedTask {
    task_id: TaskId,
    task: Task,
    priority: u8,              // 0-100 (higher = more important)
    dependencies: Vec<TaskId>,
    status: TaskQueueStatus,
    added_at: Instant,
    critical_path_length: usize,
}

pub enum TaskQueueStatus {
    Pending,
    Ready,          // All dependencies satisfied
    InProgress { worker_id: WorkerId },
    Completed,
    Failed { error: String },
    Blocked { reason: String },
}

pub struct TaskQueueConfig {
    pub max_queue_size: usize,     // Default: 1000
    pub default_priority: u8,      // Default: 50
    pub enable_load_balancing: bool, // Default: true
}

impl TaskQueue {
    pub fn new(config: TaskQueueConfig) -> Self;
    
    pub fn add_task(&mut self, task: Task, priority: u8, dependencies: Vec<TaskId>) -> Result<TaskId>;
    
    pub fn get_next_ready_task(&mut self) -> Option<QueuedTask>;
    
    pub fn mark_completed(&mut self, task_id: TaskId);
    
    pub fn mark_failed(&mut self, task_id: TaskId, error: String);
    
    pub fn get_queue_stats(&self) -> QueueStats;
    
    pub fn get_critical_path(&self) -> Vec<TaskId>;
}

pub struct QueueStats {
    pub total_tasks: usize,
    pub pending_tasks: usize,
    pub ready_tasks: usize,
    pub in_progress_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub avg_wait_time: Duration,
    pub estimated_completion: Duration,
}
```

### Topological Sort Algorithm

```
1. Build adjacency list from dependencies
2. Calculate in-degree for each task
3. Initialize queue with tasks having in-degree 0
4. While queue not empty:
   - Pop task with highest priority
   - Add to sorted list
   - Decrement in-degree of dependent tasks
   - Add newly ready tasks (in-degree 0) to queue
5. Return sorted list
```

### Critical Path Algorithm

```
1. Build DAG from dependencies
2. For each task, calculate longest path to end
3. Critical path = longest path in DAG
4. Tasks on critical path have zero slack
5. Delay in critical path = delay in entire mission
```

### Load Balancing Algorithm

```
1. Get ready tasks (dependencies satisfied)
2. Get available workers
3. Sort tasks by (critical_path_length DESC, priority DESC)
4. Assign top N tasks to N workers
5. Critical path tasks get highest priority
```

---

## ✅ Success Criteria

**Sprint is done when:**

- [ ] 3 subagents complete their tasks
- [ ] Lead agent integrates all components
- [ ] `TaskQueue` compiles and works
- [ ] 15+ tests passing (5 per subagent + 10 integration)
- [ ] Priority ordering works (high priority first)
- [ ] Dependency resolution works (no cycles)
- [ ] Critical path calculation accurate
- [ ] Load balancing distributes evenly
- [ ] Documentation updated

---

## 🔗 Dependencies

**Requires:**
- Sprint B1 complete (Worker Pool needed for load balancing)

**Blocks:**
- Sprint C2 (Decomposition needs task queue)

---

## 📝 Notes for GPT-5.4

**Subagent Pattern:**
```
Lead Agent:
  ├─ Subagent 1: Priority Scheduler (parallel)
  ├─ Subagent 2: Dependency Tracker (parallel)
  └─ Subagent 3: Load Balancer (parallel)
  ↓
Lead Agent: Integration + Topological Sort + Tests
```

**Complexity Concerns:**
- DAG operations are O(V + E) where V=tasks, E=dependencies
- Topological sort must handle 1000+ tasks efficiently
- Critical path calculation is O(V * E)
- Use caching for repeated queries

**Difficulty: HIGH**
- 3 subagents to coordinate
- DAG-based scheduling (complex algorithms)
- Priority vs dependency conflicts
- Critical path calculation

**Review Checklist:**
- [ ] No cycles in dependency graph
- [ ] Priority respected within same dependency level
- [ ] Critical path accurate (test with known DAG)
- [ ] Load balancing distributes evenly
- [ ] Memory efficient (no leaks on task completion)

---

**Start AFTER Sprint B1 complete.**
