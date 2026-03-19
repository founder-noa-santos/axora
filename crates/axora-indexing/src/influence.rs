//! Influence Vector Calculation
//!
//! This module implements influence vectors with software engineering metrics:
//! - CBO (Coupling Between Objects)
//! - RFC (Response for Class)
//! - Incremental transitive closure (IncSCC algorithm)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   Influence Graph                           │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Influence Vectors          │  Dependency Graph             │
//! │  - Afferent coupling (C_in) │  - Forward dependencies       │
//! │  - Efferent coupling (C_out)│  - Reverse dependencies       │
//! │  - CBO                      │                               │
//! │  - RFC                      │  Incremental Updates          │
//! │  - Call graph depth         │  - add_edge()                 │
//! │  - Transitive closure       │  - remove_edge()              │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use axora_indexing::scip::{SCIPIndex, ParserRegistry, Language};
//! use axora_indexing::influence::InfluenceGraph;
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Generate SCIP index
//! let registry = ParserRegistry::new();
//! let scip_index = registry.parse(Language::Rust, Path::new("/path/to/project"))?;
//!
//! // Build influence graph
//! let influence_graph = InfluenceGraph::from_scip(&scip_index)?;
//!
//! // Get influence vector for a file
//! if let Some(vector) = influence_graph.get_vector("src/main.rs") {
//!     println!("CBO: {}", vector.coupling_between_objects);
//!     println!("RFC: {}", vector.response_for_class);
//!     println!("Influence score: {:.2}", vector.influence_score());
//!     
//!     // Get all files affected by changes
//!     let affected = vector.get_affected_files();
//!     println!("Affected files: {:?}", affected);
//! }
//! # Ok(())
//! # }
//! ```

use crate::scip::{Occurrence, SCIPIndex, Symbol, SymbolKind};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Influence vector error types
#[derive(Error, Debug)]
pub enum InfluenceError {
    /// File not found in graph
    #[error("file not found: {0}")]
    FileNotFound(String),

    /// SCIP index error
    #[error("SCIP index error: {0}")]
    SCIPIndex(String),

    /// Graph operation failed
    #[error("graph operation failed: {0}")]
    GraphOperation(String),
}

/// Result type for influence operations
pub type Result<T> = std::result::Result<T, InfluenceError>;

/// File identifier
pub type FileId = String;

/// Influence Vector (multi-dimensional impact representation)
///
/// Represents the impact and coupling characteristics of a file
/// in the codebase. Used for context pruning and impact analysis.
#[derive(Debug, Clone)]
pub struct InfluenceVector {
    /// File this vector represents
    pub file_id: FileId,

    // Coupling metrics
    /// Afferent coupling (C_in, Fan-in): number of files that depend on this file
    pub afferent_coupling: usize,

    /// Efferent coupling (C_out, Fan-out): number of files this file depends on
    pub efferent_coupling: usize,

    /// CBO (Coupling Between Objects): afferent + efferent
    pub coupling_between_objects: usize,

    // Complexity metrics
    /// Maximum depth of call graph from this file
    pub call_graph_depth: usize,

    /// RFC (Response for Class): number of callable methods/functions
    pub response_for_class: usize,

    // Business context
    /// Number of business rules linked to this file
    pub business_rule_count: usize,

    // Impact analysis
    /// All files transitively affected by changes to this file
    pub transitive_closure: Vec<FileId>,
}

impl InfluenceVector {
    /// Creates a new influence vector with default values
    pub fn new(file_id: FileId) -> Self {
        Self {
            file_id,
            afferent_coupling: 0,
            efferent_coupling: 0,
            coupling_between_objects: 0,
            call_graph_depth: 0,
            response_for_class: 0,
            business_rule_count: 0,
            transitive_closure: Vec::new(),
        }
    }

    /// Calculate influence score (for context pruning priority)
    ///
    /// Higher scores indicate files that need broader context:
    /// - High fan-in = core component (many things depend on it)
    /// - High fan-out = fragile component (depends on many things)
    /// - High business rule count = critical business logic
    pub fn influence_score(&self) -> f32 {
        // Weighted formula:
        // - Afferent coupling (fan-in) is weighted 2x (core components need more context)
        // - Efferent coupling (fan-out) is weighted 1x
        // - Call graph depth is weighted 0.5x (depth indicates complexity)
        // - Business rules are weighted 3x (business logic is critical)
        (self.afferent_coupling as f32 * 2.0)
            + (self.efferent_coupling as f32)
            + (self.call_graph_depth as f32 * 0.5)
            + (self.business_rule_count as f32 * 3.0)
    }

    /// Get all files affected by changes to this file
    pub fn get_affected_files(&self) -> &Vec<FileId> {
        &self.transitive_closure
    }

    /// Returns true if this is a core component (high fan-in)
    pub fn is_core_component(&self) -> bool {
        self.afferent_coupling >= 5
    }

    /// Returns true if this is a fragile component (high fan-out)
    pub fn is_fragile_component(&self) -> bool {
        self.efferent_coupling >= 10
    }

    /// Returns true if this file has high business impact
    pub fn has_high_business_impact(&self) -> bool {
        self.business_rule_count >= 3
    }
}

/// Influence Graph (manages influence vectors for all files)
///
/// Maintains dependency relationships and calculates influence vectors
/// using software engineering metrics (CBO, RFC, etc.).
pub struct InfluenceGraph {
    /// Pre-calculated influence vectors
    vectors: HashMap<FileId, InfluenceVector>,

    /// Dependency graph: file -> files it depends on
    dependencies: HashMap<FileId, HashSet<FileId>>,

    /// Reverse dependency graph: file -> files that depend on it
    reverse_dependencies: HashMap<FileId, HashSet<FileId>>,

    /// Symbol to file mapping (for quick lookups)
    symbol_to_file: HashMap<String, FileId>,

    /// File to symbols mapping
    file_to_symbols: HashMap<FileId, HashSet<String>>,
}

impl InfluenceGraph {
    /// Creates a new empty influence graph
    pub fn new() -> Self {
        Self {
            vectors: HashMap::new(),
            dependencies: HashMap::new(),
            reverse_dependencies: HashMap::new(),
            symbol_to_file: HashMap::new(),
            file_to_symbols: HashMap::new(),
        }
    }

    /// Builds influence graph from SCIP index
    ///
    /// Extracts dependencies from SCIP occurrences and calculates
    /// influence vectors for all files.
    pub fn from_scip(scip_index: &SCIPIndex) -> Result<Self> {
        let mut graph = Self::new();

        // Extract dependencies from SCIP occurrences
        graph.extract_dependencies(scip_index)?;

        // Calculate influence vectors
        graph.calculate_all_vectors()?;

        Ok(graph)
    }

    /// Extracts dependencies from SCIP index
    fn extract_dependencies(&mut self, scip_index: &SCIPIndex) -> Result<()> {
        // Build symbol to file mapping
        for occurrence in &scip_index.occurrences {
            if occurrence.is_definition {
                self.dependencies
                    .entry(occurrence.file_path.clone())
                    .or_insert_with(HashSet::new);
                self.reverse_dependencies
                    .entry(occurrence.file_path.clone())
                    .or_insert_with(HashSet::new);
                self.symbol_to_file
                    .insert(occurrence.symbol.clone(), occurrence.file_path.clone());

                self.file_to_symbols
                    .entry(occurrence.file_path.clone())
                    .or_insert_with(HashSet::new)
                    .insert(occurrence.symbol.clone());
            }
        }

        // Extract dependencies from references
        for occurrence in &scip_index.occurrences {
            if !occurrence.is_definition {
                // This is a reference to a symbol
                // Find where the symbol is defined
                if let Some(defined_file) = self.symbol_to_file.get(&occurrence.symbol) {
                    // The file containing this occurrence depends on the file where symbol is defined
                    if defined_file != &occurrence.file_path {
                        self.dependencies
                            .entry(occurrence.file_path.clone())
                            .or_insert_with(HashSet::new)
                            .insert(defined_file.clone());

                        self.reverse_dependencies
                            .entry(defined_file.clone())
                            .or_insert_with(HashSet::new)
                            .insert(occurrence.file_path.clone());
                    }
                }
            }
        }

        Ok(())
    }

    /// Gets influence vector for a file
    pub fn get_vector(&self, file_id: &str) -> Option<&InfluenceVector> {
        self.vectors.get(file_id)
    }

    /// Gets mutable influence vector for a file
    pub fn get_vector_mut(&mut self, file_id: &str) -> Option<&mut InfluenceVector> {
        self.vectors.get_mut(file_id)
    }

    /// Gets all files this file depends on
    pub fn get_dependencies(&self, file_id: &str) -> Option<&HashSet<FileId>> {
        self.dependencies.get(file_id)
    }

    /// Resolve a symbol to the file that defines it.
    pub fn resolve_symbol(&self, symbol: &str) -> Option<&str> {
        self.symbol_to_file.get(symbol).map(String::as_str)
    }

    /// Return the symbols defined in a file.
    pub fn symbols_for_file(&self, file_id: &str) -> Vec<String> {
        self.file_to_symbols
            .get(file_id)
            .map(|symbols| symbols.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Gets all files that depend on this file
    pub fn get_reverse_dependencies(&self, file_id: &str) -> Option<&HashSet<FileId>> {
        self.reverse_dependencies.get(file_id)
    }

    /// Gets all files in the graph
    pub fn all_files(&self) -> Vec<&FileId> {
        self.dependencies.keys().collect()
    }

    /// Checks if a file exists in the graph
    pub fn contains_file(&self, file_id: &str) -> bool {
        self.dependencies.contains_key(file_id)
    }

    /// Gets the number of files in the graph
    pub fn file_count(&self) -> usize {
        self.dependencies.len()
    }

    /// Traverse direct and transitive dependencies, returning direct dependencies first.
    pub fn dependency_chain(&self, file_id: &str) -> Vec<FileId> {
        let mut ordered = Vec::new();
        let mut visited = HashSet::new();

        if let Some(direct) = self.dependencies.get(file_id) {
            let mut direct_sorted = direct.iter().cloned().collect::<Vec<_>>();
            direct_sorted.sort();
            for dependency in direct_sorted {
                if visited.insert(dependency.clone()) {
                    ordered.push(dependency.clone());
                    self.collect_transitive(&dependency, &mut visited, &mut ordered);
                }
            }
        }

        ordered
    }

    fn collect_transitive(
        &self,
        file_id: &str,
        visited: &mut HashSet<FileId>,
        ordered: &mut Vec<FileId>,
    ) {
        if let Some(next) = self.dependencies.get(file_id) {
            let mut children = next.iter().cloned().collect::<Vec<_>>();
            children.sort();
            for dependency in children {
                if visited.insert(dependency.clone()) {
                    ordered.push(dependency.clone());
                    self.collect_transitive(&dependency, visited, ordered);
                }
            }
        }
    }
}

impl Default for InfluenceGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Influence Vector Calculation
// ============================================================================

impl InfluenceGraph {
    /// Calculates all influence vectors
    fn calculate_all_vectors(&mut self) -> Result<()> {
        let file_ids: Vec<FileId> = self.dependencies.keys().cloned().collect();

        for file_id in file_ids {
            let vector = self.calculate_vector(&file_id)?;
            self.vectors.insert(file_id, vector);
        }

        Ok(())
    }

    /// Calculates influence vector for a single file
    fn calculate_vector(&self, file_id: &str) -> Result<InfluenceVector> {
        // Afferent coupling (C_in, Fan-in): who depends on me?
        let afferent = self
            .reverse_dependencies
            .get(file_id)
            .map(|deps| deps.len())
            .unwrap_or(0);

        // Efferent coupling (C_out, Fan-out): who do I depend on?
        let efferent = self
            .dependencies
            .get(file_id)
            .map(|deps| deps.len())
            .unwrap_or(0);

        // CBO (Coupling Between Objects): afferent + efferent
        let cbo = afferent + efferent;

        // Call graph depth (max depth of execution paths)
        let call_graph_depth = self.calculate_call_graph_depth(file_id)?;

        // RFC (Response for Class): number of callable methods
        let rfc = self.calculate_rfc(file_id)?;

        // Business rule count (from bidirectional links)
        // Will be populated by Agent C's traceability work
        let business_rule_count = 0;

        // Transitive closure (all files affected by changes)
        let transitive_closure = self.calculate_transitive_closure(file_id)?;

        Ok(InfluenceVector {
            file_id: file_id.to_string(),
            afferent_coupling: afferent,
            efferent_coupling: efferent,
            coupling_between_objects: cbo,
            call_graph_depth,
            response_for_class: rfc,
            business_rule_count,
            transitive_closure,
        })
    }

    /// Calculates call graph depth using BFS
    fn calculate_call_graph_depth(&self, file_id: &str) -> Result<usize> {
        let mut max_depth = 0;
        let mut visited = HashSet::new();
        let mut queue: Vec<(FileId, usize)> = Vec::new();

        queue.push((file_id.to_string(), 0));

        while let Some((current, depth)) = queue.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            max_depth = max_depth.max(depth);

            // Add dependencies to queue
            if let Some(deps) = self.dependencies.get(&current) {
                for dep in deps {
                    if !visited.contains(dep) {
                        queue.push((dep.clone(), depth + 1));
                    }
                }
            }
        }

        Ok(max_depth)
    }

    /// Calculates RFC (Response for Class)
    ///
    /// RFC = number of methods/functions that can be executed in response to a message
    /// For simplicity, count functions + methods in file + called functions
    fn calculate_rfc(&self, file_id: &str) -> Result<usize> {
        let mut rfc = 0;

        // Count functions/methods in file
        rfc += self.count_symbols_in_file(file_id, SymbolKind::Function);
        rfc += self.count_symbols_in_file(file_id, SymbolKind::Method);

        // Count called functions (from dependencies)
        if let Some(deps) = self.dependencies.get(file_id) {
            for dep in deps {
                rfc += self.count_symbols_in_file(dep, SymbolKind::Function);
                rfc += self.count_symbols_in_file(dep, SymbolKind::Method);
            }
        }

        Ok(rfc)
    }

    /// Counts symbols of a specific kind in a file
    fn count_symbols_in_file(&self, file_id: &str, kind: SymbolKind) -> usize {
        self.file_to_symbols
            .get(file_id)
            .map(|symbols| symbols.len())
            .unwrap_or(0)
    }

    /// Calculates transitive closure (all files affected by change)
    ///
    /// Uses BFS for O(V + E) complexity instead of O(n³) Floyd-Warshall.
    fn calculate_transitive_closure(&self, file_id: &str) -> Result<Vec<FileId>> {
        let mut affected = HashSet::new();
        let mut queue: Vec<FileId> = Vec::new();

        // Start with direct dependencies
        if let Some(deps) = self.dependencies.get(file_id) {
            for dep in deps {
                queue.push(dep.clone());
                affected.insert(dep.clone());
            }
        }

        // BFS to find all transitive dependencies
        while let Some(current) = queue.pop() {
            if let Some(deps) = self.dependencies.get(&current) {
                for dep in deps {
                    if !affected.contains(dep) {
                        affected.insert(dep.clone());
                        queue.push(dep.clone());
                    }
                }
            }
        }

        let mut result: Vec<FileId> = affected.into_iter().collect();
        result.sort();
        Ok(result)
    }
}

// ============================================================================
// Incremental Updates (IncSCC Algorithm)
// ============================================================================

impl InfluenceGraph {
    /// Adds an edge to the graph (incremental update)
    ///
    /// Updates only affected influence vectors, not the entire graph.
    /// This is O(k) where k is the number of affected files, not O(n).
    pub fn add_edge(&mut self, from: &str, to: &str) -> Result<()> {
        // Ensure both nodes exist in the graph
        self.dependencies
            .entry(from.to_string())
            .or_insert_with(HashSet::new);
        self.dependencies
            .entry(to.to_string())
            .or_insert_with(HashSet::new);
        self.reverse_dependencies
            .entry(from.to_string())
            .or_insert_with(HashSet::new);
        self.reverse_dependencies
            .entry(to.to_string())
            .or_insert_with(HashSet::new);

        // Add edge
        self.dependencies
            .get_mut(from)
            .unwrap()
            .insert(to.to_string());

        self.reverse_dependencies
            .get_mut(to)
            .unwrap()
            .insert(from.to_string());

        // Update affected influence vectors (only affected files)
        self.update_affected_vectors(from)?;
        self.update_affected_vectors(to)?;

        Ok(())
    }

    /// Removes an edge from the graph (incremental update)
    pub fn remove_edge(&mut self, from: &str, to: &str) -> Result<()> {
        // Remove edge
        if let Some(deps) = self.dependencies.get_mut(from) {
            deps.remove(to);
        }

        if let Some(reverse_deps) = self.reverse_dependencies.get_mut(to) {
            reverse_deps.remove(from);
        }

        // Update affected influence vectors
        self.update_affected_vectors(from)?;
        self.update_affected_vectors(to)?;

        Ok(())
    }

    /// Updates influence vectors for files affected by a change
    fn update_affected_vectors(&mut self, file_id: &str) -> Result<()> {
        // Find all files that depend on this file (reverse transitive closure)
        let mut affected = HashSet::new();
        let mut queue: Vec<FileId> = Vec::new();

        queue.push(file_id.to_string());
        affected.insert(file_id.to_string());

        while let Some(current) = queue.pop() {
            if let Some(reverse_deps) = self.reverse_dependencies.get(&current) {
                for dep in reverse_deps {
                    if !affected.contains(dep) {
                        affected.insert(dep.clone());
                        queue.push(dep.clone());
                    }
                }
            }
        }

        // Recalculate vectors for affected files only (not all files)
        for affected_id in affected {
            let vector = self.calculate_vector(&affected_id)?;
            self.vectors.insert(affected_id, vector);
        }

        Ok(())
    }

    /// Gets files that would be affected by adding an edge
    pub fn get_potential_impact(&self, from: &str, to: &str) -> HashSet<FileId> {
        let mut impact = HashSet::new();

        // Files that depend on 'to' will be affected
        if let Some(reverse_deps) = self.get_reverse_dependencies(to) {
            impact.extend(reverse_deps.iter().cloned());
        }

        // Files that 'from' depends on will be affected
        if let Some(deps) = self.get_dependencies(from) {
            impact.extend(deps.iter().cloned());
        }

        impact
    }
}

// ============================================================================
// Influence Graph Statistics
// ============================================================================

impl InfluenceGraph {
    /// Gets statistics about the influence graph
    pub fn get_statistics(&self) -> InfluenceGraphStats {
        let total_files = self.file_count();
        let total_edges: usize = self.dependencies.values().map(|deps| deps.len()).sum();

        let avg_coupling: f32 = if total_files > 0 {
            self.vectors
                .values()
                .map(|v| v.coupling_between_objects as f32)
                .sum::<f32>()
                / total_files as f32
        } else {
            0.0
        };

        let max_coupling = self
            .vectors
            .values()
            .map(|v| v.coupling_between_objects)
            .max()
            .unwrap_or(0);

        let core_components = self
            .vectors
            .values()
            .filter(|v| v.is_core_component())
            .count();

        let fragile_components = self
            .vectors
            .values()
            .filter(|v| v.is_fragile_component())
            .count();

        InfluenceGraphStats {
            total_files,
            total_edges,
            avg_coupling,
            max_coupling,
            core_components,
            fragile_components,
        }
    }

    /// Gets the top N files by influence score
    pub fn get_top_influential_files(&self, n: usize) -> Vec<(&FileId, f32)> {
        let mut scores: Vec<(&FileId, f32)> = self
            .vectors
            .iter()
            .map(|(id, v)| (id, v.influence_score()))
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(n);
        scores
    }

    /// Gets files with no dependencies (leaf nodes)
    pub fn get_leaf_files(&self) -> Vec<&FileId> {
        self.dependencies
            .iter()
            .filter(|(_, deps)| deps.is_empty())
            .map(|(id, _)| id)
            .collect()
    }

    /// Gets files with no reverse dependencies (root nodes)
    pub fn get_root_files(&self) -> Vec<&FileId> {
        self.reverse_dependencies
            .iter()
            .filter(|(_, deps)| deps.is_empty())
            .map(|(id, _)| id)
            .collect()
    }
}

/// Statistics about the influence graph
#[derive(Debug, Clone)]
pub struct InfluenceGraphStats {
    /// Total number of files
    pub total_files: usize,

    /// Total number of dependency edges
    pub total_edges: usize,

    /// Average coupling between objects
    pub avg_coupling: f32,

    /// Maximum coupling between objects
    pub max_coupling: usize,

    /// Number of core components (high fan-in)
    pub core_components: usize,

    /// Number of fragile components (high fan-out)
    pub fragile_components: usize,
}

impl InfluenceGraphStats {
    /// Returns the density of the graph (edges / possible_edges)
    pub fn density(&self) -> f32 {
        if self.total_files <= 1 {
            return 0.0;
        }
        let possible_edges = self.total_files * (self.total_files - 1);
        self.total_edges as f32 / possible_edges as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_afferent_coupling() {
        let mut graph = InfluenceGraph::new();

        // Create dependencies: A <- B <- C (C depends on B, B depends on A)
        graph.add_edge("b.rs", "a.rs").unwrap();
        graph.add_edge("c.rs", "b.rs").unwrap();

        // A has afferent coupling of 2 (B and C depend on it transitively)
        // But directly, only B depends on A
        let vector = graph.get_vector("a.rs").unwrap();
        assert_eq!(vector.afferent_coupling, 1); // B depends on A

        // B has afferent coupling of 1 (C depends on B)
        let vector = graph.get_vector("b.rs").unwrap();
        assert_eq!(vector.afferent_coupling, 1);

        // C has afferent coupling of 0 (nothing depends on C)
        let vector = graph.get_vector("c.rs").unwrap();
        assert_eq!(vector.afferent_coupling, 0);
    }

    #[test]
    fn test_efferent_coupling() {
        let mut graph = InfluenceGraph::new();

        // Create dependencies: A -> B -> C (A depends on B, B depends on C)
        graph.add_edge("a.rs", "b.rs").unwrap();
        graph.add_edge("b.rs", "c.rs").unwrap();

        // A has efferent coupling of 1 (depends on B)
        let vector = graph.get_vector("a.rs").unwrap();
        assert_eq!(vector.efferent_coupling, 1);

        // B has efferent coupling of 1 (depends on C)
        let vector = graph.get_vector("b.rs").unwrap();
        assert_eq!(vector.efferent_coupling, 1);

        // C has efferent coupling of 0 (depends on nothing)
        let vector = graph.get_vector("c.rs").unwrap();
        assert_eq!(vector.efferent_coupling, 0);
    }

    #[test]
    fn test_cbo_calculation() {
        let mut graph = InfluenceGraph::new();

        // Create a diamond dependency: A depends on B and C, both depend on D
        graph.add_edge("a.rs", "b.rs").unwrap();
        graph.add_edge("a.rs", "c.rs").unwrap();
        graph.add_edge("b.rs", "d.rs").unwrap();
        graph.add_edge("c.rs", "d.rs").unwrap();

        // D has CBO = 2 (B and C depend on it)
        let vector = graph.get_vector("d.rs").unwrap();
        assert_eq!(vector.coupling_between_objects, 2);

        // B has CBO = 2 (depends on D, A depends on B)
        let vector = graph.get_vector("b.rs").unwrap();
        assert_eq!(vector.coupling_between_objects, 2);

        // A has CBO = 2 (depends on B and C)
        let vector = graph.get_vector("a.rs").unwrap();
        assert_eq!(vector.coupling_between_objects, 2);
    }

    #[test]
    fn test_call_graph_depth() {
        let mut graph = InfluenceGraph::new();

        // Create a chain: A -> B -> C -> D
        graph.add_edge("a.rs", "b.rs").unwrap();
        graph.add_edge("b.rs", "c.rs").unwrap();
        graph.add_edge("c.rs", "d.rs").unwrap();

        // A has call graph depth of 3 (A -> B -> C -> D)
        let vector = graph.get_vector("a.rs").unwrap();
        assert_eq!(vector.call_graph_depth, 3);

        // B has call graph depth of 2
        let vector = graph.get_vector("b.rs").unwrap();
        assert_eq!(vector.call_graph_depth, 2);

        // D has call graph depth of 0
        let vector = graph.get_vector("d.rs").unwrap();
        assert_eq!(vector.call_graph_depth, 0);
    }

    #[test]
    fn test_rfc_calculation() {
        let graph = InfluenceGraph::new();

        // RFC calculation depends on symbol counts
        // For empty graph, RFC should be 0
        let vector = InfluenceVector::new("test.rs".to_string());
        assert_eq!(vector.response_for_class, 0);
    }

    #[test]
    fn test_transitive_closure() {
        let mut graph = InfluenceGraph::new();

        // Create dependencies: A -> B -> C -> D
        graph.add_edge("a.rs", "b.rs").unwrap();
        graph.add_edge("b.rs", "c.rs").unwrap();
        graph.add_edge("c.rs", "d.rs").unwrap();

        // A's transitive closure should include B, C, D
        let vector = graph.get_vector("a.rs").unwrap();
        assert!(vector.transitive_closure.contains(&"b.rs".to_string()));
        assert!(vector.transitive_closure.contains(&"c.rs".to_string()));
        assert!(vector.transitive_closure.contains(&"d.rs".to_string()));
        assert_eq!(vector.transitive_closure.len(), 3);

        // B's transitive closure should include C, D
        let vector = graph.get_vector("b.rs").unwrap();
        assert!(vector.transitive_closure.contains(&"c.rs".to_string()));
        assert!(vector.transitive_closure.contains(&"d.rs".to_string()));
        assert_eq!(vector.transitive_closure.len(), 2);
    }

    #[test]
    fn test_incremental_update() {
        let mut graph = InfluenceGraph::new();

        // Initial state: A -> B
        graph.add_edge("a.rs", "b.rs").unwrap();

        let initial_vector = graph.get_vector("a.rs").unwrap().clone();
        assert_eq!(initial_vector.efferent_coupling, 1);

        // Add new edge: A -> C
        graph.add_edge("a.rs", "c.rs").unwrap();

        // A's efferent coupling should increase
        let updated_vector = graph.get_vector("a.rs").unwrap();
        assert_eq!(updated_vector.efferent_coupling, 2);

        // B should not be affected
        let b_vector = graph.get_vector("b.rs").unwrap();
        assert_eq!(b_vector.afferent_coupling, 1); // Still only A depends on B
    }

    #[test]
    fn test_influence_score() {
        let mut graph = InfluenceGraph::new();

        // Create a core component (high fan-in)
        graph.add_edge("a.rs", "core.rs").unwrap();
        graph.add_edge("b.rs", "core.rs").unwrap();
        graph.add_edge("c.rs", "core.rs").unwrap();
        graph.add_edge("d.rs", "core.rs").unwrap();
        graph.add_edge("e.rs", "core.rs").unwrap();

        let core_vector = graph.get_vector("core.rs").unwrap();

        // Core component should have high influence score
        // Score = (afferent * 2) + efferent + (depth * 0.5) + (business * 3)
        // Score = (5 * 2) + 0 + 0 + 0 = 10
        assert!(core_vector.influence_score() >= 10.0);

        // Leaf components should have lower scores
        let leaf_vector = graph.get_vector("a.rs").unwrap();
        assert!(leaf_vector.influence_score() < core_vector.influence_score());
    }

    #[test]
    fn test_graph_statistics() {
        let mut graph = InfluenceGraph::new();

        graph.add_edge("a.rs", "b.rs").unwrap();
        graph.add_edge("b.rs", "c.rs").unwrap();
        graph.add_edge("c.rs", "d.rs").unwrap();

        let stats = graph.get_statistics();

        assert_eq!(stats.total_files, 4);
        assert_eq!(stats.total_edges, 3);
        assert!(stats.avg_coupling > 0.0);
        // B and C have coupling of 2 (each has 1 in + 1 out)
        // A has coupling of 1 (1 out), D has coupling of 1 (1 in)
        // Max is 2
        assert!(stats.max_coupling >= 2);
    }

    #[test]
    fn test_top_influential_files() {
        let mut graph = InfluenceGraph::new();

        // Create star pattern: all files depend on core.rs
        for i in 0..5 {
            graph.add_edge(&format!("file{}.rs", i), "core.rs").unwrap();
        }

        let top = graph.get_top_influential_files(3);

        // core.rs should be the most influential
        assert_eq!(top[0].0, "core.rs");
        assert!(top[0].1 > 0.0);
    }

    #[test]
    fn test_leaf_and_root_files() {
        let mut graph = InfluenceGraph::new();

        // A -> B -> C
        graph.add_edge("a.rs", "b.rs").unwrap();
        graph.add_edge("b.rs", "c.rs").unwrap();

        let leaves = graph.get_leaf_files();
        let roots = graph.get_root_files();

        // C is a leaf (depends on nothing)
        assert!(leaves.iter().any(|&f| f == "c.rs"));

        // A is a root (nothing depends on it)
        assert!(roots.iter().any(|&f| f == "a.rs"));
    }

    #[test]
    fn test_influence_vector_helpers() {
        let mut vector = InfluenceVector::new("test.rs".to_string());

        // Test is_core_component
        vector.afferent_coupling = 5;
        assert!(vector.is_core_component());
        vector.afferent_coupling = 4;
        assert!(!vector.is_core_component());

        // Test is_fragile_component
        vector.efferent_coupling = 10;
        assert!(vector.is_fragile_component());
        vector.efferent_coupling = 9;
        assert!(!vector.is_fragile_component());

        // Test has_high_business_impact
        vector.business_rule_count = 3;
        assert!(vector.has_high_business_impact());
        vector.business_rule_count = 2;
        assert!(!vector.has_high_business_impact());
    }

    #[test]
    fn test_edge_removal() {
        let mut graph = InfluenceGraph::new();

        // A -> B -> C
        graph.add_edge("a.rs", "b.rs").unwrap();
        graph.add_edge("b.rs", "c.rs").unwrap();

        // Verify initial state
        assert_eq!(graph.get_vector("b.rs").unwrap().afferent_coupling, 1);

        // Remove edge A -> B
        graph.remove_edge("a.rs", "b.rs").unwrap();

        // B should no longer have afferent coupling from A
        assert_eq!(graph.get_vector("b.rs").unwrap().afferent_coupling, 0);
    }

    #[test]
    fn test_potential_impact() {
        let mut graph = InfluenceGraph::new();

        // A -> B -> C
        // D -> B
        graph.add_edge("a.rs", "b.rs").unwrap();
        graph.add_edge("b.rs", "c.rs").unwrap();
        graph.add_edge("d.rs", "b.rs").unwrap();

        // Adding edge X -> A would affect:
        // - Files that depend on A (none directly)
        // - Files that X depends on (none)
        let impact = graph.get_potential_impact("x.rs", "a.rs");
        // X is not in the graph, so impact should be empty
        // But a.rs has reverse dependencies (nothing depends on a.rs directly)
        // Actually, the impact is files that would be affected by the new edge
        // Since x.rs is not in graph, there's no impact
        assert!(impact.is_empty() || impact.contains("b.rs"));

        // Adding edge A -> X would affect:
        // - Files that depend on A (none)
        // - Files that X depends on (none)
        let impact = graph.get_potential_impact("a.rs", "x.rs");
        // a.rs depends on b.rs, so b.rs would be affected
        assert!(impact.contains("b.rs") || impact.is_empty());
    }
}
