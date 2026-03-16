# Phase 2 Integration: Code Influence Graph & Business Rule Mapping

**Date:** 2026-03-16  
**Source:** Research — "Code Influence Graph and Business Rule Mapping: An LLM-Independent Architecture"  
**Impact:** VALIDATES R-13 + Provides Implementation Details (SCIP, IncSCC, Bidirectional Traceability)  

---

## ✅ Executive Summary

**This research VALIDATES our R-13 Influence Graph research and provides CRITICAL implementation details.**

**Key Confirmations:**
1. ✅ **Static Analysis > LLM for dependencies** — 95-99% token reduction
2. ✅ **SCIP Protocol** — Language-agnostic code indexing (replaces LSIF)
3. ✅ **Influence Vector** — Mathematical representation of code impact
4. ✅ **Incremental Transitive Closure** — IncSCC algorithm (O(n²) not O(n³))
5. ✅ **Business Rules as Markdown + YAML** — Machine-readable, human-writable
6. ✅ **Bidirectional Traceability** — @req annotations in code + YAML frontmatter

**NEW Insights (Implementation Details):**
1. **Language-Specific Parsers:** rust-analyzer, ts-morph, pyan3, scip-typescript, scip-python
2. **Influence Vector Metrics:** CBO (Coupling Between Objects), RFC (Response for Class)
3. **Context Pruning:** Deterministic graph traversal → 500-2,500 tokens (not 50,000+)
4. **Automated RTM:** Requirements Traceability Matrix (auto-generated from graph)

---

## 📊 Research Validation Matrix

| Our Decision (R-13) | This Research Says | Verdict |
|---------------------|--------------------|---------|
| Static Analysis > LLM | ✅ 95-99% token reduction | **VALIDATED** |
| Influence Graph | ✅ SCIP + Influence Vector | **VALIDATED + ENHANCED** |
| Business Rule Docs | ✅ Markdown + YAML frontmatter | **VALIDATED** |
| Bidirectional Links | ✅ @req annotations + YAML | **VALIDATED** |
| Context Pruning | ✅ Deterministic graph traversal | **VALIDATED** |
| Incremental Updates | ✅ IncSCC algorithm (O(n²)) | **VALIDATED** |

**Conclusion:** R-13 was CORRECT. This research provides IMPLEMENTATION DETAILS.

---

## 🔄 Architecture Updates (Refinements, Not Pivots)

### 1. SCIP Protocol for Language-Agnostic Indexing

**Current (from R-13):**
```rust
pub struct CodeInfluenceGraph {
    nodes: HashMap<FileId, FileNode>,
    edges: Vec<DependencyEdge>,
}
```

**Updated (with SCIP):**
```rust
pub struct CodeInfluenceGraph {
    // SCIP-indexed nodes (language-agnostic)
    scip_index: SCIPIndex,
    
    // Influence vectors (pre-calculated)
    influence_vectors: HashMap<FileId, InfluenceVector>,
    
    // Business rule links
    rule_links: HashMap<FileId, Vec<BusinessRuleId>>,
}

pub struct SCIPIndex {
    // Protocol Buffers format (not JSON)
    // Human-readable string identifiers (not opaque numeric IDs)
    // Package ownership (manager, name, version, symbol)
    symbols: HashMap<String, SymbolMetadata>,
    occurrences: Vec<Occurrence>,
}

impl CodeInfluenceGraph {
    /// Build from SCIP index (language-specific parsers)
    pub fn from_scip(scip_index: SCIPIndex) -> Result<Self> {
        // Extract nodes from SCIP occurrences
        // Build influence vectors
        // Link business rules
    }
}
```

**Why:** SCIP is the successor to LSIF — optimized Protobuf format, human-readable identifiers, cross-repository navigation.

---

### 2. Language-Specific Parsers

**NEW: Parser Registry**
```rust
pub struct ParserRegistry {
    parsers: HashMap<Language, Box<dyn CodeParser>>,
}

impl ParserRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            parsers: HashMap::new(),
        };
        
        // Register language-specific parsers
        registry.register(Language::Rust, Box::new(RustAnalyzer::new()));
        registry.register(Language::TypeScript, Box::new(TsMorph::new()));
        registry.register(Language::Python, Box::new(Pyan3::new()));
        registry.register(Language::Go, Box::new(GoAST::new()));
        
        registry
    }
    
    pub fn parse(&self, language: Language, codebase: &Path) -> Result<SCIPIndex> {
        let parser = self.parsers.get(&language)
            .ok_or_else(|| Error::UnsupportedLanguage(language))?;
        
        parser.generate_scip(codebase)
    }
}

/// Language-specific parsers
pub trait CodeParser: Send + Sync {
    fn generate_scip(&self, codebase: &Path) -> Result<SCIPIndex>;
}

pub struct RustAnalyzer; // rust-analyzer scip
pub struct TsMorph;      // ts-morph + scip-typescript
pub struct Pyan3;        // pyan3 + scip-python
pub struct GoAST;        // go/ast + godepgraph
```

**Why:** Each language requires specialized parser for accurate AST + call graph extraction.

---

### 3. Influence Vector with Metrics

**Current (from R-13):**
```rust
pub struct InfluenceVector {
    pub file_id: FileId,
    pub direct_dependencies: Vec<FileId>,
    pub reverse_dependencies: Vec<FileId>,
    pub call_graph_depth: usize,
    pub business_rule_count: usize,
    pub transitive_closure: Vec<FileId>,
}
```

**Updated (with CBO, RFC metrics):**
```rust
pub struct InfluenceVector {
    pub file_id: FileId,
    
    // Coupling metrics
    pub afferent_coupling: usize,  // C_in (Fan-in)
    pub efferent_coupling: usize,  // C_out (Fan-out)
    pub coupling_between_objects: usize, // CBO
    
    // Complexity metrics
    pub call_graph_depth: usize,
    pub response_for_class: usize, // RFC
    
    // Business context
    pub business_rule_count: usize,
    
    // Impact analysis
    pub transitive_closure: Vec<FileId>,
}

impl InfluenceVector {
    /// Calculate influence score (for context pruning)
    pub fn influence_score(&self) -> f32 {
        // High fan-in = core component (needs broader context)
        // High fan-out = fragile component (needs dependency context)
        (self.afferent_coupling as f32 * 2.0) +
        (self.efferent_coupling as f32) +
        (self.call_graph_depth as f32 * 0.5) +
        (self.business_rule_count as f32 * 3.0)
    }
}
```

**Why:** CBO and RFC are established software engineering metrics for maintainability analysis.

---

### 4. Incremental Transitive Closure (IncSCC)

**NEW: Incremental Graph Updates**
```rust
pub struct IncrementalGraph {
    // Strongly Connected Components (condensed cycles)
    sccs: Vec<StronglyConnectedComponent>,
    
    // Topological ordering (for fast reachability)
    topological_order: Vec<NodeId>,
    
    // Disjoint-set for union-find (near O(1) merges)
    disjoint_sets: UnionFind<NodeId>,
}

impl IncrementalGraph {
    /// Update graph when edge is added/removed (not full recalc)
    pub fn update_edge(&mut self, from: NodeId, to: NodeId, op: EdgeOp) -> Result<()> {
        match op {
            EdgeOp::Add => self.add_edge_incremental(from, to)?,
            EdgeOp::Remove => self.remove_edge_incremental(from, to)?,
        }
        
        // Update SCCs (only affected components)
        self.update_sccs(from, to)?;
        
        // Update topological order (local re-sort)
        self.update_topological_order(from, to)?;
        
        Ok(())
    }
    
    /// Get transitive closure in near-linear time (not O(n³))
    pub fn get_transitive_closure(&self, node: NodeId) -> Vec<NodeId> {
        // Use disjoint-set for fast reachability
        // Only traverse affected components
    }
}
```

**Why:** Full transitive closure is O(n³) — unfeasible for enterprise codebases. IncSCC reduces to O(n²) average, near-linear for typical updates.

---

### 5. Business Rule Format (Markdown + YAML)

**NEW: Standardized Format**
```markdown
---
rule_id: "AUTH-001"
title: "User Authentication Protocol"
category: "Security"
severity: "Critical"
applies_to:
  - "src/auth/login.rs"
  - "src/middleware/auth.rs"
related_rules:
  - "AUTH-002"
---

# User Authentication Protocol

## Rule Definition
All users must successfully authenticate via a cryptographically verified JWT token before accessing routes nested under the /api/secure/ namespace.

## Validation Criteria
- Token must not be expired
- Token signature must match the active RS256 public key
```

**Schema Validation (JSON Schema):**
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "rule_id": {
      "type": "string",
      "pattern": "^[A-Z]{3,4}-\\d{3}$"
    },
    "severity": {
      "type": "string",
      "enum": ["Critical", "High", "Medium", "Low"]
    },
    "applies_to": {
      "type": "array",
      "items": { "type": "string" }
    }
  },
  "required": ["rule_id", "title", "severity", "applies_to"]
}
```

**Why:** Markdown for human readability, YAML for machine parsing, JSON Schema for validation.

---

### 6. Bidirectional Traceability

**NEW: @req Annotations in Code**
```rust
/// Authenticates the user credentials against the secure database.
/// @req AUTH-001
/// @implements docs/business_rules/AUTH-001.md
pub fn authenticate(user: &str, pass: &str) -> Result<Token> {
    // Cryptographic validation logic...
}
```

**ESLint Plugin (TypeScript):**
```javascript
// .eslintrc.js
module.exports = {
  plugins: ['traceability'],
  rules: {
    'traceability/require-req-annotation': ['error', {
      exportPriority: ['public', 'api'],
      pattern: '^@req\\s+([A-Z]{3,4}-\\d{3})$'
    }]
  }
};
```

**Why:** Bidirectional links prevent configuration drift (rule updated but code ignored, or vice-versa).

---

### 7. Context Pruning (95-99% Token Reduction)

**Current (from R-13):**
```rust
impl ContextManager {
    pub fn allocate(&mut self, task: &Task, agent: &Agent) -> TaskContext {
        let domains = self.extract_domains(task);
        let mut context = Vec::new();
        for domain in domains {
            let rag_results = self.domain_rag.retrieve(&domain, task.query).await?;
            context.extend(rag_results);
        }
        TaskContext::new(context)
    }
}
```

**Updated (with Influence Graph):**
```rust
impl ContextManager {
    pub fn allocate(&mut self, task: &Task, agent: &Agent) -> TaskContext {
        // 1. Extract mentioned files (lexical matching)
        let mentioned_files = self.extract_mentioned_files(task);
        
        // 2. Get influence vectors (pre-calculated)
        let mut influenced_files = Vec::new();
        for file in mentioned_files {
            let vector = self.influence_graph.get_influence_vector(file);
            influenced_files.extend(vector.get_affected_files());
        }
        
        // 3. Extract business rules (bidirectional links)
        let business_rules = self.get_applicable_business_rules(&influenced_files);
        
        // 4. Prune context (only mathematically proven dependencies)
        TaskContext::new(influenced_files, business_rules)
        
        // Result: 500-2,500 tokens (not 50,000+)
    }
}
```

**Token Reduction Comparison:**

| Approach | Token Count | Savings |
|----------|-------------|---------|
| **Brute-Force (full dirs)** | 50,000+ | Baseline |
| **RAG (semantic search)** | 10,000-20,000 | 60-80% |
| **Influence Graph (deterministic)** | 500-2,500 | **95-99%** |

---

## 📋 Implementation Sprints (NEW)

Based on this research, we need to add:

### Sprint 16: SCIP Indexing (Agent B)
- Register language-specific parsers (rust-analyzer, ts-morph, pyan3)
- Generate SCIP indexes (Protobuf format)
- Build influence graph from SCIP

### Sprint 17: Influence Vector Calculation (Agent B)
- Calculate CBO, RFC metrics
- Compute transitive closure (IncSCC algorithm)
- Pre-calculate influence scores

### Sprint 18: Business Rule Documentation (Agent A)
- Define Markdown + YAML format
- Create JSON Schema for validation
- Document 10+ business rules (examples)

### Sprint 19: Bidirectional Traceability (Agent C)
- Parse @req annotations (AST extraction)
- Link code → rules in graph
- Implement ESLint plugin (TypeScript validation)

### Sprint 20: Context Pruning (Agent B)
- Integrate influence graph with ContextManager
- Implement deterministic graph traversal
- Benchmark token reduction (target: 95%+)

---

## ✅ Validation Metrics (From Research)

| Metric | Target | Measurement |
|--------|--------|-------------|
| Token Reduction | 95-99% | Before/after comparison |
| Indexing Speed | >1,000 LOC/sec | SCIP generation time |
| Graph Update Latency | <1ms (incremental) | Edge add/remove time |
| Transitive Closure | O(n²) worst-case | Algorithm complexity |
| Business Rule Coverage | 100% critical rules | Traceability audit |
| Context Quality | Zero noise, zero hallucination | LLM output accuracy |

---

## 🔗 Updated References

- [`research/prompts/13-influence-graph-business-rules.md`](../research/prompts/13-influence-graph-business-rules.md) — R-13 research (VALIDATED)
- [`PHASE-2-INTEGRATION-REACT-PATTERNS.md`](./PHASE-2-INTEGRATION-REACT-PATTERNS.md) — ReAct patterns
- [`PHASE-2-PIVOT-GRAPH-WORKFLOW.md`](./PHASE-2-PIVOT-GRAPH-WORKFLOW.md) — Graph pivot (VALIDATED)

---

**This research CONFIRMS our R-13 direction and provides IMPLEMENTATION DETAILS.**

**No major pivots needed — just refinements to implementation.**
