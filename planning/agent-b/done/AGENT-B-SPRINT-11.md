# Agent B — Sprint 11: Context Distribution Pivot (Graph + RAG)

**Sprint:** 11 of Phase 2  
**File:** `crates/openakta-cache/src/context.rs` + `crates/openakta-cache/src/rag.rs`  
**Estimated Tokens:** ~80K output tokens  

---

## 🎯 Task

Pivot Context Distribution from DDD-based to **Graph-Based + Domain RAG**.

### Context

R-10 research proved:
- DDD bounded contexts are OVER-ENGINEERING
- Domain knowledge should be in RAG, not agent structure
- Coordination must be O(N), not O(N²)

**Your job:** Update Context Distribution to use Graph + RAG pattern.

---

## 📋 Deliverables

### 1. Refactor context.rs

**Remove:**
- Any DDD-specific code (bounded contexts, domain teams)
- Complex cross-domain routing logic

**Keep:**
- `ContextManager` struct
- `TaskContext` struct
- `allocate()` / `merge()` methods

**Add:**
```rust
pub struct ContextManager {
    shared_context: SharedContext,
    task_contexts: HashMap<TaskId, TaskContext>,
    
    // NEW: Domain RAG (not domain agents)
    domain_rag: DomainRagStore,  // Vector stores per domain
}

pub struct DomainRagStore {
    // One vector store per domain
    domains: HashMap<DomainId, VectorStore>,
    
    // Late-interaction retrieval (ColBERT-style)
    retrieval_strategy: RetrievalStrategy,
}

impl ContextManager {
    pub fn allocate(&mut self, task: &Task, agent: &Agent) -> TaskContext {
        // Extract mentioned domains from task
        let domains = self.extract_domains(task);
        
        // Retrieve domain knowledge (RAG, not agents)
        let mut context = Vec::new();
        for domain in domains {
            let rag_results = self.domain_rag.retrieve(&domain, task.query).await?;
            context.extend(rag_results);
        }
        
        // Allocate minimal context (only retrieved knowledge)
        TaskContext::new(context)
    }
}
```

---

### 2. Create rag.rs (NEW FILE)

**File:** `crates/openakta-cache/src/rag.rs`

**Purpose:** Domain RAG implementation (Experience-as-Parameters)

**Structure:**
```rust
//! Domain RAG (Retrieval-Augmented Generation)
//!
//! This module implements "Experience-as-Parameters" pattern:
//! - Domain knowledge is in vector stores, not agent structure
//! - Agents are generalists with domain-specific retrieval
//! - Coordination is O(N), not O(N²)

use qdrant_client::qdrant::*;
use serde::{Deserialize, Serialize};

/// Domain-specific vector store
#[derive(Debug, Clone)]
pub struct DomainRagStore {
    domains: HashMap<String, VectorStore>,
    strategy: RetrievalStrategy,
}

/// Retrieval strategy (late-interaction, hybrid, etc.)
#[derive(Debug, Clone)]
pub enum RetrievalStrategy {
    /// Dense vectors only (semantic)
    DenseOnly,
    /// Hybrid: BM25 + dense vectors
    Hybrid,
    /// Late-interaction (ColBERT-style)
    LateInteraction,
}

impl DomainRagStore {
    /// Create new domain RAG store
    pub fn new(strategy: RetrievalStrategy) -> Self;
    
    /// Add domain knowledge
    pub fn add_domain(&mut self, domain_id: &str, store: VectorStore);
    
    /// Retrieve domain knowledge
    pub async fn retrieve(&self, domain: &str, query: &str, k: usize) -> Result<Vec<RagResult>>;
    
    /// Add past success to memory (Experience-as-Parameters)
    pub async fn add_experience(&mut self, domain: &str, task: &str, result: &str);
}

/// Past success memory (Experience-as-Parameters)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub task_description: String,
    pub successful_pattern: String,
    pub reasoning_trace: String,
    pub timestamp: u64,
    pub domain: String,
}
```

**Key Functions:**
- `retrieve()` — Hybrid search (BM25 + vectors)
- `add_experience()` — Store past successes
- `late_interaction_retrieve()` — ColBERT-style retrieval (optional, advanced)

---

### 3. Update DocHealth Metrics

**File:** `crates/openakta-cache/src/living.rs` (expand existing)

**Add:**
```rust
pub struct DocHealth {
    // Existing metrics
    pub coverage_ratio: f32,
    pub freshness_index: f32,
    pub stale_percentage: f32,
    
    // NEW: RAG-specific metrics
    pub retrieval_precision_at_k: f32,  // Precision@k for RAG
    pub retrieval_recall_at_k: f32,     // Recall@k for RAG
    pub retrieval_latency_ms: f32,      // Time to retrieve
    
    // NEW: Graph workflow metrics
    pub coordination_overhead: f32,     // Should be O(N), track actual
    pub token_efficiency: f32,          // <10% overhead target
}
```

---

### 4. Add Hybrid Search Implementation

**File:** `crates/openakta-cache/src/rag.rs` (add to existing)

```rust
impl DomainRagStore {
    /// Hybrid search: BM25 (lexical) + dense vectors (semantic)
    pub async fn hybrid_retrieve(
        &self,
        domain: &str,
        query: &str,
        k: usize,
    ) -> Result<Vec<RagResult>> {
        // Parallel retrieval
        let (vector_results, keyword_results) = tokio::join!(
            self.vector_search(domain, query, k),
            self.keyword_search(domain, query, k),
        );
        
        // Merge + rerank
        let merged = self.rerank_and_merge(
            vector_results?,
            keyword_results?,
        );
        
        Ok(merged)
    }
    
    /// Rerank with cross-encoder (optional, for high precision)
    fn rerank_and_merge(&self, vector: Vec<RagResult>, keyword: Vec<RagResult>) -> Vec<RagResult> {
        // Reciprocal Rank Fusion
        // Then cross-encoder reranking (top 20 → top 10)
    }
}
```

---

## 📁 File Boundaries

**Create:**
- `crates/openakta-cache/src/rag.rs` (NEW)

**Update:**
- `crates/openakta-cache/src/context.rs` (refactor to use RAG)
- `crates/openakta-cache/src/living.rs` (expand DocHealth)

**DO NOT Edit:**
- `crates/openakta-agents/` (Agent C's domain)
- `crates/openakta-docs/` (Agent A's domain)

---

## 🧪 Tests Required

```rust
#[test]
fn test_domain_rag_retrieve() { }

#[test]
fn test_hybrid_search_precision() { }

#[test]
fn test_experience_as_parameters() { }

#[test]
fn test_context_allocation_with_rag() { }

#[test]
fn test_token_efficiency_vs_ddd() { }

#[test]
fn test_coordination_overhead_linear() { }

#[test]
fn test_late_interaction_retrieval() { }

#[test]
fn test_doc_health_rag_metrics() { }
```

---

## ✅ Success Criteria

- [ ] `context.rs` refactored (no DDD code, uses RAG)
- [ ] `rag.rs` created (Domain RAG implementation)
- [ ] Hybrid search implemented (BM25 + vectors)
- [ ] DocHealth metrics expanded (RAG-specific)
- [ ] 8+ tests passing
- [ ] Token overhead <10% (vs 40%+ for DDD)
- [ ] Coordination overhead O(N) (not O(N²))

---

## 🔗 References

- [`PHASE-2-PIVOT-GRAPH-WORKFLOW.md`](../shared/PHASE-2-PIVOT-GRAPH-WORKFLOW.md) — Main pivot doc
- [`research/prompts/10-ddd-agents-validation.md`](../research/prompts/10-ddd-agents-validation.md) — R-10 research
- [`research/prompts/13-influence-graph-business-rules.md`](../research/prompts/13-influence-graph-business-rules.md) — RAG patterns

---

**Start NOW. Focus on RAG-based domain knowledge, not DDD-based agent structure.**
