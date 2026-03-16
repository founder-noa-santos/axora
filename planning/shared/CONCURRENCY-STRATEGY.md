# Concurrency & Task Decomposition Strategy

**Date:** 2026-03-16  
**Status:** Research Ready (R-11)

---

## 🎯 Problem Statement

**Current AXORA behavior:**
- User gives complex mission → Single agent works sequentially
- Context window fills up quickly
- No parallelization
- Slow execution

**Desired AXORA behavior:**
- User gives complex mission → Automatically decomposed
- Multiple agents work **concurrently**
- Each agent has **minimal context**
- **3-5x faster** execution

---

## 📋 Two Key Documents Created

### 1. [`CONCURRENT-IMPLEMENTATION.md`](./CONCURRENT-IMPLEMENTATION.md)

**Purpose:** Enable **immediate parallel work** on Phase 2

**Wave 1 (Start NOW):**
- **Agent A:** Sprint 3 (Code Minification)
- **Agent B:** Sprint 5 (TOON Serialization)
- **Agent C:** Sprint 3b (Heartbeat)

**Why concurrent:**
- ✅ Different files (no overlap)
- ✅ No shared state
- ✅ Independent testing
- ✅ Low conflict risk

**Wave 2 (After Wave 1):**
- **Agent D:** Sprint 6a (Doc Format)
- **Agent E:** Sprint 6b (Doc RAG)
- **Agent F:** Sprint 6c (Living Docs)

**Dependencies:** 6a → 6b → 6c (staggered start)

---

### 2. [`research/prompts/11-concurrency-react-loops.md`](./research/prompts/11-concurrency-react-loops.md)

**Purpose:** Research **foundational patterns** for concurrent execution

**Research Questions:**
1. How to automatically decompose missions into concurrent tasks?
2. How to adapt ReAct loops for multi-agent scenarios?
3. How to distribute context across agents?
4. What coordination patterns work best?

**Proposed Patterns:**

#### Pattern 1: Mission Breakdown
```
Mission: "Implement authentication"
├─ Parallel Group 1 (concurrent)
│  ├─ Design schema (Architect)
│  ├─ Research best practices (Researcher)
│  └─ Set up structure (Coder)
├─ Parallel Group 2 (depends on 1)
│  ├─ Implement user model (Coder)
│  ├─ Implement JWT utils (Coder)
│  └─ Write tests (Tester)
└─ Parallel Group 3 (depends on 2)
   ├─ Login endpoint (Coder)
   ├─ Signup endpoint (Coder)
   └─ Integration tests (Tester)
```

#### Pattern 2: Multi-Agent ReAct Loop
```
Coordinator: Decompose mission → Assign to workers
Worker Agent Loop:
  Thought → Action → Observation → Repeat
Shared State: All agents see observations
Sync Points: Between parallel groups
```

#### Pattern 3: Context Distribution
- Each agent gets **minimal context** for its task
- **Shared state** for cross-task awareness
- **Pull-based** (agents request what they need)

---

## 🚀 Immediate Actions

### Action 1: Start Wave 1 (NOW)

**Dispatch 3 agents concurrently:**

**Agent A Prompt:** Sprint 3 (Code Minification)
- File: `crates/axora-cache/src/minifier.rs`
- Task: Implement whitespace removal, identifier compression
- Estimated: 8 hours

**Agent B Prompt:** Sprint 5 (TOON Serialization)
- File: `crates/axora-cache/src/toon.rs`
- Task: Implement TOON encoder/decoder
- Estimated: 8 hours

**Agent C Prompt:** Sprint 3b (Heartbeat)
- File: `crates/axora-agents/src/heartbeat.rs`
- Task: Implement hybrid heartbeat (timer + event)
- Estimated: 8 hours

**Coordination:** Minimal (different crates, no overlap)

---

### Action 2: Execute R-11 Research (Parallel)

**Research Agent:** R-11 (Concurrency + ReAct Loops)
- Estimated: 3-4 hours
- Deliverables:
  - Literature review
  - Industry analysis
  - Pattern designs
  - Implementation plan

**Why parallel:** Research doesn't block Wave 1 implementation

---

### Action 3: Prepare Wave 2 (After Wave 1)

**Agents D, E, F:** Stand by for documentation sprints
- Start after Wave 1 completes
- Staggered: 6a → 6b → 6c

---

## 📊 Timeline

```
Now (T+0h):
├─ Wave 1: Agents A, B, C start (concurrent)
└─ R-11 Research starts (parallel track)

T+8h:
├─ Wave 1 completes
├─ R-11 Research completes
└─ Wave 2 preparation

T+24h:
├─ Wave 2: Agents D, E, F start (staggered)
└─ Decision on DDD (based on R-10 validation)

T+48h:
├─ Wave 2 completes
└─ Integration testing
```

---

## ⚠️ Conflict Prevention

### File Boundaries (Wave 1)

**Agent A (Minification):**
```
crates/axora-cache/src/minifier.rs ← ONLY this file
```

**Agent B (TOON):**
```
crates/axora-cache/src/toon.rs ← ONLY this file
```

**Agent C (Heartbeat):**
```
crates/axora-agents/src/heartbeat.rs ← ONLY this file
```

**No overlap = No conflicts**

---

### Merge Strategy

1. **Each agent creates PR** when done
2. **Different reviewers** (Agent A reviews B, etc.)
3. **Run tests** before merging (`cargo test --workspace`)
4. **Merge to main** after approval

---

## 🎯 Success Metrics

**Wave 1 Success:**
- ✅ All 3 sprints complete in <8 hours
- ✅ 24+ new tests passing
- ✅ No merge conflicts
- ✅ No regressions

**R-11 Success:**
- ✅ Clear decomposition algorithm
- ✅ Multi-agent ReAct design
- ✅ Context distribution strategy
- ✅ Implementation plan (<40 hours)

---

## 📋 Agent Prompts Ready

**For Wave 1:** See `CONCURRENT-IMPLEMENTATION.md` for detailed prompts

**For R-11:** See `research/prompts/11-concurrency-react-loops.md`

**Each prompt includes:**
- Clear task description
- File boundaries
- Success criteria
- Estimated time
- Next sprint info (for continuity)

---

## ✅ Summary

**Ready to execute:**
1. **Wave 1:** 3 concurrent agents (Sprints 3, 5, 3b)
2. **R-11:** Research on concurrency patterns
3. **Wave 2:** Prepared (Sprints 6a, 6b, 6c)

**No conflicts expected** if agents stay in assigned files.

**Estimated completion:** 48 hours for all Wave 1 + R-11.

---

**Awaiting user confirmation to dispatch agents.**
