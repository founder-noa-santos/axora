# Experience-as-Parameters: RAG-Based Expertise

**Date:** 2026-03-16
**Status:** ADOPTED
**Replaces:** Team-Based Expertise Accumulation (REJECTED)
**Source:** R-10 Research Findings

---

## 🎯 Overview

### The Anthropomorphism Fallacy

**Claim (REJECTED):** "Agents accumulate expertise through team collaboration over time"

**Reality:** 
> "Agents do not learn interactively like human engineers; their efficacy relies entirely on RAG architectures and state externalization."

**Translation:** "Domain expertise" = Better retrieval, not team structure

### What This Means

| Human Team | AI Agent System |
|------------|-----------------|
| Learns from experience | ❌ No internal learning |
| Builds expertise over time | ❌ No memory between runs |
| Shares knowledge via collaboration | ❌ No implicit knowledge transfer |
| **Externalizes via documentation** | ✅ **RAG retrieval** |

**Key Insight:** Agents don't "learn" — they **retrieve better**.

---

## 🏗️ Architecture

### Three Memory Types

```
┌─────────────────────────────────────────────────────────────────┐
│                    Agent Memory Architecture                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────┐  ┌──────────────────┐  ┌───────────────┐ │
│  │ Semantic Memory  │  │ Episodic Memory  │  │Procedural Mem.│ │
│  ├──────────────────┤  ├──────────────────┤  ├───────────────┤ │
│  │ • API contracts  │  │ • Debug sessions │  │ • SKILL.md    │ │
│  │ • Data schemas   │  │ • Terminal output│  │ • Workflows   │ │
│  │ • Past patterns  │  │ • Decision traces│  │ • Triggers    │ │
│  └──────────────────┘  └──────────────────┘  └───────────────┘ │
│           │                    │                    │           │
│           └────────────────────┼────────────────────┘           │
│                                │                                 │
│                       ┌────────▼────────┐                       │
│                       │  Unified RAG    │                       │
│                       │  Retrieval      │                       │
│                       └─────────────────┘                       │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 📚 Semantic Memory (Vector Store)

### What It Stores

**Factual knowledge about domains:**
- API contracts (endpoints, request/response schemas)
- Data models (database schemas, type definitions)
- Design patterns (auth flows, payment processing)
- Best practices (security, performance)

### Structure

```rust
pub struct SemanticMemory {
    stores: HashMap<String, VectorStore>,
}

impl SemanticMemory {
    pub fn new() -> Self {
        let mut stores = HashMap::new();
        
        // Domain-specific stores
        stores.insert("auth", VectorStore::new("auth-contracts"));
        stores.insert("billing", VectorStore::new("billing-schemas"));
        stores.insert("api", VectorStore::new("api-patterns"));
        
        // Cross-domain knowledge
        stores.insert("patterns", VectorStore::new("design-patterns"));
        stores.insert("best_practices", VectorStore::new("best-practices"));
        
        Self { stores }
    }
    
    pub async fn retrieve(&self, domain: &str, query: &str) -> Result<Vec<Document>> {
        let store = self.stores.get(domain)
            .ok_or(Error::UnknownDomain(domain.to_string()))?;
        
        // Retrieve API contracts, schemas, patterns
        store.hybrid_search(query, 10).await
    }
}
```

### Example Content

```json
{
  "id": "auth-001",
  "domain": "auth",
  "type": "api_contract",
  "content": {
    "endpoint": "POST /api/auth/login",
    "request": {
      "username": "string",
      "password": "string"
    },
    "response": {
      "token": "string",
      "expires_in": "number"
    },
    "patterns": ["jwt", "session"],
    "security": ["rate_limit", "bcrypt"]
  }
}
```

---

## 📔 Episodic Memory (Conversation Logs)

### What It Stores

**Specific past experiences:**
- Debugging sessions (problem → solution)
- Terminal outputs (commands, errors, fixes)
- Decision traces (why a choice was made)
- Code review feedback

### Structure

```rust
pub struct EpisodicMemory {
    logs: VectorStore,
    max_age_days: u64,
}

impl EpisodicMemory {
    pub fn new() -> Self {
        Self {
            logs: VectorStore::new("conversation-logs"),
            max_age_days: 30, // Keep 30 days of history
        }
    }
    
    pub async fn add_experience(&mut self, experience: &Experience) -> Result<()> {
        // Store conversation with metadata
        self.logs.insert(&ExperienceDocument {
            id: uuid(),
            timestamp: Utc::now(),
            task: experience.task.clone(),
            conversation: experience.conversation.clone(),
            outcome: experience.outcome.clone(),
            tokens_used: experience.tokens_used,
        }).await
    }
    
    pub async fn retrieve_similar(&self, task: &str) -> Result<Vec<Experience>> {
        // Find similar past experiences
        let docs = self.logs.similarity_search(task, 5).await?;
        
        // Filter by recency (prefer recent experiences)
        let recent = docs.iter()
            .filter(|d| d.age_days() < self.max_age_days)
            .take(3)
            .collect();
        
        Ok(recent)
    }
}
```

### Example Content

```json
{
  "id": "exp-2026-03-15-001",
  "timestamp": "2026-03-15T10:30:00Z",
  "task": "Fix OAuth token refresh bug",
  "conversation": [
    {"role": "user", "content": "Token refresh returns 401"},
    {"role": "assistant", "content": "Let me check the refresh endpoint..."},
    {"role": "tool", "content": "Found: token expiry check uses wrong field"},
    {"role": "assistant", "content": "Fixed: changed `exp` to `expires_at`"}
  ],
  "outcome": "success",
  "tokens_used": 2500,
  "files_changed": ["src/auth/token.rs"]
}
```

---

## 🔧 Procedural Memory (Skill Files)

### What It Stores

**Executable workflows:**
- SKILL.md files (agent-native skills)
- Trigger conditions (when to use skill)
- Step-by-step procedures

### Structure

```rust
pub struct ProceduralMemory {
    skills: HashMap<String, Skill>,
    trigger_index: TriggerIndex,
}

impl ProceduralMemory {
    pub fn load_skills(&mut self, skills_dir: &Path) -> Result<()> {
        for entry in fs::read_dir(skills_dir)? {
            let path = entry?.path();
            if path.extension() == Some("md".as_ref()) {
                let skill = Skill::from_markdown(&path)?;
                self.skills.insert(skill.id.clone(), skill);
                
                // Index triggers
                for trigger in &skill.triggers {
                    self.trigger_index.add(trigger, &skill.id);
                }
            }
        }
        Ok(())
    }
    
    pub fn get_relevant_skills(&self, context: &str) -> Vec<&Skill> {
        // Find skills whose triggers match context
        let trigger_ids = self.trigger_index.match_triggers(context);
        
        trigger_ids.iter()
            .filter_map(|id| self.skills.get(id))
            .collect()
    }
}
```

### Example SKILL.md

```markdown
---
id: rust-auth-implementation
name: Rust Authentication Implementation
triggers:
  - "implement auth"
  - "add login"
  - "OAuth integration"
  - "JWT tokens"
domains:
  - rust
  - backend
---

# Authentication Implementation Skill

## When to Use

- User requests authentication feature
- Adding OAuth provider integration
- Implementing JWT token handling

## Procedure

1. **Check existing auth patterns**
   - Query semantic memory for auth contracts
   - Retrieve past auth implementations

2. **Generate auth boilerplate**
   - Use JWT template from patterns
   - Apply project-specific conventions

3. **Add security measures**
   - Rate limiting on login endpoint
   - Bcrypt password hashing
   - Token expiry validation

4. **Write tests**
   - Unit tests for token validation
   - Integration tests for login flow
   - Security tests for common vulnerabilities

## Files to Create

- `src/auth/mod.rs` — Auth module
- `src/auth/token.rs` — Token handling
- `src/auth/login.rs` — Login endpoint
- `tests/auth_test.rs` — Auth tests
```

---

## 🔍 Retrieval Strategy

### Late-Interaction (ColBERT-Style)

**Why not single embedding?**
- Loses token-level relevance
- Poor for code (structure matters)
- 15-20% lower precision

**Late-interaction approach:**
```rust
pub struct ColBERTRetriever {
    embedder: EmbeddingModel,
    index: ColBERTIndex,
}

impl ColBERTRetriever {
    pub fn retrieve(&self, query: &str, k: usize) -> Result<Vec<Document>> {
        // 1. Embed query tokens individually
        let query_embeddings = self.embedder.encode_tokens(query);
        
        // 2. For each query token, find max similarity in document
        let scores = self.index.documents.iter().map(|doc| {
            query_embeddings.iter().map(|q_emb| {
                doc.token_embeddings.iter()
                    .map(|d_emb| cosine_similarity(q_emb, d_emb))
                    .max()
                    .unwrap_or(0.0)
            }).sum::<f32>()
        }).collect::<Vec<f32>>();
        
        // 3. Top-k documents by score
        Ok(self.top_k(&scores, k))
    }
}
```

### Hybrid Search (BM25 + Vectors)

```rust
pub fn hybrid_search(
    query: &str,
    bm25_index: &BM25Index,
    vector_index: &VectorIndex,
    k: usize,
) -> Vec<Document> {
    // 1. BM25 search (keyword matching)
    let bm25_results = bm25_index.search(query, k * 2);
    
    // 2. Vector search (semantic matching)
    let vector_results = vector_index.search(query, k * 2);
    
    // 3. Merge with reciprocal rank fusion
    let merged = reciprocal_rank_fusion(
        bm25_results,
        vector_results,
        k,
    );
    
    // 4. Rerank with cross-encoder (optional, for precision)
    let reranked = cross_encoder_rerank(query, merged, k);
    
    reranked
}
```

### Top-k with Reranking

```rust
pub fn rerank(query: &str, candidates: Vec<Document>, k: usize) -> Vec<Document> {
    // Use cross-encoder for fine-grained scoring
    let scores = candidates.iter().map(|doc| {
        cross_encoder.score(query, &doc.content)
    }).collect::<Vec<f32>>();
    
    // Sort by score, take top-k
    let mut indexed: Vec<_> = candidates.iter().zip(scores.iter()).collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
    
    indexed.iter().take(k).map(|(doc, _)| (*doc).clone()).collect()
}
```

---

## ⚡ Token Efficiency

### Comparison: DDD vs RAG

| Approach | Token Overhead | Why |
|----------|---------------|-----|
| **DDD Teams** | 40%+ | ACL translation + merging + routing |
| **RAG Retrieval** | <10% | Just retrieve relevant context |

### RAG Token Breakdown

```
User Request: "Add OAuth login"

RAG Retrieval:
- Auth patterns:      ~200 tokens
- Past successes:     ~100 tokens
- API contracts:      ~150 tokens
- SKILL triggers:     ~50 tokens
─────────────────────────────────
Total RAG overhead:   ~500 tokens (<10% of 8K context)

DDD Teams Overhead:
- Auth team context:  ~800 tokens
- ACL translation:    ~400 tokens
- Routing decision:   ~300 tokens
- Merge coordination: ~500 tokens
─────────────────────────────────
Total DDD overhead:   ~2000 tokens (25%+ of 8K context)
```

### Optimization Techniques

**1. Retrieve Only Relevant Patterns**
```rust
// Bad: Retrieve entire domain store
let all_auth = rag.retrieve_all("auth").await?; // 5000+ tokens

// Good: Retrieve only relevant patterns
let relevant = rag.retrieve("auth", "OAuth login flow").await?; // ~500 tokens
```

**2. Late-Binding Context**
```rust
// Bad: Include all context upfront
let context = build_full_context(); // 3000+ tokens

// Good: Add context only when needed
let context = build_minimal_context();
if needs_more_info(&context) {
    context.add(retrieve_specific("token_validation"));
}
```

**3. Compress Retrieved Documents**
```rust
// Compress retrieved patterns to essential info
let compressed = retrieved.iter().map(|doc| {
    DocumentSummary {
        id: doc.id.clone(),
        key_points: extract_key_points(&doc.content),
    }
}).collect();
```

---

## 📊 Performance Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Retrieval Latency | <100ms | P95 latency |
| Token Overhead | <10% | RAG tokens / total tokens |
| Precision@5 | >80% | Relevant docs in top 5 |
| Recall@10 | >90% | All relevant docs in top 10 |

---

## 🔗 Related Documents

- [`GRAPH-WORKFLOW-DESIGN.md`](./GRAPH-WORKFLOW-DESIGN.md) — Graph-based workflow
- [`PHASE-2-PIVOT-GRAPH-WORKFLOW.md`](./PHASE-2-PIVOT-GRAPH-WORKFLOW.md) — Pivot decision
- [`DDD-TDD-AGENT-TEAMS.md`](./DDD-TDD-AGENT-TEAMS.md) — Historical analysis (REJECTED)

---

## 📝 Key Takeaways

1. **Agents don't learn** — They retrieve better
2. **Expertise is externalized** — In vector stores, not agent structure
3. **RAG is 4x more token-efficient** — <10% vs 40%+ for DDD
4. **Late-interaction retrieval** — 15-20% better for code
5. **Hybrid search** — BM25 + vectors for precision + recall

---

**"Expertise accumulation" is a metaphor. The engineering reality is RAG.**
