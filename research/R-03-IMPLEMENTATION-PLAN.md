# R-03 Research Summary & Implementation Plan

**Date:** 2026-03-16  
**Status:** ✅ Research Complete → 🔄 Ready for Implementation  
**Research:** [R-03 Findings](./findings/token-efficiency/R-03-result.md)  
**Architecture:** [architecture-token-efficiency.md](../docs/architecture-token-efficiency.md)

---

## Executive Summary

R-03 research on **Token Efficiency & Compression** is complete. Seven architectural decisions have been made (ADR-021 through ADR-027) defining a comprehensive token optimization strategy.

**Key Outcome:** AXORA will achieve **90% cost reduction** through multi-layer optimization:
- Prefix Prompt Caching (50-90% savings)
- Diff-Based Code Communication (89-98% savings)
- Code Minification (24-42% savings)
- TOON Serialization (50-60% savings)
- Multi-Tier Caching (70-80% hit rate)
- MetaGlyph Symbolic Language (62-81% savings)

**Business Impact:** Monthly costs reduced from $7,650 to $765 (90% savings) at scale.

---

## Decisions Made

| ADR | Decision | Impact |
|-----|----------|--------|
| ADR-021 | Multi-Layer Token Optimization | 90% overall cost reduction |
| ADR-022 | Prefix Prompt Caching | 90% savings on static content |
| ADR-023 | Diff-Based Code Communication | 89-98% reduction in code transfer |
| ADR-024 | Code Minification Pipeline | 24-42% reduction, no quality loss |
| ADR-025 | TOON Serialization | 50-60% smaller tool outputs |
| ADR-026 | Multi-Tier Caching | 70-80% cache hit rate |
| ADR-027 | MetaGlyph Symbolic Language | 62-81% shorter instructions |

---

## Implementation Roadmap

### Sprint 0: Foundation (Week 1-2)

**Goal:** Set up caching infrastructure and prompt reorganization

**Tasks:**
- [ ] Add dependencies to `Cargo.toml`:
  ```toml
  [dependencies]
  # Caching
  dashmap = "5.5"
  rocksdb = "0.22"
  qdrant-client = "1.9"
  
  # Embeddings for semantic cache
  ort = "2.0"  # ONNX Runtime
  ndarray = "0.15"
  
  # Hashing
  sha2 = "0.10"
  hex = "0.4"
  
  # Diff generation
  unified-diff = "0.3"
  patch = "0.5"
  ```

- [ ] Create `crates/axora-cache/` crate structure
- [ ] Implement basic L1 cache with DashMap:
  ```rust
  pub struct L1Cache {
      cache: DashMap<String, CacheEntry>,
  }
  
  impl L1Cache {
      pub fn get(&self, key: &str) -> Option<CacheEntry>;
      pub fn set(&self, key: &str, value: CacheEntry);
      pub fn invalidate(&self, keys: &[String]);
  }
  ```

- [ ] Reorganize agent prompts for prefix caching:
  ```rust
  pub struct PromptBuilder {
      static_prefix: String,  // Cached (90% discount)
      dynamic_suffix: String, // Uncached
  }
  ```

**Deliverable:** L1 cache working, prompts reorganized

**Success Criteria:**
- L1 cache latency: <10μs
- Prefix cache hit rate: >90%
- Immediate cost reduction: 40-50%

---

### Sprint 1: Diff-Based Communication (Week 3-4)

**Goal:** Implement diff generation and application pipeline

**Tasks:**
- [ ] Implement diff engine:
  ```rust
  pub struct DiffEngine {
      repo_path: PathBuf,
  }
  
  impl DiffEngine {
      pub fn generate_diff(&self, file_path: &str, new_content: &str) -> Result<CodeDiff>;
      pub fn apply_diff(&self, diff: &CodeDiff) -> Result<()>;
      pub fn validate_diff(&self, diff: &CodeDiff) -> Result<bool>;
  }
  ```

- [ ] Implement unified diff parser:
  ```rust
  pub struct CodeDiff {
      file_path: String,
      unified_diff: String,  // @@ -10,7 +10,9 @@
      hunk_count: usize,
  }
  ```

- [ ] Add diff validation (syntax check, unit tests)
- [ ] Implement fallback for failed diffs (full file review)
- [ ] Integrate with agent communication protocol

**Deliverable:** `axora-diff` crate with working diff pipeline

**Success Criteria:**
- Diff generation: <50ms per file
- Diff application success rate: >95%
- Token savings: 89-98% vs full file transfer

---

### Sprint 2: Code Minification (Week 5-6)

**Goal:** Implement code minification pipeline

**Tasks:**
- [ ] Implement whitespace stripper:
  ```rust
  pub fn strip_whitespace(code: &str) -> String {
      code.lines()
          .map(|l| l.trim_end())
          .collect::<Vec<_>>()
          .join("\n")
  }
  ```

- [ ] Implement identifier compressor:
  ```rust
  pub struct IdentifierMap {
      mapping: HashMap<String, String>,  // original ↔ minified
  }
  
  impl IdentifierMap {
      pub fn compress(&self, code: &str) -> String;
      pub fn decompress(&self, code: &str) -> String;
  }
  ```

- [ ] Implement comment stripper
- [ ] Add language-specific minifiers (Rust, TypeScript, Python, etc.)
- [ ] Test quality impact (Fill-in-the-Middle benchmarks)

**Deliverable:** `axora-minifier` crate with working minification

**Success Criteria:**
- Minification latency: <10ms per file
- Token savings: 24-42%
- Quality degradation: <2% (Pass@1 metrics)

---

### Sprint 3: TOON Serialization (Week 7-8)

**Goal:** Implement Token-Optimized Object Notation

**Tasks:**
- [ ] Define TOON schema format:
  ```rust
  pub struct ToonSchema {
      fields: HashMap<String, u8>,  // field_name → field_id
  }
  ```

- [ ] Implement TOON serializer:
  ```rust
  pub struct ToonSerializer {
      schema: ToonSchema,
  }
  
  impl ToonSerializer {
      pub fn serialize(&self, data: &Value) -> Result<String>;
      pub fn deserialize(&self, toon: &str) -> Result<Value>;
  }
  ```

- [ ] Implement schema registry (shared across agents)
- [ ] Integrate with MCP tool outputs
- [ ] Benchmark token savings vs JSON

**Deliverable:** `axora-toon` crate with serialization

**Success Criteria:**
- Token savings: 50-60% vs JSON
- Serialization latency: <5ms
- Round-trip accuracy: 100%

---

### Sprint 4: L2 Cache (RocksDB) (Week 9)

**Goal:** Implement disk-backed L2 cache

**Tasks:**
- [ ] Set up RocksDB configuration:
  ```rust
  use rocksdb::{DB, Options};
  
  pub struct L2Cache {
      db: DB,
  }
  
  impl L2Cache {
      pub fn new(path: &str) -> Result<Self> {
          let mut opts = Options::default();
          opts.create_if_missing(true);
          opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
          
          let db = DB::open(&opts, path)?;
          Ok(Self { db })
      }
  }
  ```

- [ ] Implement cache tiering (L1 → L2):
  ```rust
  pub async fn get(&self, query: &str) -> Option<CacheEntry> {
      // L1 first
      if let Some(entry) = self.l1.get(query) {
          return Some(entry);
      }
      
      // L2 fallback
      let key = sha256(query);
      if let Some(entry) = self.l2.get(&key)? {
          self.l1.insert(query.to_string(), entry.clone());
          return Some(entry);
      }
      
      None
  }
  ```

- [ ] Implement TTL-based expiration
- [ ] Add cache statistics tracking

**Deliverable:** L2 cache integrated

**Success Criteria:**
- L2 latency: <5ms
- Combined L1+L2 hit rate: 50-60%
- Disk usage: <1GB for typical workload

---

### Sprint 5: L3 Cache (Semantic) (Week 10-11)

**Goal:** Implement semantic vector cache with Qdrant

**Tasks:**
- [ ] Set up Qdrant client:
  ```rust
  use qdrant_client::{Qdrant, QdrantConfig};
  
  pub struct L3Cache {
      client: Qdrant,
      embedder: ONNXEmbedder,
  }
  ```

- [ ] Implement local embedder (ONNX Runtime):
  ```rust
  pub struct ONNXEmbedder {
      session: ort::Session,
  }
  
  impl ONNXEmbedder {
      pub async fn embed(&self, text: &str) -> Result<Vec<f32>>;
  }
  ```

- [ ] Implement semantic search with threshold:
  ```rust
  pub async fn search(&self, query: &str, min_similarity: f32) -> Option<CacheEntry> {
      let embedding = self.embedder.embed(query).await?;
      
      let results = self.client
          .search_points(SearchPoints {
              collection_name: "cache",
              vector: embedding,
              limit: 1,
              score_threshold: Some(min_similarity),
              ..Default::default()
          })
          .await?;
      
      results.result.first().map(|r| r.payload.clone())
  }
  ```

- [ ] Tune similarity thresholds (0.98 for code, 0.92 for NL)
- [ ] Implement cache population on miss

**Deliverable:** L3 semantic cache working

**Success Criteria:**
- L3 latency: <50ms
- Semantic cache hit rate: 20-30%
- Total cache hit rate (L1+L2+L3): 70-80%

---

### Sprint 6: MetaGlyph Parser (Week 12-13)

**Goal:** Implement symbolic metalanguage parser

**Tasks:**
- [ ] Define MetaGlyph operators:
  ```rust
  pub enum MetaGlyph {
      Membership,    // ∈
      Exclusion,     // ∉
      Intersection,  // ∩
      Union,         // ∪
      Implication,   // ⇒
      Negation,      // ¬
      Composition,   // ∘
      // ...
  }
  ```

- [ ] Implement parser:
  ```rust
  pub struct MetaGlyphParser {
      symbol_map: HashMap<MetaGlyph, String>,
  }
  
  impl MetaGlyphParser {
      pub fn parse(&self, expression: &str) -> Result<LogicalForm>;
      pub fn generate(&self, logical_form: &LogicalForm) -> String;
  }
  ```

- [ ] Create MetaGlyph dictionary (cached in system prompt)
- [ ] Test model fidelity (GPT-5.2, Claude 3.7, Kimi K2)
- [ ] Rewrite agent system prompts using MetaGlyph

**Deliverable:** `axora-metaglyph` crate with parser

**Success Criteria:**
- Model fidelity: >90% accuracy
- Token savings: 62-81%
- Parser latency: <5ms

---

### Sprint 7: Integration & Optimization (Week 14-15)

**Goal:** Integrate all optimization layers

**Tasks:**
- [ ] Create `TokenOptimizer` struct:
  ```rust
  pub struct TokenOptimizer {
      cache: MultiTierCache,
      diff_engine: DiffEngine,
      minifier: CodeMinifier,
      toon: ToonSerializer,
      metaglyph: MetaGlyphParser,
  }
  
  impl TokenOptimizer {
      pub async fn optimize_request(&self, request: &AgentRequest) -> Result<OptimizedRequest>;
      pub async fn optimize_response(&self, response: &AgentResponse) -> Result<OptimizedResponse>;
  }
  ```

- [ ] Wire all components together
- [ ] End-to-end latency profiling
- [ ] Optimize bottlenecks
- [ ] Cost analysis dashboard

**Deliverable:** Fully integrated token optimization pipeline

**Success Criteria:**
- End-to-end overhead: <50ms
- Total cost reduction: 90%
- No quality degradation

---

### Sprint 8: Benchmarking & Validation (Week 16-17)

**Goal:** Validate performance and cost savings

**Tasks:**
- [ ] Run cost benchmarks:
  - Before optimization: $255/day
  - After optimization: $25.50/day
  - Validate 90% savings

- [ ] Run quality benchmarks:
  - SWE-Bench Lite (Pass@1 metrics)
  - Fill-in-the-Middle (minified code)
  - MetaGlyph fidelity testing

- [ ] Profile latency:
  - L1 cache: <10μs
  - L2 cache: <5ms
  - L3 cache: <50ms
  - Diff generation: <50ms
  - Minification: <10ms

- [ ] Document performance results
- [ ] A/B test: optimized vs naive

**Deliverable:** Performance report, validated savings

**Success Criteria:**
- All performance targets met
- 90% cost reduction validated
- Quality degradation <3%

---

## Testing Strategy

### Unit Tests
- Cache get/set/invalidation
- Diff generation and application
- Minification/de-minification
- TOON serialization round-trip
- MetaGlyph parsing

### Integration Tests
- End-to-end token optimization pipeline
- Cache tiering (L1 → L2 → L3)
- Diff validation and fallback
- Multi-agent communication with optimization

### Quality Tests
- SWE-Bench Lite (Pass@1)
- Fill-in-the-Middle (minified code accuracy)
- MetaGlyph fidelity (>90%)
- A/B testing (optimized vs naive)

---

## Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Cache invalidation bugs | High | Medium | Comprehensive testing, dependency-based invalidation |
| Diff application failures | High | Medium | Fallback to full file review, validation before apply |
| Minification quality loss | Medium | Low | A/B testing, adaptive compression |
| Semantic cache false positives | Medium | Medium | High similarity threshold (0.98 for code) |
| MetaGlyph fidelity issues | High | Medium | Fidelity testing, fallback to natural language |

---

## Success Metrics

**After Implementation:**
- ✅ Cost Reduction: 90% ($7,650 → $765/month at scale)
- ✅ Cache Hit Rate: 70-80%
- ✅ Diff Token Savings: 89-98%
- ✅ Minification Savings: 24-42%
- ✅ TOON Savings: 50-60%
- ✅ MetaGlyph Savings: 62-81%
- ✅ Quality Degradation: <3%

**Business Impact:**
- ✅ Viable unit economics at scale
- ✅ 13-79% latency reduction
- ✅ Competitive advantage (lowest cost structure)
- ✅ Sustainable multi-agent operations

---

## Next Steps

1. **Start Sprint 0** (Foundation) - Week 1-2
2. **Await R-04 research** (Local Indexing) - May refine embedding choices
3. **Parallel: R-05 research** (Model Optimization) - Local model costs
4. **Parallel: R-06 research** (Agent Architecture) - Orchestration impact

---

## Related Documents

- [ADR-021: Multi-Layer Token Optimization](../research/DECISIONS.md#adr-021)
- [ADR-022: Prefix Prompt Caching](../research/DECISIONS.md#adr-022)
- [ADR-023: Diff-Based Code Communication](../research/DECISIONS.md#adr-023)
- [ADR-024: Code Minification Pipeline](../research/DECISIONS.md#adr-024)
- [ADR-025: TOON Serialization](../research/DECISIONS.md#adr-025)
- [ADR-026: Multi-Tier Caching](../research/DECISIONS.md#adr-026)
- [ADR-027: MetaGlyph Symbolic Language](../research/DECISIONS.md#adr-027)
- [R-03 Research Findings](./findings/token-efficiency/R-03-result.md)
