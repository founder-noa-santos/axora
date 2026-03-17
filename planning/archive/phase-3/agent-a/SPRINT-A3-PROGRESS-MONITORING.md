# Phase 3 Sprint A3: Progress Monitoring & Reporting

**Agent:** A (Documentation + Memory Specialist)  
**Sprint:** A3  
**Priority:** MEDIUM  
**Estimated:** 8 hours  
**Subagents:** ENABLED (GPT-5.4)

---

## 🎯 Mission

Implement real-time progress monitoring with ETA calculation, blocker detection, and user-facing status reports.

**Context:** Phase 2 has no progress tracking (user must manually check terminals). Phase 3 needs automatic progress reports.

---

## 📋 Subagents Assignment

**You are Lead Agent. Delegate to 2 subagents:**

### Subagent 1: Progress Tracker
**Task:** Implement progress tracking with ETA calculation
**File:** `crates/axora-agents/src/monitor/progress_tracker.rs`
**Deliverables:**
- `ProgressTracker` struct
- `track_task(task_id, status)` method
- `calculate_eta()` based on historical velocity
- `get_progress_percentage()` returns 0-100%
- 5+ tests

### Subagent 2: Blocker Detector + Reporter
**Task:** Implement blocker detection and status reporting
**File:** `crates/axora-agents/src/monitor/reporter.rs`
**Deliverables:**
- `BlockerDetector` struct
- `detect_blockers()` identifies stalled tasks
- `StatusReport` struct (progress, ETA, blockers)
- `generate_report()` for user
- 5+ tests

---

## 🏗️ Lead Agent Responsibilities

**You must:**

1. **Coordinate Subagents:**
   - Assign tasks to 2 subagents
   - Review tracker + reporter implementations
   - Ensure accurate ETA calculation

2. **Integrate Components:**
   - Create `crates/axora-agents/src/monitor.rs` (main module)
   - Combine tracker + detector + reporter
   - Export unified `ProgressMonitor` struct

3. **Implement Real-Time Updates:**
   - Subscribe to blackboard updates
   - Update progress in real-time
   - Push reports every 30 seconds

4. **Write Integration Tests:**
   - Test ETA accuracy (within 20% of actual)
   - Test blocker detection (detects stalled tasks)
   - Test report generation (all info included)

5. **Update Documentation:**
   - Add module to `crates/axora-agents/src/lib.rs`
   - Add progress monitoring examples

---

## 📐 Technical Spec

### Progress Monitor Interface

```rust
pub struct ProgressMonitor {
    tracker: ProgressTracker,
    detector: BlockerDetector,
    reporter: Reporter,
    blackboard: Arc<BlackboardV2>,
}

pub struct ProgressTracker {
    tasks: DashMap<TaskId, TaskProgress>,
    start_time: Instant,
    historical_velocity: Vec<f32>,
}

pub struct TaskProgress {
    task_id: TaskId,
    status: TaskStatus,
    started_at: Option<Instant>,
    completed_at: Option<Instant>,
}

pub enum TaskStatus {
    Pending,
    InProgress { progress: f32 },
    Completed,
    Failed { error: String },
    Blocked { reason: String },
}

pub struct StatusReport {
    pub mission_id: String,
    pub progress_percentage: f32,
    pub eta: Duration,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub blocked_tasks: Vec<BlockerInfo>,
    pub elapsed_time: Duration,
}

pub struct BlockerInfo {
    pub task_id: TaskId,
    pub reason: String,
    pub stalled_since: Duration,
}

impl ProgressMonitor {
    pub fn new(blackboard: Arc<BlackboardV2>) -> Self;
    
    pub fn update_progress(&self, task_id: TaskId, status: TaskStatus);
    
    pub fn get_report(&self) -> StatusReport;
    
    pub fn start_realtime_reporting(&self, interval: Duration);
}
```

### ETA Calculation Algorithm

```
1. Get completed tasks with durations
2. Calculate average task duration (velocity)
3. Get remaining tasks count
4. ETA = remaining_tasks * average_duration
5. Adjust for parallelism (divide by worker_count)
```

### Blocker Detection Algorithm

```
1. Get all InProgress tasks
2. For each task, check time since last update
3. If time > threshold (e.g., 5 minutes), mark as blocked
4. Investigate reason (worker dead? dependency missing?)
5. Add to blocker list in report
```

---

## ✅ Success Criteria

**Sprint is done when:**

- [ ] 2 subagents complete their tasks
- [ ] Lead agent integrates all components
- [ ] `ProgressMonitor` compiles and works
- [ ] 10+ tests passing (5 per subagent + 5 integration)
- [ ] ETA accuracy within 20% of actual
- [ ] Blocker detection works (detects stalled tasks)
- [ ] Real-time reporting works (updates every 30s)
- [ ] Documentation updated

---

## 🔗 Dependencies

**Requires:**
- Sprint A2 complete (Blackboard v2 for real-time updates)

**Blocks:**
- None (final monitoring layer)

---

## 📝 Notes for GPT-5.4

**Subagent Pattern:**
```
Lead Agent:
  ├─ Subagent 1: Progress Tracker (parallel)
  └─ Subagent 2: Blocker Detector + Reporter (parallel)
  ↓
Lead Agent: Integration + Real-Time + Tests
```

**ETA Accuracy:**
- Track historical velocity (last 10 tasks)
- Adjust for task complexity (simple vs complex)
- Account for parallelism (multiple workers)

**Review Checklist:**
- [ ] ETA updates as tasks complete
- [ ] Blockers detected within 1 minute
- [ ] Reports include all required info
- [ ] No memory leaks (old tasks cleaned up)

---

**Start AFTER Sprint A2 complete.**
