# R-13: Code Influence Graph & Business Rule Mapping (LLM-Independent)

**Priority:** 🔴 CRITICAL (Potential 95%+ Token Savings)  
**Status:** 📋 Research Prompt Ready  
**Estimated Research Time:** 4-6 hours  

---

## 💡 Core Insight

**Problem:** Currently we rely on LLMs to understand:
- ❌ What code influences what (dependencies)
- ❌ What business rules apply to what code
- ❌ What context is needed for a task

**Token Cost:** Extremely high (LLM must analyze entire codebase)

**Proposed Solution:** **Static analysis + explicit documentation** (NO LLM needed)

**Key Insight:**
> "Instead of asking LLM to abstract what code does and infer business rules, we:
> 1. Statically analyze code dependencies (AST, imports, call graphs)
> 2. Explicitly document business rules
> 3. Link code to business rules explicitly
> 4. Use graph traversal to find what influences what"

**Token Savings:** **95%+** (no LLM for dependency analysis or rule mapping)

---

## 🎯 Research Objectives

### 1. Code Influence Graph

**Question:** How to build a dependency graph WITHOUT LLM?

**Approaches to Research:**
- **AST-based analysis** (parse code, extract imports, function calls)
- **Static analysis tools** (existing tools for each language)
- **Call graph construction** (who calls whom)
- **Data flow analysis** (what data flows where)

**Expected Output:**
```rust
pub struct CodeInfluenceGraph {
    nodes: HashMap<FileId, FileNode>,
    edges: Vec<DependencyEdge>,
}

pub struct FileNode {
    pub file_path: PathBuf,
    pub imports: Vec<Import>,
    pub exports: Vec<Export>,
    pub function_calls: Vec<FunctionCall>,
    pub business_rules: Vec<BusinessRuleId>, // explicit links
}

pub struct DependencyEdge {
    pub from: FileId,
    pub to: FileId,
    pub dep_type: DependencyType, // Import, Call, Data, BusinessRule
    pub strength: f32, // how strong is this influence?
}
```

**Usage:**
```rust
// Get all files influenced by changes to auth.rs
let influenced = graph.get_influenced_files("src/auth.rs");

// Returns:
// - src/auth/login.rs (direct import)
// - src/auth/jwt.rs (direct import)
// - src/api/routes.rs (calls auth functions)
// - src/business/auth_rules.md (linked business rule)
// ...and transitive dependencies
```

---

### 2. Business Rule Documentation

**Question:** How to document business rules so they're machine-readable?

**Approach:** **Explicit documentation with structured format**

**Example:**
```markdown
# Business Rule: AUTH-001 — User Authentication

## Rule
Users must authenticate with valid credentials before accessing protected resources.

## Applies To
- `src/auth/login.rs` (login logic)
- `src/auth/jwt.rs` (token generation)
- `src/middleware/auth.rs` (auth middleware)

## Related Rules
- AUTH-002 (Password Requirements)
- AUTH-003 (Session Management)

## Code Links
```rust
// src/auth/login.rs:42
pub fn authenticate(user: &str, pass: &str) -> Result<Token> {
    // This function implements AUTH-001
}
```

## Tests
- `tests/auth/login_test.rs` (validates AUTH-001)
```

**Key Properties:**
- ✅ Machine-parseable (structured format)
- ✅ Explicit code links (file paths, line numbers)
- ✅ Rule relationships (related rules)
- ✅ No abstraction needed (code speaks for itself)

---

### 3. Influence Vector Concept

**Question:** How to represent "influence" as a vector?

**Concept:** Each file has an **influence vector**:
```rust
pub struct InfluenceVector {
    pub file_id: FileId,
    pub direct_dependencies: Vec<FileId>,    // files this file imports
    pub reverse_dependencies: Vec<FileId>,   // files that import this file
    pub call_graph_depth: usize,              // how deep in call graph
    pub business_rule_count: usize,           // how many business rules
    pub transitive_closure: Vec<FileId>,     // all influenced files
}

impl InfluenceVector {
    /// Get files that would be affected by changes to this file
    pub fn get_affected_files(&self) -> &Vec<FileId> {
        &self.transitive_closure
    }
    
    /// Get business rules that apply to this file
    pub fn get_business_rules(&self) -> Vec<BusinessRule>;
}
```

**Visualization:**
```
File: src/auth/login.rs

Influence Vector:
├─ Direct Dependencies: [src/auth/jwt.rs, src/db/users.rs]
├─ Reverse Dependencies: [src/api/routes.rs, src/middleware/auth.rs]
├─ Call Graph Depth: 3 (login → validate → hash → bcrypt)
├─ Business Rules: [AUTH-001, AUTH-002, AUTH-004]
└─ Transitive Closure: [12 files total would be affected]
```

---

### 4. Business Rule ↔ Code Linking

**Question:** How to link business rules to code WITHOUT LLM abstraction?

**Approach:** **Explicit bidirectional links**

**From Business Rule → Code:**
```markdown
## Applies To
- `src/auth/login.rs:42-58` (authenticate function)
- `src/auth/jwt.rs:15-30` (token generation)
```

**From Code → Business Rule:**
```rust
/// Implements business rule: AUTH-001
/// See: docs/business_rules/AUTH-001.md
pub fn authenticate(user: &str, pass: &str) -> Result<Token> {
    // ...
}
```

**Automated Validation:**
```rust
// Validate all business rules have code links
for rule in business_rules {
    assert!(rule.has_code_links(), "Rule {} has no code links", rule.id);
    
    // Validate code links exist
    for link in rule.code_links {
        assert!(file_exists(&link.file), "File {} doesn't exist", link.file);
        assert!(has_function(&link.file, &link.function), "Function {} not found", link.function);
    }
}

// Validate all code has business rule links (for business logic files)
for file in business_logic_files {
    assert!(file.has_business_rule_comment(), "File {} has no business rule link", file.path);
}
```

---

## 🔬 Research Questions

### 1. Static Analysis Tools

**Questions:**
- What static analysis tools exist for each language?
- Can they extract import graphs?
- Can they extract call graphs?
- Can they extract data flow?

**Tools to Research:**

**Rust:**
- `rust-analyzer` (already extracts imports, functions)
- `cargo call-graph` (call graph generation)
- `depgraph` (dependency visualization)

**TypeScript:**
- `ts-morph` (AST manipulation)
- `madge` (dependency graphs)
- `ts-prune` (unused code detection)

**Python:**
- `ast` module (built-in AST parsing)
- `pyan` (call graph analysis)
- `vulture` (dead code detection)

**Go:**
- `go/ast` (built-in AST)
- `guru` (program analysis)
- `digraph` (dependency graphs)

---

### 2. Graph Construction

**Questions:**
- How to efficiently build influence graphs for large codebases?
- Incremental updates (when code changes)?
- How to handle transitive dependencies?
- How to weight edge strength?

**Approaches:**
- **Full rebuild** (simple, but slow for large codebases)
- **Incremental** (only update changed files + dependents)
- **Lazy evaluation** (compute on-demand)

---

### 3. Business Rule Documentation Format

**Questions:**
- What format is best? (Markdown, YAML, JSON?)
- How to make it machine-parseable?
- How to validate links are correct?
- How to keep docs in sync with code?

**Formats to Evaluate:**
- **Markdown with frontmatter** (human-readable + machine-parseable)
- **YAML** (structured, easy to parse)
- **JSON Schema** (strict validation)
- **Custom DSL** (domain-specific language for business rules)

---

### 4. Integration with OPENAKTA

**Questions:**
- How does this integrate with Living Docs (Sprint 6)?
- How does Context Distribution (Sprint 8) use influence vectors?
- How does Task Decomposition (Sprint 7) use business rules?
- How much token savings vs LLM-based approach?

**Integration Points:**
```rust
// Context Manager uses influence graph
impl ContextManager {
    pub fn allocate(&mut self, task: &Task, agent: &Agent) -> TaskContext {
        // Get files mentioned in task
        let mentioned_files = self.extract_mentioned_files(task);
        
        // Get all influenced files (from influence graph)
        let mut influenced_files = Vec::new();
        for file in mentioned_files {
            influenced_files.extend(self.influence_graph.get_influenced_files(file));
        }
        
        // Get business rules that apply
        let business_rules = self.get_applicable_business_rules(&influenced_files);
        
        // Allocate minimal context (only influenced files + relevant rules)
        TaskContext::new(influenced_files, business_rules)
    }
}

// Task Decomposer uses business rules
impl MissionDecomposer {
    pub fn decompose(&self, mission: &str) -> Result<DecomposedMission> {
        // Extract business rules from mission
        let rules = self.extract_business_rules(mission);
        
        // Get code linked to those rules
        let linked_code = self.get_linked_code(rules);
        
        // Decompose based on business rule boundaries
        self.decompose_by_rules(rules, linked_code)
    }
}
```

---

## 📊 Expected Token Savings

### Current Approach (LLM-Based)

```
User: "Implement login feature"

LLM must analyze:
- Entire codebase to find auth-related files
- Infer business rules from code
- Infer dependencies from imports
- Context: ~50,000 tokens (full codebase scan)

Token cost: HIGH (LLM processes everything)
```

### Proposed Approach (Static Analysis)

```
User: "Implement login feature"

Static analysis:
- Parse imports → find auth files (no LLM)
- Read business rule docs → get linked files (no LLM)
- Traverse influence graph → get dependencies (no LLM)
- Context: ~500 tokens (only influenced files + rules)

Token cost: MINIMAL (LLM only processes relevant context)

Savings: 50,000 → 500 tokens = **99% reduction**
```

---

## 🏭 Industry Precedents

### Existing Tools with Similar Concepts

| Tool | Approach | Relevance |
|------|----------|-----------|
| **Sourcegraph** | Static analysis + code graph | ⭐⭐⭐⭐⭐ |
| **Semantic** | Code intelligence platform | ⭐⭐⭐⭐ |
| **CodeQL** | Query-based code analysis | ⭐⭐⭐⭐ |
| **Understand** | Code metrics + dependency graphs | ⭐⭐⭐ |
| **CodeScene** | Code + organizational insights | ⭐⭐⭐ |

**Key Insight:** These tools do **static analysis WITHOUT LLM** and provide:
- Dependency graphs
- Impact analysis
- Code hotspots
- Business context (some)

**OPENAKTA Differentiator:** Combine static analysis with **explicit business rule documentation** (not just code analysis).

---

## 📋 Research Plan

### Phase 1: Static Analysis Tools (2 hours)
- [ ] Research tools for Rust, TypeScript, Python, Go
- [ ] Evaluate AST parsing capabilities
- [ ] Evaluate call graph extraction
- [ ] Evaluate incremental analysis

### Phase 2: Business Rule Formats (1 hour)
- [ ] Research documentation formats
- [ ] Evaluate Markdown vs YAML vs JSON
- [ ] Design validation rules
- [ ] Design sync mechanisms (code ↔ docs)

### Phase 3: Influence Graph Design (1 hour)
- [ ] Design graph data structure
- [ ] Design influence vector
- [ ] Design transitive closure algorithm
- [ ] Design incremental update strategy

### Phase 4: Integration Plan (1 hour)
- [ ] How to integrate with Living Docs
- [ ] How to integrate with Context Distribution
- [ ] How to integrate with Task Decomposition
- [ ] Token savings estimation

---

## 📊 Deliverables

### 1. Static Analysis Report

**File:** `research/findings/influence-graph/STATIC-ANALYSIS-TOOLS.md`

**Content:**
- Tools evaluated per language
- Capabilities comparison
- Recommendations for OPENAKTA

---

### 2. Business Rule Format Specification

**File:** `research/findings/influence-graph/BUSINESS-RULE-FORMAT.md`

**Content:**
- Recommended format (Markdown + frontmatter?)
- Schema definition
- Validation rules
- Examples

---

### 3. Influence Graph Design

**File:** `research/findings/influence-graph/INFLUENCE-GRAPH-DESIGN.md`

**Content:**
- Data structure design
- Influence vector definition
- Transitive closure algorithm
- Incremental update strategy

---

### 4. Integration Plan

**File:** `research/findings/influence-graph/INTEGRATION-PLAN.md`

**Content:**
- Integration with existing OPENAKTA components
- API design
- Token savings estimation
- Implementation roadmap

---

## ✅ Success Criteria

Research is successful when:
- [ ] Static analysis tools identified for all target languages
- [ ] Business rule format specified and validated
- [ ] Influence graph design complete
- [ ] Integration plan with OPENAKTA defined
- [ ] Token savings estimated (target: 95%+ reduction)
- [ ] Implementation roadmap created

---

## 🚨 Why This Is CRITICAL

**This research could fundamentally change OPENAKTA's architecture:**

**Current:**
```
User Request → LLM analyzes everything → High token cost
```

**After:**
```
User Request → Static analysis (no LLM) → Influence graph → Minimal context → LLM
                                              ↓
                                         95%+ token savings
```

**This is not just optimization — this is a paradigm shift.**

---

**Ready to execute. This research could be the key to making OPENAKTA truly scalable and cost-effective.**
