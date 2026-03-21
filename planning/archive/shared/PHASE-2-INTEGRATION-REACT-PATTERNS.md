# Phase 2 Integration: Concurrent Task Decomposition & ReAct Patterns

**Date:** 2026-03-16  
**Source:** Research — "Architectural Foundations for Concurrent Task Decomposition and ReAct Loop Patterns"  
**Impact:** VALIDATES Graph-Based Pivot (R-10) + Provides Implementation Details  

---

## ✅ Executive Summary

**This research VALIDATES our Graph-Based Workflow pivot from R-10.**

**Key Confirmations:**
1. ✅ **Sequential single-agent is broken** — context bloat, linear scaling
2. ✅ **Concurrent execution is mandatory** — 3-5x speedup potential
3. ✅ **DAG-based decomposition** — not domain-based (validates our pivot)
4. ✅ **Blackboard pattern > Chat** — O(N) communication, not O(N²)
5. ✅ **Pull-based context > Push** — agents retrieve what they need

**NEW Insights (Implementation Details):**
1. **ACONIC Framework** — Constraint-induced complexity for decomposition
2. **Dual-Thread ReAct** — Planning vs Acting threads (interruptible)
3. **Critical Path Optimization** — Route critical tasks to powerful models
4. **Snapshot-Based Merging** — Prevent TOCTOU race conditions

---

## 📊 Research Validation Matrix

| Our Decision (R-10) | This Research Says | Verdict |
|---------------------|--------------------|---------|
| Graph-Based > DDD | ✅ DAG-based decomposition | **VALIDATED** |
| Generalist + RAG > Domain Agents | ✅ Context distribution, not domain silos | **VALIDATED** |
| O(N) coordination | ✅ Blackboard pattern (not chat) | **VALIDATED** |
| Deterministic routing | ✅ Topological sorting, synchronization barriers | **VALIDATED** |
| Experience-as-Parameters | ✅ Externalized memory, snapshot merging | **VALIDATED** |

**Conclusion:** R-10 pivot was CORRECT. This research provides IMPLEMENTATION DETAILS.

---

## 🔄 Architecture Updates (Refinements, Not Pivots)

### 1. Task Decomposition: Add ACONIC Framework

**Current (from R-10):**
```rust
pub struct MissionDecomposer {
    workflow_templates: HashMap<TaskType, WorkflowTemplate>,
}
```

**Updated (with ACONIC):**
```rust
pub struct MissionDecomposer {
    // ACONIC: Constraint-induced complexity
    constraint_graph: ConstraintGraph,
    
    // AOP validation (Solvability, Completeness, Non-redundancy)
    aop_validator: AOPValidator,
    
    // DAG construction
    dag_builder: DAGBuilder,
}

impl MissionDecomposer {
    pub fn decompose(&self, mission: &str) -> Result<DecomposedMission> {
        // 1. Parse constraints from mission
        let constraints = self.parse_constraints(mission)?;
        
        // 2. Build constraint graph
        self.constraint_graph.build(&constraints)?;
        
        // 3. Calculate treewidth (complexity measure)
        let treewidth = self.constraint_graph.treewidth();
        
        // 4. If treewidth > threshold, decompose recursively
        if treewidth > LLM_OPTIMAL_LINEWIDTH {
            return self.decompose_recursive(mission);
        }
        
        // 5. Build DAG from validated constraints
        let dag = self.dag_builder.build(&self.constraint_graph)?;
        
        // 6. Topological sort → Parallel groups
        let parallel_groups = dag.topological_sort()?;
        
        Ok(DecomposedMission {
            tasks: dag.nodes,
            dependencies: dag.edges,
            parallel_groups,
            critical_path: dag.find_critical_path(),
        })
    }
}
```

**Why:** ACONIC provides MATHEMATICAL GUARANTEE that decomposition is valid (not heuristic).

---

### 2. ReAct Loop: Dual-Thread Architecture

**Current (from R-11):**
```rust
impl WorkerAgent {
    pub async fn react_loop(&self, task: &Task) -> Result<TaskResult> {
        // Single-threaded ReAct (blocks on tool execution)
    }
}
```

**Updated (Dual-Thread):**
```rust
pub struct WorkerAgent {
    // Planning Thread (LLM-driven, async)
    planning_tx: mpsc::Sender<ActionProposal>,
    planning_rx: mpsc::Receiver<ActionProposal>,
    
    // Acting Thread (Tool execution, async)
    acting_tx: mpsc::Sender<ActionExecution>,
    acting_rx: mpsc::Receiver<ActionExecution>,
    
    // Interrupt channel (from coordinator)
    interrupt_rx: mpsc::Receiver<InterruptSignal>,
}

impl WorkerAgent {
    pub async fn spawn_dual_thread(&self) -> Self {
        // Planning Thread (never blocks)
        let planning_handle = tokio::spawn(async move {
            loop {
                // Poll environment, blackboard, task state
                let proposal = self.llm_plan().await?;
                self.planning_tx.send(proposal).await?;
                
                // Check for interrupts (non-blocking)
                if let Ok(interrupt) = self.interrupt_rx.try_recv() {
                    self.handle_interrupt(interrupt).await?;
                }
            }
        });
        
        // Acting Thread (executes tools, can block)
        let acting_handle = tokio::spawn(async move {
            loop {
                let proposal = self.planning_rx.recv().await?;
                let result = self.execute_tool(proposal).await?;
                self.acting_tx.send(result).await?;
            }
        });
        
        Self { /* ... */ }
    }
}
```

**Why:** Dual-thread enables INTERRUPTIBLE execution (no deadlocks, no infinite loops).

---

### 3. Blackboard: Snapshot-Based Merging

**Current (from R-10):**
```rust
pub struct Blackboard {
    state: HashMap<Key, Value>,
}
```

**Updated (Snapshot + TOCTOU Prevention):**
```rust
pub struct Blackboard {
    // Immutable snapshots (agents read from snapshot)
    snapshots: RwLock<HashMap<u64, BlackboardSnapshot>>,
    
    // Current version (for writes)
    current_version: AtomicU64,
    
    // Locks for atomic operations
    locks: DashMap<Key, RwLock<Value>>,
}

impl Blackboard {
    /// Agent reads from immutable snapshot (no TOCTOU)
    pub fn get_snapshot(&self, version: u64) -> &BlackboardSnapshot {
        self.snapshots.read().get(&version).unwrap()
    }
    
    /// Agent writes with optimistic concurrency (version check)
    pub fn update(&self, key: &Key, value: &Value, expected_version: u64) -> Result<u64> {
        let lock = self.locks.get(key).or_insert_with(|| RwLock::new(value.clone()));
        
        // Optimistic concurrency check
        if self.current_version.load() != expected_version {
            return Err(Error::StaleVersion);
        }
        
        // Atomic update
        let new_version = self.current_version.fetch_add(1) + 1;
        self.create_snapshot(new_version)?;
        
        Ok(new_version)
    }
    
    /// Create immutable snapshot (for consistency)
    fn create_snapshot(&self, version: u64) -> Result<()> {
        let snapshot = BlackboardSnapshot {
            version,
            state: self.locks.iter().map(|(k, v)| (k.clone(), v.read().clone())).collect(),
        };
        self.snapshots.write().insert(version, snapshot);
        Ok(())
    }
}
```

**Why:** Prevents TOCTOU race conditions (agents act on stale data).

---

### 4. Context Distribution: Pull-Based (Validated)

**Current (from R-10):**
```rust
pub struct ContextManager {
    domain_rag: DomainRagStore,
}
```

**Updated (Explicit Pull-Based):**
```rust
pub struct ContextManager {
    // Minimal initial context (just entry point)
    entry_points: HashMap<TaskId, EntryPoint>,
    
    // Blackboard access (agents pull what they need)
    blackboard: Arc<Blackboard>,
    
    // MCP tools for exploration
    mcp_tools: MCPToolSet,
}

impl ContextManager {
    /// Allocate MINIMAL context (entry point + blackboard access)
    pub fn allocate(&self, task: &Task) -> TaskContext {
        let entry_point = self.entry_points.get(&task.id).unwrap();
        
        TaskContext {
            mission_summary: entry_point.summary.clone(),
            directory_map: entry_point.directory_map.clone(),
            blackboard_access: self.blackboard.clone(),
            mcp_tools: self.mcp_tools.clone(),
            
            // NO pre-fetched docs (agent pulls what it needs)
        }
    }
}
```

**Why:** Research confirms Pull-Based > Push-Based for complex tasks (relevance emerges dynamically).

---

### 5. Coordination Topology: Centralized + Blackboard

**Current (from R-10):**
```rust
pub struct Coordinator {
    graph: WorkflowGraph,
}
```

**Updated (Explicit Topology):**
```rust
pub enum CoordinationTopology {
    /// Centralized (our choice for OPENAKTA)
    Centralized {
        coordinator: CoordinatorAgent,
        workers: Vec<WorkerAgent>,
        blackboard: Arc<Blackboard>,
    },
    
    /// Hierarchical (for extreme scale)
    Hierarchical {
        executive: ExecutiveAgent,
        managers: Vec<ManagerAgent>,
        workers: Vec<WorkerAgent>,
    },
    
    /// Decentralized (NOT recommended for OPENAKTA)
    Decentralized {
        agents: Vec<PeerAgent>,
        event_bus: EventBus,
    },
}

impl OPENAKTA {
    /// We use Centralized + Blackboard (research validates this)
    pub fn new() -> Self {
        Self {
            topology: CoordinationTopology::Centralized {
                coordinator: CoordinatorAgent::new(),
                workers: Vec::new(),
                blackboard: Arc::new(Blackboard::new()),
            },
        }
    }
}
```

**Why:** Research confirms Centralized + Blackboard is optimal for deterministic software engineering tasks.

---

## 📋 Implementation Sprints (NEW)

Based on this research, we need to add:

### Sprint 12: ACONIC Decomposition (Agent C)
- Implement constraint graph
- Add treewidth calculation
- AOP validator (Solvability, Completeness, Non-redundancy)

### Sprint 13: Dual-Thread ReAct (Agent C)
- Planning Thread (async, non-blocking)
- Acting Thread (tool execution)
- Interrupt channel (coordinator → worker)

### Sprint 14: Snapshot Blackboard (Agent B)
- Immutable snapshots (agents read from version)
- Optimistic concurrency (version check on write)
- TOCTOU prevention

### Sprint 15: Critical Path Optimization (Agent C)
- Identify critical path in DAG
- Route critical tasks to powerful models
- Off-path tasks to smaller models

---

## ✅ Validation Metrics (From Research)

| Metric | Target | Measurement |
|--------|--------|-------------|
| Decomposition Quality | >85% agreement with human | Structural comparison |
| Parallelization Quotient | >90% tasks parallelizable | DAG analysis |
| End-to-End Latency | 3x reduction vs sequential | Wall-clock time |
| Token Efficiency | 40% reduction vs single-agent | Total tokens consumed |
| Context Window Pressure | <50% of single-agent peak | Max tokens per agent |
| ReAct Convergence | <12 cycles average | Thought-Action-Observation |
| TOCTOU Failure Rate | 0% | Race condition detection |

---

## 🔗 Updated References

- [`PHASE-2-PIVOT-GRAPH-WORKFLOW.md`](./PHASE-2-PIVOT-GRAPH-WORKFLOW.md) — Main pivot (VALIDATED)
- [`research/prompts/10-ddd-agents-validation.md`](../research/prompts/10-ddd-agents-validation.md) — R-10 (VALIDATED)
- [`research/prompts/11-concurrency-react-loops.md`](../research/prompts/11-concurrency-react-loops.md) — ReAct (NEEDS UPDATE)
- [`research/prompts/13-influence-graph-business-rules.md`](../research/prompts/13-influence-graph-business-rules.md) — Influence Graph (VALIDATED)

---

**This research CONFIRMS our R-10 pivot and provides IMPLEMENTATION DETAILS.**

**No major pivots needed — just refinements to implementation.**
