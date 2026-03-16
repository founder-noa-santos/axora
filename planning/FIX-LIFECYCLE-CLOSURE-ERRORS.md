# Fix: lifecycle.rs Closure Trait Errors (Fn vs FnMut)

**Priority:** 🔴 BLOCKER (prevents compilation)  
**File:** `crates/axora-memory/src/lifecycle.rs`  
**Estimated Fix Time:** 5-10 minutes  

---

## Errors

```
error[E0596]: cannot borrow `deleted` as mutable, as it is a captured variable in a `Fn` closure
   --> crates/axora-memory/src/lifecycle.rs:781:17
    |
374 |           should_delete: impl Fn(&S) -> std::result::Result<bool, LifecycleError>,
    |                          -------------------------------------------------------- change this to accept `FnMut` instead of `Fn`
...
781 | |                 deleted.push(s.id.clone());
    | |                 ^^^^^^^ cannot borrow as mutable
```

**Same error at:**
- Line 781 (test_prune_procedural)
- Line 814 (test_prune_semantic)
- Line 859 (test_resolve_conflicts)

---

## Root Cause

**Function signatures accept `impl Fn(...)` but closures passed to them mutate captured variables.**

| Trait | Mutable Access | Immutable Captures |
|-------|---------------|-------------------|
| `Fn` | ❌ No | ✅ Yes |
| `FnMut` | ✅ Yes | ✅ Yes |
| `FnOnce` | ✅ Yes | ❌ Consumes self |

**The closures need `FnMut` because they mutate `deleted` vector:**
```rust
// Test code (line 781)
let mut deleted = Vec::new();
skills.prune_procedural(
    skills_vec,
    |skill| {
        deleted.push(skill.id.clone()); // ← MUTATES captured variable
        Ok(true)
    }
)
```

---

## Affected Functions

### 1. `prune_procedural<S: MemoryTrait + Clone>` (line 374)

**Current:**
```rust
pub async fn prune_procedural<S: MemoryTrait + Clone>(
    &self,
    skills: Vec<S>,
    should_delete: impl Fn(&S) -> std::result::Result<bool, LifecycleError>,
) -> Result<PruningReport>
```

**Fix:**
```rust
pub async fn prune_procedural<S: MemoryTrait + Clone>(
    &self,
    skills: Vec<S>,
    mut should_delete: impl FnMut(&S) -> std::result::Result<bool, LifecycleError>,
) -> Result<PruningReport>
```

---

### 2. `prune_semantic` (similar issue)

**Current:**
```rust
pub async fn prune_semantic(
    &self,
    should_delete: impl Fn(&SemanticMemory) -> Result<bool, LifecycleError>,
) -> Result<PruningReport>
```

**Fix:**
```rust
pub async fn prune_semantic(
    &self,
    mut should_delete: impl FnMut(&SemanticMemory) -> Result<bool, LifecycleError>,
) -> Result<PruningReport>
```

---

### 3. `resolve_conflicts` (line 447)

**Current:**
```rust
pub async fn resolve_conflicts(
    &self,
    should_delete: impl Fn(&str) -> Result<bool, LifecycleError>,
) -> Result<ConflictResolutionReport>
```

**Fix:**
```rust
pub async fn resolve_conflicts(
    &self,
    mut should_delete: impl FnMut(&str) -> Result<bool, LifecycleError>,
) -> Result<ConflictResolutionReport>
```

---

## Why This Pattern

**The `should_delete` closure needs to mutate external state:**

```rust
// Common pattern for pruning/garbage collection
let mut deleted = Vec::new(); // Track what was deleted (for reporting)

prune_fn(|item| {
    deleted.push(item.id.clone()); // ← Needs FnMut
    Ok(should_delete(item))
})
```

**This is correct design:**
1. Decide whether to delete based on criteria
2. Track what was deleted for reporting/auditing

**`FnMut` is the correct trait bound** since the closure has side effects (mutating the tracking vector).

---

## Action Items

1. [ ] Change `impl Fn(...)` to `impl FnMut(...)` in `prune_procedural` (line 374)
2. [ ] Change `impl Fn(...)` to `impl FnMut(...)` in `prune_semantic`
3. [ ] Change `impl Fn(...)` to `impl FnMut(...)` in `resolve_conflicts` (line 447)
4. [ ] Add `mut` keyword to all three parameter names
5. [ ] Run `cargo test -p axora-memory lifecycle` to verify fix
6. [ ] Verify all lifecycle tests pass

---

## Expected Result

**After fix:**
```
cargo test -p axora-memory lifecycle
   Compiling axora-memory v0.1.0
   Finished `test` profile [unoptimized + debuginfo]
   Running unittests src/lib.rs

running 10 tests
test lifecycle::tests::test_ebbinghaus_decay ... ok
test lifecycle::tests::test_memory_strength_calculation ... ok
test lifecycle::tests::test_prune_episodic ... ok
test lifecycle::tests::test_prune_procedural ... ok
test lifecycle::tests::test_prune_semantic ... ok
test lifecycle::tests::test_resolve_conflicts ... ok
test lifecycle::tests::test_utility_update ... ok
test lifecycle::tests::test_utility_calculation ... ok
test lifecycle::tests::test_combined_threshold_pruning ... ok
test lifecycle::tests::test_end_to_end_lifecycle ... ok

test result: ok. 10 passed; 0 failed
```

---

**This is a pre-existing bug in Sprint 31 implementation. Not related to Agent C's work. Fix and continue.**
