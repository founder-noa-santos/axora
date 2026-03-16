# Agent B — Sprint 20: Context Pruning

**Phase:** 2  
**Sprint:** 20 (Implementation)  
**File:** `crates/axora-cache/src/context_pruning.rs`  
**Priority:** HIGH (depends on Sprint 17)  
**Estimated Tokens:** ~100K output  

---

## 🎯 Task

Implement **Context Pruning** using Influence Graph for 95-99% token reduction.

### Context

Research provides CRITICAL implementation details:
- **Deterministic Graph Traversal** — Not semantic search (exact dependencies)
- **Influence Vector** — Pre-calculated impact metrics
- **Business Rules** — Explicitly linked constraints
- **Token Reduction:** 50,000+ → 500-2,500 tokens (95-99% savings)

**Your job:** Implement context pruning (depends on Sprint 17 Influence Vector).

---

## 📋 Deliverables

### 1. Create context_pruning.rs

**File:** `crates/axora-cache/src/context_pruning.rs`

**Core Structure:**
```rust
//! Context Pruning
//!
//! This module implements deterministic context pruning using Influence Graph:
//! - Graph traversal (not semantic search)
//! - Influence vectors (pre-calculated impact)
//! - Business rules (explicitly linked)
//! - 95-99% token reduction

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use crate::influence::{InfluenceGraph, InfluenceVector};
use crate::traceability::TraceabilityMatrix;

/// Context Manager (prunes context using influence graph)
pub struct ContextManager {
    influence_graph: InfluenceGraph,
    traceability_matrix: TraceabilityMatrix,
    business_rules_path: PathBuf,
}

impl ContextManager {
    /// Create new context manager
    pub fn new(
        influence_graph: InfluenceGraph,
        traceability_matrix: TraceabilityMatrix,
        business_rules_path: PathBuf,
    ) -> Self {
        Self {
            influence_graph,
            traceability_matrix,
            business_rules_path,
        }
    }
    
    /// Allocate context for task (pruned, not brute-force)
    pub fn allocate(&mut self, task: &Task, agent: &Agent) -> Result<TaskContext> {
        // 1. Extract mentioned files (lexical matching)
        let mentioned_files = self.extract_mentioned_files(task)?;
        
        // 2. Get influence vectors (pre-calculated)
        let mut influenced_files = HashSet::new();
        for file in &mentioned_files {
            if let Some(vector) = self.influence_graph.get_vector(file.to_file_id()) {
                influenced_files.extend(vector.get_affected_files());
            }
        }
        
        // 3. Extract business rules (bidirectional links)
        let business_rules = self.get_applicable_business_rules(&influenced_files)?;
        
        // 4. Prune context (only mathematically proven dependencies)
        let context = TaskContext::new(influenced_files, business_rules);
        
        // Log token reduction
        let original_tokens = self.estimate_brute_force_tokens(&mentioned_files)?;
        let pruned_tokens = context.estimate_tokens();
        let savings = ((original_tokens - pruned_tokens) as f32 / original_tokens as f32) * 100.0;
        
        tracing::info!(
            "Context pruning: {} → {} tokens ({:.1}% savings)",
            original_tokens,
            pruned_tokens,
            savings
        );
        
        Ok(context)
    }
}
```

---

### 2. Implement Deterministic Graph Traversal

**File:** `crates/axora-cache/src/context_pruning.rs` (add to existing)

```rust
impl ContextManager {
    /// Extract mentioned files (lexical matching, not LLM)
    fn extract_mentioned_files(&self, task: &Task) -> Result<HashSet<PathBuf>> {
        let mut files = HashSet::new();
        
        // Extract file paths from task description (regex)
        let path_regex = Regex::new(r"(src/[\w/]+\.(?:rs|ts|py|go|js|jsx|tsx))")?;
        
        for cap in path_regex.captures_iter(&task.description) {
            let file_path = PathBuf::from(&cap[1]);
            if file_path.exists() {
                files.insert(file_path);
            }
        }
        
        // Extract function/class names (for symbol-based lookup)
        let symbol_regex = Regex::new(r"(\w+)::(\w+)")?;
        for cap in symbol_regex.captures_iter(&task.description) {
            let module = &cap[1];
            let symbol = &cap[2];
            
            // Lookup symbol in influence graph
            if let Some(file_id) = self.influence_graph.lookup_symbol(module, symbol) {
                if let Some(file_path) = self.influence_graph.get_file_path(file_id) {
                    files.insert(file_path.to_path_buf());
                }
            }
        }
        
        Ok(files)
    }
    
    /// Get applicable business rules (from traceability matrix)
    fn get_applicable_business_rules(
        &self,
        influenced_files: &HashSet<PathBuf>,
    ) -> Result<HashSet<BusinessRule>> {
        let mut rules = HashSet::new();
        
        for file in influenced_files {
            // Get rules for this file
            let file_rules = self.traceability_matrix.get_rules_for_code(file);
            
            for link in file_rules {
                // Load business rule from file
                let rule = self.load_business_rule(&link.rule_id)?;
                rules.insert(rule);
            }
        }
        
        Ok(rules)
    }
}
```

---

### 3. Implement TaskContext

**File:** `crates/axora-cache/src/context_pruning.rs` (add to existing)

```rust
/// Task Context (pruned, minimal)
pub struct TaskContext {
    /// Source files (only influenced files)
    pub source_files: HashSet<PathBuf>,
    
    /// Business rules (only applicable rules)
    pub business_rules: HashSet<BusinessRule>,
    
    /// Token count (for monitoring)
    pub token_count: usize,
}

impl TaskContext {
    /// Create new task context
    pub fn new(
        source_files: HashSet<PathBuf>,
        business_rules: HashSet<BusinessRule>,
    ) -> Self {
        let token_count = Self::estimate_tokens(&source_files, &business_rules);
        
        Self {
            source_files,
            business_rules,
            token_count,
        }
    }
    
    /// Estimate token count
    fn estimate_tokens(
        source_files: &HashSet<PathBuf>,
        business_rules: &HashSet<BusinessRule>,
    ) -> usize {
        let mut total = 0;
        
        // Source files (~100 tokens per file average)
        for file in source_files {
            if let Ok(content) = std::fs::read_to_string(file) {
                total += content.len() / 4; // Rough estimate
            }
        }
        
        // Business rules (~200 tokens per rule average)
        for rule in business_rules {
            total += rule.content.len() / 4;
        }
        
        total
    }
    
    /// Get token count
    pub fn estimate_tokens(&self) -> usize {
        self.token_count
    }
}

/// Business Rule (loaded from Markdown file)
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct BusinessRule {
    pub rule_id: String,
    pub title: String,
    pub category: String,
    pub severity: String,
    pub content: String,
}
```

---

### 4. Add Token Reduction Benchmarking

**File:** `crates/axora-cache/src/context_pruning.rs` (add to existing)

```rust
/// Token reduction benchmark
pub struct TokenReductionBenchmark {
    pub original_tokens: usize,
    pub pruned_tokens: usize,
    pub savings_percentage: f32,
}

impl ContextManager {
    /// Benchmark token reduction
    pub fn benchmark_token_reduction(&self, task: &Task) -> Result<TokenReductionBenchmark> {
        // Estimate brute-force tokens (full directories)
        let mentioned_files = self.extract_mentioned_files(task)?;
        let original_tokens = self.estimate_brute_force_tokens(&mentioned_files)?;
        
        // Get pruned tokens
        let context = self.allocate(task, &Agent::dummy())?;
        let pruned_tokens = context.estimate_tokens();
        
        // Calculate savings
        let savings_percentage = ((original_tokens - pruned_tokens) as f32 / original_tokens as f32) * 100.0;
        
        Ok(TokenReductionBenchmark {
            original_tokens,
            pruned_tokens,
            savings_percentage,
        })
    }
    
    /// Estimate brute-force tokens (full directories)
    fn estimate_brute_force_tokens(&self, mentioned_files: &HashSet<PathBuf>) -> Result<usize> {
        let mut total = 0;
        
        for file in mentioned_files {
            // Get parent directory
            if let Some(parent) = file.parent() {
                // Count all files in directory (brute-force approach)
                for entry in walkdir::WalkDir::new(parent) {
                    let entry = entry?;
                    if entry.path().extension().map_or(false, |ext| is_code_extension(ext)) {
                        if let Ok(content) = std::fs::read_to_string(entry.path()) {
                            total += content.len() / 4;
                        }
                    }
                }
            }
        }
        
        Ok(total)
    }
}
```

---

### 5. Integrate with Existing Context Manager

**File:** `crates/axora-cache/src/context.rs` (UPDATE from Sprint 11)

```rust
// Add to existing ContextManager
impl ContextManager {
    /// Use influence graph for context pruning (if available)
    pub fn allocate_with_influence_graph(
        &mut self,
        task: &Task,
        agent: &Agent,
    ) -> Result<TaskContext> {
        if let Some(pruner) = &self.context_pruner {
            // Use influence graph (95-99% savings)
            pruner.allocate(task, agent)
        } else {
            // Fallback to RAG-based (60-80% savings)
            self.allocate_legacy(task, agent)
        }
    }
}
```

---

## 📁 File Boundaries

**Create:**
- `crates/axora-cache/src/context_pruning.rs` (NEW)

**Update:**
- `crates/axora-cache/src/lib.rs` (add module export)
- `crates/axora-cache/src/context.rs` (integrate pruning)

**DO NOT Edit:**
- `crates/axora-agents/` (Agent C's domain)
- `crates/axora-docs/` (Agent A's domain)

**Dependencies:**
- Sprint 17 (Influence Vector Calculation) — must complete first

---

## 🧪 Tests Required

```rust
#[test]
fn test_mentioned_file_extraction() { }

#[test]
fn test_influence_vector_lookup() { }

#[test]
fn test_business_rule_extraction() { }

#[test]
fn test_context_pruning() { }

#[test]
fn test_token_reduction_benchmark() { }

#[test]
fn test_95_percent_savings() { }

#[test]
fn test_deterministic_graph_traversal() { }

#[test]
fn test_fallback_to_rag() { }
```

---

## ✅ Success Criteria

- [ ] `context_pruning.rs` created (deterministic pruning)
- [ ] Graph traversal works (not semantic search)
- [ ] Business rules extracted correctly
- [ ] Token reduction benchmark works
- [ ] 95-99% token savings achieved
- [ ] Fallback to RAG works (if influence graph unavailable)
- [ ] 8+ tests passing
- [ ] Performance: <10ms for context allocation

---

## 🔗 References

- [`AGENT-B-SPRINT-17.md`](./AGENT-B-SPRINT-17.md) — Influence Vector (dependency)
- [`PHASE-2-INTEGRATION-INFLUENCE-GRAPH.md`](../shared/PHASE-2-INTEGRATION-INFLUENCE-GRAPH.md) — Integration doc
- Research document — Context pruning spec

---

**Start AFTER Sprint 17 (Influence Vector Calculation) is complete.**

**Priority: HIGH — this is the payoff for all the influence graph work.**

**Dependencies:**
- Sprint 17 (Influence Vector Calculation) — must complete first

**Blocks:**
- None (final step in context optimization chain)
