//! Graph-pruned context retrieval with TOON serialization.

use crate::error::AgentError;
use crate::provider::{ModelBoundaryPayload, ModelBoundaryPayloadType};
use crate::Result;
use axora_indexing::{InfluenceGraph, SCIPIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Graph retrieval configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphRetrievalConfig {
    /// Hard token budget enforced during traversal.
    pub token_budget: usize,
    /// Maximum number of documents to hydrate.
    pub max_documents: usize,
}

impl Default for GraphRetrievalConfig {
    fn default() -> Self {
        Self {
            token_budget: 2_000,
            max_documents: 8,
        }
    }
}

/// Retrieval request anchored to a file or symbol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphRetrievalRequest {
    /// Task identifier.
    pub task_id: String,
    /// Natural-language query.
    pub query: String,
    /// Focal file when already known.
    pub focal_file: Option<String>,
    /// Focal symbol when already known.
    pub focal_symbol: Option<String>,
}

/// Hydrated document selected for the model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HydratedDocument {
    /// File path.
    pub file_path: String,
    /// Estimated token count.
    pub token_count: usize,
    /// Whether this document is a direct dependency of the focal file.
    pub direct_dependency: bool,
    /// Symbols associated with the document.
    pub symbols: Vec<String>,
    /// Content snippet or full chunk.
    pub content: String,
}

/// Retrieval diagnostic entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetrievalDiagnostic {
    /// Diagnostic kind.
    pub kind: String,
    /// Affected file when known.
    pub file_path: Option<String>,
    /// Human-readable reason.
    pub message: String,
}

/// Result of graph retrieval.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphRetrievalResult {
    /// Selected documents in priority order.
    pub documents: Vec<HydratedDocument>,
    /// Diagnostics emitted during traversal.
    pub diagnostics: Vec<RetrievalDiagnostic>,
    /// TOON payload sent to the model.
    pub toon_payload: String,
    /// Total tokens used by hydrated content.
    pub tokens_used: usize,
}

/// Graph-pruned retriever built on top of SCIP and InfluenceGraph.
pub struct GraphRetriever {
    scip_index: SCIPIndex,
    influence_graph: InfluenceGraph,
    documents: HashMap<String, String>,
    config: GraphRetrievalConfig,
}

impl GraphRetriever {
    /// Create a new graph retriever.
    pub fn new(
        scip_index: SCIPIndex,
        influence_graph: InfluenceGraph,
        documents: HashMap<String, String>,
        config: GraphRetrievalConfig,
    ) -> Self {
        Self {
            scip_index,
            influence_graph,
            documents,
            config,
        }
    }

    /// Retrieve anchored context within a hard token budget.
    pub fn retrieve(&self, request: &GraphRetrievalRequest) -> Result<GraphRetrievalResult> {
        let mut diagnostics = Vec::new();
        let focal_file = if let Some(file) = &request.focal_file {
            Some(file.clone())
        } else if let Some(symbol) = &request.focal_symbol {
            self.influence_graph.resolve_symbol(symbol).map(str::to_string)
        } else {
            None
        };

        let focal_file = focal_file.ok_or_else(|| {
            AgentError::ExecutionFailed("graph retrieval requires focal_file or focal_symbol".to_string())
        })?;

        let mut candidates = vec![focal_file.clone()];
        candidates.extend(self.influence_graph.dependency_chain(&focal_file));

        let direct_dependencies = self
            .influence_graph
            .get_dependencies(&focal_file)
            .cloned()
            .unwrap_or_default();

        let mut tokens_used = 0usize;
        let mut documents = Vec::new();

        for candidate in candidates {
            if documents.len() >= self.config.max_documents {
                diagnostics.push(RetrievalDiagnostic {
                    kind: "budget_exhausted".to_string(),
                    file_path: Some(candidate),
                    message: "max_documents reached before traversal completed".to_string(),
                });
                break;
            }

            let content = match self.documents.get(&candidate) {
                Some(content) => content.clone(),
                None => {
                    diagnostics.push(RetrievalDiagnostic {
                        kind: "dependency_omitted".to_string(),
                        file_path: Some(candidate),
                        message: "document content not available for hydrated dependency".to_string(),
                    });
                    continue;
                }
            };

            let token_count = estimate_tokens(&content);
            if tokens_used + token_count > self.config.token_budget {
                diagnostics.push(RetrievalDiagnostic {
                    kind: "budget_exhausted".to_string(),
                    file_path: Some(candidate),
                    message: "token budget exhausted before dependency could be hydrated".to_string(),
                });
                continue;
            }

            tokens_used += token_count;
            documents.push(HydratedDocument {
                file_path: candidate.clone(),
                token_count,
                direct_dependency: direct_dependencies.contains(&candidate),
                symbols: self.influence_graph.symbols_for_file(&candidate),
                content,
            });
        }

        if documents.is_empty() {
            return Err(AgentError::ExecutionFailed(
                "graph retrieval could not hydrate any documents".to_string(),
            )
            .into());
        }

        let payload = ModelBoundaryPayload {
            payload_type: ModelBoundaryPayloadType::Retrieval,
            task_id: request.task_id.clone(),
            title: request.query.clone(),
            description: request.query.clone(),
            task_type: "retrieval".to_string(),
            target_files: documents.iter().map(|doc| doc.file_path.clone()).collect(),
            target_symbols: self.scip_index.symbols.iter().map(|symbol| symbol.symbol.clone()).collect(),
            context_spans: documents
                .iter()
                .flat_map(|doc| {
                    doc.symbols
                        .iter()
                        .map(move |symbol| format!("{}::{symbol}", doc.file_path))
                })
                .collect(),
            context_pack: None,
        };

        Ok(GraphRetrievalResult {
            documents,
            diagnostics,
            toon_payload: payload.to_toon()?,
            tokens_used,
        })
    }
}

fn estimate_tokens(content: &str) -> usize {
    content.len() / 4
}

#[cfg(test)]
mod tests {
    use super::*;
    use axora_indexing::{Occurrence, PackageInfo, Symbol, SymbolKind};

    fn sample_index() -> SCIPIndex {
        let mut index = SCIPIndex::new(PackageInfo::new("cargo", "demo", "0.1.0"));
        index.symbols.push(Symbol::new("auth::login", SymbolKind::Function, "fn login()"));
        index.symbols.push(Symbol::new("db::query", SymbolKind::Function, "fn query()"));
        index.occurrences.push(
            Occurrence::new("src/auth.rs", 1, 0, "auth::login", true).with_snippet("fn login() {}"),
        );
        index.occurrences.push(
            Occurrence::new("src/auth.rs", 5, 0, "db::query", false).with_snippet("use db::query;"),
        );
        index.occurrences.push(
            Occurrence::new("src/db.rs", 1, 0, "db::query", true).with_snippet("fn query() {}"),
        );
        index
    }

    #[test]
    fn test_graph_retrieval_prefers_direct_dependencies() {
        let scip = sample_index();
        let influence = InfluenceGraph::from_scip(&scip).unwrap();
        let documents = HashMap::from([
            ("src/auth.rs".to_string(), "fn login() { query(); }".to_string()),
            ("src/db.rs".to_string(), "fn query() {}".to_string()),
        ]);
        let retriever = GraphRetriever::new(scip, influence, documents, GraphRetrievalConfig::default());
        let result = retriever
            .retrieve(&GraphRetrievalRequest {
                task_id: "task-1".to_string(),
                query: "fix auth login".to_string(),
                focal_file: Some("src/auth.rs".to_string()),
                focal_symbol: None,
            })
            .unwrap();

        assert_eq!(result.documents[0].file_path, "src/auth.rs");
        assert!(result.documents.iter().any(|doc| doc.file_path == "src/db.rs"));
    }

    #[test]
    fn test_graph_retrieval_respects_token_budget() {
        let scip = sample_index();
        let influence = InfluenceGraph::from_scip(&scip).unwrap();
        let documents = HashMap::from([
            ("src/auth.rs".to_string(), "a".repeat(200)),
            ("src/db.rs".to_string(), "b".repeat(4000)),
        ]);
        let retriever = GraphRetriever::new(
            scip,
            influence,
            documents,
            GraphRetrievalConfig {
                token_budget: 100,
                max_documents: 4,
            },
        );
        let result = retriever
            .retrieve(&GraphRetrievalRequest {
                task_id: "task-1".to_string(),
                query: "fix auth login".to_string(),
                focal_file: Some("src/auth.rs".to_string()),
                focal_symbol: None,
            })
            .unwrap();

        assert_eq!(result.documents.len(), 1);
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == "budget_exhausted"));
    }
}
