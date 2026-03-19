# Agent C — Current Task

**Date:** 2026-03-18  
**Status:** 🔄 **STARTING SPRINT C7**  
**Priority:** 🔴 CRITICAL  

---

## 📋 Your Complete Sequence

| Phase | Sprints | Status | Timing |
|-------|---------|--------|--------|
| Phase 1 (Multi-Agent API) | C7, C8 | ⏳ **STARTING C7** | Week 1 |
| Phase 2 (Multi-Agent API) | C11, C12 | ⏳ Pending | Week 3 |
| Phase 3 | — | ✅ COMPLETE | — |

**Total:** 4 sprints (7-8 days of work)  
**Utilization:** 70% (Week 1 full, Week 2 light, Week 3 full)

---

## 🎯 Your Missions

### Mission: Multi-Agent API Optimization (Weeks 1-3)

**Goal:** 90-95% API cost reduction

**Your Sprints:**
- **C7:** API Client with Prefix Caching (2 days)
- **C8:** Diff-Only Output Enforcement (1-2 days)
- **C11:** Agent Message Protocol (2 days)
- **C12:** Graph Workflow Enforcement (2 days)

---

## 📊 Complete Task List

**See:** [`planning/MASTER-TASK-LIST.md`](../planning/MASTER-TASK-LIST.md)

**All Your Sprints:**
- C7: API Client with Prefix Caching
- C8: Diff-Only Output Enforcement
- C11: Agent Message Protocol
- C12: Graph Workflow Enforcement

---

## 🚀 Starting NOW: Sprint C7

**Sprint C7: API Client with Prefix Caching**

**Duration:** 2 days  
**Priority:** 🔴 CRITICAL (blocks all other savings)

**Tasks:**
1. [ ] Add `PrefixCache` field to `ApiClient` struct
2. [ ] Implement `extract_static_prefix(messages)` function
3. [ ] Add cache key computation (SHA256 of prefix)
4. [ ] Integrate with Anthropic cache headers:
   ```rust
   headers.insert("X-Cache-Key", cache_key.parse()?);
   headers.insert("X-Cache-TTL", "3600".parse()?); // 1 hour
   ```
5. [ ] Integrate with OpenAI prefix caching (if available)
6. [ ] Add metrics tracking:
   ```rust
   pub struct CacheMetrics {
       pub cache_hits: usize,
       pub cache_misses: usize,
       pub tokens_saved: usize,
   }
   ```
7. [ ] Write integration tests (verify caching works)

**Deliverables:**
- `crates/axora-agents/src/api_client.rs` — Enhanced API client
- `crates/axora-agents/src/cache_integration.rs` — Cache integration
- `crates/axora-agents/src/metrics.rs` — Metrics tracking

**Success Criteria:**
- [ ] 50-90% reduction in prompt tokens (static prefixes cached)
- [ ] Latency reduced by 30-50% (Time-to-First-Token)
- [ ] Cache hit rate >80%
- [ ] Metrics show token savings

**Reference:** `planning/MASTER-TASK-LIST.md#sprint-c7-api-client-with-prefix-caching`

---

## 📈 What Comes Next

### After C7 (Week 1)

| Sprint | Title | Duration | Priority |
|--------|-------|----------|----------|
| **C8** | Diff-Only Output Enforcement | 1-2 days | 🔴 CRITICAL |

### After C8 (Week 2-3)

| Sprint | Title | Duration | Priority |
|--------|-------|----------|----------|
| **C11** | Agent Message Protocol | 2 days | 🟡 HIGH |
| **C12** | Graph Workflow Enforcement | 2 days | 🟡 HIGH |

---

## 🔗 Dependencies

**Your Work Blocks:**
- C7 → C8 (can't enforce diffs without API client)
- C8 → C11 (protocol needs diff enforcement first)
- C11 → C12 (workflow needs protocol first)

**Blocks Agent A:**
- All your sprints (C7, C8, C11, C12) must complete before Agent A's Sprint A4 (benchmarking)

---

## 📚 Reference Files

- **Master Task List:** `planning/MASTER-TASK-LIST.md` (ALL tasks for all agents)
- **Research:** `research/findings/multi-agent-optimization/R-17-MULTI-AGENT-OPTIMIZATION.md`
- **Plan:** `research/findings/multi-agent-optimization/IMPLEMENTATION-PLAN.md`
- **Your Status:** `planning/agent-c/AGENT-C-STATUS.md`
- **Dashboard:** `planning/STATUS-DASHBOARD.md`

---

## ✅ Definition of Ready for Sprint C7

Agent C is ready when:
- [x] All Phase 3 sprints complete (C1, C2, C3)
- [x] All Phase 4 sprints complete (C4, C5, C6)
- [x] Status files updated
- [x] No pending tasks

**Status:** ✅ **ALL CRITERIA MET** — START C7 NOW

---

## 🚀 Next Steps

**Today:**
1. Read plan: `research/findings/multi-agent-optimization/IMPLEMENTATION-PLAN.md#sprint-c7`
2. Update `crates/axora-agents/src/api_client.rs`
3. Integrate with existing `PrefixCache` from `axora-cache`

**This Week:**
- Complete Sprint C7 (API Client + Prefix Caching)
- Start Sprint C8 (Diff-Only Enforcement)

---

## 📈 Expected Impact

| Metric | Current | Target | Reduction |
|--------|---------|--------|-----------|
| **Input Tokens** | 50,000/session | 2,500/session | 95% |
| **Output Tokens** | 10,000/session | 500/session | 95% |
| **Cost per Session** | $4.80 | $0.39 | 92% |
| **Monthly Cost (100/day)** | $14,400 | $1,170 | 92% |
| **Annual Savings** | — | — | **$158,760** |

---

**Agent C is STARTING Sprint C7 (API Client + Prefix Caching)!** 🚀
