# Graph-Based Deterministic Workflow

**Date:** 2026-03-16
**Status:** ADOPTED
**Replaces:** DDD Agent Teams (REJECTED)
**Source:** R-10 Research Findings

---

## 📋 Overview

### What

A **state machine with deterministic nodes** where:
- **Nodes** = Agent roles (Planner, Executor, Reviewer)
- **Edges** = Explicit state transitions with guard conditions
- **Memory** = Domain-specific RAG (not agent-specialized knowledge)

### Why

| Metric | DDD Teams | Graph + RAG | Improvement |
|--------|-----------|-------------|-------------|
| Coordination | O(N²) | O(N) | **Linear scaling** |
| Token Overhead | 40%+ | <10% | **75% reduction** |
| Cross-Domain | Complex routing | Direct access | **Simple** |
| Implementation | 120h+ | ~40h | **66% less** |

### Who

**Target:** Individual developers (OPENAKTA's primary user)

**Not for:** Enterprise teams with 20+ concurrent agents (use DDD if needed)

---

## 🏗️ Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────────────────┐
│                    Graph-Based Workflow                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐                  │
│  │ Planner  │───▶│ Executor │───▶│ Reviewer │                  │
│  │  Node    │    │  Node    │    │  Node    │                  │
│  └──────────┘    └──────────┘    └──────────┘                  │
│       │               │               │                         │
│       ▼               ▼               ▼                         │
│  ┌─────────────────────────────────────────────────┐           │
│  │           Domain RAG (Vector Stores)            │           │
│  ├─────────────┬─────────────┬─────────────────┤           │
│  │ Auth Store  │Billing Store│ Past Successes  │           │
│  └─────────────┴─────────────┴─────────────────┘           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Key Principles

1. **Deterministic Routing** — No semantic routing decisions
2. **Explicit State Transitions** — Guard conditions validate before moving
3. **Error Edges** — Retry or escalate on failure
4. **Domain Knowledge in RAG** — Not embedded in agent structure

---

## 🎯 Nodes (Agent Roles)

### Planner Node

**Responsibility:** Decompose tasks, create execution graph

**Input:** User request (natural language)

**Output:** Structured task graph with dependencies

**Example:**
```rust
pub struct PlannerNode {
    id: String,
    llm: LlmClient,
    rag: DomainRag,
}

impl PlannerNode {
    pub async fn execute(&self, request: &str) -> Result<TaskGraph> {
        // Retrieve relevant domain patterns
        let patterns = self.rag.retrieve("past_plans", request).await?;
        
        // Decompose into subtasks
        let graph = self.llm.decompose(request, &patterns).await?;
        
        // Validate graph structure
        self.validate_graph(&graph)?;
        
        Ok(graph)
    }
}
```

**Token Efficiency:** Retrieves only relevant planning patterns (<5% overhead)

---

### Executor Node

**Responsibility:** Write code, execute tasks

**Input:** Task from Planner, domain context from RAG

**Output:** Code changes, test results

**Key Feature:** **Full repository access** (not bounded by domain)

**Example:**
```rust
pub struct ExecutorNode {
    id: String,
    llm: LlmClient,
    rag: DomainRag,
    codebase: CodebaseIndex,
}

impl ExecutorNode {
    pub async fn execute(&self, task: &Task) -> Result<CodeChange> {
        // Retrieve domain-specific patterns
        let auth_patterns = self.rag.retrieve("auth", &task.description).await?;
        let billing_patterns = self.rag.retrieve("billing", &task.description).await?;
        
        // Get full repo context (not bounded)
        let context = self.codebase.get_relevant_files(&task).await?;
        
        // Write code with all context
        let code = self.llm.generate(&task, &auth_patterns, &billing_patterns, &context).await?;
        
        Ok(code)
    }
}
```

**Why not domain-specialized?**
- Individual dev tasks often cross domains (e.g., "Add paid OAuth tier")
- Full context access is more efficient than domain routing
- Domain knowledge comes from RAG, not agent structure

---

### Reviewer Node

**Responsibility:** Validate code, run tests, approve changes

**Input:** Code changes from Executor

**Output:** Approval, rejection with feedback

**Example:**
```rust
pub struct ReviewerNode {
    id: String,
    llm: LlmClient,
    test_runner: TestRunner,
}

impl ReviewerNode {
    pub async fn execute(&self, changes: &CodeChange) -> Result<Review> {
        // Run tests
        let test_result = self.test_runner.run(&changes.files).await?;
        
        // Validate code quality
        let quality = self.llm.review(&changes.code).await?;
        
        // Approve or reject
        if test_result.passed && quality.score > 0.8 {
            Ok(Review::Approved)
        } else {
            Ok(Review::Rejected {
                reason: format!("Tests: {:?}, Quality: {:?}", test_result, quality),
            })
        }
    }
}
```

---

## 🔗 Edges (Deterministic Routing)

### Edge Types

| Type | Description | Example |
|------|-------------|---------|
| **Success** | Task completed, move to next node | Executor → Reviewer |
| **Retry** | Transient failure, retry same node | Executor → Executor (max 3x) |
| **Escalate** | Permanent failure, move to error handler | Executor → ErrorNode |
| **Conditional** | Branch based on guard condition | If tests pass → Reviewer, else → Executor |

### Guard Conditions

```rust
pub struct Edge {
    from: NodeId,
    to: NodeId,
    guard: Box<dyn Guard>,
}

pub trait Guard {
    fn evaluate(&self, state: &WorkflowState) -> bool;
}

// Example guards
pub struct TestsPassedGuard;
impl Guard for TestsPassedGuard {
    fn evaluate(&self, state: &WorkflowState) -> bool {
        state.test_result.map_or(false, |r| r.passed)
    }
}

pub struct MaxRetriesGuard {
    max_retries: usize,
}
impl Guard for MaxRetriesGuard {
    fn evaluate(&self, state: &WorkflowState) -> bool {
        state.retry_count < self.max_retries
    }
}
```

### Error Edges

```rust
// Every node has error edges
pub enum ErrorEdge {
    Retry { max_attempts: usize },
    Escalate { to: NodeId },
    Fallback { to: NodeId, message: String },
}

// Example: Executor error handling
impl ExecutorNode {
    pub fn on_error(&self, error: &Error) -> ErrorEdge {
        match error {
            Error::Transient(_) => ErrorEdge::Retry { max_attempts: 3 },
            Error::MissingContext(_) => ErrorEdge::Fallback {
                to: "planner".to_string(),
                message: "Need more context".to_string(),
            },
            _ => ErrorEdge::Escalate {
                to: "error-handler".to_string(),
            },
        }
    }
}
```

---

## 🧠 Domain Knowledge (RAG, not Agents)

### Vector Stores Per Domain

```rust
pub struct DomainRag {
    stores: HashMap<String, VectorStore>,
}

impl DomainRag {
    pub fn new() -> Self {
        let mut stores = HashMap::new();
        
        // Domain-specific stores
        stores.insert("auth", VectorStore::new("auth-patterns"));
        stores.insert("billing", VectorStore::new("billing-patterns"));
        stores.insert("api", VectorStore::new("api-patterns"));
        stores.insert("ui", VectorStore::new("ui-patterns"));
        
        // Cross-domain memory
        stores.insert("past_successes", VectorStore::new("past-wins"));
        stores.insert("mistakes", VectorStore::new("lessons-learned"));
        
        Self { stores }
    }
    
    pub async fn retrieve(&self, domain: &str, query: &str) -> Result<Vec<Document>> {
        let store = self.stores.get(domain)
            .ok_or(Error::UnknownDomain(domain.to_string()))?;
        
        // Hybrid search: BM25 + vectors
        let bm25_results = store.bm25_search(query, 10).await?;
        let vector_results = store.vector_search(query, 10).await?;
        
        // Merge and rerank
        let merged = self.rerank(bm25_results, vector_results);
        
        Ok(merged)
    }
}
```

### Late-Interaction Retrieval (ColBERT-Style)

```rust
// Instead of single embedding, use token-level embeddings
pub struct ColBERTIndex {
    token_embeddings: Vec<Vec<f32>>,
    max_similarities: MaxSimIndex,
}

impl ColBERTIndex {
    pub fn retrieve(&self, query: &str, k: usize) -> Vec<Document> {
        // Embed query tokens
        let query_embeddings = self.embedder.encode(query);
        
        // Max similarity per token
        let scores = query_embeddings
            .iter()
            .map(|q_emb| {
                self.max_similarities
                    .max_similarity(q_emb)
            })
            .sum::<f32>();
        
        // Top-k documents
        self.top_k(scores, k)
    }
}
```

**Why late-interaction?**
- Better precision for code retrieval
- Captures token-level relevance
- 15-20% better than single embedding for code

---

## 📊 Comparison: DDD vs Graph + RAG

### Architecture Comparison

| Aspect | DDD Teams | Graph + RAG | Winner |
|--------|-----------|-------------|--------|
| **Agent Structure** | Domain-specialized teams | Generalist nodes | Graph |
| **Knowledge Storage** | Team memory (implicit) | Vector stores (explicit) | Graph |
| **Coordination** | ACL translation + merging | Direct edges | Graph |
| **Cross-Domain** | Complex routing | Direct RAG access | Graph |
| **Error Handling** | Team-level retry | Node-level retry + escalation | Graph |
| **Token Overhead** | 40%+ (ACL + merging) | <10% (just RAG) | Graph |
| **Implementation** | 120+ hours | ~40 hours | Graph |

### Performance Comparison

| Scenario | DDD Teams | Graph + RAG | Winner |
|----------|-----------|-------------|--------|
| **Single-domain task** | Fast (specialist) | Fast (RAG retrieval) | Tie |
| **Cross-domain task** | Slow (routing overhead) | Fast (direct access) | Graph |
| **Sequential tasks** | Slow (handoffs) | Fast (pipeline) | Graph |
| **Parallel tasks** | Fast (team parallelism) | Medium (node limits) | DDD |
| **Learning curve** | High (DDD concepts) | Low (state machine) | Graph |

### When to Use Each

**Use Graph + RAG (OPENAKTA default):**
- Individual developers
- <10 concurrent agents
- Mixed domain tasks
- Low latency requirements
- Token efficiency critical

**Use DDD (enterprise only):**
- 20+ concurrent agents
- Strict domain boundaries required
- Parallel task execution dominant
- Token cost not a concern

---

## 🚀 Implementation Plan

### Phase 1: State Machine Primitives (8 hours)

```rust
// Core primitives
pub trait Node {
    fn id(&self) -> &str;
    async fn execute(&self, state: &WorkflowState) -> Result<NodeOutput>;
    fn on_error(&self, error: &Error) -> ErrorEdge;
}

pub struct Edge {
    from: String,
    to: String,
    guard: Box<dyn Guard>,
}

pub struct WorkflowGraph {
    nodes: HashMap<String, Box<dyn Node>>,
    edges: Vec<Edge>,
    state: WorkflowState,
}
```

**Deliverables:**
- [ ] Node trait definition
- [ ] Edge with guard conditions
- [ ] WorkflowGraph execution engine
- [ ] Error handling with retry/escalation

---

### Phase 2: Domain RAG Integration (16 hours)

```rust
// Domain-specific vector stores
pub struct DomainRag {
    auth_store: VectorStore,
    billing_store: VectorStore,
    past_successes: VectorStore,
}

// Late-interaction retrieval
pub struct ColBERTIndex {
    index: VectorStore,
    embedder: EmbeddingModel,
}
```

**Deliverables:**
- [ ] Domain vector stores (auth, billing, api, ui)
- [ ] Past successes memory bank
- [ ] Late-interaction retrieval implementation
- [ ] Hybrid search (BM25 + vectors)

---

### Phase 3: Integration (16 hours)

```rust
// Integrate with existing agents
pub struct AgentWithGraph {
    agent: Agent,
    graph: WorkflowGraph,
    rag: DomainRag,
}

// Migration path from DDD concepts
impl AgentWithGraph {
    pub fn from_ddd_config(ddd_config: DddConfig) -> Self {
        // Convert domain teams to domain RAG stores
        let rag = DomainRag::from_ddd_teams(&ddd_config.teams);
        
        // Create generalist nodes
        let graph = WorkflowGraph::new()
            .add_node(PlannerNode::new())
            .add_node(ExecutorNode::new())
            .add_node(ReviewerNode::new());
        
        Self {
            agent: ddd_config.agent,
            graph,
            rag,
        }
    }
}
```

**Deliverables:**
- [ ] Integrate graph with existing agent framework
- [ ] Migration path from DDD concepts
- [ ] Documentation updates
- [ ] Integration tests

---

## 📈 Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Coordination Overhead | O(N) | Measure edges traversed |
| Token Efficiency | <10% overhead | Compare input vs RAG tokens |
| Cross-Domain Latency | <5% penalty | Measure cross-domain task time |
| Implementation Time | <40 hours | Track development time |
| User Satisfaction | >80% positive | User feedback surveys |

---

## 🔗 Related Documents

- [`PHASE-2-PIVOT-GRAPH-WORKFLOW.md`](./PHASE-2-PIVOT-GRAPH-WORKFLOW.md) — Pivot decision
- [`RAG-EXPERTISE-DESIGN.md`](./RAG-EXPERTISE-DESIGN.md) — RAG-based expertise design
- [`DDD-TDD-AGENT-TEAMS.md`](./DDD-TDD-AGENT-TEAMS.md) — Historical DDD analysis (REJECTED)

---

## 📝 Design Decisions

### Why Deterministic Routing?

**Problem:** Semantic routing (LLM decides next node) is:
- Unpredictable (same input → different paths)
- Hard to debug (why did it choose X?)
- Token expensive (need reasoning in prompt)

**Solution:** Deterministic routing with guard conditions:
- Predictable (same input → same path)
- Debuggable (check guard condition)
- Token efficient (no reasoning needed)

### Why Generalist Agents?

**Problem:** Domain-specialized agents:
- Can't handle cross-domain tasks efficiently
- Require complex routing logic
- Create artificial knowledge silos

**Solution:** Generalist agents with domain RAG:
- Handle any task (full repo access)
- Simple routing (deterministic graph)
- Domain knowledge from retrieval (not structure)

### Why Late-Interaction Retrieval?

**Problem:** Single embedding retrieval:
- Loses token-level relevance
- Poor for code (structure matters)
- 15-20% lower precision

**Solution:** Late-interaction (ColBERT-style):
- Token-level embeddings
- Max similarity per token
- Better code retrieval

---

**This architecture is OPTIMIZED for individual developers, not enterprise teams.**

**For enterprise use cases (20+ agents), consider DDD with the understanding of coordination overhead.**
