# Agent B — Sprint 21: Sliding-Window Semaphores

**Phase:** 2  
**Sprint:** 21 (Implementation)  
**File:** `crates/openakta-cache/src/concurrency.rs`  
**Priority:** HIGH (prevents resource starvation)  
**Estimated Tokens:** ~100K output  

---

## 🎯 Task

Implement **Sliding-Window Semaphore Concurrency** (from Dify pattern) for throttled task execution.

### Context

Competitive analysis validates our approach and provides CRITICAL implementation details:
- **Sliding-Window Semaphores** — Throttle concurrency based on resources
- **Pre-flight Token Calculator** — Prevent mid-flight token overflow
- **Dify Pattern** — Production-validated (20K+ stars)

**Your job:** Implement sliding-window concurrency (prevents resource starvation).

---

## 📋 Deliverables

### 1. Create concurrency.rs

**File:** `crates/openakta-cache/src/concurrency.rs`

**Core Structure:**
```rust
//! Sliding-Window Semaphore Concurrency
//!
//! This module implements production-grade concurrency throttling:
//! - Semaphore-based throttling (prevents resource starvation)
//! - Pre-flight token calculation (prevents mid-flight overflow)
//! - Dify pattern (validated in production)

use tokio::sync::Semaphore;
use std::sync::Arc;

/// Concurrent executor with sliding-window throttling
pub struct ConcurrentExecutor {
    // Sliding window semaphore (limits concurrent tasks)
    semaphore: Arc<Semaphore>,
    
    // Pre-flight token calculator
    token_calculator: TokenCalculator,
    
    // Configuration
    max_concurrent: usize,
    max_tokens_per_task: usize,
}

/// Pre-flight token calculator
pub struct TokenCalculator {
    // Token estimation model
    model: TokenEstimationModel,
}

impl TokenCalculator {
    pub fn new() -> Self {
        Self {
            model: TokenEstimationModel::default(),
        }
    }
    
    /// Estimate tokens for task (pre-flight check)
    pub fn estimate(&self, task: &Task) -> Result<usize> {
        // Estimate based on:
        // - Context size (influenced files + business rules)
        // - Expected output size (based on task complexity)
        let context_tokens = task.context.estimate_tokens();
        let output_tokens = self.model.estimate_output(&task.description)?;
        
        Ok(context_tokens + output_tokens)
    }
}

impl ConcurrentExecutor {
    /// Create new executor with throttling
    pub fn new(max_concurrent: usize, max_tokens_per_task: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            token_calculator: TokenCalculator::new(),
            max_concurrent,
            max_tokens_per_task,
        }
    }
    
    /// Execute tasks with sliding-window throttling
    pub async fn execute_with_throttle(
        &self,
        tasks: Vec<Task>,
    ) -> Result<Vec<TaskResult>> {
        let mut handles = Vec::new();
        
        for task in tasks {
            // Pre-flight token check (prevent mid-flight overflow)
            let estimated_tokens = self.token_calculator.estimate(&task)?;
            if estimated_tokens > self.max_tokens_per_task {
                return Err(Error::TokenBudgetExceeded {
                    estimated: estimated_tokens,
                    limit: self.max_tokens_per_task,
                });
            }
            
            // Acquire semaphore permit (throttles concurrency)
            let permit = self.semaphore.clone().acquire_owned().await?;
            
            // Spawn task (releases permit when complete)
            let handle = tokio::spawn({
                let task = task.clone();
                async move {
                    let result = execute_task(task).await;
                    drop(permit); // Release semaphore
                    result
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all tasks (sliding window ensures throughput)
        futures::future::try_join_all(handles).await
    }
}
```

---

### 2. Integrate with ContextManager

**File:** `crates/openakta-cache/src/context.rs` (UPDATE)

```rust
// Add to existing ContextManager
impl ContextManager {
    /// Allocate context with pre-flight token check
    pub fn allocate_with_budget_check(
        &mut self,
        task: &Task,
        agent: &Agent,
        max_tokens: usize,
    ) -> Result<TaskContext> {
        let context = self.allocate(task, agent)?;
        
        // Pre-flight check
        if context.estimate_tokens() > max_tokens {
            return Err(Error::ContextTooLarge {
                estimated: context.estimate_tokens(),
                limit: max_tokens,
            });
        }
        
        Ok(context)
    }
}
```

---

### 3. Add Configuration

**File:** `crates/openakta-cache/src/concurrency.rs` (add to existing)

```rust
/// Concurrency configuration
#[derive(Debug, Clone)]
pub struct ConcurrencyConfig {
    /// Max concurrent tasks (sliding window)
    pub max_concurrent: usize,
    
    /// Max tokens per task (pre-flight limit)
    pub max_tokens_per_task: usize,
    
    /// Rate limit (requests per minute)
    pub rate_limit_rpm: usize,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10, // Dify default
            max_tokens_per_task: 50_000, // 50K tokens per task
            rate_limit_rpm: 100, // 100 requests/minute
        }
    }
}
```

---

## 📁 File Boundaries

**Create:**
- `crates/openakta-cache/src/concurrency.rs` (NEW)

**Update:**
- `crates/openakta-cache/src/lib.rs` (add module export)
- `crates/openakta-cache/src/context.rs` (add budget check)

**DO NOT Edit:**
- `crates/openakta-agents/` (Agent C's domain)
- `crates/openakta-docs/` (Agent A's domain)

---

## 🧪 Tests Required

```rust
#[test]
fn test_semaphore_throttling() { }

#[test]
fn test_pre_flight_token_check() { }

#[test]
fn test_token_budget_exceeded() { }

#[test]
fn test_sliding_window_throughput() { }

#[test]
fn test_concurrent_task_execution() { }

#[test]
fn test_semaphore_permit_release() { }

#[test]
fn test_rate_limiting() { }

#[test]
fn test_context_budget_check() { }
```

---

## ✅ Success Criteria

- [ ] `concurrency.rs` created (sliding-window semaphores)
- [ ] Pre-flight token calculator works
- [ ] Semaphore throttling works (max concurrent enforced)
- [ ] Token budget check works (prevents overflow)
- [ ] Rate limiting works (requests/minute)
- [ ] 8+ tests passing
- [ ] Performance: No deadlock, no starvation

---

## 🔗 References

- [`PHASE-2-INTEGRATION-COMPETITIVE-ANALYSIS.md`](../shared/PHASE-2-INTEGRATION-COMPETITIVE-ANALYSIS.md) — Competitive analysis
- Research document — Dify pattern spec

---

**Start AFTER Sprint 17 (Influence Vector) is complete.**

**Priority: HIGH — prevents resource starvation in production.**

**Dependencies:**
- Sprint 17 (Influence Vector) — recommended but not required

**Blocks:**
- None (infrastructure improvement)
