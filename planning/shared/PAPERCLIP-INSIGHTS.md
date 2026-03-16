# Paperclip Insights for AXORA

**Analyzed:** Paperclip (https://github.com/paperclipai/paperclip)  
**Date:** 2026-03-16  
**Context:** Orchestration platform for 20+ autonomous AI agents

---

## 🔍 Key Differences: Paperclip vs AXORA

| Aspect | Paperclip | AXORA |
|--------|-----------|-------|
| **Target Audience** | Teams running 20+ agents | Individual developers |
| **Deployment** | Cloud, multi-tenant | Local-first, cloud-optional |
| **Database** | PostgreSQL | SQLite |
| **Language** | Node.js/TypeScript (96%) | Rust |
| **Agent Model** | Hire external agents | 10 native agents + extensible |
| **Primary Value** | Orchestration at scale | Ready-to-use agent team |

**Key Insight:** Different audiences, different priorities.

---

## ✅ Adopted Insights

### 1. Budget Tracking Per Agent
**Added to:** Phase 2, Sprint 2 (Diff-Based Communication)

**What:** Track token costs per agent with automatic throttling

**Why Adopted:**
- BYOK users need cost visibility
- Aligns with AXORA's token efficiency differentiator
- Low implementation cost (add counters to existing message flow)

**Implementation:**
```rust
pub struct AgentBudget {
    agent_id: String,
    monthly_limit_tokens: usize,
    used_tokens: usize,
    throttled: bool,
}
```

---

### 2. Immutable Audit Logging
**Added to:** Phase 2, Sprint 3 (Code Minification)

**What:** Complete conversation tracing and immutable action logs

**Why Adopted:**
- Valuable for debugging even for individual devs
- Accountability for agent actions
- Low cost (append-only log to existing storage)

**Implementation:**
```rust
pub struct AuditLog {
    timestamp: u64,
    agent_id: String,
    action: String,
    task_id: Option<String>,
    result: String,
    hash: String, // Immutable verification
}
```

---

## ⏸️ Deferred Insights

### 1. Heartbeat System
**Status:** Backlog (Phase 4+)

**What:** Scheduled agent wake-ups for recurring work

**Why Deferred:**
- AXORA has 5-10 agents (not 20+)
- Current state machine supports Idle/Executing states
- Adds complexity without immediate benefit

**Revisit When:** We have 20+ agents or users request scheduled tasks.

---

### 2. Bring Your Own Agent (BYOA)
**Status:** Backlog (Phase 3+)

**What:** Allow external agents/runtimes alongside native agents

**Why Deferred:**
- Focus on delivering quality native agents first
- Adds integration complexity
- Native agents are a differentiator

**Revisit When:** Native agents are solid and users request specific external integrations.

---

## ❌ Rejected Insights

### 1. Multi-Company Architecture
**Status:** Rejected

**Why:** AXORA targets individual developers, not teams/orgs.

---

### 2. PostgreSQL
**Status:** Rejected

**Why:** AXORA is local-first. SQLite is the right choice.

---

### 3. Node.js/TypeScript Stack
**Status:** Rejected

**Why:** Rust is core to AXORA's value (performance, safety, local-first).

---

## 📊 Decision Summary

| Insight | Decision | Rationale |
|---------|----------|-----------|
| Budget tracking | ✅ Adopted | Aligns with token efficiency |
| Audit logging | ✅ Adopted | Debug/accountability value |
| Heartbeat system | ⏸️ Deferred | Overkill for 5-10 agents |
| BYOA | ⏸️ Deferred | Focus on native agents first |
| Multi-company | ❌ Rejected | Wrong audience |
| PostgreSQL | ❌ Rejected | Wrong architecture |
| Node.js stack | ❌ Rejected | Rust is core value |

---

## 🎯 Impact on AXORA Roadmap

**Phase 2 Adjustments:**
- Sprint 2: Added budget tracking
- Sprint 3: Added immutable audit logging

**No other changes to roadmap.**

**Total Additional Work:** ~4 hours (budget counters + append-only log)

**User Benefit:** Cost visibility + accountability

---

**Conclusion:** Paperclip has valuable patterns for **orchestration at scale**. AXORA focuses on **ready-to-use agent team for individuals**. We adopted what aligns (budget, audit), deferred what's premature (heartbeat, BYOA), and rejected what doesn't fit (multi-company, PostgreSQL).
