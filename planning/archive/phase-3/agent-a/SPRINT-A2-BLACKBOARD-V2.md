# Phase 3 Sprint A2: Blackboard v2 (Versioned Shared State)

**Agent:** A (Documentation + Memory Specialist)  
**Sprint:** A2  
**Priority:** HIGH  
**Estimated:** 8 hours  
**Subagents:** ENABLED (GPT-5.4)

---

## 🎯 Mission

Implement Blackboard v2 with versioned context, subscribe/notify pattern, and atomic updates.

**Context:** Phase 2 Blackboard has snapshot-based state (TOCTOU vulnerabilities). Phase 3 needs versioned blackboard for conflict-free concurrent access.

---

## 📋 Subagents Assignment

**You are Lead Agent. Delegate to 2 subagents:**

### Subagent 1: Versioning System
**Task:** Implement versioned context with atomic updates
**File:** `crates/openakta-cache/src/blackboard/v2_versioning.rs`
**Deliverables:**
- `VersionedContext` struct
- `version()` returns current version
- `update_with_version()` atomic update with version check
- `get_since_version()` returns diff since version
- 5+ tests

### Subagent 2: Subscribe/Notify Pattern
**Task:** Implement subscribe/notify for real-time updates
**File:** `crates/openakta-cache/src/blackboard/v2_pubsub.rs`
**Deliverables:**
- `Subscriber` trait
- `subscribe(key)` → `SubscriptionId`
- `publish(key, value)` notifies all subscribers
- `unsubscribe(id)` removes subscription
- 5+ tests

---

## 🏗️ Lead Agent Responsibilities

**You must:**

1. **Coordinate Subagents:**
   - Assign tasks to 2 subagents
   - Review versioning + pubsub implementations
   - Ensure no race conditions

2. **Integrate Components:**
   - Create `crates/openakta-cache/src/blackboard/v2.rs` (main module)
   - Combine versioning + pubsub
   - Export unified `BlackboardV2` struct

3. **Implement Diff-Based Push:**
   - Compute diff between versions
   - Send only changes (not full context)
   - 60-80% reduction in update size

4. **Write Integration Tests:**
   - Test concurrent updates (no conflicts)
   - Test version tracking (no stale reads)
   - Test subscriber notifications (all notified)

5. **Update Documentation:**
   - Update `crates/openakta-cache/src/lib.rs`
   - Add Blackboard v2 examples

---

## 📐 Technical Spec

### Blackboard v2 Interface

```rust
pub struct BlackboardV2 {
    state: DashMap<String, VersionedValue>,
    subscribers: DashMap<String, Vec<Subscription>>,
    current_version: AtomicU64,
}

struct VersionedValue {
    value: serde_json::Value,
    version: u64,
    timestamp: u64,
}

pub struct Subscription {
    id: SubscriptionId,
    sender: mpsc::Sender<Update>,
}

pub struct Update {
    key: String,
    old_value: Option<serde_json::Value>,
    new_value: serde_json::Value,
    version: u64,
}

impl BlackboardV2 {
    pub fn new() -> Self;
    
    pub fn get(&self, key: &str) -> Option<serde_json::Value>;
    
    pub fn get_with_version(&self, key: &str, since_version: u64) -> Result<Update>;
    
    pub fn set(&self, key: &str, value: serde_json::Value) -> Result<u64>;
    
    pub fn subscribe(&self, key: &str) -> SubscriptionId;
    
    pub fn unsubscribe(&self, id: SubscriptionId);
    
    pub fn current_version(&self) -> u64;
}
```

### Atomic Update Algorithm

```
1. Read current version (V1)
2. Compute new value
3. Compare-and-swap: if version == V1, set new value + increment version
4. If CAS fails, retry from step 1
5. Notify all subscribers with diff
```

### Diff Computation

```
1. Serialize old and new values
2. Compute JSON diff (additions, deletions, modifications)
3. Serialize diff (typically 10-20% of full value)
4. Send diff to subscribers
```

---

## ✅ Success Criteria

**Sprint is done when:**

- [ ] 2 subagents complete their tasks
- [ ] Lead agent integrates all components
- [ ] `BlackboardV2` compiles and works
- [ ] 10+ tests passing (5 per subagent + 5 integration)
- [ ] Concurrent updates work (no conflicts)
- [ ] Version tracking works (no stale reads)
- [ ] Diff-based push achieves 80% size reduction
- [ ] Documentation updated

---

## 🔗 Dependencies

**Requires:**
- Sprint A1 complete (Context Compacting)

**Blocks:**
- Sprint C2 (Decomposition needs blackboard)
- Sprint C3 (Merging needs blackboard)

---

## 📝 Notes for GPT-5.4

**Subagent Pattern:**
```
Lead Agent:
  ├─ Subagent 1: Versioning (parallel)
  └─ Subagent 2: PubSub (parallel)
  ↓
Lead Agent: Integration + Diff + Tests
```

**Concurrency Concerns:**
- Use `DashMap` for concurrent access
- Use `AtomicU64` for version counter
- Use `mpsc` channels for notifications
- Test with 100+ concurrent updates

**Review Checklist:**
- [ ] No race conditions in versioning
- [ ] All subscribers notified
- [ ] Diff computation correct
- [ ] Memory leaks (unsubscribe works)

---

**Start AFTER Sprint A1 complete.**
