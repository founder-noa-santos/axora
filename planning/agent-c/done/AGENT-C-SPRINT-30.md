# Agent C — Sprint 30: MemGAS Retrieval (GMM + Entropy Routing)

**Phase:** 2  
**Sprint:** 30 (Memory Architecture)  
**File:** `crates/openakta-memory/src/memgas_retriever.rs`  
**Priority:** HIGH (precise context curation)  
**Estimated Tokens:** ~100K output  

---

## 🎯 Task

Implement **MemGAS Retrieval** with GMM clustering and entropy-based routing.

### Context

R-14 research provides CRITICAL implementation details:
- **Multi-Granularity Retrieval** — Turn-level, session summaries, keyword clusters
- **GMM Clustering** — Accept Set vs Reject Set (not arbitrary top-K)
- **Entropy-Based Router** — Select optimal granularity dynamically
- **Association Graph** — Semantic edges between related memories

**Your job:** Implement MemGAS retriever (precise context curation).

---

## 📋 Deliverables

### 1. Create memgas_retriever.rs

**File:** `crates/openakta-memory/src/memgas_retriever.rs`

**Core Structure:**
```rust
//! MemGAS Retrieval
//!
//! This module implements multi-granularity retrieval:
//! - GMM clustering (Accept Set vs Reject Set)
//! - Entropy-based routing
//! - Association graph for semantic edges

use linfa::clustering::GaussianMixtureModel;
use ndarray::{Array1, Array2};

/// MemGAS retriever
pub struct MemGASRetriever {
    vector_store: VectorStore,
    gmm_model: GaussianMixtureModel,
    association_graph: MemoryAssociationGraph,
}

impl MemGASRetriever {
    /// Create new retriever
    pub fn new(vector_store: VectorStore) -> Result<Self> {
        let gmm_model = GaussianMixtureModel::default();
        
        Ok(Self {
            vector_store,
            gmm_model,
            association_graph: MemoryAssociationGraph::new(),
        })
    }
    
    /// Cluster memories using GMM (not static top-K)
    pub fn cluster_memories(&self, memories: &[Memory]) -> Result<GMMClustering> {
        // Convert memories to feature vectors
        let features = self.memories_to_features(memories)?;
        
        // Apply GMM clustering
        let clustering = self.gmm_model.fit(&features)?;
        
        // Separate into Accept Set (relevant) and Reject Set (noise)
        let accept_set = clustering
            .assignments
            .iter()
            .enumerate()
            .filter(|(_, &cluster)| cluster == 0)
            .map(|(i, _)| memories[i].clone())
            .collect();
        
        let reject_set = clustering
            .assignments
            .iter()
            .enumerate()
            .filter(|(_, &cluster)| cluster != 0)
            .map(|(i, _)| memories[i].clone())
            .collect();
        
        Ok(GMMClustering { accept_set, reject_set })
    }
    
    /// Entropy-based router selects optimal granularity
    pub fn retrieve(&self, query: &str, granularity: Granularity) -> Result<ContextPayload> {
        match granularity {
            Granularity::Auto => {
                let entropy = self.calculate_entropy(query)?;
                
                if entropy > HIGH_ENTROPY_THRESHOLD {
                    self.retrieve_by_summary(query)
                } else {
                    self.retrieve_by_turn(query)
                }
            }
            Granularity::TurnLevel => self.retrieve_by_turn(query),
            Granularity::SessionSummary => self.retrieve_by_summary(query),
            Granularity::KeywordCluster => self.retrieve_by_keywords(query),
        }
    }
}

pub enum Granularity {
    Auto,
    TurnLevel,
    SessionSummary,
    KeywordCluster,
}
```

---

## 📁 File Boundaries

**Create:**
- `crates/openakta-memory/src/memgas_retriever.rs` (NEW)

**Update:**
- `crates/openakta-cache/src/context.rs` (integrate MemGAS)

**DO NOT Edit:**
- `crates/openakta-cache/` (Agent B's domain)
- `crates/openakta-indexing/` (Agent B's domain)
- `crates/openakta-docs/` (Agent A's other work)

---

## 🧪 Tests Required

```rust
#[test]
fn test_gmm_clustering() { }
#[test]
fn test_accept_reject_sets() { }
#[test]
fn test_entropy_calculation() { }
#[test]
fn test_entropy_based_routing() { }
#[test]
fn test_context_manager_integration() { }
```

---

## ✅ Success Criteria

- [ ] `memgas_retriever.rs` created
- [ ] GMM clustering works
- [ ] Entropy-based routing works
- [ ] 5+ tests passing

---

## 🔗 References

- [`PHASE-2-INTEGRATION-MEMORY-ARCHITECTURE.md`](../shared/PHASE-2-INTEGRATION-MEMORY-ARCHITECTURE.md)

---

**⚠️ DEPENDENCIES:** Requires **A-26 (Semantic)** and **C-29 (Consolidation)** to be complete.

**Start AFTER Agent A completes Sprint 26 AND Agent C completes Sprint 29.**
