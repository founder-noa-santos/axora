# R-11: Concurrent Task Decomposition & ReAct Loop Patterns

**Priority:** 🔴 CRITICAL (Foundational for Multi-Agent Coordination)  
**Status:** 📋 Research Prompt Ready  
**Estimated Research Time:** 3-4 hours  

---

## Context & Motivation

**Problem:** When a user gives AXORA a complex mission (e.g., "implement authentication system"), we currently:
- ❌ Assign to a single agent
- ❌ Agent works sequentially on everything
- ❌ Context window fills up quickly
- ❌ No parallelization
- ❌ Slow execution

**Goal:** Automatically decompose missions into concurrent subtasks:
- ✅ Break mission into independent tasks
- ✅ Assign to multiple agents
- ✅ Agents work in parallel
- ✅ Coordinate results
- ✅ Faster execution, less context per agent

---

## Core Research Questions

### 1. Task Decomposition Strategies

**Questions:**
- How to automatically break down complex missions?
- What makes tasks **independent** (can run concurrently)?
- What makes tasks **dependent** (must run sequentially)?
- How to identify **critical path** vs parallelizable work?

**Sub-questions:**
- Should decomposition be **rule-based** (patterns) or **LLM-based** (dynamic)?
- How to handle **cross-task dependencies**?
- What's the right **granularity** (fine vs coarse tasks)?

---

### 2. ReAct Loop Patterns

**Background:** ReAct (Reason + Act) is a proven pattern for LLM agents:
```
Thought → Action → Observation → Thought → Action → ...
```

**Questions:**
- How to adapt ReAct for **multi-agent** scenarios?
- Should each agent have its own ReAct loop?
- Or should there be a **coordinator ReAct** loop?
- How to handle **inter-agent communication** within ReAct?

**Sub-questions:**
- What's the **state** in multi-agent ReAct?
- How to **share observations** between agents?
- When should agents **synchronize** vs work independently?

---

### 3. Mental Organization for Concurrency

**Questions:**
- How to structure agent "thinking" for parallel work?
- What data structures represent **task graphs**?
- How to track **progress** across concurrent tasks?
- How to detect **blockers** and **dependencies**?

**Sub-questions:**
- Should agents maintain **local state** only?
- Or should there be **shared state** (blackboard)?
- How to prevent **race conditions** in agent reasoning?

---

### 4. Context Distribution

**Questions:**
- How to split context across multiple agents?
- What context is **shared** vs **task-specific**?
- How to prevent **context duplication**?
- How to ensure agents have **just enough context**?

**Sub-questions:**
- Should context be **pushed** to agents or **pulled** on demand?
- How to handle **context updates** during execution?
- What's the **cost tradeoff** (more agents vs more context per agent)?

---

### 5. Coordination Patterns

**Questions:**
- What coordination patterns work for multi-agent concurrency?
- **Centralized coordinator** vs **decentralized consensus**?
- How to handle **conflicts** between agents?
- When to **synchronize** vs let agents work independently?

**Sub-questions:**
- Should coordination be **explicit** (messages) or **implicit** (shared state)?
- How frequent should **check-ins** be?
- What's the **overhead** of coordination?

---

## 🔬 Academic Literature Review

### Search Terms

1. **Multi-Agent Task Decomposition**
   - "multi-agent task decomposition"
   - "automated task breakdown multi-agent systems"
   - "parallel task allocation agents"

2. **ReAct + Multi-Agent**
   - "ReAct multi-agent systems"
   - "reasoning acting agent teams"
   - "iterative reasoning multi-agent"

3. **Task Planning + LLM**
   - "LLM task planning decomposition"
   - "chain of thought task breakdown"
   - "tree of thoughts multi-agent"

4. **Concurrency + Coordination**
   - "concurrent agent coordination"
   - "parallel agent execution"
   - "multi-agent synchronization"

### Key Venues

- **AAMAS** (Autonomous Agents and Multiagent Systems)
- **IJCAI** (Artificial Intelligence)
- **ICSE** (Software Engineering)
- **NeurIPS** (Machine Learning)
- **arXiv** (cs.MA, cs.AI, cs.SE)

---

## 🏭 Industry Patterns

### Existing Approaches

| Framework | Task Decomposition | Concurrency | ReAct | Gaps |
|-----------|-------------------|-------------|-------|------|
| **AutoGen** | ⚠️ Manual | ⚠️ Limited | ❌ No | No auto-decomposition |
| **CrewAI** | ⚠️ Sequential | ❌ No | ❌ No | Sequential only |
| **LangGraph** | ✅ State-based | ⚠️ Limited | ⚠️ Partial | Complex setup |
| **OpenDevin** | ⚠️ LLM-based | ⚠️ Limited | ✅ Yes | Early stage |
| **Devika** | ⚠️ LLM-based | ❌ No | ✅ Yes | Single-agent focus |

**Opportunity:** First framework with **automatic concurrent decomposition** + **ReAct loops**.

---

## 💡 Proposed AXORA Patterns

### Pattern 1: Mission Breakdown

```rust
pub struct MissionDecomposer {
    llm: LLM,
    rules: Vec<DecompositionRule>,
}

pub struct DecomposedMission {
    tasks: Vec<Task>,
    dependencies: Vec<Dependency>,
    critical_path: Vec<TaskId>,
    parallel_groups: Vec<Vec<TaskId>>,
}

impl MissionDecomposer {
    pub fn decompose(&self, mission: &Mission) -> Result<DecomposedMission> {
        // 1. LLM suggests breakdown
        let suggestions = self.llm.decompose(mission)?;
        
        // 2. Apply rules to validate
        let validated = self.apply_rules(suggestions)?;
        
        // 3. Identify dependencies
        let deps = self.identify_dependencies(validated)?;
        
        // 4. Group into parallel sets
        let groups = self.topological_sort(deps)?;
        
        Ok(DecomposedMission {
            tasks: validated,
            dependencies: deps,
            critical_path: self.find_critical_path(validated, deps)?,
            parallel_groups: groups,
        })
    }
}
```

**Example:**
```
Mission: "Implement authentication system"

Decomposition:
├─ Parallel Group 1 (can run concurrently)
│  ├─ Task 1.1: Design database schema (Agent: Architect)
│  ├─ Task 1.2: Research auth best practices (Agent: Researcher)
│  └─ Task 1.3: Set up project structure (Agent: Coder)
│
├─ Parallel Group 2 (depends on Group 1)
│  ├─ Task 2.1: Implement user model (Agent: Coder)
│  ├─ Task 2.2: Implement JWT utilities (Agent: Coder)
│  └─ Task 2.3: Write auth tests (Agent: Tester)
│
└─ Parallel Group 3 (depends on Group 2)
   ├─ Task 3.1: Implement login endpoint (Agent: Coder)
   ├─ Task 3.2: Implement signup endpoint (Agent: Coder)
   └─ Task 3.3: Integration tests (Agent: Tester)
```

---

### Pattern 2: Multi-Agent ReAct Loop

```rust
pub struct MultiAgentReAct {
    coordinator: CoordinatorAgent,
    workers: Vec<WorkerAgent>,
    shared_state: SharedState,
}

pub struct ReActCycle {
    thought: String,
    action: Action,
    observation: Observation,
}

impl MultiAgentReAct {
    pub async fn execute(&mut self, mission: &Mission) -> Result<()> {
        // Decompose mission
        let decomposed = self.decomposer.decompose(mission)?;
        
        // Execute parallel groups
        for group in decomposed.parallel_groups {
            // Start all tasks in group concurrently
            let mut handles = Vec::new();
            
            for task_id in group {
                let task = decomposed.tasks.get(task_id).unwrap();
                let agent = self.select_agent(task);
                
                // Start agent's ReAct loop
                let handle = tokio::spawn(async move {
                    agent.react_loop(task).await
                });
                
                handles.push(handle);
            }
            
            // Wait for all tasks in group to complete
            let results = futures::future::join_all(handles).await;
            
            // Check for failures
            self.handle_failures(results)?;
            
            // Update shared state
            self.shared_state.update(results)?;
        }
        
        Ok(())
    }
}

impl WorkerAgent {
    pub async fn react_loop(&self, task: &Task) -> Result<TaskResult> {
        let mut state = ReActState::new(task);
        
        loop {
            // Thought: Reason about current state
            let thought = self.llm.think(&state)?;
            
            // Action: Decide what to do
            let action = self.llm.decide_action(&state, &thought)?;
            
            // Execute action
            let observation = self.execute_action(action).await?;
            
            // Update state
            state.add_cycle(thought, action, observation);
            
            // Check if task is complete
            if self.is_complete(&state)? {
                return Ok(state.finalize());
            }
            
            // Check if stuck (too many cycles)
            if state.cycle_count > MAX_CYCLES {
                return Err(Error::StuckInLoop);
            }
        }
    }
}
```

**Key Features:**
- Each agent has **own ReAct loop** (local reasoning)
- **Shared state** for observations (global awareness)
- **Coordinator** manages parallel groups
- **Synchronization points** between groups

---

### Pattern 3: Context Distribution

```rust
pub struct ContextManager {
    shared_context: SharedContext,
    task_contexts: HashMap<TaskId, TaskContext>,
}

pub struct TaskContext {
    required_docs: Vec<DocId>,
    required_code: Vec<FileId>,
    related_tasks: Vec<TaskId>,
    agent_state: AgentState,
}

impl ContextManager {
    pub fn allocate(&mut self, task: &Task, agent: &Agent) -> TaskContext {
        // Minimal context for this specific task
        let mut ctx = TaskContext::new();
        
        // Add task-specific code files
        ctx.required_code.extend(task.affected_files());
        
        // Add relevant documentation
        ctx.required_docs.extend(self.find_relevant_docs(task));
        
        // Add related task results (if dependent)
        for dep in task.dependencies {
            ctx.related_tasks.push(dep);
            ctx.merge(self.task_contexts.get(dep).unwrap());
        }
        
        ctx
    }
}
```

**Principle:** Each agent gets **minimal context** needed for its task, not full mission context.

---

## 🧪 Validation Experiments

### Experiment 1: Decomposition Quality

**Setup:**
- 10 complex missions (e.g., "implement auth", "add payment system")
- **Method A:** Manual decomposition (human)
- **Method B:** LLM-based decomposition (AXORA)

**Metrics:**
- Task count (more = finer granularity)
- Parallelization % (tasks that can run concurrently)
- Dependency accuracy (vs human baseline)
- User preference (which breakdown do users prefer?)

**Hypothesis:** LLM decomposition achieves **80%+ agreement** with human.

---

### Experiment 2: Concurrent vs Sequential

**Setup:**
- 5 missions with clear parallel subtasks
- **Method A:** Single agent, sequential execution
- **Method B:** Multiple agents, concurrent execution

**Metrics:**
- Total execution time
- Token usage (total across all agents)
- Context per agent (avg tokens)
- Success rate (% missions completed correctly)

**Hypothesis:** Concurrent execution is **3-5x faster** with **50% less context per agent**.

---

### Experiment 3: ReAct Loop Effectiveness

**Setup:**
- 10 tasks requiring iterative reasoning
- **Method A:** Direct execution (no ReAct)
- **Method B:** ReAct loop (Thought → Action → Observation)

**Metrics:**
- Success rate
- Cycles to completion (avg)
- Error recovery (how often ReAct corrects mistakes)
- User satisfaction

**Hypothesis:** ReAct achieves **20% higher success rate** on complex tasks.

---

## 📊 Success Criteria

Concurrent task decomposition is successful when:

1. ✅ **Decomposition accuracy** >80% (vs human baseline)
2. ✅ **Parallelization** >50% of tasks can run concurrently
3. ✅ **Execution speedup** 3-5x vs sequential
4. ✅ **Context reduction** 50% less per agent
5. ✅ **Success rate** >90% for decomposed missions
6. ✅ **ReAct loops** converge in <10 cycles (avg)

---

## 🎯 Research Plan

### Phase 1: Literature Review (1.5 hours)
- [ ] Search AAMAS, IJCAI, NeurIPS proceedings
- [ ] Review ReAct papers (Yao et al.)
- [ ] Summarize findings

### Phase 2: Industry Analysis (1 hour)
- [ ] Deep-dive into AutoGen, CrewAI, LangGraph
- [ ] Analyze OpenDevin, Devika approaches
- [ ] Identify gaps

### Phase 3: Pattern Design (1 hour)
- [ ] Mission decomposition algorithm
- [ ] Multi-agent ReAct loop design
- [ ] Context distribution strategy

### Phase 4: Implementation Plan (30 min)
- [ ] Break down into sprints
- [ ] Estimate effort
- [ ] Identify risks

---

## 📋 Expected Deliverables

1. **Research Findings** (`research/findings/concurrency/R-11-result.md`)
   - Literature review summary
   - Industry analysis
   - Recommended patterns

2. **Architecture Design** (`docs/CONCURRENCY-ARCHITECTURE.md`)
   - Mission decomposer
   - Multi-agent ReAct loop
   - Context distribution

3. **Implementation Plan** (new sprints added to phase plan)
   - Sprint 7: Mission Decomposition
   - Sprint 8: Multi-Agent ReAct
   - Sprint 9: Context Distribution

---

## 🔗 Related Research

- [R-02: Inter-Agent Communication](./prompts/02-inter-agent-communication.md) — Message passing
- [R-06: Agent Architecture](./prompts/06-agent-architecture-orchestration.md) — Orchestration patterns
- [R-09: Documentation Management](./prompts/09-documentation-management.md) — Context for docs
- [HEARTBEAT-REANALYSIS.md](../planning/HEARTBEAT-REANALYSIS.md) — Agent lifecycle

---

**Ready to execute this research.** This will provide the foundation for **concurrent multi-agent execution** in AXORA.
