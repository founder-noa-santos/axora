# Phase 2 Integration: Memory Architecture (R-14)

**Date:** 2026-03-16  
**Source:** Research — R-14: Memory Architecture for AI Agent Systems  
**Impact:** CRITICAL — Enables compounding autonomy, 90% token reduction for repetitive tasks  

---

## ✅ Executive Summary

**This research provides the FOUNDATION for true agent learning and compounding expertise.**

**Key Insights:**
1. ✅ **Tripartite Memory Architecture** — Semantic, Episodic, Procedural
2. ✅ **Hybrid Storage** — Vector DB (semantic) + SQLite (episodic) + File-system (procedural)
3. ✅ **Consolidation Pipeline** — Episodic → Procedural (experience replay)
4. ✅ **MemGAS Retrieval** — GMM clustering + entropy-based routing
5. ✅ **Memory Decay** — Ebbinghaus Forgetting Curve + utility-based pruning

**Economic Impact:**
- **First encounter:** 50,000 tokens, 10 minutes
- **After consolidation:** 5,000 tokens, 2 minutes
- **Savings:** 90% token reduction, 80% time reduction

---

## 🔄 Architecture Updates

### 1. Tripartite Memory Domains

**NEW: Three Memory Subsystems**

```rust
pub enum MemoryDomain {
    /// Semantic: Factual knowledge (API contracts, schemas, docs)
    Semantic,
    
    /// Episodic: Chronological experiences (conversations, debug traces)
    Episodic,
    
    /// Procedural: Learned skills (SKILL.md workflows)
    Procedural,
}
```

**Semantic Memory:**
- **Storage:** Vector Database (Qdrant)
- **Content:** API contracts, database schemas, architectural docs
- **Retrieval:** Top-k similarity search
- **Integration:** Living Docs (Sprint 6) → Semantic Vector Store

**Episodic Memory:**
- **Storage:** SQLite (time-series)
- **Content:** Conversations, terminal I/O, success/failure states
- **Retrieval:** Time-bound queries, trajectory extraction
- **Integration:** All agent actions logged chronologically

**Procedural Memory:**
- **Storage:** File-system (SKILL.md format)
- **Content:** Learned workflows, heuristics, diagnostic patterns
- **Retrieval:** Trigger-based matching
- **Integration:** Task Decomposition (Sprint 7) queries procedural store

---

### 2. Consolidation Pipeline (Episodic → Procedural)

**NEW: Background Consolidation Worker**

```rust
pub struct ConsolidationPipeline {
    episodic_store: EpisodicStore,
    distillation_model: LightweightLLM, // Claude Haiku
    procedural_store: ProceduralStore,
}

impl ConsolidationPipeline {
    /// Trigger consolidation based on success/failure/frequency
    pub fn check_triggers(&self, session: &Session) -> Option<ConsolidationTrigger> {
        // Success-based: Complex task completed successfully
        // Failure-based: Catastrophic failure loop detected
        // Frequency-based: Identical sequence observed N times
    }
    
    /// Extract trajectory from episodic logs
    pub fn extract_trajectory(&self, trigger: &ConsolidationTrigger) -> Trajectory {
        // Parse episodic logs
        // Filter out pleasantries, failed loops, redundant outputs
        // Isolate core causal chain (observation-action pairs)
    }
    
    /// Distill trajectory into procedural skill
    pub fn distill(&self, trajectory: &Trajectory) -> Result<Skill> {
        // Use lightweight LLM (Haiku) for distillation
        // Generate SKILL.md with YAML frontmatter
        // Replace hardcoded values with parameters
    }
    
    /// Validate skill before deployment
    pub fn validate(&self, skill: &Skill) -> Result<()> {
        // Review Mode: Human-in-the-loop validation
        // OR Teacher-Student Verification: Higher-capacity orchestrator verifies
    }
}
```

**Trigger Conditions:**
1. **Success-based:** Complex debugging session resolved
2. **Failure-based:** Catastrophic failure loop (learn anti-pattern)
3. **Frequency-based:** Identical sequence observed 3+ times

**Distillation Process:**
1. Extract trajectory (filter noise)
2. Multi-faceted distillation (Haiku analyzes success/failure patterns)
3. Pattern generalization (replace hardcoded values with parameters)
4. Synthesize SKILL.md format

**Validation:**
- **Review Mode:** Human approves before deployment
- **Teacher-Student:** Orchestrator verifies against semantic memory

---

### 3. MemGAS Retrieval Optimization

**NEW: Multi-Granularity Retrieval with GMM Clustering**

```rust
pub struct MemGASRetriever {
    vector_store: VectorStore,
    gmm_model: GaussianMixtureModel,
    association_graph: MemoryAssociationGraph,
}

impl MemGASRetriever {
    /// Generate multi-granularity metadata
    pub fn generate_metadata(&self, session: &Session) -> MultiGranularityMemory {
        MultiGranularityMemory {
            turn_level: session.turns, // Micro-level segments
            session_summary: self.summarize(session), // Macro-level summary
            keyword_clusters: self.extract_keywords(session), // Isolated clusters
        }
    }
    
    /// Cluster memories using GMM (not static top-k)
    pub fn cluster_memories(&self, memories: &[Memory]) -> GMMClustering {
        // Apply Gaussian Mixture Model
        // Cluster into Accept Set (relevant) and Reject Set (noise)
        // Update association graph with semantic edges
    }
    
    /// Entropy-based router selects optimal granularity
    pub fn retrieve(&self, query: &str) -> Result<ContextPayload> {
        // Evaluate relevance distribution (entropy)
        // Select optimal granularity:
        // - Broad query → session summaries
        // - Specific query → turn-level code blocks
    }
}
```

**Why GMM over Top-K:**
- Top-K retrieves fragmented partial truths
- GMM clusters by probabilistic relevance
- Accept Set vs Reject Set (not arbitrary K cutoff)

**Entropy-Based Routing:**
- Broad architectural inquiry → High-level session summaries
- Specific syntax error → Exact turn-level code block

---

### 4. Memory Lifecycle Management

**NEW: Decay + Pruning Algorithms**

```rust
pub struct MemoryLifecycle {
    decay_model: EbbinghausDecay,
    utility_tracker: UtilityTracker,
}

impl MemoryLifecycle {
    /// Calculate memory strength (Ebbinghaus Forgetting Curve)
    pub fn calculate_strength(&self, memory: &Memory) -> f32 {
        let time_decay = self.decay_model.exponential_decay(memory.created_at);
        let importance_boost = memory.importance_scalar;
        let retrieval_reinforcement = memory.retrieval_count as f32;
        
        time_decay * importance_boost * retrieval_reinforcement.log()
    }
    
    /// Utility-based refinement (track success rate)
    pub fn update_utility(&self, skill_id: &str, outcome: SkillOutcome) {
        match outcome {
            SkillOutcome::Success => self.utility_tracker.increment_success(skill_id),
            SkillOutcome::Failure => self.utility_tracker.increment_failure(skill_id),
            SkillOutcome::HumanCorrection => self.utility_tracker.slash_utility(skill_id),
        }
    }
    
    /// Prune memories below thresholds
    pub fn prune(&self, stores: &mut MemoryStores) -> Result<()> {
        for memory in stores.all_memories() {
            let strength = self.calculate_strength(memory);
            let utility = self.utility_tracker.get_utility(&memory.id);
            
            if strength < STRENGTH_THRESHOLD || utility < UTILITY_THRESHOLD {
                stores.delete(&memory.id)?;
            }
        }
    }
}
```

**Ebbinghaus Forgetting Curve:**
- Memory strength decays exponentially over time
- Retrieval acts as reinforcement (resets counter)
- Important memories have higher scalar

**Utility-Based Refinement:**
- Track success/failure rate for each SKILL.md
- Frequently retrieved but consistently failing → Slash utility
- Below combined threshold → Auto-purge

---

### 5. Integration with Existing OPENAKTA

**Living Docs (Sprint 6) → Semantic Memory:**
```rust
// Living Docs output → Semantic Vector Store
living_docs.on_update(|doc| {
    semantic_store.insert(SemanticMemory {
        content: doc.content,
        embedding: embed(doc.content),
        metadata: doc.metadata,
    });
});
```

**Task Decomposition (Sprint 7) → Procedural Memory:**
```rust
// Orchestrator queries procedural store before delegation
let subtasks = decompose(mission)?;
for subtask in &subtasks {
    if let Some(skill) = procedural_store.match_trigger(subtask) {
        // Package subtask with procedural heuristic
        worker.execute_with_skill(subtask, skill)?;
    } else {
        // Worker must reason from first principles
        worker.execute(subtask)?;
    }
}
```

**Context Distribution (Sprint 8) → MemGAS Retrieval:**
```rust
// Context Manager uses MemGAS for precise curation
let context = context_manager.allocate(task, agent)?;
let retrieved = memgas_retriever.retrieve(&task.description)?;
context.enrich(retrieved); // Only relevant memories (not noise)
```

---

## 📋 NEW Implementation Sprints

Based on R-14, we need to add:

### Agent A (Documentation + Memory Specialist)
- **Sprint 26:** Semantic Memory Store (Vector DB integration)
- **Sprint 27:** Episodic Memory Store (SQLite time-series)
- **Sprint 28:** Procedural Memory Store (SKILL.md file-system)
- **Sprint 29:** Consolidation Pipeline (episodic → procedural)
- **Sprint 30:** MemGAS Retrieval (GMM clustering + entropy routing)
- **Sprint 31:** Memory Lifecycle (decay + pruning)

### Agent B (Storage Infrastructure)
- **Sprint 26:** Vector DB Schema (semantic memory)
- **Sprint 27:** SQLite Schema (episodic memory)
- **Sprint 28:** SKILL.md Format (procedural memory)

### Agent C (Integration)
- **Sprint 26:** Living Docs → Semantic Integration
- **Sprint 27:** Task Decomposition → Procedural Integration
- **Sprint 28:** Context Distribution → MemGAS Integration

---

## ✅ Validation Metrics (from Research)

| Metric | Target | Measurement |
|--------|--------|-------------|
| Token Reduction (repetitive tasks) | 90% | Before/after consolidation |
| Execution Time Reduction | 80% | First encounter vs subsequent |
| Memory Strength Accuracy | >95% | Ebbinghaus model fit |
| Utility Tracking Accuracy | >90% | Success/failure prediction |
| Pruning Precision | <5% false positives | Deleted but needed |
| Retrieval Precision@K | >85% | Relevant memories retrieved |

---

## 🔗 Updated References

- [`PHASE-2-PIVOT-GRAPH-WORKFLOW.md`](./PHASE-2-PIVOT-GRAPH-WORKFLOW.md) — Graph pivot
- [`PHASE-2-INTEGRATION-COMPETITIVE-ANALYSIS.md`](./PHASE-2-INTEGRATION-COMPETITIVE-ANALYSIS.md) — Competitive analysis
- [`PHASE-2-INTEGRATION-INFLUENCE-GRAPH.md`](./PHASE-2-INTEGRATION-INFLUENCE-GRAPH.md) — Influence graph
- [`PHASE-2-INTEGRATION-REACT-PATTERNS.md`](./PHASE-2-INTEGRATION-REACT-PATTERNS.md) — ReAct patterns
- Research document — R-14 Memory Architecture

---

**This research is FOUNDATIONAL — enables true agent learning and compounding expertise.**

**Priority: CRITICAL — must implement before Phase 3.**
