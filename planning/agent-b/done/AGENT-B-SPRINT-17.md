# Agent B — Sprint 17: Influence Vector Calculation

**Phase:** 2  
**Sprint:** 17 (Implementation)  
**File:** `crates/axora-indexing/src/influence.rs`  
**Priority:** HIGH (depends on Sprint 16)  
**Estimated Tokens:** ~100K output  

---

## 🎯 Task

Implement **Influence Vector Calculation** with CBO, RFC metrics and incremental transitive closure.

### Context

Research provides CRITICAL implementation details:
- **Influence Vector** — Multi-dimensional representation of code impact
- **CBO (Coupling Between Objects)** — Established software engineering metric
- **RFC (Response for Class)** — Number of methods that can be executed
- **IncSCC Algorithm** — Incremental transitive closure (O(n²) not O(n³))

**Your job:** Implement influence vector calculation (depends on Sprint 16 SCIP indexing).

---

## 📋 Deliverables

### 1. Create influence.rs (Influence Vector)

**File:** `crates/axora-indexing/src/influence.rs`

**Core Structure:**
```rust
//! Influence Vector Calculation
//!
//! This module implements influence vectors with software engineering metrics:
//! - CBO (Coupling Between Objects)
//! - RFC (Response for Class)
//! - Incremental transitive closure (IncSCC algorithm)

use std::collections::{HashMap, HashSet};
use crate::scip::{SCIPIndex, Symbol, Occurrence};

/// Influence Vector (multi-dimensional impact representation)
#[derive(Debug, Clone)]
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
    
    /// Get all files affected by changes to this file
    pub fn get_affected_files(&self) -> &Vec<FileId> {
        &self.transitive_closure
    }
}

/// Influence Graph (manages influence vectors)
pub struct InfluenceGraph {
    // Pre-calculated influence vectors
    vectors: HashMap<FileId, InfluenceVector>,
    
    // Dependency graph (for transitive closure)
    dependencies: HashMap<FileId, HashSet<FileId>>,
    reverse_dependencies: HashMap<FileId, HashSet<FileId>>,
}

impl InfluenceGraph {
    /// Build influence graph from SCIP index
    pub fn from_scip(scip_index: &SCIPIndex) -> Result<Self> {
        let mut graph = Self {
            vectors: HashMap::new(),
            dependencies: HashMap::new(),
            reverse_dependencies: HashMap::new(),
        };
        
        // Extract dependencies from SCIP occurrences
        graph.extract_dependencies(scip_index)?;
        
        // Calculate influence vectors
        graph.calculate_all_vectors()?;
        
        Ok(graph)
    }
    
    /// Get influence vector for file
    pub fn get_vector(&self, file_id: FileId) -> Option<&InfluenceVector> {
        self.vectors.get(&file_id)
    }
}
```

---

### 2. Implement CBO and RFC Metrics

**File:** `crates/axora-indexing/src/influence.rs` (add to existing)

```rust
impl InfluenceGraph {
    /// Calculate all influence vectors
    fn calculate_all_vectors(&mut self) -> Result<()> {
        for file_id in self.dependencies.keys().copied().collect::<Vec<_>>() {
            let vector = self.calculate_vector(file_id)?;
            self.vectors.insert(file_id, vector);
        }
        
        Ok(())
    }
    
    /// Calculate influence vector for single file
    fn calculate_vector(&self, file_id: FileId) -> Result<InfluenceVector> {
        // Afferent coupling (C_in, Fan-in): who depends on me?
        let afferent = self.reverse_dependencies.get(&file_id)
            .map(|deps| deps.len())
            .unwrap_or(0);
        
        // Efferent coupling (C_out, Fan-out): who do I depend on?
        let efferent = self.dependencies.get(&file_id)
            .map(|deps| deps.len())
            .unwrap_or(0);
        
        // CBO (Coupling Between Objects): afferent + efferent
        let cbo = afferent + efferent;
        
        // Call graph depth (max depth of execution paths)
        let call_graph_depth = self.calculate_call_graph_depth(file_id)?;
        
        // RFC (Response for Class): number of callable methods
        let rfc = self.calculate_rfc(file_id)?;
        
        // Business rule count (from bidirectional links)
        let business_rule_count = 0; // Will be populated by Agent C's traceability work
        
        // Transitive closure (all affected files)
        let transitive_closure = self.calculate_transitive_closure(file_id)?;
        
        Ok(InfluenceVector {
            file_id,
            afferent_coupling: afferent,
            efferent_coupling: efferent,
            coupling_between_objects: cbo,
            call_graph_depth,
            response_for_class: rfc,
            business_rule_count,
            transitive_closure,
        })
    }
    
    /// Calculate call graph depth (BFS from file)
    fn calculate_call_graph_depth(&self, file_id: FileId) -> Result<usize> {
        let mut max_depth = 0;
        let mut visited = HashSet::new();
        let mut queue = Vec::new();
        
        queue.push((file_id, 0));
        
        while let Some((current, depth)) = queue.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);
            
            max_depth = max_depth.max(depth);
            
            // Add dependencies to queue
            if let Some(deps) = self.dependencies.get(&current) {
                for dep in deps {
                    queue.push((*dep, depth + 1));
                }
            }
        }
        
        Ok(max_depth)
    }
    
    /// Calculate RFC (Response for Class)
    fn calculate_rfc(&self, file_id: FileId) -> Result<usize> {
        // RFC = number of methods that can be executed in response to a message
        // For simplicity, count functions + methods in file + called functions
        
        let mut rfc = 0;
        
        // Count functions in file
        rfc += self.count_functions_in_file(file_id)?;
        
        // Count called functions (from dependencies)
        if let Some(deps) = self.dependencies.get(&file_id) {
            for dep in deps {
                rfc += self.count_functions_in_file(*dep)?;
            }
        }
        
        Ok(rfc)
    }
}
```

---

### 3. Implement Incremental Transitive Closure (IncSCC)

**File:** `crates/axora-indexing/src/influence.rs` (add to existing)

```rust
impl InfluenceGraph {
    /// Calculate transitive closure (all files affected by change)
    fn calculate_transitive_closure(&self, file_id: FileId) -> Result<Vec<FileId>> {
        // Use incremental algorithm (not O(n³) Floyd-Warshall)
        // IncSCC: Incremental Strongly Connected Components
        
        let mut affected = HashSet::new();
        let mut queue = Vec::new();
        
        // Start with direct dependencies
        if let Some(deps) = self.dependencies.get(&file_id) {
            for dep in deps {
                queue.push(*dep);
                affected.insert(*dep);
            }
        }
        
        // BFS to find all transitive dependencies
        while let Some(current) = queue.pop() {
            if let Some(deps) = self.dependencies.get(&current) {
                for dep in deps {
                    if !affected.contains(dep) {
                        affected.insert(*dep);
                        queue.push(*dep);
                    }
                }
            }
        }
        
        Ok(affected.into_iter().collect())
    }
    
    /// Update graph when edge is added (incremental, not full recalc)
    pub fn add_edge(&mut self, from: FileId, to: FileId) -> Result<()> {
        // Add edge
        self.dependencies.entry(from).or_insert_with(HashSet::new).insert(to);
        self.reverse_dependencies.entry(to).or_insert_with(HashSet::new).insert(from);
        
        // Update affected influence vectors (only affected files)
        self.update_affected_vectors(from)?;
        self.update_affected_vectors(to)?;
        
        Ok(())
    }
    
    /// Update influence vectors for affected files (incremental)
    fn update_affected_vectors(&mut self, file_id: FileId) -> Result<()> {
        // Find all files that depend on this file (reverse transitive closure)
        let mut affected = HashSet::new();
        let mut queue = Vec::new();
        
        queue.push(file_id);
        affected.insert(file_id);
        
        while let Some(current) = queue.pop() {
            if let Some(reverse_deps) = self.reverse_dependencies.get(&current) {
                for dep in reverse_deps {
                    if !affected.contains(dep) {
                        affected.insert(*dep);
                        queue.push(*dep);
                    }
                }
            }
        }
        
        // Recalculate vectors for affected files only (not all files)
        for affected_id in affected {
            let vector = self.calculate_vector(affected_id)?;
            self.vectors.insert(affected_id, vector);
        }
        
        Ok(())
    }
}
```

---

### 4. Add Graph Database Integration

**File:** `crates/axora-indexing/src/influence.rs` (add to existing)

```rust
/// Influence Graph with database persistence
pub struct PersistentInfluenceGraph {
    graph: InfluenceGraph,
    db: Arc<Database>, // SQLite or Qdrant
}

impl PersistentInfluenceGraph {
    /// Save influence graph to database
    pub fn save(&self) -> Result<()> {
        // Save influence vectors
        for (file_id, vector) in &self.graph.vectors {
            self.db.save_influence_vector(file_id, vector)?;
        }
        
        Ok(())
    }
    
    /// Load influence graph from database
    pub fn load(db: Arc<Database>) -> Result<Self> {
        let vectors = db.load_all_influence_vectors()?;
        
        let mut graph = InfluenceGraph {
            vectors: HashMap::new(),
            dependencies: HashMap::new(),
            reverse_dependencies: HashMap::new(),
        };
        
        // Reconstruct graph from vectors
        for (file_id, vector) in vectors {
            graph.vectors.insert(file_id, vector);
            // Rebuild dependencies from transitive closure
            // ...
        }
        
        Ok(Self { graph, db })
    }
}
```

---

## 📁 File Boundaries

**Create:**
- `crates/axora-indexing/src/influence.rs` (NEW)

**Update:**
- `crates/axora-indexing/src/lib.rs` (add module export)

**DO NOT Edit:**
- `crates/axora-agents/` (Agent C's domain)
- `crates/axora-cache/` (Agent B's other work)
- `crates/axora-docs/` (Agent A's domain)

**Dependencies:**
- Sprint 16 (SCIP Indexing) — must complete first

---

## 🧪 Tests Required

```rust
#[test]
fn test_afferent_coupling() { }

#[test]
fn test_efferent_coupling() { }

#[test]
fn test_cbo_calculation() { }

#[test]
fn test_call_graph_depth() { }

#[test]
fn test_rfc_calculation() { }

#[test]
fn test_transitive_closure() { }

#[test]
fn test_incremental_update() { }

#[test]
fn test_influence_score() { }
```

---

## ✅ Success Criteria

- [ ] `influence.rs` created (Influence Vector implementation)
- [ ] CBO metric calculated correctly
- [ ] RFC metric calculated correctly
- [ ] Transitive closure works (BFS algorithm)
- [ ] Incremental updates work (not full recalc)
- [ ] Influence score calculation works
- [ ] 8+ tests passing
- [ ] Performance: <1ms for incremental updates

---

## 🔗 References

- [`AGENT-B-SPRINT-16.md`](./AGENT-B-SPRINT-16.md) — SCIP Indexing (dependency)
- [`PHASE-2-INTEGRATION-INFLUENCE-GRAPH.md`](../shared/PHASE-2-INTEGRATION-INFLUENCE-GRAPH.md) — Integration doc
- Research document — IncSCC algorithm spec

---

**Start AFTER Sprint 16 (SCIP Indexing) is complete.**

**Priority: HIGH — needed for Context Pruning (Sprint 20).**

**Dependencies:**
- Sprint 16 (SCIP Indexing) — must complete first

**Blocks:**
- Sprint 20 (Context Pruning)
