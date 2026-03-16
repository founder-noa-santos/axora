# R-15: Context Compacting, Sharing & Distribution for Multi-Agent Systems

**Priority:** 🔴 CRITICAL (enables long-running agents, reduces costs, solves coordination)  
**Status:** 📋 Research Prompt Ready  
**Estimated Research Time:** 4-5 hours  

---

## Context & Motivation

**Problem 1: Context Explosion**
LLM agents accumulate massive context over time:
- Conversation history (every turn)
- Retrieved documents (RAG results)
- Tool outputs (terminal logs, file contents)
- Intermediate reasoning (thoughts, plans)

**Without compaction:**
- Context window fills in 10-20 turns
- Token costs explode ($0.03-0.12 per 1K tokens)
- Latency increases (more tokens = slower inference)
- Quality degrades ("lost in the middle" phenomenon)

**Problem 2: Context Sharing Between Agents**
Multi-agent systems need to share context efficiently:
- How to avoid sending full context to every agent?
- How to propagate changes when context updates?
- How to maintain consistency across agents?
- How to avoid duplication (150K tokens for 3 agents)?

**Problem 3: Context Versioning**
When context changes mid-execution:
- How do agents know context changed?
- Do we re-send entire context or just diff?
- How to handle concurrent modifications?

---

## 🔬 Research Questions

### Part 1: Context Compacting

#### 1. What is Context Compacting?

**Questions:**
- What exactly is "context compacting"?
- How does it differ from summarization?
- What are the core techniques?
- What's the state-of-the-art?

**Sub-questions:**
- Is it lossy or lossless compression?
- Does it preserve reasoning quality?
- How much compression is achievable?

---

#### 2. Core Compacting Techniques

**Techniques to Research:**

##### A. Summarization-Based
- **Rolling Summary:** Summarize last N turns
- **Hierarchical Summary:** Summaries of summaries
- **Key-Point Extraction:** Extract only critical information
- **Abstractive vs Extractive:** Which works better?

##### B. Memory-Based
- **Episodic Memory:** Store raw history externally
- **Semantic Memory:** Store embeddings, retrieve on demand
- **Working Memory:** Keep only active context in-window

##### C. Structural
- **Template-Based:** Fixed structure, fill slots
- **Schema-Based:** JSON/YAML structure, prune fields
- **Tree-Based:** Hierarchical context, prune branches

##### D. Attention-Based
- **Sparse Attention:** Attend only to relevant tokens
- **Sliding Window:** Keep only recent N tokens
- **Importance Scoring:** Score tokens, prune low-importance

---

### Part 2: Context Sharing & Distribution

#### 3. Multi-Agent Context Architecture

**Questions:**
- How do production systems share context between agents?
- Centralized (Coordinator has all) vs Decentralized (each agent has own)?
- What is the industry standard?

**Architectures to Research:**

##### A. Centralized Blackboard
```
Coordinator → Blackboard (shared state) → Workers read/write
```
- Who maintains blackboard?
- How do workers subscribe to changes?
- How to handle concurrent writes?

##### B. Publish-Subscribe
```
Agent publishes context update → Message bus → Subscribers receive
```
- What message bus? (NATS, Redis Pub/Sub, etc.)
- How to filter relevant updates?
- How to handle late-joining agents?

##### C. Coordinator-Mediated
```
All agents talk to Coordinator → Coordinator routes context
```
- Does Coordinator become bottleneck?
- How much context does Coordinator store?
- How to scale to 100+ agents?

---

#### 4. Context Distribution Patterns

**Patterns to Research:**

##### A. Full Broadcast (Inefficient)
```
Context changes → Send full context to ALL agents
```
- Token cost: O(N × context_size)
- When is this acceptable?

##### B. Diff-Based Push (Efficient)
```
Context changes → Compute diff → Send diff to affected agents
```
- Token cost: O(N × diff_size)
- How to compute diff efficiently?
- How to know which agents are affected?

##### C. Pull-Based (On-Demand)
```
Agents request context when needed → Coordinator responds
```
- Token cost: O(requests × context_size)
- How do agents know what to request?
- Latency implications?

##### D. Versioned Context
```
Each context has version hash
Agent includes version in request
If mismatch → Server sends diff
```
- Token cost: O(mismatches × diff_size)
- How to version context?
- How to handle version conflicts?

---

#### 5. Context Consistency

**Questions:**
- How to maintain consistency when multiple agents modify context?
- What happens when Agent A modifies file that Agent B is reading?
- How to prevent race conditions?

**Strategies to Research:**

##### A. Lock-Based
```
Agent A locks context → Modifies → Unlocks → Agent B can read
```
- Blocking vs non-blocking?
- Deadlock prevention?

##### B. Snapshot-Based
```
Agent A reads snapshot at time T
Agent B modifies at time T+1
Agent A continues with old snapshot (consistent view)
```
- How to detect stale reads?
- When to refresh snapshot?

##### C. CRDT (Conflict-Free Replicated Data Types)
```
Each agent has local copy
Modifications merge automatically (mathematical guarantees)
```
- Applicable to LLM context?
- Implementation complexity?

---

#### 6. Industry Implementations

**Systems to Research:**

| System | Context Sharing Approach | Token Efficiency | Consistency Model |
|--------|-------------------------|-----------------|-------------------|
| **MemGPT / Letta** | ? | ? | ? |
| **LangGraph** | ? | ? | ? |
| **AutoGen** | ? | ? | ? |
| **CrewAI** | ? | ? | ? |
| **OpenDevin / OpenHands** | ? | ? | ? |
| **Dify** | ? | ? | ? |

**For each:**
- How do they share context between agents?
- Do they use blackboard, pub-sub, or coordinator?
- How do they handle context updates?
- Open source implementation available?

---

### Part 3: Context Versioning & Updates

#### 7. Context Change Propagation

**Questions:**
- When context changes, how are agents notified?
- Polling (agent checks periodically) vs Push (server notifies)?
- What's the industry standard?

**Approaches:**

##### A. Polling (Dumb, Works)
```rust
// Agent checks every N turns
if context.has_changed() {
    agent.refresh_context();
}
```
**Pros:** Simple, no coordination needed  
**Cons:** Wasted tokens, latency, may miss changes

##### B. Push-Based (Better)
```rust
// Coordinator notifies when context changes
coordinator.notify_agents(
    changed_files: ["auth.rs"],
    change_type: "modified"
);
```
**Pros:** Efficient, real-time  
**Cons:** Need coordination infrastructure

##### C. Versioned Diff (Best)
```rust
// Each context has version hash
context_version: "abc123"

// Agent includes version in request
if server_version != client_version {
    send_context_diff(old_version, new_version);
}
```
**Pros:** Minimal tokens, only sends changes  
**Cons:** Need versioning infrastructure

---

#### 8. Message History Patterns

**Questions:**
- Is full message history sent with every request?
- What are the alternatives?
- What do OpenAI/Anthropic recommend?

**Patterns:**

##### A. Full History (OpenAI Standard)
```json
{
  "messages": [
    {"role": "user", "content": "turn 1"},
    {"role": "assistant", "content": "turn 1"},
    {"role": "user", "content": "turn 2"},
    {"role": "assistant", "content": "turn 2"},
    ... // EVERYTHING, always
  ]
}
```
**Problem:** 100 turns = 200 messages = context explodes

##### B. System + Messages (Anthropic)
```json
{
  "system": "...", // Doesn't count toward message limit
  "messages": [...] // Only conversation turns
}
```
**Advantage:** System context separate from conversation

##### C. Hierarchical Memory (Recommended for AXORA)
```json
{
  "system": "...", // Fixed instructions
  
  "working_memory": {
    "recent_turns": [...], // Last 5 turns (full)
    "summary": "...", // Turns 6-20 (summarized)
    "episodic_refs": [...] // Turns 21+ (external refs)
  },
  
  "context": {
    "files": ["auth.rs v3"],
    "skills": ["DEBUG_AUTH_FAILURE"],
    "goals": ["Implement login"]
  }
}
```
**Advantage:** Context stays constant (~5K tokens) regardless of turns

---

### Part 4: AXORA Integration

#### 9. Coordinator + Blackboard Architecture

**Questions:**
- How does compacting integrate with Coordinator pattern?
- Should Coordinator compact worker contexts?
- How to compact across multiple agents (shared context)?

**Proposed Architecture:**
```rust
Coordinator {
    global_state: GlobalState, // High-level view (not details)
    blackboard: Blackboard, // Shared state (all agents read/write)
    compactor: ContextCompactor, // Compacts context
}

WorkerAgent {
    local_context: CompactContext, // Task-specific + blackboard slice
    blackboard_access: BlackboardHandle, // On-demand access
}

// When context changes:
Coordinator::update_context(changes) {
    // 1. Update blackboard
    blackboard.apply(changes);
    
    // 2. Notify affected agents (only diff, not full context)
    let affected = find_affected_agents(&changes);
    for agent in affected {
        let diff = compactor.compute_diff(agent, &changes);
        agent.update_context(diff); // Only send changes
    }
}
```

---

#### 10. Memory Architecture Integration

**Questions:**
- How does compacting integrate with Tripartite Memory (R-14)?
- Should Episodic Memory be compacted differently than Semantic?
- How does Coordinator compact worker contexts?

**Integration Points:**

##### Episodic Memory (Chronological)
- Store raw history externally (SQLite)
- Keep summarized version in-context
- Retrieve full details on demand

##### Semantic Memory (Factual)
- Already compact (embeddings)
- No compaction needed
- Retrieve relevant facts on demand

##### Procedural Memory (Skills)
- Compact learned skills (remove redundant steps)
- Keep only high-utility skills in-context
- Archive low-utility skills externally

---

## 📊 Competitive Analysis

### Research Each Competitor

For each system below, research:
1. Do they compact context? How?
2. How do they share context between agents?
3. How do they handle context updates?
4. What compression ratios achieved?
5. Open source implementation?

**Systems:**
- MemGPT / Letta
- LangChain (ConversationalSummaryBufferMemory)
- LangGraph (state sharing)
- AutoGen (group chat context)
- CrewAI (crew context sharing)
- OpenDevin / OpenHands
- Dify
- Devin (if info available)
- Cursor (if info available)

---

## 📋 Research Plan

### Phase 1: Literature Review (1.5 hours)
- [ ] Search for "LLM context compression" papers
- [ ] Search for "multi-agent context sharing" papers
- [ ] Review arXiv for recent work (2024-2025)
- [ ] Check ACL, NeurIPS, ICLR, AAMAS proceedings

### Phase 2: Industry Analysis (1.5 hours)
- [ ] Research MemGPT/Letta approach (memory + context)
- [ ] Research LangGraph approach (state sharing)
- [ ] Research AutoGen approach (group chat)
- [ ] Research proprietary systems (Claude, GPT-4, Devin)

### Phase 3: Technique Evaluation (1 hour)
- [ ] Compare compression ratios
- [ ] Compare context sharing patterns
- [ ] Identify best practices
- [ ] Identify anti-patterns

### Phase 4: AXORA Recommendations (1 hour)
- [ ] Which techniques to adopt?
- [ ] Implementation priority
- [ ] Sprint breakdown
- [ ] Integration with Memory Architecture (R-14)
- [ ] Integration with Coordinator (DADD)

---

## 📊 Deliverables

### 1. Research Findings

**File:** `research/findings/context-compacting/R-15-result.md`

**Content:**
- Definition of context compacting
- Core techniques (categorized)
- Industry implementations
- Compression metrics & targets
- Context sharing patterns
- Context versioning strategies

### 2. Technique Comparison

**File:** `research/findings/context-compacting/TECHNIQUE-COMPARISON.md`

**Content:**
- Side-by-side comparison (compacting techniques)
- Side-by-side comparison (sharing patterns)
- Compression ratios
- Quality trade-offs
- Implementation complexity

### 3. Architecture Recommendation

**File:** `research/findings/context-compacting/ARCHITECTURE-RECOMMENDATION.md`

**Content:**
- Recommended architecture for AXORA
- Coordinator + Blackboard design
- Context compaction integration
- Context sharing protocol
- Context versioning strategy

### 4. Implementation Plan

**File:** `research/findings/context-compacting/IMPLEMENTATION-PLAN.md`

**Content:**
- Sprint breakdown
- Estimated effort
- Dependencies
- Integration with Memory Architecture
- Integration with Coordinator (DADD)

---

## ✅ Success Criteria

Research is successful when:
- [ ] Context compacting clearly defined
- [ ] 5+ compacting techniques identified and compared
- [ ] 3+ context sharing patterns identified
- [ ] 3+ context versioning strategies identified
- [ ] 5+ industry implementations analyzed
- [ ] Compression ratios quantified (targets set)
- [ ] Quality trade-offs documented
- [ ] Recommended architecture for AXORA defined
- [ ] Implementation plan created
- [ ] Integration with Memory Architecture defined
- [ ] Integration with Coordinator (DADD) defined

---

## 🔗 Related Research

- [R-14: Memory Architecture](./14-memory-architecture.md) — Episodic/Semantic/Procedural
- [R-09: Documentation Management](./09-documentation-management.md) — Living docs
- [COORDINATOR-AND-DADD-THOUGHTS.md](../../planning/COORDINATOR-AND-DADD-THOUGHTS.md) — Coordinator + hierarchy
- [PROJECT-STATUS-AND-FUTURE.md](../../planning/PROJECT-STATUS-AND-FUTURE.md) — Future planning

---

## 🚨 Why This Is CRITICAL

**Without context management:**
- Agents die after 10-20 turns (context overflow)
- Costs explode ($10-50 per session)
- Quality degrades (context pollution)
- Can't run long-running tasks
- Can't coordinate multiple agents efficiently

**With context management:**
- Agents run 100+ turns (compacting)
- Costs reduced 60-80% ($2-10 per session)
- Quality maintained (relevant context only)
- Can run multi-hour tasks
- Can coordinate 100+ agents efficiently

**This is the difference between "toy" and "production".**

---

**Ready to execute. This research enables production-ready long-running multi-agent systems.**
