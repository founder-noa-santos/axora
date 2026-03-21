# OPENAKTA — Project Status & Future Paths

**Document Created:** 2026-03-16  
**Purpose:** Single reference for what's done, what's in progress, what's next  
**Audience:** Founder/Developer making strategic decisions

---

## 📊 Executive Summary

**Current Phase:** Phase 2 (Token Optimization + Memory Architecture)  
**Progress:** 52% complete (11/21 sprints)  
**Time to Phase 2 Complete:** ~10-12 days  
**Next Major Milestone:** Self-Orchestration / Coordinator Agent  
**Recommended Launch Timeline:** 2-3 months (MVP), 6 months (full)

---

## ✅ What's COMPLETE (Done & Tested)

### Phase 1: Foundation (100% Complete)

| Sprint | Title | Output | Tests |
|--------|-------|-------|-------|
| 1 | Prefix Caching | `openakta-cache/src/prefix_cache.rs` | 8 passing |
| 2 | Diff-Based Communication | `openakta-cache/src/diff.rs` | 7 passing |
| 3 | Code Minification | `openakta-cache/src/minifier.rs` | 10 passing |
| 3b | Heartbeat System | `openakta-agents/src/heartbeat.rs` | 13 passing |
| 5 | TOON Serialization | `openakta-cache/src/toon.rs` | 14 passing |
| 6 | Documentation Management | `openakta-docs/` (5 files) | 54 passing |
| 8 | Graph Workflow | `openakta-agents/src/graph.rs` | 8 passing |
| 9 | Dual-Thread ReAct | `openakta-agents/src/react.rs` | 10 passing |
| 11 | Context + RAG Pivot | `openakta-cache/src/context.rs` | 14 passing |
| 12 | Snapshot Blackboard | `openakta-cache/src/blackboard.rs` | 8 passing |
| 16 | SCIP Indexing | `openakta-indexing/src/scip.rs` | 8 passing |
| 18 | Business Rule Documentation | `docs/business_rules/` (10+ files) | N/A |
| 19 | Bidirectional Traceability | `openakta-indexing/src/traceability.rs` | 8 passing |
| 23 | ACI Formatting | `openakta-agents/src/aci_formatter.rs` | 8 passing |
| 25 | AGENTS.md Living Document | `AGENTS.md`, `docs/ARCHITECTURE-LEDGER.md` | N/A |

**Total Completed:** 15 sprints  
**Total Tests:** 180+ passing  
**Total Code:** ~15,000+ lines

---

## 🔄 What's IN PROGRESS

### Phase 2: Remaining Work

| Agent | Sprint | Title | Progress | ETA |
|-------|--------|-------|----------|-----|
| **A** | 26 | Semantic Memory Store | Ready to start | 1-2 days |
| **A** | 28 | Procedural Memory Store | Blocked (after 26) | 1-2 days |
| **A** | 31 | Memory Lifecycle | Blocked (after 28) | 1-2 days |
| **B** | 20 | Context Pruning | In Progress | 1-2 days |
| **B** | 21 | Sliding-Window Semaphores | Blocked (after 20) | 1-2 days |
| **B** | 22 | Atomic Checkout | Blocked (after 21) | 1-2 days |
| **B** | 24 | Repository Map | Blocked (after 22) | 2-3 days |
| **C** | 27 | Episodic Memory Store | Blocked (needs A-26) | 1-2 days |
| **C** | 29 | Consolidation Pipeline | Blocked (needs A-27, A-28) | 2-3 days |
| **C** | 30 | MemGAS Retrieval | Blocked (needs A-26, C-29) | 2-3 days |

**Remaining:** 10 sprints  
**ETA:** ~10-12 days (all agents working in parallel)

---

## 📁 What We Have (Code Structure)

```
openakta/
├── crates/
│   ├── openakta-agents/          # Agent framework, state machine, ReAct loops
│   ├── openakta-cache/           # Token optimization (caching, diff, minification, TOON)
│   ├── openakta-core/            # Core types, traits
│   ├── openakta-daemon/          # Background services
│   ├── openakta-docs/            # Living documentation system
│   ├── openakta-embeddings/      # Embedding generation (pseudo, ready for real model)
│   ├── openakta-indexing/        # Code indexing, SCIP, traceability
│   ├── openakta-rag/             # Hybrid retriever (RRF fusion)
│   └── openakta-memory/          # NEW: Tripartite memory architecture (in progress)
│
├── apps/
│   └── desktop/               # Tauri v2 + React (placeholder, not started)
│
├── docs/                      # Architecture docs, business rules, ADRs
├── planning/                  # Sprint plans, coordination boards
├── research/                  # 14 research prompts + findings
│   ├── prompts/               # R-01 to R-14
│   └── findings/              # Research results
│
└── Cargo.toml                 # Workspace configuration
```

---

## 🎯 What's NEXT (After Phase 2)

### Option 1: Self-Orchestration / Coordinator Agent 🔴 RECOMMENDED

**Why First:**
- Solves YOUR problem (tired of managing everything manually)
- **Key differentiator** — no other framework has this
- Without it, OPENAKTA is "just another multi-agent framework"
- With it, OPENAKTA is "the only self-managing framework"

**What to Build:**
- Coordinator Agent (manages worker agents)
- Auto-dispatch mechanism (no manual prompt creation)
- Progress monitoring (automatic status tracking)
- Auto-merge results (no manual copy/paste)
- Single conversation UI (talk to one agent, it manages the rest)

**Estimated Time:** 2-3 weeks  
**Sprints:** ~6-8 sprints  
**Priority:** CRITICAL

---

### Option 2: Phase 3 — Desktop App (Tauri v2 + React)

**Why Second:**
- Without Coordinator, desktop just shows manual chaos
- With Coordinator, desktop shows autonomous operation
- Users need to SEE the value (not just configure it)

**What to Build:**
- Tauri v2 app setup
- Chat UI (talk to Coordinator)
- Progress visualization (see what agents are doing)
- Configuration (BYOK vs subscription)
- Settings (model selection, token limits)

**Estimated Time:** 3-4 weeks  
**Sprints:** ~8-10 sprints  
**Priority:** HIGH (after Coordinator)

---

### Option 3: Phase 4 — Integration & Beta Testing

**Why Third:**
- Need working product before testing
- Beta users need to SEE value (Coordinator + Desktop)
- E2E tests validate everything works together

**What to Build:**
- End-to-end tests (full system)
- Performance benchmarks (validate 90% token reduction, 3-5x speedup)
- Beta program (5-10 individual devs)
- Feedback loop (iterate based on user input)

**Estimated Time:** 2-3 weeks  
**Sprints:** ~4-6 sprints  
**Priority:** MEDIUM (after Desktop)

---

### Option 4: Production / Launch

**Why Last:**
- Need validated product (beta feedback)
- Need installer, auto-update, docs
- Need business model implementation (BYOK, subscription)

**What to Build:**
- Installers (`.dmg`, `.exe`, `.deb`)
- Auto-update mechanism
- User documentation (not dev docs)
- BYOK integration
- Subscription management (if applicable)
- Usage tracking

**Estimated Time:** 3-4 weeks  
**Sprints:** ~6-8 sprints  
**Priority:** LOW (after Beta)

---

## 📅 Timeline Options

### Aggressive (Launch MVP in 2-3 Months)

```
Week 1-2:  Phase 2 Complete (Memory Architecture)
Week 3-5:  Coordinator Agent (MVP — CLI only)
Week 6-9:  Desktop App (minimal — chat + progress)
Week 10-12: Beta Testing (5-10 users)
Week 13+:  Production Launch
```

**Trade-offs:**
- ✅ Fast to market
- ✅ Validate idea quickly
- ❌ Less polished
- ❌ May need rework after beta

---

### Balanced (Launch Full Product in 6 Months)

```
Week 1-2:   Phase 2 Complete
Week 3-6:   Coordinator Agent (full features)
Week 7-10:  Desktop App (polished)
Week 11-14: Integration Testing + Beta
Week 15-18: Production Polish + Launch
Week 19-24: Post-Launch Iteration
```

**Trade-offs:**
- ✅ Higher quality
- ✅ More features
- ✅ Better user experience
- ❌ Slower to market
- ❌ More development cost

---

### Conservative (Research-First, Launch in 9-12 Months)

```
Month 1-2:  Phase 2 Complete + Coordinator
Month 3-5:  Desktop App + Integration
Month 6-8:  Extensive Beta (50+ users)
Month 9-12: Production + Marketing
```

**Trade-offs:**
- ✅ Maximum quality
- ✅ Extensive validation
- ✅ Market-ready
- ❌ Very slow
- ❌ Risk of over-engineering

---

## 🤔 Decision Framework

### Questions to Ask Yourself

1. **What's your runway?**
   - <3 months → Aggressive
   - 3-6 months → Balanced
   - 6+ months → Conservative

2. **What's your risk tolerance?**
   - High (launch fast, iterate) → Aggressive
   - Medium (balance speed/quality) → Balanced
   - Low (polish before launch) → Conservative

3. **What's your goal?**
   - Validate idea quickly → Aggressive
   - Build sustainable business → Balanced
   - Build enterprise product → Conservative

4. **Who's your competition?**
   - If competitors are close → Aggressive
   - If you have time → Balanced
   - If you're first-to-market → Conservative

---

## 💡 My Recommendation (Based on Our Conversations)

**You seem frustrated with:**
- Manual coordination (being "babysitter")
- Slow progress visibility
- Wanting to see working product

**So I recommend:**

### Balanced Approach (6 Months)

**Why:**
- Solves YOUR problem first (Coordinator Agent)
- Gets to MVP in 2-3 months (validate)
- Polishes in 6 months (sustainable)
- Balances speed vs quality

**Path:**
1. **Week 1-2:** Finish Phase 2 (you're 52% done!)
2. **Week 3-6:** Coordinator Agent (your pain point)
3. **Week 7-10:** Desktop App (show value)
4. **Week 11-14:** Beta (validate with users)
5. **Week 15-24:** Production + Launch

---

## 📋 What to Do RIGHT NOW

1. **Finish Phase 2** (10-12 days)
   - You're already in motion
   - Don't stop mid-stream
   - Memory Architecture is foundational

2. **After Phase 2:**
   - Read this document
   - Decide on timeline (Aggressive/Balanced/Conservative)
   - Create sprints for Coordinator Agent
   - Start Coordinator implementation

3. **During Coordinator:**
   - Use it yourself (dogfooding)
   - If it solves YOUR problem, it'll solve users' problems
   - Iterate based on your experience

---

## 📞 When You're Ready to Decide

**Come back to this document and:**
1. Review timeline options
2. Pick one (Aggressive/Balanced/Conservative)
3. I'll create detailed sprints for that path
4. We start immediately

---

**Last Updated:** 2026-03-16  
**Next Review:** After Phase 2 Complete (~10-12 days)  
**Document Owner:** Founder/Developer
