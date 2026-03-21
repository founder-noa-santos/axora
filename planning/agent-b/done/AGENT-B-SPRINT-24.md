# Agent B — Sprint 24: Repository Map (AST + Graph Ranking)

**Phase:** 2  
**Sprint:** 24 (Implementation)  
**File:** `crates/openakta-indexing/src/repository_map.rs`  
**Priority:** CRITICAL (achieves 90% token reduction)  
**Estimated Tokens:** ~120K output  

---

## 🎯 Task

Implement **Repository Map with AST + Graph Ranking** (from Aider pattern) for 90%+ token reduction.

### Context

Competitive analysis provides CRITICAL implementation details:
- **AST-based parsing** (tree-sitter) — Extract symbols, not raw text
- **Graph Ranking** (PageRank algorithm) — Identify most referenced symbols
- **Aider Pattern** — Production-validated (90%+ token reduction)

**Your job:** Implement repository map (achieves 90%+ token reduction target).

---

## 📋 Deliverables

### 1. Create repository_map.rs

**File:** `crates/openakta-indexing/src/repository_map.rs`

**Core Structure:**
```rust
//! Repository Map with AST + Graph Ranking
//!
//! This module implements production-grade token optimization:
//! - AST-based parsing (tree-sitter)
//! - Graph ranking (PageRank algorithm)
//! - Aider pattern (90%+ token reduction)

use tree_sitter::{Parser, Tree, Node};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::page_rank;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Repository mapper (AST + graph ranking)
pub struct RepositoryMapper {
    parser: Parser,
    graph: DiGraph<Symbol, f32>,
    symbol_to_index: HashMap<String, NodeIndex>,
}

/// Symbol entity
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: PathBuf,
    pub line_range: (usize, usize),
    pub signature: String,
    pub references: usize,
}

/// Symbol kind
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function,
    Class,
    Interface,
    Method,
    Variable,
    Type,
}

/// Repository map (compressed representation)
pub struct RepositoryMap {
    pub symbols: Vec<Symbol>,
    pub token_count: usize,
}

impl RepositoryMapper {
    /// Create new mapper
    pub fn new() -> Self {
        Self {
            parser: Parser::new(),
            graph: DiGraph::new(),
            symbol_to_index: HashMap::new(),
        }
    }
    
    /// Build repository map (AST + PageRank)
    pub fn build_map(&mut self, codebase_path: &Path) -> Result<RepositoryMap> {
        // Parse all files with tree-sitter
        for entry in walkdir::WalkDir::new(codebase_path) {
            let entry = entry?;
            if is_code_file(entry.path()) {
                self.parse_file(entry.path())?;
            }
        }
        
        // Calculate PageRank (identify most referenced symbols)
        let ranks = page_rank(&self.graph, 0.85);
        
        // Build compressed map (top N symbols by rank)
        let mut symbols: Vec<_> = self.graph.node_indices()
            .map(|idx| (idx, ranks[idx.index()]))
            .collect();
        
        symbols.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        let top_symbols = symbols.into_iter()
            .take(100) // Top 100 symbols (fits in ~1000 tokens)
            .map(|(idx, _)| self.graph[idx].clone())
            .collect();
        
        Ok(RepositoryMap {
            symbols: top_symbols,
            token_count: self.estimate_tokens(&top_symbols),
        })
    }
    
    /// Parse single file (extract symbols)
    fn parse_file(&mut self, file_path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(file_path)?;
        let tree = self.parser.parse(&content, None)
            .ok_or_else(|| Error::ParseFailed)?;
        
        // Extract symbols from AST
        self.extract_symbols(tree.root_node(), file_path, &content)?;
        
        Ok(())
    }
    
    /// Extract symbols from AST
    fn extract_symbols(
        &mut self,
        node: Node,
        file_path: &Path,
        content: &str,
    ) -> Result<()> {
        // Check if node is a symbol (function, class, etc.)
        if let Some(symbol) = self.node_to_symbol(node, file_path, content) {
            // Add to graph
            let idx = self.graph.add_node(symbol.clone());
            self.symbol_to_index.insert(symbol.name.clone(), idx);
            
            // Add edges (references to other symbols)
            self.extract_references(node, &symbol, content)?;
        }
        
        // Recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_symbols(child, file_path, content)?;
        }
        
        Ok(())
    }
    
    /// Estimate token count for map
    fn estimate_tokens(&self, symbols: &[Symbol]) -> usize {
        // ~10 tokens per symbol (name + signature + file)
        symbols.len() * 10
    }
}
```

---

### 2. Integrate with ContextManager

**File:** `crates/openakta-cache/src/context.rs` (UPDATE)

```rust
// Add to existing ContextManager
impl ContextManager {
    /// Allocate context with repository map (90%+ reduction)
    pub fn allocate_with_repo_map(
        &mut self,
        task: &Task,
        agent: &Agent,
        repo_map: &RepositoryMap,
    ) -> Result<TaskContext> {
        // Extract mentioned symbols from task
        let mentioned_symbols = self.extract_mentioned_symbols(task)?;
        
        // Get top-ranked symbols from repo map
        let relevant_symbols = repo_map.symbols.iter()
            .filter(|s| mentioned_symbols.contains(&s.name) || s.references > 10)
            .take(50) // Top 50 relevant symbols
            .cloned()
            .collect();
        
        // Build context from symbols (not raw files)
        let context = self.build_context_from_symbols(&relevant_symbols)?;
        
        // Log token reduction
        let original_tokens = self.estimate_brute_force_tokens(task)?;
        let pruned_tokens = context.estimate_tokens();
        let savings = ((original_tokens - pruned_tokens) as f32 / original_tokens as f32) * 100.0;
        
        tracing::info!(
            "Repository map context: {} → {} tokens ({:.1}% savings)",
            original_tokens,
            pruned_tokens,
            savings
        );
        
        Ok(context)
    }
}
```

---

### 3. Add tree-sitter Dependencies

**File:** `crates/openakta-indexing/Cargo.toml` (UPDATE)

```toml
[dependencies]
tree-sitter = "0.22"
tree-sitter-rust = "0.22"
tree-sitter-typescript = "0.22"
tree-sitter-python = "0.22"
petgraph = "0.6"
walkdir = "2.4"
```

---

## 📁 File Boundaries

**Create:**
- `crates/openakta-indexing/src/repository_map.rs` (NEW)

**Update:**
- `crates/openakta-indexing/src/lib.rs` (add module export)
- `crates/openakta-indexing/Cargo.toml` (add tree-sitter deps)
- `crates/openakta-cache/src/context.rs` (integrate repo map)

**DO NOT Edit:**
- `crates/openakta-agents/` (Agent C's domain)
- `crates/openakta-docs/` (Agent A's domain)

---

## 🧪 Tests Required

```rust
#[test]
fn test_ast_parsing() { }

#[test]
fn test_symbol_extraction() { }

#[test]
fn test_reference_extraction() { }

#[test]
fn test_pagerank_calculation() { }

#[test]
fn test_top_symbols_selection() { }

#[test]
fn test_token_count_estimation() { }

#[test]
fn test_90_percent_reduction() { }

#[test]
fn test_context_manager_integration() { }
```

---

## ✅ Success Criteria

- [ ] `repository_map.rs` created (AST + PageRank)
- [ ] tree-sitter parsing works
- [ ] Symbol extraction works
- [ ] Reference extraction works
- [ ] PageRank calculation works
- [ ] Top symbols selection works
- [ ] 90%+ token reduction achieved
- [ ] 8+ tests passing

---

## 🔗 References

- [`PHASE-2-INTEGRATION-COMPETITIVE-ANALYSIS.md`](../shared/PHASE-2-INTEGRATION-COMPETITIVE-ANALYSIS.md) — Competitive analysis
- Research document — Aider pattern spec

---

**⚠️ CRITICAL SPRINT: This achieves the 90% token reduction target.**

**Priority: CRITICAL — core differentiator for OPENAKTA.**

**Dependencies:**
- None (can start independently)

**Blocks:**
- Sprint 20 (Context Pruning) — enhances with repo map
