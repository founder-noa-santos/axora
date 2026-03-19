# Context Compacting Implementation Plan

**Date:** 2026-03-18  
**Status:** Ready for Implementation  
**Priority:** 🔴 CRITICAL  
**Estimated Duration:** 4 weeks (12 sprints)  
**Owner:** Agent B (Storage/Context Specialist)  

---

## 🎯 Goal

Implement context compacting infrastructure to enable **long-running multi-agent systems (100+ turns)** with **60-80% cost reduction** while maintaining **>95% reasoning accuracy**.

---

## 📊 Current State Analysis

### What We Have

| Component | Status | Location | Gap |
|-----------|--------|----------|-----|
| Blackboard v2 | ✅ Implemented | `crates/axora-cache/src/blackboard/v2.rs` | Not CRDT-based (no parallel writes) |
| PrefixCache | ✅ Implemented | `crates/axora-cache/src/prefix_cache.rs` | Static prefixes only |
| Diff | ✅ Implemented | `crates/axora-cache/src/diff.rs` | File diffs only (not state diffs) |
| Worker Agents | ✅ Implemented | `crates/axora-agents/src/worker_pool.rs` | No hierarchical memory |

### What We Need to Build

| Component | Priority | Effort |
|-----------|----------|--------|
| CRDT Blackboard | 🔴 CRITICAL | 3-4 days |
| Diff-Based Event Bus | 🔴 CRITICAL | 2-3 days |
| Hierarchical Memory Structure | 🔴 CRITICAL | 2 days |
| TOON Serializer | 🟡 HIGH | 1-2 days |
| Rolling Summary | 🟡 HIGH | 2 days |
| Semantic Memory (Vector DB) | 🟡 HIGH | 3 days |
| Latent Compilation (KV Cache) | 🟡 MEDIUM | 4-5 days |
| ACON Integration | 🟡 MEDIUM | 3 days |

---

## 📋 Phase Breakdown

### Phase 1: Foundation (Week 1-2)

**Goal:** CRDT Blackboard + Diff-Based Event Bus + Hierarchical Memory

---

#### Sprint CC1: CRDT Blackboard (Yjs Integration)

**Owner:** Agent B  
**Duration:** 3-4 days  
**Priority:** 🔴 CRITICAL

**Tasks:**
1. [ ] Add `yrs` (Yjs Rust port) crate to workspace
2. [ ] Implement `CRDTBlackboard` struct:
   ```rust
   pub struct CRDTBlackboard {
       doc: Doc,
       state_map: YMap,
       text_map: YText,
   }
   ```
3. [ ] Implement Y.Map for key-value state:
   ```rust
   impl CRDTBlackboard {
       pub fn set_value(&self, key: &str, value: Value) -> Result<()>;
       pub fn get_value(&self, key: &str) -> Option<Value>;
   }
   ```
4. [ ] Implement Y.Text for text/code generation:
   ```rust
   impl CRDTBlackboard {
       pub fn get_text(&self, id: &str) -> YTextRef;
       pub fn insert_text(&self, id: &str, pos: u32, text: &str) -> Result<()>;
   }
   ```
5. [ ] Add Strong Eventual Consistency guarantees
6. [ ] Write tests (concurrent write scenarios)

**Deliverables:**
- `crates/axora-cache/src/crdt_blackboard.rs` — CRDT implementation
- `crates/axora-cache/tests/crdt_concurrency_test.rs` — Concurrency tests

**Success Criteria:**
- [ ] Multiple agents can write concurrently without locks
- [ ] All replicas converge to same state (SEC)
- [ ] Zero text corruption in concurrent writes
- [ ] Latency <10ms per operation

---

#### Sprint CC2: Diff-Based Event Bus

**Owner:** Agent B  
**Duration:** 2-3 days  
**Priority:** 🔴 CRITICAL

**Tasks:**
1. [ ] Add `json-patch` crate (RFC 6902 implementation)
2. [ ] Implement `DiffGenerator`:
   ```rust
   pub struct DiffGenerator {
       previous_state: Value,
       current_state: Value,
   }
   
   impl DiffGenerator {
       pub fn generate_diff(&self) -> Vec<JsonPatchOperation>;
   }
   ```
3. [ ] Integrate with NATS JetStream (or Redis Pub/Sub):
   ```rust
   pub struct EventBus {
       client: async_nats::Client,
   }
   
   impl EventBus {
       pub async fn publish_diff(&self, topic: &str, diff: Vec<JsonPatchOperation>) -> Result<()>;
       pub async fn subscribe_diff(&self, topic: &str) -> Result<Receiver<Vec<JsonPatchOperation>>>;
   }
   ```
4. [ ] Add topic-specific subscriptions (agents subscribe only to relevant topics)
5. [ ] Implement diff compression (delta encoding)
6. [ ] Write tests (diff generation, pub/sub)

**Deliverables:**
- `crates/axora-cache/src/diff_generator.rs` — Diff generation
- `crates/axora-cache/src/event_bus.rs` — Pub/sub integration

**Success Criteria:**
- [ ] 89-98% reduction in state update tokens
- [ ] Pub/sub latency <20ms P95
- [ ] Topic filtering works correctly
- [ ] Diff compression achieves 50%+ additional savings

---

#### Sprint CC3: Hierarchical Memory Structure

**Owner:** Agent B  
**Duration:** 2 days  
**Priority:** 🔴 CRITICAL

**Tasks:**
1. [ ] Define `HierarchicalMemory` struct:
   ```rust
   pub struct HierarchicalMemory {
       system_prompt: String,           // Immutable, cached
       working_context: HashMap<String, Value>,  // Key-value pairs
       recent_events: VecDeque<Event>,  // Rolling window (N=10)
       semantic_summaries: Vec<SemanticSummary>, // Dynamic injection
   }
   ```
2. [ ] Implement strict token budget enforcement:
   ```rust
   impl HierarchicalMemory {
       pub fn enforce_budget(&mut self, max_tokens: usize) -> Result<()> {
           // Evict oldest events first
           // Then evict least relevant semantic summaries
       }
   }
   ```
3. [ ] Add JSON patch-based updates for working context:
   ```rust
   impl HierarchicalMemory {
       pub fn apply_patch(&mut self, patch: JsonPatch) -> Result<()>;
   }
   ```
4. [ ] Implement rolling window for recent events (FIFO eviction)
5. [ ] Add semantic summary injection (relevance-based retrieval)
6. [ ] Write tests (budget enforcement, eviction)

**Deliverables:**
- `crates/axora-agents/src/hierarchical_memory.rs` — Memory structure
- `crates/axora-agents/src/memory_budget.rs` — Token budget enforcement

**Success Criteria:**
- [ ] Context window stays bounded (<8K tokens)
- [ ] FIFO eviction works correctly
- [ ] JSON patch updates are lossless
- [ ] Semantic summary injection is relevance-based

---

### Phase 2: Compaction Engines (Week 2-3)

**Goal:** TOON Serializer + Rolling Summary + Semantic Memory

---

#### Sprint CC4: TOON Serializer

**Owner:** Agent B  
**Duration:** 1-2 days  
**Priority:** 🟡 HIGH

**Tasks:**
1. [ ] Define TOON schema format:
   ```rust
   pub struct TOONSchema {
       fields: Vec<FieldDefinition>,
   }
   
   pub struct FieldDefinition {
       name: String,
       field_type: TOONType,
   }
   ```
2. [ ] Implement TOON encoder (JSON → TOON):
   ```rust
   impl TOONEncoder {
       pub fn encode(&self, json: &Value) -> Result<String> {
           // Strip keys, output comma-separated values only
       }
   }
   ```
3. [ ] Implement TOON decoder (TOON → JSON):
   ```rust
   impl TOONDecoder {
       pub fn decode(&self, toon: &str, schema: &TOONSchema) -> Result<Value>;
   }
   ```
4. [ ] Add schema caching (define once, reuse)
5. [ ] Benchmark token savings (target: 80% reduction)
6. [ ] Write tests (encoding/decoding round-trip)

**Deliverables:**
- `crates/axora-cache/src/toon/schema.rs` — Schema definition
- `crates/axora-cache/src/toon/encoder.rs` — JSON → TOON
- `crates/axora-cache/src/toon/decoder.rs` — TOON → JSON

**Success Criteria:**
- [ ] 80% token reduction for repetitive data structures
- [ ] Lossless encoding/decoding (round-trip preserves data)
- [ ] Schema caching works correctly
- [ ] Encoding/decoding latency <5ms

---

#### Sprint CC5: Rolling Summary

**Owner:** Agent B  
**Duration:** 2 days  
**Priority:** 🟡 HIGH

**Tasks:**
1. [ ] Implement `RollingSummary` manager:
   ```rust
   pub struct RollingSummary {
       recent_turns: VecDeque<Turn>,  // Last N=10 turns (verbatim)
       historical_summary: String,     // Running summary
       max_turns: usize,
   }
   ```
2. [ ] Add token pressure monitoring:
   ```rust
   impl RollingSummary {
       pub fn check_pressure(&self, current_tokens: usize, max_tokens: usize) -> PressureLevel;
   }
   ```
3. [ ] Implement recursive summarization trigger:
   ```rust
   impl RollingSummary {
       pub fn flush_oldest_turns(&mut self) -> Result<String> {
           // Evict oldest turns
           // Summarize them
           // Append to historical_summary
       }
   }
   ```
4. [ ] Add semantic drift prevention (preserve key facts)
5. [ ] Write tests (pressure monitoring, summarization)

**Deliverables:**
- `crates/axora-agents/src/rolling_summary.rs` — Summary manager
- `crates/axora-agents/src/pressure_monitor.rs` — Token pressure monitoring

**Success Criteria:**
- [ ] Automatic summarization when context approaches saturation
- [ ] 5x-10x compression ratio for historical turns
- [ ] Semantic drift <5% (key facts preserved)
- [ ] Summarization latency <500ms

---

#### Sprint CC6: Semantic Memory (Vector DB Integration)

**Owner:** Agent B  
**Duration:** 3 days  
**Priority:** 🟡 HIGH

**Tasks:**
1. [ ] Integrate with Qdrant Embedded (from Local-First RAG mission)
2. [ ] Implement `SemanticMemory` struct:
   ```rust
   pub struct SemanticMemory {
       vector_store: QdrantClient,
       max_age_days: u64,
   }
   ```
3. [ ] Add fact extraction from interactions:
   ```rust
   impl SemanticMemory {
       pub async fn extract_facts(&self, interaction: &Interaction) -> Result<Vec<Fact>>;
   }
   ```
4. [ ] Implement relevance-based retrieval:
   ```rust
   impl SemanticMemory {
       pub async fn retrieve_relevant(&self, context: &str, k: usize) -> Result<Vec<Fact>>;
   }
   ```
5. [ ] Add automatic eviction (facts older than max_age_days)
6. [ ] Write tests (fact extraction, retrieval relevance)

**Deliverables:**
- `crates/axora-memory/src/semantic_memory.rs` — Semantic memory
- `crates/axora-memory/src/fact_extractor.rs` — Fact extraction

**Success Criteria:**
- [ ] Facts extracted accurately from interactions
- [ ] Retrieval relevance >90% (precision@k)
- [ ] Automatic eviction works correctly
- [ ] Integration with Qdrant works seamlessly

---

### Phase 3: Advanced Optimization (Week 3-4)

**Goal:** Latent Compilation + ACON Integration + Benchmarking

---

#### Sprint CC7: Latent Compilation (KV Cache)

**Owner:** Agent B  
**Duration:** 4-5 days  
**Priority:** 🟡 MEDIUM

**Tasks:**
1. [ ] Research ICAE (In-context Autoencoder) implementation
2. [ ] Implement `LatentCompiler`:
   ```rust
   pub struct LatentCompiler {
       autoencoder: ICAEModel,
       compression_ratio: usize,
   }
   ```
3. [ ] Add context → latent embedding compilation:
   ```rust
   impl LatentCompiler {
       pub async fn compile(&self, context: &str) -> Result<Vec<f32>>;
   }
   ```
4. [ ] Implement latent → context decom pilation:
   ```rust
   impl LatentCompiler {
       pub async fn decompile(&self, latent: &[f32]) -> Result<String>;
   }
   ```
5. [ ] Add KV cache tensor storage
6. [ ] Benchmark compression ratios (target: 16x-32x)
7. [ ] Write tests (compilation/decompilation accuracy)

**Deliverables:**
- `crates/axora-cache/src/latent_compiler.rs` — Latent compilation
- `crates/axora-cache/src/ic ae_model.rs` — ICAE model wrapper

**Success Criteria:**
- [ ] 16x-32x compression ratio achieved
- [ ] Near-zero loss in fine-grained reasoning
- [ ] Compilation/decompilation latency <100ms
- [ ] KV cache storage works correctly

---

#### Sprint CC8: ACON Integration

**Owner:** Agent B  
**Duration:** 3 days  
**Priority:** 🟡 MEDIUM

**Tasks:**
1. [ ] Research ACON (Agent Context Optimization) framework
2. [ ] Implement failure-driven guidelines:
   ```rust
   pub struct ACONGuidelines {
       failure_patterns: Vec<FailurePattern>,
       task_awareness: TaskContext,
   }
   ```
3. [ ] Add history distillation:
   ```rust
   impl ACONDistiller {
       pub async fn distill(&self, history: &[Turn], guidelines: &ACONGuidelines) -> Result<String>;
   }
   ```
4. [ ] Implement semantic drift prevention
5. [ ] Benchmark reasoning accuracy improvement (target: +46%)
6. [ ] Write tests (distillation accuracy, drift prevention)

**Deliverables:**
- `crates/axora-agents/src/acon/guidelines.rs` — ACON guidelines
- `crates/axora-agents/src/acon/distiller.rs` — History distillation

**Success Criteria:**
- [ ] 26%-54% peak token reduction
- [ ] 46% improvement in downstream reasoning accuracy
- [ ] Zero semantic drift (facts preserved)
- [ ] Distillation latency <1s

---

#### Sprint CC9: Performance Benchmarking

**Owner:** Agent A (Documentation Specialist)  
**Duration:** 2 days  
**Priority:** 🟡 MEDIUM

**Tasks:**
1. [ ] Set up benchmark suite:
   ```rust
   #[bench]
   fn benchmark_crdt_operations(b: &mut Bencher);
   
   #[bench]
   fn benchmark_diff_generation(b: &mut Bencher);
   
   #[bench]
   fn benchmark_context_compaction(b: &mut Bencher);
   ```
2. [ ] Measure compression ratios (target: 10:1 to 30:1)
3. [ ] Measure reasoning accuracy (>95%)
4. [ ] Measure latency (<100ms P95)
5. [ ] Generate validation report

**Deliverables:**
- `benches/context_compacting_bench.rs` — Benchmark suite
- `docs/CONTEXT-COMPACTING-VALIDATION.md` — Validation report

**Success Criteria:**
- [ ] All metrics meet targets
- [ ] Benchmark suite is reproducible
- [ ] Validation report is comprehensive

---

## 📈 Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Compression Ratio** | 10:1 to 30:1 | Tokens before / after |
| **Reasoning Accuracy** | >95% (no degradation) | Task success rate |
| **Context Window Size** | <8K tokens (bounded) | Max tokens per agent |
| **Token Cost Reduction** | 60-80% | $/session comparison |
| **Latency Reduction** | 30-50% | Time-to-first-token |
| **Concurrency Speedup** | 3-5x | Parallel vs sequential |

---

## 🔗 Dependencies

### Internal Dependencies

| Sprint | Depends On | Blocked By |
|--------|------------|------------|
| CC1 | None | None |
| CC2 | CC1 | None |
| CC3 | CC1, CC2 | None |
| CC4 | CC3 | None |
| CC5 | CC3 | None |
| CC6 | Local-First RAG (Qdrant) | B5, B6 |
| CC7 | CC6 | None |
| CC8 | CC5 | None |
| CC9 | All sprints | None |

### External Dependencies

| Dependency | Purpose | Status |
|------------|---------|--------|
| `yrs` crate | Yjs Rust port | ✅ Available |
| `json-patch` crate | RFC 6902 implementation | ✅ Available |
| `async-nats` crate | NATS JetStream client | ✅ Available |
| Qdrant Embedded | Vector store for semantic memory | 🔄 In Progress (Sprint B6) |

---

## 📅 Timeline

### Week 1-2: Foundation
- CC1: CRDT Blackboard ✅
- CC2: Diff-Based Event Bus ✅
- CC3: Hierarchical Memory Structure ✅

### Week 2-3: Compaction Engines
- CC4: TOON Serializer ✅
- CC5: Rolling Summary ✅
- CC6: Semantic Memory ✅

### Week 3-4: Advanced Optimization
- CC7: Latent Compilation ✅
- CC8: ACON Integration ✅
- CC9: Performance Benchmarking ✅

**Total Duration:** 4 weeks (9 sprints)

---

## ✅ Definition of Done

Phase is complete when:
- [ ] All 9 sprints complete
- [ ] All tests passing (unit + integration)
- [ ] All benchmarks meet targets
- [ ] Compression ratio 10:1 to 30:1 validated
- [ ] Reasoning accuracy >95% validated
- [ ] Documentation complete

---

**Ready to execute. This plan enables long-running multi-agent systems with 60-80% cost reduction.**
