//! Graph-pruned context retrieval with TOON serialization.

use crate::provider::{ModelBoundaryPayload, ModelBoundaryPayloadType};
use crate::Result;
use openakta_indexing::SCIPIndex;
use openakta_rag::{
    StructuralCodeRetrievalRequest, StructuralCodeRetrievalResult, StructuralCodeRetriever,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use openakta_rag::{
    StructuralCodeRetrievalConfig as GraphRetrievalConfig,
    StructuralCodeRetrievalRequest as GraphRetrievalRequest,
    StructuralHydratedDocument as HydratedDocument,
    StructuralRetrievalDiagnostic as RetrievalDiagnostic,
};

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
    inner: StructuralCodeRetriever,
}

impl GraphRetriever {
    /// Create a new graph retriever.
    pub fn new(
        scip_index: SCIPIndex,
        influence_graph: openakta_indexing::InfluenceGraph,
        documents: HashMap<String, String>,
        config: GraphRetrievalConfig,
    ) -> Self {
        Self {
            scip_index,
            inner: StructuralCodeRetriever::new(influence_graph, documents, config),
        }
    }

    /// Retrieve anchored context within a hard token budget.
    pub fn retrieve(&self, request: &GraphRetrievalRequest) -> Result<GraphRetrievalResult> {
        let result: StructuralCodeRetrievalResult =
            self.inner.retrieve(&StructuralCodeRetrievalRequest {
                task_id: request.task_id.clone(),
                query: request.query.clone(),
                focal_file: request.focal_file.clone(),
                focal_symbol: request.focal_symbol.clone(),
            })?;

        let payload = ModelBoundaryPayload {
            payload_type: ModelBoundaryPayloadType::Retrieval,
            task_id: request.task_id.clone(),
            title: request.query.clone(),
            description: request.query.clone(),
            task_type: "retrieval".to_string(),
            target_files: result
                .documents
                .iter()
                .map(|doc| doc.file_path.clone())
                .collect(),
            target_symbols: self
                .scip_index
                .symbols
                .iter()
                .map(|symbol| symbol.symbol.clone())
                .collect(),
            context_spans: result
                .documents
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
            documents: result.documents,
            diagnostics: result.diagnostics,
            toon_payload: payload.to_toon()?,
            tokens_used: result.tokens_used,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openakta_indexing::{Occurrence, PackageInfo, Symbol, SymbolKind};

    fn sample_index() -> SCIPIndex {
        let mut index = SCIPIndex::new(PackageInfo::new("cargo", "demo", "0.1.0"));
        index.symbols.push(Symbol::new(
            "auth::login",
            SymbolKind::Function,
            "fn login()",
        ));
        index
            .symbols
            .push(Symbol::new("db::query", SymbolKind::Function, "fn query()"));
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
        let influence = openakta_indexing::InfluenceGraph::from_scip(&scip).unwrap();
        let documents = HashMap::from([
            (
                "src/auth.rs".to_string(),
                "fn login() { query(); }".to_string(),
            ),
            ("src/db.rs".to_string(), "fn query() {}".to_string()),
        ]);
        let retriever =
            GraphRetriever::new(scip, influence, documents, GraphRetrievalConfig::default());
        let result = retriever
            .retrieve(&GraphRetrievalRequest {
                task_id: "task-1".to_string(),
                query: "fix auth login".to_string(),
                focal_file: Some("src/auth.rs".to_string()),
                focal_symbol: None,
            })
            .unwrap();

        assert_eq!(result.documents[0].file_path, "src/auth.rs");
        assert!(result
            .documents
            .iter()
            .any(|doc| doc.file_path == "src/db.rs"));
    }

    #[test]
    fn test_graph_retrieval_requires_anchor() {
        let scip = sample_index();
        let influence = openakta_indexing::InfluenceGraph::from_scip(&scip).unwrap();
        let retriever = GraphRetriever::new(
            scip,
            influence,
            HashMap::from([("src/auth.rs".to_string(), "fn login() {}".to_string())]),
            GraphRetrievalConfig::default(),
        );
        let result = retriever.retrieve(&GraphRetrievalRequest {
            task_id: "task-1".to_string(),
            query: "find auth".to_string(),
            focal_file: None,
            focal_symbol: None,
        });
        assert!(result.is_err());
    }
}
