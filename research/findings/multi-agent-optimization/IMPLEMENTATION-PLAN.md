# Multi-Agent API Cost Optimization — Implementation Plan

**Date:** 2026-03-18  
**Status:** Ready for Implementation  
**Priority:** 🔴 CRITICAL  
**Estimated Duration:** 4 weeks (8 sprints)  
**Team:** Agents A, B, C  

---

## 🎯 Goal

Reduce multi-agent API costs by **90-95%** while improving latency by **30-50%** through:
1. ✅ Prefix caching (50-90% savings on input tokens)
2. ✅ Diff-only communication (89-98% savings on output tokens)
3. ✅ Graph-based context pruning (95-99% savings on context)
4. ✅ Binary agent protocol (eliminates negotiation tokens)

---

## 📊 Current State vs Target

### Current State (Without Optimizations)

| Component | Status | Location |
|-----------|--------|----------|
| PrefixCache | ✅ Implemented (not connected) | `crates/axora-cache/src/prefix_cache.rs` |
| Diff | ✅ Implemented (not enforced) | `crates/axora-cache/src/diff.rs` |
| InfluenceGraph | ✅ Implemented (simple parsing) | `crates/axora-indexing/src/influence.rs` |
| Blackboard v2 | ✅ Implemented | `crates/axora-cache/src/blackboard/v2.rs` |
| API Client | ❌ No caching | `crates/axora-agents/` |
| Agent Output | ❌ Not enforced (full files) | `crates/axora-agents/` |
| SCIP Indexing | ❌ Not implemented | — |
| Graph Retriever | ❌ Not implemented | — |
| Agent Protocol | ❌ Natural language | — |

### Target State (With All Optimizations)

| Metric | Current | Target | Reduction |
|--------|---------|--------|-----------|
| **Input Tokens** | 50,000/session | 2,500/session | 95% |
| **Output Tokens** | 10,000/session | 500/session | 95% |
| **Context Tokens** | 50,000/session | 500-2,500/session | 95-99% |
| **Cost per Session** | $4.80 | $0.39 | 92% |
| **Monthly Cost (100/day)** | $14,400 | $1,170 | 92% |

---

## 📋 Phase Breakdown

### Phase 1: API Integration (Week 1)

**Goal:** Connect existing PrefixCache and Diff to API clients

---

#### Sprint C7: API Client with Prefix Caching

**Owner:** Agent C (Implementation Specialist)  
**Duration:** 2 days  
**Priority:** 🔴 CRITICAL (blocks all other savings)

**Tasks:**
1. [ ] Add `PrefixCache` field to `ApiClient` struct
2. [ ] Implement `extract_static_prefix(messages)` function
3. [ ] Add cache key computation (SHA256 of prefix)
4. [ ] Integrate with Anthropic cache headers:
   ```rust
   headers.insert("X-Cache-Key", cache_key.parse()?);
   headers.insert("X-Cache-TTL", "3600".parse()?); // 1 hour
   ```
5. [ ] Integrate with OpenAI prefix caching (if available)
6. [ ] Add metrics tracking:
   ```rust
   pub struct CacheMetrics {
       pub cache_hits: usize,
       pub cache_misses: usize,
       pub tokens_saved: usize,
   }
   ```
7. [ ] Write integration tests (verify caching works)

**Deliverables:**
- `crates/axora-agents/src/api_client.rs` — Enhanced API client
- `crates/axora-agents/src/cache_integration.rs` — Cache integration
- `crates/axora-agents/src/metrics.rs` — Metrics tracking

**Success Criteria:**
- [ ] 50-90% reduction in prompt tokens (static prefixes cached)
- [ ] Latency reduced by 30-50% (Time-to-First-Token)
- [ ] Cache hit rate >80%
- [ ] Metrics show token savings

**Dependencies:** None (can start immediately)

---

#### Sprint C8: Diff-Only Agent Output Enforcement

**Owner:** Agent C (Implementation Specialist)  
**Duration:** 1-2 days  
**Priority:** 🔴 CRITICAL (blocks 89-98% savings)

**Tasks:**
1. [ ] Create `DiffEnforcer` struct:
   ```rust
   pub struct DiffEnforcer {
       max_full_write_bytes: usize, // Default: 100
   }
   ```
2. [ ] Implement `validate_output(output: &AgentOutput) -> Result<()>`:
   - Check for full file writes (>100 bytes)
   - Check for diff format (---, +++, @@, +, -)
   - Reject non-compliant outputs
3. [ ] Add system prompt (`prompts/diff_only.md`):
   ```markdown
   You MUST output changes as unified diffs only.
   
   Format:
   --- a/path/to/file.rs
   +++ b/path/to/file.rs
   @@ -10,7 +10,8 @@
    unchanged line
   -removed line
   +added line
   +another added line
   
   NEVER write full files. ONLY output diffs.
   ```
4. [ ] Add auto-conversion (full write → diff):
   ```rust
   pub fn convert_full_to_diff(full_write: &str, original: &str) -> UnifiedDiff {
       UnifiedDiff::generate(original, full_write, "old", "new")
   }
   ```
5. [ ] Add metrics (diff size vs full file size)
6. [ ] Write tests

**Deliverables:**
- `crates/axora-agents/src/diff_enforcer.rs` — Output validator
- `crates/axora-agents/src/prompts/diff_only.md` — System prompt
- `crates/axora-agents/src/converter.rs` — Full-to-diff converter

**Success Criteria:**
- [ ] 89-98% reduction in output tokens
- [ ] Zero full file writes (100% diffs)
- [ ] Agent compliance >95%

**Dependencies:** Sprint C7 (need API client for testing)

---

### Phase 2: Graph-Based Context (Week 2)

**Goal:** Implement SCIP indexing and context pruning

---

#### Sprint B9: SCIP Indexing (Language-Agnostic Parsing)

**Owner:** Agent B (Storage/Context Specialist)  
**Duration:** 3-4 days  
**Priority:** 🟡 HIGH (enables accurate dependency tracking)

**Tasks:**
1. [ ] Define SCIP Protobuf schema (`scip.proto`):
   ```protobuf
   message Symbol {
       string symbol = 1;
       string language = 2;
       string package = 3;
       repeated string relationships = 4;
   }
   
   message Occurrence {
       string relative_path = 1;
       repeated int32 range = 2;
       string symbol = 3;
   }
   ```
2. [ ] Add `rust-analyzer` for Rust parsing:
   ```rust
   pub struct RustAnalyzer;
   impl CodeParser for RustAnalyzer {
       fn generate_scip(&self, codebase: &Path) -> Result<SCIPIndex>;
   }
   ```
3. [ ] Add `scip-typescript` for TS/JS parsing
4. [ ] Add `scip-python` for Python parsing
5. [ ] Integrate with `InfluenceGraph` (replace simple parsing)
6. [ ] Write tests (verify symbol extraction accuracy)

**Deliverables:**
- `crates/axora-indexing/src/scip.rs` — SCIP index
- `crates/axora-indexing/src/parsers/` — Language-specific parsers
- `crates/axora-indexing/src/scip.proto` — Protobuf schema
- `crates/axora-indexing/src/parsers/rust.rs` — Rust parser
- `crates/axora-indexing/src/parsers/typescript.rs` — TS parser
- `crates/axora-indexing/src/parsers/python.rs` — Python parser

**Success Criteria:**
- [ ] Accurate symbol extraction (functions, classes, imports)
- [ ] Cross-file dependency detection
- [ ] Parsing speed >100 files/sec
- [ ] Support for Rust, TypeScript, Python

**Dependencies:** None (independent of API client)

---

#### Sprint B10: Context Pruning (Graph-Based Retrieval)

**Owner:** Agent B (Storage/Context Specialist)  
**Duration:** 2-3 days  
**Priority:** 🔴 CRITICAL (blocks 95-99% savings)

**Tasks:**
1. [ ] Create `GraphRetriever` struct:
   ```rust
   pub struct GraphRetriever {
       influence_graph: InfluenceGraph,
       vector_store: VectorStore,
   }
   ```
2. [ ] Implement `retrieve_relevant_context()`:
   ```rust
   pub fn retrieve_relevant_context(
       &self,
       query: &str,
       file_id: &str,
       max_tokens: usize,
   ) -> Result<Vec<Document>> {
       // 1. Get influence vector
       let vector = self.influence_graph.get_vector(file_id)?;
       
       // 2. Traverse dependencies (BFS with token budget)
       let affected_files = self.traverse_dependencies(
           &vector.direct_dependencies,
           max_tokens,
       );
       
       // 3. Retrieve only affected files
       let documents = self.vector_store.get_batch(&affected_files)?;
       
       Ok(documents)
   }
   ```
3. [ ] Implement dependency graph traversal:
   ```rust
   fn traverse_dependencies(
       &self,
       dependencies: &[FileId],
       max_tokens: usize,
   ) -> Vec<FileId> {
       let mut result = Vec::new();
       let mut tokens_used = 0;
       let mut queue = dependencies.to_vec();
       
       while let Some(file_id) = queue.pop() {
           let file_size = self.estimate_tokens(&file_id);
           
           if tokens_used + file_size > max_tokens {
               break; // Budget exceeded
           }
           
           result.push(file_id.clone());
           tokens_used += file_size;
           
           // Add transitive dependencies
           if let Some(vector) = self.influence_graph.get_vector(&file_id) {
               queue.extend(vector.direct_dependencies.iter().cloned());
           }
       }
       
       result
   }
   ```
4. [ ] Integrate with existing RAG pipeline
5. [ ] Add metrics (tokens retrieved vs tokens available)
6. [ ] Write tests

**Deliverables:**
- `crates/axora-rag/src/graph_retriever.rs` — Graph-based retrieval
- `crates/axora-rag/src/pruning.rs` — Context pruning logic
- `crates/axora-rag/src/traversal.rs` — Graph traversal

**Success Criteria:**
- [ ] 95-99% reduction in context tokens (50K → 500-2.5K)
- [ ] Retrieval latency <100ms
- [ ] No missing dependencies (100% recall)
- [ ] Token budget enforcement

**Dependencies:** Sprint B9 (need SCIP indexing)

---

### Phase 3: Agent Communication Protocol (Week 3)

**Goal:** Replace natural language with binary/JSON protocol

---

#### Sprint C11: Agent Message Protocol (Protobuf/JSON)

**Owner:** Agent C (Implementation Specialist)  
**Duration:** 2 days  
**Priority:** 🟡 HIGH (eliminates negotiation tokens)

**Tasks:**
1. [ ] Define `AgentMessage` enum:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum AgentMessage {
       TaskAssigned {
           task_id: String,
           task_type: TaskType,
           context: Vec<FileId>,
           deadline_ms: u64,
       },
       ProgressUpdate {
           task_id: String,
           percent_complete: f32,
           current_step: String,
       },
       ResultSubmitted {
           task_id: String,
           result_type: ResultType,
           diff: UnifiedDiff,
           tokens_used: usize,
       },
       BlockerAlert {
           task_id: String,
           blocker_type: BlockerType,
           message: String,
       },
   }
   ```
2. [ ] Implement Protobuf serialization (or compact JSON)
3. [ ] Integrate with Blackboard v2 (publish/subscribe):
   ```rust
   impl Agent {
       pub fn publish_message(&self, message: AgentMessage) {
           self.blackboard.publish("agent_messages", message);
       }
       
       pub fn subscribe_messages(&self) -> Receiver<AgentMessage> {
           self.blackboard.subscribe("agent_messages")
       }
   }
   ```
4. [ ] Update agents to use protocol (not natural language)
5. [ ] Add validation (schema enforcement)
6. [ ] Write tests

**Deliverables:**
- `crates/axora-agents/src/protocol.rs` — Message protocol
- `crates/axora-agents/src/protocol.proto` — Protobuf schema
- `crates/axora-agents/src/protocol_integration.rs` — Integration tests

**Success Criteria:**
- [ ] Zero natural language negotiations
- [ ] Message size <500 bytes (vs 5K+ for natural language)
- [ ] 100% schema compliance
- [ ] All agents use protocol

**Dependencies:** Sprint C7 (need API client for testing)

---

#### Sprint C12: Graph Workflow Enforcement (Deterministic Execution)

**Owner:** Agent C (Implementation Specialist)  
**Duration:** 2 days  
**Priority:** 🟡 HIGH (prevents infinite loops)

**Tasks:**
1. [ ] Define workflow graph (states, transitions):
   ```rust
   pub struct WorkflowGraph {
       states: Vec<State>,
       transitions: Vec<Transition>,
   }
   
   pub enum State {
       Pending,
       InProgress,
       WaitingForInput,
       Completed,
       Failed,
   }
   ```
2. [ ] Enforce deterministic execution (no loops):
   ```rust
   impl WorkflowGraph {
       pub fn validate_no_loops(&self) -> Result<()> {
           // DFS to detect cycles
       }
   }
   ```
3. [ ] Add timeout enforcement:
   ```rust
   pub fn with_timeout<F, R>(&self, duration: Duration, f: F) -> Result<R>
   where
       F: FnOnce() -> R,
   {
       // Execute with timeout
   }
   ```
4. [ ] Integrate with Coordinator (state machine)
5. [ ] Add metrics (execution time, success rate)
6. [ ] Write tests

**Deliverables:**
- `crates/axora-agents/src/workflow.rs` — Workflow graph
- `crates/axora-agents/src/state_machine.rs` — State machine
- `crates/axora-agents/src/timeout.rs` — Timeout enforcement

**Success Criteria:**
- [ ] Zero infinite loops
- [ ] 100% task completion (no hangs)
- [ ] Execution time <30 seconds per task

**Dependencies:** Sprint C11 (need protocol for state transitions)

---

### Phase 4: Validation & Documentation (Week 4)

**Goal:** Measure and document token savings

---

#### Sprint A4: Token Savings Benchmarking

**Owner:** Agent A (Documentation Specialist)  
**Duration:** 2 days  
**Priority:** 🟡 MEDIUM (validation)

**Tasks:**
1. [ ] Set up benchmark suite:
   ```rust
   #[bench]
   fn benchmark_prefix_caching(b: &mut Bencher) {
       // Measure tokens sent with/without caching
   }
   
   #[bench]
   fn benchmark_diff_communication(b: &mut Bencher) {
       // Measure tokens sent with diffs vs full files
   }
   
   #[bench]
   fn benchmark_context_pruning(b: &mut Bencher) {
       // Measure tokens retrieved with/without graph pruning
   }
   ```
2. [ ] Measure prefix caching savings (target: 50-90%)
3. [ ] Measure diff communication savings (target: 89-98%)
4. [ ] Measure context pruning savings (target: 95-99%)
5. [ ] Measure protocol efficiency (target: 90%+)
6. [ ] Generate report

**Deliverables:**
- `benches/token_savings_bench.rs` — Benchmark suite
- `docs/TOKEN-SAVINGS-VALIDATION.md` — Validation report

**Success Criteria:**
- [ ] Validated 50-90% prefix caching savings
- [ ] Validated 89-98% diff communication savings
- [ ] Validated 95-99% context pruning savings
- [ ] Total validated savings: 90-95%

**Dependencies:** All previous sprints (need complete system)

---

#### Sprint A5: Production Readiness & Documentation

**Owner:** Agent A (Documentation Specialist)  
**Duration:** 2 days  
**Priority:** 🟡 MEDIUM (polish)

**Tasks:**
1. [ ] Write user documentation:
   - How to enable optimizations
   - Configuration options
   - Expected savings
2. [ ] Write API documentation:
   - For developers extending the system
   - Protocol specification
   - Integration guide
3. [ ] Create migration guide:
   - From natural language to protocol
   - From full files to diffs
4. [ ] Add troubleshooting guide:
   - Common issues
   - Debugging tips
5. [ ] Create demo video (show token savings)
6. [ ] Final validation

**Deliverables:**
- `docs/MULTI-AGENT-OPTIMIZATION.md` — User guide
- `docs/API-COST-OPTIMIZATION.md` — Developer guide
- `docs/MIGRATION-GUIDE.md` — Migration guide
- Demo video (screen recording)

**Success Criteria:**
- [ ] Documentation complete
- [ ] Demo shows 90%+ cost reduction
- [ ] Production-ready

**Dependencies:** Sprint A4 (need benchmark results)

---

## 📊 Resource Allocation

### Team

| Agent | Role | Sprints | Time Commitment |
|-------|------|---------|-----------------|
| **A** | Documentation | A4, A5 | 50% (1 week) |
| **B** | Storage/Context | B9, B10 | 100% (1 week) |
| **C** | Implementation | C7, C8, C11, C12 | 100% (2 weeks) |

### Infrastructure

| Resource | Requirement | Provided By |
|----------|-------------|-------------|
| API Credits | $100 for testing | Existing budget |
| Benchmarking | Large codebases | Open source repos |
| CI/CD | Standard Rust CI | GitHub Actions |

---

## 📈 Success Metrics

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| **Prefix Caching Savings** | 50-90% | Tokens sent to API |
| **Diff Communication Savings** | 89-98% | Output tokens |
| **Context Pruning Savings** | 95-99% | Context tokens |
| **Protocol Efficiency** | 90%+ | Binary vs natural language |
| **Total Cost Reduction** | 90-95% | Monthly API bill |
| **Latency Reduction** | 30-50% | Time-to-First-Token |

---

## 🚨 Risk Management

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| API doesn't support caching | Low | High | Use Anthropic/OpenAI (both support) |
| Agents resist diff-only | Medium | High | Auto-conversion + enforcement |
| SCIP parsing too slow | Medium | Medium | Cache ASTs, incremental parsing |
| Protocol too complex | Low | Medium | Keep schema simple, document well |

---

## 🔗 Dependencies

### Internal Dependencies

| Sprint | Depends On | Blocked By |
|--------|------------|------------|
| C7 | None | None |
| C8 | C7 | None |
| B9 | None | None |
| B10 | B9 | None |
| C11 | C7 | None |
| C12 | C11 | None |
| A4 | All sprints | None |
| A5 | A4 | None |

### External Dependencies

| Dependency | Purpose | Status |
|------------|---------|--------|
| Anthropic API | Prompt caching | ✅ Available |
| OpenAI API | Prefix caching | ✅ Available |
| rust-analyzer | Rust parsing | ✅ Available |
| scip-typescript | TS parsing | ✅ Available |
| scip-python | Python parsing | ✅ Available |

---

## 📅 Timeline

### Week 1: API Integration
- Sprint C7: API Client with Prefix Caching ✅
- Sprint C8: Diff-Only Enforcement ✅

### Week 2: Graph-Based Context
- Sprint B9: SCIP Indexing ✅
- Sprint B10: Context Pruning ✅

### Week 3: Agent Communication Protocol
- Sprint C11: Agent Message Protocol ✅
- Sprint C12: Graph Workflow Enforcement ✅

### Week 4: Validation & Documentation
- Sprint A4: Token Savings Benchmarking ✅
- Sprint A5: Production Readiness ✅

**Total Duration:** 4 weeks (8 sprints)

---

## ✅ Definition of Done

Project is complete when:
- [ ] All 8 sprints complete
- [ ] All tests passing (unit + integration)
- [ ] All benchmarks meet targets
- [ ] Validated 90-95% cost reduction
- [ ] Documentation complete
- [ ] Production-ready

---

**Ready to execute. This plan enables 90-95% reduction in multi-agent API costs.**
