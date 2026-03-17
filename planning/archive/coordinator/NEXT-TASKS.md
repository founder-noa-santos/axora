# Next Tasks — Ready to Dispatch

**Generated:** 2026-03-16  
**Purpose:** Quick reference for coordinator to dispatch next tasks

---

## 🎯 Immediate Dispatch (Agents Free NOW)

### Agent A — FREE ✅

**Dispatch:** Sprint 18  
**File:** `agent-a/AGENT-A-SPRINT-18.md`  
**Title:** Business Rule Documentation  
**Priority:** 🔴 **HIGH** (blocks C-19)  
**Estimated:** ~70K tokens  
**Dependencies:** None  
**Blocks:** C-19 (Agent C Sprint 19)

**Why Urgent:**
- Agent A is IDLE
- Blocks Agent C (also IDLE)
- No dependencies, can start immediately

**Command to Dispatch:**
```
Agent A: Start Sprint 18 (Business Rule Documentation)
File: agent-a/AGENT-A-SPRINT-18.md
Priority: HIGH (blocks C-19)
```

---

### Agent C — FREE but BLOCKED ⚠️

**Status:** BLOCKED (waiting for A-18)  
**Next Sprint:** 19 (blocked by A-18)  
**Alternative:** None available

**Action:**
- **Cannot dispatch yet** (waiting for A-18)
- Monitor A-18 progress
- Dispatch C-19 immediately when A-18 complete

**Command to Dispatch (when A-18 complete):**
```
Agent C: Start Sprint 19 (Bidirectional Traceability)
File: agent-c/AGENT-C-SPRINT-19.md
Priority: MEDIUM
Unblocked by: A-18 complete
```

---

## 🔄 In Progress (Monitor Only)

### Agent B — BUSY 🔄

**Current:** Sprint 16 (SCIP Indexing)  
**Status:** IN PROGRESS  
**Next:** Sprint 17 (after 16 complete)

**Action:**
- No dispatch needed (has clear pipeline)
- Monitor progress
- Prepare Sprint 17 for dispatch when 16 complete

**Command to Dispatch (when 16 complete):**
```
Agent B: Start Sprint 17 (Influence Vector Calculation)
File: agent-b/AGENT-B-SPRINT-17.md
Priority: HIGH (blocks 20)
Unblocked by: 16 complete
```

---

## 📊 Summary

| Agent | Status | Action Required |
|-------|--------|-----------------|
| **A** | ✅ FREE | **DISPATCH 18 NOW** |
| **B** | 🔄 BUSY | Monitor (pipeline: 16 → 17 → 20) |
| **C** | ⚠️ BLOCKED | Wait for A-18, then dispatch 19 |

---

## 🚨 Blocker Resolution

### Active Blocker: C-19 ← A-18

**Resolution:**
1. ✅ Dispatch A-18 immediately (Agent A free)
2. 🔄 Monitor A-18 progress
3. 📋 Dispatch C-19 when A-18 complete

**ETA:**
- A-18: ~70K tokens (~1-2 days)
- C-19: Can start after A-18 complete

---

## 📋 Coordinator Checklist

**Immediate (Now):**
- [ ] Dispatch A-18 to Agent A
- [ ] Confirm Agent A started Sprint 18

**Monitor:**
- [ ] Check B-16 progress (daily)
- [ ] Prepare B-17 for dispatch (when 16 complete)

**Pending:**
- [ ] Dispatch C-19 when A-18 complete
- [ ] Dispatch B-17 when B-16 complete

---

**Generated:** 2026-03-16  
**Next Update:** When A-18 or B-16 complete
