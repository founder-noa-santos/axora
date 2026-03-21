# Agent C — Sprint 9: Dual-Thread ReAct Implementation

**Phase:** 2  
**Sprint:** 9 (Implementation)  
**File:** `crates/openakta-agents/src/react.rs` + `crates/openakta-agents/src/coordinator.rs`  
**Priority:** CRITICAL (core execution engine)  
**Estimated Tokens:** ~150K output  

---

## 🎯 Task

Implement **Dual-Thread ReAct Loop** with interruptible execution (from concurrent task decomposition research).

### Context

Research validates our Graph-Based pivot and adds CRITICAL improvements:
- **Dual-Thread Architecture** — Planning (async) + Acting (tool execution)
- **Interruptible Execution** — No deadlocks, no infinite loops
- **Coordinator-Driven** — Centralized orchestration with Blackboard

**Your job:** Implement this ReAct engine using Agent A's ACONIC docs + Agent B's Blackboard.

---

## 📋 Deliverables

### 1. Create react.rs (Dual-Thread ReAct)

**File:** `crates/openakta-agents/src/react.rs`

**Core Structure:**
```rust
//! Dual-Thread ReAct Loop
//!
//! This module implements interruptible ReAct execution:
//! - Planning Thread (async, non-blocking, LLM-driven)
//! - Acting Thread (tool execution, can block)
//! - Interrupt Channel (coordinator → worker)

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use serde::{Deserialize, Serialize};

/// ReAct cycle (Thought → Action → Observation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactCycle {
    pub thought: String,
    pub action: Action,
    pub observation: Observation,
    pub cycle_number: u32,
}

/// Action (tool call)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub tool_name: String,
    pub parameters: serde_json::Value,
}

/// Observation (tool result)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub success: bool,
    pub result: serde_json::Value,
    pub error: Option<String>,
}

/// Interrupt signal (from coordinator)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterruptSignal {
    /// Stop current action (state changed)
    Stop { reason: String },
    
    /// Priority change (new task)
    PriorityChange { new_priority: u32 },
    
    /// Context update (blackboard changed)
    ContextUpdate { new_snapshot_version: u64 },
}

/// Dual-Thread ReAct Agent
pub struct DualThreadReactAgent {
    // Planning Thread (LLM-driven, async)
    planning_tx: mpsc::Sender<ActionProposal>,
    planning_rx: mpsc::Receiver<ActionProposal>,
    planning_handle: Option<JoinHandle<Result<()>>>,
    
    // Acting Thread (tool execution)
    acting_tx: mpsc::Sender<ActionExecution>,
    acting_rx: mpsc::Receiver<ActionExecution>,
    acting_handle: Option<JoinHandle<Result<()>>>,
    
    // Interrupt channel (from coordinator)
    interrupt_tx: mpsc::Sender<InterruptSignal>,
    interrupt_rx: mpsc::Receiver<InterruptSignal>,
    
    // State
    current_cycle: u32,
    max_cycles: u32,
}

impl DualThreadReactAgent {
    /// Spawn dual-thread agent
    pub async fn spawn(
        task: Task,
        blackboard: Arc<Blackboard>,
        tools: ToolSet,
    ) -> Result<Self> {
        // Create channels
        let (planning_tx, planning_rx) = mpsc::channel(32);
        let (acting_tx, acting_rx) = mpsc::channel(32);
        let (interrupt_tx, interrupt_rx) = mpsc::channel(32);
        
        // Spawn Planning Thread (async, non-blocking)
        let planning_handle = tokio::spawn({
            let planning_tx = planning_tx.clone();
            let interrupt_rx = interrupt_rx.clone();
            async move {
                Self::planning_thread(task, blackboard.clone(), planning_tx, interrupt_rx).await
            }
        });
        
        // Spawn Acting Thread (tool execution)
        let acting_handle = tokio::spawn({
            let acting_tx = acting_tx.clone();
            async move {
                Self::acting_thread(acting_tx, acting_rx, tools).await
            }
        });
        
        Ok(Self {
            planning_tx,
            planning_rx,
            planning_handle: Some(planning_handle),
            acting_tx,
            acting_rx,
            acting_handle: Some(acting_handle),
            interrupt_tx,
            interrupt_rx,
            current_cycle: 0,
            max_cycles: 12, // Research target: <12 cycles average
        })
    }
    
    /// Planning Thread (LLM-driven, never blocks)
    async fn planning_thread(
        task: Task,
        blackboard: Arc<Blackboard>,
        planning_tx: mpsc::Sender<ActionProposal>,
        interrupt_rx: mpsc::Receiver<InterruptSignal>,
    ) -> Result<()> {
        let mut cycle = 0;
        
        loop {
            // Check for interrupts (non-blocking)
            if let Ok(interrupt) = interrupt_rx.try_recv() {
                // Handle interrupt (reflection phase)
                Self::handle_interrupt(&interrupt, &blackboard).await?;
                continue;
            }
            
            // Get current snapshot (immutable, no TOCTOU)
            let snapshot_version = blackboard.get_current_version();
            let snapshot = blackboard.get_snapshot(snapshot_version).unwrap();
            
            // LLM generates thought + action proposal
            let (thought, action) = Self::llm_plan(&task, &snapshot).await?;
            
            // Send action proposal to Acting Thread
            planning_tx.send(ActionProposal {
                thought,
                action,
                snapshot_version,
            }).await?;
            
            cycle += 1;
            
            // Check cycle limit (prevent infinite loops)
            if cycle > 12 {
                return Err(Error::MaxCyclesExceeded);
            }
        }
    }
    
    /// Acting Thread (tool execution, can block)
    async fn acting_thread(
        acting_tx: mpsc::Sender<ActionExecution>,
        acting_rx: mpsc::Receiver<ActionExecution>,
        tools: ToolSet,
    ) -> Result<()> {
        loop {
            // Wait for action proposal from Planning Thread
            let proposal = acting_rx.recv().await?;
            
            // Execute tool (can block)
            let observation = tools.execute(&proposal.action).await?;
            
            // Send observation back to Planning Thread
            acting_tx.send(ActionExecution {
                proposal,
                observation,
            }).await?;
        }
    }
    
    /// Handle interrupt (reflection phase)
    async fn handle_interrupt(
        interrupt: &InterruptSignal,
        blackboard: Arc<Blackboard>,
    ) -> Result<()> {
        match interrupt {
            InterruptSignal::Stop { reason } => {
                // Flush pending action, stop
                tracing::warn!("Interrupt received: {}", reason);
            }
            
            InterruptSignal::PriorityChange { new_priority } => {
                // Re-prioritize task
                tracing::info!("Priority changed to {}", new_priority);
            }
            
            InterruptSignal::ContextUpdate { new_snapshot_version } => {
                // Force reflection phase (merge new context)
                let reflection = blackboard.create_reflection_phase(
                    *new_snapshot_version,
                    serde_json::Value::Null, // pending action
                )?;
                
                // LLM merges new context
                Self::llm_reflect(&reflection).await?;
            }
        }
        
        Ok(())
    }
}
```

---

### 2. Create coordinator.rs (Centralized Orchestrator)

**File:** `crates/openakta-agents/src/coordinator.rs`

**Core Structure:**
```rust
//! Centralized Coordinator (Orchestrator-Worker Topology)
//!
//! This module implements the Centralized Coordinator pattern:
//! - Maintains DAG (from ACONIC decomposer)
//! - Spawns workers for parallel groups
//! - Enforces synchronization barriers

use crate::react::DualThreadReactAgent;
use crate::blackboard::Blackboard;

/// Centralized Coordinator
pub struct Coordinator {
    // DAG (from ACONIC decomposer)
    dag: DAG,
    
    // Blackboard (shared state)
    blackboard: Arc<Blackboard>,
    
    // Active workers
    workers: HashMap<TaskId, DualThreadReactAgent>,
    
    // Synchronization
    barrier: Barrier,
}

impl Coordinator {
    /// Execute mission (DAG-based)
    pub async fn execute_mission(&mut self, mission: DecomposedMission) -> Result<MissionResult> {
        // Execute parallel groups sequentially
        for (group_idx, group) in mission.parallel_groups.iter().enumerate() {
            tracing::info!("Executing parallel group {} ({} tasks)", group_idx, group.len());
            
            // Spawn all workers in group concurrently
            let mut handles = Vec::new();
            
            for task_id in group {
                let task = mission.tasks.get(task_id).unwrap();
                let blackboard = self.blackboard.clone();
                let tools = self.get_tools_for_task(task)?;
                
                // Spawn worker (dual-thread ReAct)
                let handle = tokio::spawn(async move {
                    let mut worker = DualThreadReactAgent::spawn(task, blackboard, tools).await?;
                    worker.execute().await
                });
                
                handles.push(handle);
            }
            
            // Synchronization barrier (wait for all workers)
            let results = futures::future::join_all(handles).await;
            
            // Check for failures
            for (task_id, result) in group.iter().zip(results) {
                match result {
                    Ok(Ok(task_result)) => {
                        // Success → write to blackboard
                        self.blackboard.merge_result(
                            task_result.updates,
                            task_result.base_version,
                        )?;
                    }
                    Ok(Err(e)) => {
                        // Task failed → retry or escalate
                        tracing::error!("Task {} failed: {}", task_id, e);
                        // Handle retry logic
                    }
                    Err(e) => {
                        // Worker panicked
                        tracing::error!("Worker for task {} panicked: {}", task_id, e);
                    }
                }
            }
            
            // Barrier complete → next group
            self.barrier.arrive_and_wait().await;
        }
        
        Ok(MissionResult {
            success: true,
            total_time: self.start_time.elapsed(),
        })
    }
    
    /// Send interrupt to worker (for context updates)
    pub fn interrupt_worker(&self, task_id: TaskId, signal: InterruptSignal) -> Result<()> {
        let worker = self.workers.get(&task_id).unwrap();
        worker.send_interrupt(signal)?;
        Ok(())
    }
}
```

---

### 3. Implement Critical Path Optimization

**File:** `crates/openakta-agents/src/coordinator.rs` (add to existing)

```rust
impl Coordinator {
    /// Route critical path tasks to powerful models
    fn get_tools_for_task(&self, task: &Task) -> Result<ToolSet> {
        let is_critical = self.dag.critical_path.contains(&task.id);
        
        if is_critical {
            // Critical path → powerful model (frontier LLM)
            tracing::info!("Task {} is on critical path → routing to powerful model", task.id);
            Ok(ToolSet::with_powerful_llm())
        } else {
            // Off-path → smaller/faster model (SLM)
            tracing::info!("Task {} is off critical path → routing to SLM", task.id);
            Ok(ToolSet::with_small_llm())
        }
    }
}
```

---

## 📁 File Boundaries

**Create:**
- `crates/openakta-agents/src/react.rs` (NEW)
- `crates/openakta-agents/src/coordinator.rs` (NEW)

**Update:**
- `crates/openakta-agents/src/lib.rs` (add module exports)

**DO NOT Edit:**
- `crates/openakta-cache/` (Agent B's domain)
- `crates/openakta-docs/` (Agent A's domain)

**Dependencies:**
- Agent A's ACONIC docs (for DAG structure)
- Agent B's Blackboard (for shared state)

---

## 🧪 Tests Required

```rust
#[test]
fn test_dual_thread_spawn() { }

#[test]
fn test_planning_thread_non_blocking() { }

#[test]
fn test_acting_thread_tool_execution() { }

#[test]
fn test_interrupt_handling() { }

#[test]
fn test_reflection_phase() { }

#[test]
fn test_max_cycles_prevention() { }

#[test]
fn test_coordinator_parallel_execution() { }

#[test]
fn test_synchronization_barrier() { }

#[test]
fn test_critical_path_routing() { }

#[test]
fn test_full_mission_execution() { }
```

---

## ✅ Success Criteria

- [ ] `react.rs` created (dual-thread ReAct)
- [ ] `coordinator.rs` created (centralized orchestrator)
- [ ] Planning Thread is non-blocking (async)
- [ ] Acting Thread executes tools (can block)
- [ ] Interrupt handling works (reflection phase)
- [ ] Max cycles prevention (<12 cycles average)
- [ ] Synchronization barriers work
- [ ] Critical path routing works (powerful vs SLM)
- [ ] 10+ tests passing

---

## 🔗 References

- [`AGENT-A-SPRINT-12.md`](../agent-a/AGENT-A-SPRINT-12.md) — ACONIC docs (dependency)
- [`AGENT-B-SPRINT-12.md`](../agent-b/AGENT-B-SPRINT-12.md) — Blackboard (dependency)
- [`PHASE-2-INTEGRATION-REACT-PATTERNS.md`](../shared/PHASE-2-INTEGRATION-REACT-PATTERNS.md) — Integration doc
- Research document — Dual-thread ReAct spec

---

**Start AFTER Agent A and Agent B complete Sprint 12.**

**Priority: CRITICAL — this is the core execution engine.**

**Dependencies:**
- Agent A: ACONIC-DECOMPOSITION-DESIGN.md
- Agent B: Blackboard implementation
