//! Deterministic structural code retrieval built on SCIP and the influence graph.

use crate::error::RagError;
use crate::Result;
use openakta_indexing::{InfluenceGraph, SCIPIndex};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Structural retrieval configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuralCodeRetrievalConfig {
    /// Hard token budget enforced during traversal.
    pub token_budget: usize,
    /// Maximum number of hydrated documents.
    pub max_documents: usize,
}

impl Default for StructuralCodeRetrievalConfig {
    fn default() -> Self {
        Self {
            token_budget: 2_000,
            max_documents: 8,
        }
    }
}

/// Structural retrieval request anchored to a file or symbol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuralCodeRetrievalRequest {
    /// Task identifier.
    pub task_id: String,
    /// Natural-language query.
    pub query: String,
    /// Focal file when already known.
    pub focal_file: Option<String>,
    /// Focal symbol when already known.
    pub focal_symbol: Option<String>,
}

/// Hydrated code document selected for the model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuralHydratedDocument {
    /// File path.
    pub file_path: String,
    /// Estimated token count.
    pub token_count: usize,
    /// Whether this file is a direct dependency of the anchor.
    pub direct_dependency: bool,
    /// Symbols associated with the file.
    pub symbols: Vec<String>,
    /// Content snippet or full chunk.
    pub content: String,
}

/// Structural retrieval diagnostic entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuralRetrievalDiagnostic {
    /// Diagnostic kind.
    pub kind: String,
    /// Affected file when known.
    pub file_path: Option<String>,
    /// Human-readable reason.
    pub message: String,
}

/// Structural retrieval result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuralCodeRetrievalResult {
    /// Selected documents in priority order.
    pub documents: Vec<StructuralHydratedDocument>,
    /// Diagnostics emitted during traversal.
    pub diagnostics: Vec<StructuralRetrievalDiagnostic>,
    /// Total tokens used.
    pub tokens_used: usize,
}

/// Deterministic retriever backed by SCIP and the influence graph.
pub struct StructuralCodeRetriever {
    influence_graph: InfluenceGraph,
    documents: HashMap<String, String>,
    config: StructuralCodeRetrievalConfig,
}

impl StructuralCodeRetriever {
    /// Create a new structural retriever.
    pub fn new(
        influence_graph: InfluenceGraph,
        documents: HashMap<String, String>,
        config: StructuralCodeRetrievalConfig,
    ) -> Self {
        Self {
            influence_graph,
            documents,
            config,
        }
    }

    /// Build a retriever from a SCIP index and loaded documents.
    pub fn from_scip(
        scip_index: &SCIPIndex,
        documents: HashMap<String, String>,
        config: StructuralCodeRetrievalConfig,
    ) -> Result<Self> {
        let influence_graph = InfluenceGraph::from_scip(scip_index)
            .map_err(|err| RagError::Retrieval(err.to_string()))?;
        Ok(Self::new(influence_graph, documents, config))
    }

    /// Retrieve anchored code context within the configured budget.
    pub fn retrieve(
        &self,
        request: &StructuralCodeRetrievalRequest,
    ) -> Result<StructuralCodeRetrievalResult> {
        let mut diagnostics = Vec::new();
        let focal_file = if let Some(file) = &request.focal_file {
            Some(file.clone())
        } else if let Some(symbol) = &request.focal_symbol {
            self.influence_graph
                .resolve_symbol(symbol)
                .map(str::to_string)
        } else {
            None
        };

        let focal_file = focal_file.ok_or_else(|| {
            RagError::Retrieval(
                "structural code retrieval requires focal_file or focal_symbol".to_string(),
            )
        })?;

        let mut ordered = vec![focal_file.clone()];
        ordered.extend(self.influence_graph.dependency_chain(&focal_file));
        ordered.extend(
            self.influence_graph
                .calculate_transitive_closure(&focal_file)
                .map_err(|err| RagError::Retrieval(err.to_string()))?,
        );

        let direct_dependencies = self
            .influence_graph
            .get_dependencies(&focal_file)
            .cloned()
            .unwrap_or_default();

        let mut visited = HashSet::new();
        let mut tokens_used = 0usize;
        let mut documents = Vec::new();
        for candidate in ordered {
            if !visited.insert(candidate.clone()) {
                continue;
            }
            if documents.len() >= self.config.max_documents {
                diagnostics.push(StructuralRetrievalDiagnostic {
                    kind: "budget_exhausted".to_string(),
                    file_path: Some(candidate),
                    message: "max_documents reached before traversal completed".to_string(),
                });
                break;
            }

            let Some(content) = self.documents.get(&candidate).cloned() else {
                diagnostics.push(StructuralRetrievalDiagnostic {
                    kind: "dependency_omitted".to_string(),
                    file_path: Some(candidate),
                    message: "document content not available for hydrated dependency".to_string(),
                });
                continue;
            };

            let token_count = estimate_tokens(&content);
            if tokens_used + token_count > self.config.token_budget {
                diagnostics.push(StructuralRetrievalDiagnostic {
                    kind: "budget_exhausted".to_string(),
                    file_path: Some(candidate),
                    message: "token budget exhausted before dependency could be hydrated"
                        .to_string(),
                });
                continue;
            }

            tokens_used += token_count;
            documents.push(StructuralHydratedDocument {
                file_path: candidate.clone(),
                token_count,
                direct_dependency: direct_dependencies.contains(&candidate),
                symbols: self.influence_graph.symbols_for_file(&candidate),
                content,
            });
        }

        if documents.is_empty() {
            return Err(RagError::Retrieval(
                "structural code retrieval could not hydrate any documents".to_string(),
            )
            .into());
        }

        Ok(StructuralCodeRetrievalResult {
            documents,
            diagnostics,
            tokens_used,
        })
    }
}

fn estimate_tokens(content: &str) -> usize {
    content.len() / 4
}
