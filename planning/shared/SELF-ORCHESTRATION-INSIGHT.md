# AXORA Meta-Insight: Self-Orchestrating Agent System

**Date:** 2026-03-16  
**Priority:** 🔴 CRITICAL (Core Differentiator)  
**Source:** User feedback during Wave 1 execution

---

## 🎯 Problem Identified

**Current Workflow (Manual Orchestration):**

```
User → Creates prompt for Agent A → Copy/Paste → Terminal 1
     → Waits → Checks status → Creates prompt for Agent B → Copy/Paste → Terminal 2
     → Waits → Checks status → Creates prompt for Agent C → Copy/Paste → Terminal 3
     → Waits → Validates → Merges → Creates next prompts...
     
User Role: Babysitter + Pigeon Courier
```

**Pain Points:**
- ❌ User must **manually check** each agent's status
- ❌ User must **create prompts** for each agent
- ❌ User must **copy/paste** between terminals
- ❌ User must **validate** each sprint completion
- ❌ User must **coordinate** dependencies between agents
- ❌ **Zero automation** — user does all orchestration

**User Quote:**
> "Isso seria algo que poderia ser automatizado... eu tenho que gerenciar tudo, ficar te falando para ir ver se ele terminou, para tu criar o proximo prompt, eu copiar e mandar para ele... isso seria algo que poderia ser automatizado com eu tendo uma sessao que converso com um agente principal, e ainda sim ter acesso as outras sessoes mas tu poder controlar elas ao inves de que eu tenha que ser babysitter de todo mundo, e pombo correio."

---

## ✨ Solution Vision: Self-Orchestrating AXORA

**Desired Workflow (Automated Orchestration):**

```
User → Main Agent (Coordinator) → "Implement Phase 2"
     │
     ├─→ Auto-dispatches Agent A (Sprint 3)
     ├─→ Auto-dispatches Agent B (Sprint 5)
     ├─→ Auto-dispatches Agent C (Sprint 3b)
     │
     ├─→ Monitors progress (no user intervention)
     ├─→ Validates completions (auto-tests)
     ├─→ Creates next prompts (auto-generated)
     ├─→ Handles dependencies (auto-coordination)
     │
     └─→ Reports to user: "Phase 2 complete ✅"
     
User Role: Decision Maker (not Babysitter)
```

**Key Features:**
- ✅ **Single conversation** with main coordinator agent
- ✅ **Auto-dispatch** to sub-agents
- ✅ **Auto-monitoring** (no manual status checks)
- ✅ **Auto-validation** (tests run automatically)
- ✅ **Auto-coordination** (dependencies handled)
- ✅ **User can still access** individual agent sessions (optional)

---

## 🏗️ Architecture Design

### Main Coordinator Agent

```rust
pub struct CoordinatorAgent {
    user_session: UserSession,
    sub_agents: Vec<SubAgent>,
    sprint_backlog: SprintBacklog,
    dependency_graph: DependencyGraph,
}

impl CoordinatorAgent {
    pub async fn handle_user_request(&mut self, request: &UserRequest) {
        // 1. Decompose request into sprints
        let sprints = self.decompose(request).await;
        
        // 2. Dispatch to sub-agents (concurrently)
        for sprint in sprints {
            let agent = self.select_agent(&sprint);
            agent.dispatch(sprint).await;
        }
        
        // 3. Monitor progress (background)
        self.monitor_progress().await;
        
        // 4. Auto-validate completions
        for sprint in sprints {
            if sprint.is_complete() {
                self.validate(sprint).await;
                
                // 5. Auto-create next prompt
                let next_sprint = self.create_next_sprint(sprint);
                self.dispatch(next_sprint).await;
            }
        }
        
        // 6. Report to user (only when meaningful)
        self.report_to_user().await;
    }
}
```

### Sub-Agent Interface

```rust
pub struct SubAgent {
    id: AgentId,
    sprint: Sprint,
    status: AgentStatus,
    output: AgentOutput,
}

pub enum AgentStatus {
    Working,
    WaitingForReview,
    Complete,
    Blocked(String),
}

impl SubAgent {
    pub async fn dispatch(&mut self, sprint: Sprint) {
        self.sprint = sprint;
        self.status = AgentStatus::Working;
        
        // Execute in isolated session
        self.output = self.execute().await;
        
        // Auto-run tests
        if self.output.tests_pass() {
            self.status = AgentStatus::Complete;
        } else {
            self.status = AgentStatus::Blocked("Tests failed".to_string());
        }
        
        // Auto-report to coordinator
        self.report_to_coordinator().await;
    }
}
```

### Dependency Management

```rust
pub struct DependencyGraph {
    sprints: HashMap<SprintId, Sprint>,
    dependencies: Vec<(SprintId, SprintId)>, // (from, to)
}

impl DependencyGraph {
    pub fn can_start(&self, sprint_id: &SprintId) -> bool {
        // Check if all dependencies are complete
        self.get_dependencies(sprint_id)
            .iter()
            .all(|dep_id| self.sprints.get(dep_id).unwrap().is_complete())
    }
    
    pub fn get_next_sprints(&self) -> Vec<SprintId> {
        // Get all sprints that can start now
        self.sprints
            .iter()
            .filter(|(id, _)| self.can_start(id))
            .map(|(id, _)| id.clone())
            .collect()
    }
}
```

---

## 📋 User Experience

### Before (Current)

```
User: "Dispara Wave 1"

Assistant: "Criei 3 prompts. Abra 3 terminais e copie:
- planning/AGENT-A-SPRINT-3.md
- planning/AGENT-B-SPRINT-5.md
- planning/AGENT-C-SPRINT-3B.md"

User: *opens 3 terminals*
User: *copies prompts*
User: *waits*
User: "Agent A terminou?"

Assistant: "Vou verificar..."
*checks status*
Assistant: "Sim, completo!"

User: "Cria proximo prompt para Agent A"

Assistant: *creates prompt*

User: *copies prompt*
User: *pastes in terminal*
...
```

### After (Self-Orchestrating)

```
User: "Implementa Phase 2"

Coordinator Agent: "Decompondo em 6 sprints...
Dispatching:
- Agent A: Sprint 3 (Code Minification)
- Agent B: Sprint 5 (TOON Serialization)
- Agent C: Sprint 3b (Heartbeat)

Monitorando progresso..."

*8 hours later*

Coordinator Agent: "✅ Wave 1 Complete!
- Agent A: ✅ Sprint 3 (10 testes)
- Agent B: ✅ Sprint 5 (14 testes)
- Agent C: ✅ Sprint 3b (13 testes)

Auto-dispatching Wave 2:
- Agent A: Sprint 6 (Docs)
- Agent B: Sprint 8 (Context)
- Agent C: Sprint 7 (Decomposition)

Continue? [Y/n]"

User: "Y"

*continues automatically*
```

---

## 🎯 Implementation Plan

### Phase 1: Basic Coordinator (16 hours)

- [ ] `CoordinatorAgent` struct
- [ ] Sub-agent dispatch (sequential first)
- [ ] Status monitoring (polling)
- [ ] Basic validation (test pass/fail)

### Phase 2: Concurrent Dispatch (16 hours)

- [ ] Concurrent sub-agent execution
- [ ] Dependency graph
- [ ] Auto-coordination (wait for deps)

### Phase 3: Auto-Validation (16 hours)

- [ ] Auto-run tests (`cargo test`)
- [ ] Parse test results
- [ ] Auto-retry on failure

### Phase 4: Auto-Prompt Generation (24 hours)

- [ ] LLM-based prompt generation
- [ ] Context from previous sprint
- [ ] Dependency-aware prompts

### Phase 5: User Interface (24 hours)

- [ ] Single conversation UI
- [ ] Sub-agent session viewer (optional)
- [ ] Progress dashboard
- [ ] Decision points (user input when needed)

---

## 🔗 Integration with Existing AXORA

### Uses Existing Components

| Component | How It's Used |
|-----------|---------------|
| **Mission Decomposer** (Sprint 7) | Decompose user request into sprints |
| **Heartbeat** (Sprint 3b) | Monitor sub-agent health |
| **Context Distribution** (Sprint 8) | Give sub-agents minimal context |
| **Documentation** (Sprint 6) | Auto-document decisions |
| **ADR System** (Sprint 6) | Capture coordination decisions |

### New Components Needed

| Component | Purpose |
|-----------|---------|
| `CoordinatorAgent` | Main orchestrator |
| `SubAgent` | Isolated execution |
| `DependencyGraph` | Track sprint dependencies |
| `AutoValidator` | Run tests, validate output |
| `PromptGenerator` | Create next prompts automatically |

---

## 📊 Success Metrics

| Metric | Before | After | Target |
|--------|--------|-------|--------|
| User interventions per sprint | 5-10 | 0-1 | <1 |
| Time to dispatch next sprint | 5-10 min | <1 min | <30s |
| Context switches for user | 3-5 | 0-1 | <1 |
| User satisfaction | Low | High | >4.0/5.0 |

---

## 🚨 Why This Is a KEY Differentiator

**No other agent framework has:**
- ✅ Self-orchestration (all are manual dispatch)
- ✅ Auto-validation (all require manual test running)
- ✅ Auto-prompt-generation (all require manual prompt creation)
- ✅ Single conversation UI (all require managing multiple sessions)

**This is AXORA's killer feature:**
> "The only agent framework that manages itself — you talk to one agent, it manages the rest."

---

## ✅ Next Steps

1. **Add to Phase 3 backlog** (after token optimization)
2. **Create research prompt** for coordinator patterns
3. **Design UI mockups** for single conversation
4. **Prioritize vs other Phase 3 work**

---

**This insight came from real user pain during Wave 1 execution.**
**It should be a CORE feature of AXORA, not an afterthought.**
