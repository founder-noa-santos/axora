# Domain-Driven Agent Teams (DDD/TDD) — Innovation Analysis

## ⚠️ STATUS: REJECTED (2026-03-16)

**This document contains historical analysis of DDD Agent Teams.**

**Decision:** DEFERRED indefinitely, **Graph-Based Workflow ADOPTED** instead.

**Reason:** DDD is enterprise over-engineering. Individual developers need:
- Low latency (not high coordination overhead)
- Simple architecture (not bounded contexts + ACLs)
- Holistic context (not siloed domain teams)

**See:** [`PHASE-2-PIVOT-GRAPH-WORKFLOW.md`](./PHASE-2-PIVOT-GRAPH-WORKFLOW.md) for the new architecture.

---

## 📚 Lessons Learned (from R-10 Research)

The R-10 validation research revealed critical insights that led to rejecting DDD:

### 1. Coordination Overhead is Quadratic
```
N agents → N(N-1)/2 communication paths

3 agents → 3 paths
5 agents → 10 paths
10 agents → 45 paths  ← Token inflation explodes
```
**Impact:** 3-15x token overhead for team coordination

### 2. "Expertise Accumulation" is Anthropomorphism
> "Agents do not learn interactively like human engineers; their efficacy relies entirely on RAG architectures and state externalization."

**Reality:** Domain expertise = Better retrieval, not team structure

### 3. Cross-Domain Routing is a Bottleneck
- 20-40% token overhead for ACL translation
- High failure rate for complex cross-domain tasks
- Recursive handoffs cause latency spikes

### 4. Industry Avoids DDD for Good Reason
- **AutoGen:** Flat specialization (avoids DDD)
- **CrewAI:** Role-based, NOT domain-based
- **LangGraph:** State machine, NO team metaphor

### 5. Specialist vs Generalist Performance
- ✅ Specialists win on **parallelizable** tasks (80.9% improvement)
- ❌ Specialists LOSE on **sequential** tasks (39-70% degradation)
- **Individual dev work is mostly sequential** → Generalists win

---

## 🔄 New Architecture: Graph-Based Workflow

Instead of DDD Agent Teams, AXORA now uses:

```
User Request → Deterministic Graph → Generalist Agents + Domain RAG → Output
```

**Key differences:**
- Agents are NOT domain-specialized
- Domain knowledge is in RAG, not agent structure
- Coordination is O(N), not O(N²)
- Token overhead <10%, not 40%+

**See:** [`GRAPH-WORKFLOW-DESIGN.md`](./GRAPH-WORKFLOW-DESIGN.md) for full design.

---

**Original analysis preserved below for historical reference.**

---

## 🔍 Current AXORA Agent Structure

**Current model:** Flat specialization
```
┌─────────────────────────────────────────────────────────┐
│              AXORA Agent Pool                            │
├─────────────────────────────────────────────────────────┤
│  Architect │ Coder │ Reviewer │ Tester │ Debugger │ ... │
└─────────────────────────────────────────────────────────┘
```

**Problem:** All agents work on **everything**. No domain expertise.

---

## 💡 Your Insight: Domain-Specialized Teams

You proposed organizing agents by **domain** (DDD) or **test** (TDD):

### Option 1: DDD Agent Teams

```
┌─────────────────────────────────────────────────────────┐
│              AXORA Domain Teams                          │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌───────────────┐  ┌───────────────┐  ┌─────────────┐ │
│  │ Auth Domain   │  │ Payment Domain│  │ API Domain  │ │
│  ├───────────────┤  ├───────────────┤  ├─────────────┤ │
│  │ • Coder       │  │ • Coder       │  │ • Coder     │ │
│  │ • Tester      │  │ • Tester      │  │ • Tester    │ │
│  │ • Reviewer    │  │ • Reviewer    │  │ • Reviewer  │ │
│  └───────────────┘  └───────────────┘  └─────────────┘ │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**Each domain team has:**
- Specialist coder (knows auth patterns, security best practices)
- Specialist tester (knows auth test cases, edge cases)
- Specialist reviewer (knows auth vulnerabilities)

---

### Option 2: TDD Agent Teams

```
┌─────────────────────────────────────────────────────────┐
│              AXORA Test-Driven Teams                     │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌───────────────┐  ┌───────────────┐  ┌─────────────┐ │
│  │ Unit Tests    │  │ Integration   │  │ E2E Tests   │ │
│  ├───────────────┤  ├───────────────┤  ├─────────────┤ │
│  │ • Test Writer │  │ • Test Writer │  │ • Test Writer││
│  │ • Code Fixer  │  │ • Code Fixer  │  │ • Code Fixer││
│  └───────────────┘  └───────────────┘  └─────────────┘ │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**Each test team has:**
- Test writer specialist
- Code fixer (makes tests pass)

---

## 🎯 Evaluation: DDD vs TDD vs Current

| Aspect | Current (Flat) | DDD Teams | TDD Teams |
|--------|---------------|-----------|-----------|
| **Domain expertise** | ❌ None | ✅ High | ❌ Low |
| **Code consistency** | ❌ Variable | ✅ High per domain | ❌ Variable |
| **Test coverage** | ⚠️ Generic | ✅ Domain-specific | ✅ High |
| **Scalability** | ⚠️ Limited | ✅ Excellent | ⚠️ Limited |
| **Innovation level** | ❌ Commodity | ✅ **High** | ⚠️ Medium |
| **Implementation cost** | ✅ Low | ⚠️ Medium | ✅ Low |

---

## 🚀 DDD Agent Teams — Why This Is Innovative

### 1. Mirrors Human Team Structure

**Real-world software teams:**
```
Auth Team → Specialists in auth, security, OAuth, JWT
Payment Team → Specialists in payments, PCI compliance, fraud
API Team → Specialists in REST, GraphQL, versioning
```

**AXORA DDD teams:** Same structure, but AI agents.

**Benefit:** Users already understand this model. No learning curve.

---

### 2. Domain Expertise Accumulation

**Current model:**
- Agent works on auth today, payments tomorrow, API next week
- No accumulated expertise
- Every task starts from zero

**DDD model:**
- Auth coder works **only on auth tasks**
- Learns patterns: OAuth flows, JWT validation, session management
- Builds **domain-specific knowledge base**
- Gets **better over time** at auth tasks

**Benefit:** Compound expertise, like human specialists.

---

### 3. Code Consistency Per Domain

**Current model:**
- Different coders work on same domain
- Inconsistent patterns, styles, architectures

**DDD model:**
- Same auth coder (or small team) owns auth domain
- Consistent patterns across all auth code
- Easier to maintain, review, debug

**Benefit:** Code quality through ownership.

---

### 4. Natural Boundaries (Bounded Contexts)

**DDD principle:** Bounded contexts prevent leakage between domains.

**AXORA application:**
- Auth team doesn't touch payment code
- Payment team doesn't touch API code
- Clear boundaries, no accidental coupling

**Benefit:** Architecture enforcement.

---

## 📊 Proposed AXORA DDD Structure

### Default Domain Teams (Pre-configured)

| Domain Team | Specialist Agents | Responsibilities |
|-------------|-------------------|------------------|
| **Auth & Security** | Coder, Tester, Reviewer | Auth, OAuth, JWT, sessions, security |
| **Data & Models** | Coder, Tester, Reviewer | Database, ORM, migrations, models |
| **API & Routes** | Coder, Tester, Reviewer | REST, GraphQL, routes, controllers |
| **UI & Frontend** | Coder, Tester, Reviewer | Components, pages, styles, state |
| **Testing & QA** | Test Writer, Code Fixer | Unit, integration, E2E tests |
| **DevOps & Deploy** | Coder, Tester | CI/CD, Docker, infra, monitoring |

### User Can Customize

```rust
pub struct DomainTeam {
    name: String,
    agents: Vec<AgentId>,
    bounded_context: Vec<String>, // File patterns, e.g., "src/auth/**"
}

// User config
let config = AgentConfig {
    teams: vec![
        DomainTeam::new("Auth", vec!["coder-1", "tester-1"])
            .with_context(vec!["src/auth/**", "src/security/**"]),
        
        DomainTeam::new("Payments", vec!["coder-2", "tester-2"])
            .with_context(vec!["src/payments/**"]),
    ],
};
```

---

## 🔄 Integration with Current Architecture

### State Machine + Heartbeat + DDD

```rust
pub struct DomainTeam {
    id: String,
    name: String,
    agents: Vec<AgentId>,
    bounded_context: Vec<String>,
    state: TeamState,
    heartbeat: Heartbeat,
}

pub enum TeamState {
    Idle,           // All agents sleeping
    Active,         // Some agents working
    WaitingReview,  // Blocked on review
    Completed,      // All tasks done
}

impl DomainTeam {
    pub fn handle_heartbeat(&mut self) -> Result<()> {
        // Wake team, check for new tasks in domain
        if let Some(task) = self.get_pending_task_in_context() {
            self.assign_to_specialist(task)?;
        } else {
            // No work, return to sleep
            self.heartbeat.schedule_wake(Duration::from_secs(30));
        }
        
        Ok(())
    }
}
```

### NATS Communication

```rust
// Domain-specific channels
let auth_subject = "axora.team.auth.tasks";
let payment_subject = "axora.team.payment.tasks";

// Agents subscribe to their domain channel
nats.subscribe(auth_subject, |msg| {
    // Auth team wakes, processes task
    auth_team.handle_task(msg);
});
```

---

## 💡 Innovation Assessment

### Is DDD Agent Teams Innovative?

**Short answer:** **YES** — This is **highly innovative** for AI agent frameworks.

**Evidence:**

| Framework | Domain Teams? | Notes |
|-----------|---------------|-------|
| AutoGen | ❌ No | Flat agent pool |
| CrewAI | ❌ No | Role-based, not domain-based |
| LangGraph | ❌ No | State machine, no domains |
| Paperclip | ⚠️ Partial | Company structure, not DDD |
| **AXORA (proposed)** | ✅ **YES** | **First to combine DDD + agents** |

**Why this is innovative:**
1. **First framework** to apply DDD bounded contexts to agent teams
2. **Domain expertise accumulation** (agents get better at their domain)
3. **Natural architecture enforcement** (teams can't cross boundaries)
4. **Mirrors human team structure** (intuitive for users)

---

## 🎯 Recommendation

### ✅ ADOPT DDD Agent Teams

**Implementation Plan:**
- **Phase 2, Sprint 4** (new sprint)
- **Effort:** ~16 hours (team structure + bounded contexts + routing)
- **Risk:** Medium (new concept, needs testing)

**Why:**
1. **Differentiator:** No other agent framework does this
2. **User value:** Domain expertise, code consistency
3. **Scalability:** Natural way to scale to 20+ agents
4. **Innovation:** First-to-market with DDD + agents

---

## 📋 Revised Phase 2 Plan

### Sprint 1: Prefix Caching ✅ COMPLETE

### Sprint 2: Diff-Based Communication ✅ COMPLETE
- [x] Unified diff generation
- [x] Patch application
- [x] Token savings measurement
- [x] **NEW:** Budget tracking per agent

### Sprint 3: Code Minification + Heartbeat 🔄 IN PROGRESS
- [x] Whitespace removal
- [x] Identifier compression
- [x] Comment stripping
- [x] Immutable audit logging
- [x] **NEW:** Heartbeat system (timer + event-driven)

### Sprint 4: DDD Agent Teams 📋 NEW
- [ ] Domain team structure
- [ ] Bounded context configuration
- [ ] Task routing to domain teams
- [ ] Domain-specific expertise tracking
- [ ] **Tests:** 10+ passing

### Sprint 5: TOON Serialization 📋 PLANNED
- [ ] TOON encoder/decoder
- [ ] Schema management
- [ ] JSON → TOON conversion

---

## 📊 Final Comparison

| Approach | Innovation | User Value | Implementation Cost | Recommendation |
|----------|------------|------------|---------------------|----------------|
| Current (Flat) | ❌ Low | ⚠️ Medium | ✅ Low | Reject |
| TDD Teams | ⚠️ Medium | ⚠️ Medium | ✅ Low | Defer |
| **DDD Teams** | ✅ **High** | ✅ **High** | ⚠️ Medium | **ADOPT** |

---

**Conclusion:** DDD Agent Teams are **innovative, valuable, and feasible**. This is a **key differentiator** for AXORA. Adopt in Phase 2, Sprint 4.
