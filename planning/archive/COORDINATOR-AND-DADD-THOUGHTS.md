# Coordinator Agent & DADD Architecture — Foundational Thoughts

**Document Created:** 2026-03-16  
**Type:** Conceptual exploration, not implementation plan  
**Purpose:** Capture founder's thinking about Coordinator architecture and DADD hierarchy

---

## 💭 Core Insight: User Experience

### The Problem We're Solving

**Current State (Pre-Coordinator):**
- User opens 3 terminals
- User copies prompts manually
- User is "babysitter" and "pigeon courier"
- User manages dependencies between agents
- User merges results manually
- **User does all orchestration**

**Desired State (With Coordinator):**
- User opens ONE session
- User talks to ONE agent (Coordinator)
- Coordinator manages everything behind the scenes
- User doesn't know/care about worker agents
- **Coordinator does all orchestration**

---

## 🧠 Key Design Principles

### 1. Single Conversation Illusion

**User Perspective:**
```
User: "Implement authentication system"

Coordinator: "I'll handle that. Give me 10 minutes."

[10 minutes later]

Coordinator: "Done. Here's what I built:
- Login/logout endpoints
- JWT token management
- Password hashing with bcrypt
- 15 tests passing

Want me to explain anything or make changes?"
```

**Behind the Scenes:**
```
Coordinator spawns:
- Agent A: Documentation (writes API spec)
- Agent B: Implementation (writes code)
- Agent C: Testing (writes tests)
- Agent D: Review (validates security)

All coordinated, merged, validated — user sees ONE result.
```

**Key Principle:** User NEVER talks to worker agents directly.

---

### 2. Global State Awareness

**Problem:** Multiple sessions, agents stepping on each other's work

**Example of Chaos (Without Coordinator):**
```
Session 1: Agent A modifying auth.rs
Session 2: Agent B modifying auth.rs (unaware of Session 1)
Session 3: Agent C deleting auth.rs (thinks it's obsolete)

Result: Conflicts, overwrites, broken code.
```

**Solution (With Coordinator):**
```
Session 1: User asks for login feature
  → Coordinator locks auth.rs for Session 1
  → Agent A modifies auth.rs
  
Session 2: User asks for password reset
  → Coordinator sees auth.rs is locked
  → Coordinator queues Session 2 OR
  → Coordinator assigns Agent B to password_reset.rs (parallel file)
  
Session 3: User asks to cleanup
  → Coordinator knows auth.rs is in-use
  → Coordinator prevents deletion
```

**Key Principle:** Coordinator has GLOBAL AWARENESS of:
- Which files are being modified (by which session)
- Which agents are working on what
- Dependencies between tasks
- Conflicts before they happen

---

### 3. Dynamic Agent Lifecycle

**Coordinator Can:**
- **Spawn** new agents (when workload increases)
- **Pause** agents (waiting for dependencies)
- **Resume** agents (when dependencies resolved)
- **Terminate** agents (task complete or conflict)
- **Update** agents (new information available)
- **Merge** agents (consolidate results)

**Example:**
```
Session 1: "Implement login"
  → Coordinator spawns Agent A (Coder)
  → Agent A starts coding

Session 2: "Add tests for login"
  → Coordinator spawns Agent B (Tester)
  → Agent B needs Agent A's code (dependency)
  → Coordinator PAUSES Agent B
  → Agent A completes
  → Coordinator RESUMES Agent B
  → Agent B writes tests

Session 3: "Actually, use OAuth instead of password"
  → Coordinator UPDATES Agent A (new requirements)
  → Agent A pivots to OAuth
  → Coordinator UPDATES Agent B (tests changed)
  → Agent B updates tests
```

**Key Principle:** Agents are EPHEMERAL, Coordinator is PERMANENT.

---

## 🏗️ DADD: Domain Agent Driven Development

### The Hierarchy Concept

**Inspiration:** Corporate org charts, but for AI agents

**Current Multi-Agent Systems:**
```
Orchestrator → Worker 1
            → Worker 2
            → Worker 3
            → Worker 4
            ... (flat, Orchestrator talks to everyone)
```

**Problem:** Orchestrator becomes bottleneck, overwhelmed with details.

**DADD Hierarchy:**
```
                    Orchestrator (CEO)
                          ↓
              Area Superiors (VPs)
                ↓        ↓        ↓
            Chiefs    Chiefs   Chiefs (Directors)
              ↓          ↓        ↓
           Leads      Leads    Leads (Managers)
              ↓          ↓        ↓
         Guardians  Guardians Guardians (Senior Devs)
              ↓          ↓        ↓
          Interns    Interns  Interns (Juniors)
```

---

### How DADD Works

**Example: User asks "Implement authentication"**

```
User → Orchestrator
  → Orchestrator → Area Superior (Security Domain)
    → Area Superior → Chief (Auth Systems)
      → Chief → Lead (Login/Logout)
        → Lead → Guardian (Session Management)
          → Guardian → Intern (Token Storage)
          
Each level:
- Receives high-level directive from above
- Delegates specific tasks below
- Validates work from below
- Reports status to above
```

**Key Insight:** Orchestrator NEVER talks to Interns directly.

---

### Responsibility Flow

**Downward (Task Delegation):**
```
Orchestrator: "Implement authentication"
  ↓
Area Superior: "Break into: login, logout, session management"
  ↓
Chief: "Login needs: UI, backend, database"
  ↓
Lead: "Backend needs: endpoint, validation, hashing"
  ↓
Guardian: "Hashing needs: bcrypt, salt generation"
  ↓
Intern: "Implement bcrypt with salt length 12"
```

**Upward (Status Reporting):**
```
Intern: "Bcrypt implemented, tests passing"
  ↓
Guardian: "Validated hashing implementation"
  ↓
Lead: "Backend complete, ready for integration"
  ↓
Chief: "Login system complete, 3/3 components done"
  ↓
Area Superior: "Auth domain 50% complete (login done, logout pending)"
  ↓
Orchestrator: "Overall progress: 50%"
```

---

### Why Hierarchy Helps

**1. Orchestrator Stays Clean:**
- Doesn't know implementation details
- Only knows high-level progress
- Can manage 100s of agents through hierarchy
- Never overwhelmed

**2. Specialization at Each Level:**
- Interns: Specific tasks (implement function X)
- Guardians: Code quality (review, validate)
- Leads: Feature completeness (all parts working)
- Chiefs: Domain integration (features work together)
- Area Superiors: Cross-domain coordination
- Orchestrator: User intent → high-level directives

**3. Failure Isolation:**
- Intern fails → Guardian catches it
- Guardian fails → Lead catches it
- Lead fails → Chief catches it
- Failure never reaches Orchestrator unless systemic

**4. Scalability:**
- Add more Interns under Guardians
- Add more Guardians under Leads
- Add more Leads under Chiefs
- Orchestrator still manages same number of Area Superiors

---

## ⚖️ Tension: Hierarchy vs Global Consciousness

### The Paradox

**Hierarchy Says:**
- Information flows through CHANNELS
- Intern doesn't know what's happening in other domains
- Orchestrator has high-level view, not details
- Knowledge is HIERARCHICAL

**Global Consciousness Says:**
- All agents share knowledge INSTANTLY
- Every agent knows everything relevant
- No silos, no information hoarding
- Knowledge is UNIFIED

**These seem contradictory. Are they?**

---

### Possible Resolution: Layered Consciousness

**Analogy:** Human brain

```
Conscious Mind (Orchestrator)
  - Aware of high-level goals
  - Doesn't know neuron-level details
  
Subconscious (Area Superiors + Chiefs)
  - Manages automatic processes
  - Coordinates without conscious input
  
Neural Clusters (Leads + Guardians)
  - Specialized processing
  - Local coordination
  
Individual Neurons (Interns)
  - Fire based on local signals
  - Don't know "big picture"
```

**Yet:** Brain has GLOBAL consciousness despite hierarchy.

**How?** Through INTEGRATION LAYERS:

```
Interns → share state with Guardians
Guardians → share state with Leads + other Guardians
Leads → share state with Chiefs + other Leads
Chiefs → share state with Area Superiors + other Chiefs
Area Superiors → share state with Orchestrator

Orchestrator → broadcasts updates back down
```

**Result:** Hierarchy for DELEGATION, Network for AWARENESS.

---

### Practical Implementation

**Blackboard Pattern (Shared State):**
```
All agents read/write to shared blackboard:
- Intern writes: "Completed function X"
- Guardian reads: Validates Intern's work
- Lead reads: Sees Guardian's validation
- Chief reads: Aggregates Leads' progress
- Orchestrator reads: High-level status

Orchestrator writes: "Priority changed to OAuth"
- Chief reads: Updates domain plans
- Lead reads: Updates feature tasks
- Guardian reads: Updates Intern's directives
- Intern reads: Changes implementation
```

**Hierarchy for:** Task delegation, validation, escalation  
**Blackboard for:** Global awareness, synchronization

---

## 🤔 Open Questions (To Explore Later)

### 1. How Many Levels?

**Current Proposal:** 6 levels (Intern → Guardian → Lead → Chief → Area → Orchestrator)

**Question:** Is 6 too many? Too few?

**Trade-offs:**
- More levels = better specialization, more latency
- Fewer levels = faster communication, Orchestrator more loaded

**Hypothesis:** 4-5 levels optimal for most projects
- Intern
- Guardian  
- Lead/Chief (combined)
- Area Superior
- Orchestrator

---

### 2. How Are Domains Defined?

**Options:**
- **By Feature:** Auth, Payments, API, UI
- **By Technology:** Frontend, Backend, Database
- **By Task Type:** Coding, Testing, Documentation
- **Dynamic:** Created on-demand based on task

**Hypothesis:** Hybrid approach
- Top levels: By Feature (stable domains)
- Lower levels: By Task Type (flexible)

---

### 3. Who Decides Domain Boundaries?

**Options:**
- **Pre-defined:** Architect defines domains upfront
- **Emergent:** Domains emerge from task patterns
- **User-defined:** User specifies domains
- **Coordinator-decided:** Coordinator creates domains dynamically

**Hypothesis:** Pre-defined for core domains, emergent for sub-domains

---

### 4. What Happens When Domains Conflict?

**Example:** Auth Domain vs Payments Domain both need user_id

**Options:**
- **Orchestrator Arbitrates:** Escalate to top
- **Area Superiors Negotiate:** Peer-level resolution
- **Shared Sub-domain:** Create User domain (shared)
- **First-Come-Wins:** Whoever claims it first

**Hypothesis:** Escalate to Orchestrator only if Area Superiors can't resolve

---

### 5. How Does Learning Happen?

**Current Memory Architecture:** Episodic, Semantic, Procedural

**Question:** Where is memory stored in hierarchy?

**Options:**
- **Centralized:** All memory in Orchestrator
- **Distributed:** Each level has own memory
- **Domain-based:** Each domain has own memory
- **Hybrid:** Shared semantic, distributed episodic/procedural

**Hypothesis:** Hybrid
- Semantic: Shared (facts are universal)
- Episodic: Domain-based (experiences are domain-specific)
- Procedural: Level-based (skills are role-specific)

---

## 💡 Key Insights (So Far)

1. **User talks to ONE agent** (Orchestrator), never workers
2. **Orchestrator has GLOBAL AWARENESS** (all sessions, all agents)
3. **DADD Hierarchy** prevents Orchestrator overload
4. **Responsibility flows DOWN, status flows UP**
5. **Hierarchy + Blackboard** = Delegation + Awareness
6. **6 levels may be too many** (consider 4-5)
7. **Domains need clear boundaries** (feature-based?)
8. **Memory architecture must align** with hierarchy

---

## 📝 Next Steps (When Ready)

1. **Explore DADD hierarchy depth** (4 vs 5 vs 6 levels)
2. **Define domain boundaries** (how to split codebase)
3. **Design blackboard pattern** (shared state mechanism)
4. **Align with Memory Architecture** (where does memory live)
5. **Create Coordinator sprints** (implementation plan)

---

**Document Status:** Conceptual exploration  
**Next Review:** After Phase 2 Complete  
**Decision Pending:** DADD hierarchy depth + domain definitions
