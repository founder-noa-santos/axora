//! Dense code retrieval pipeline with a shared final stage.

use crate::error::RagError;
use crate::final_stage::{
    FusedCandidate, RetrievalDocument, UnifiedFinalStage, UnifiedFinalStageResult,
};
use crate::{CrossEncoderScorer, Result};
use openakta_embeddings::CodeEmbedder;
use openakta_indexing::DenseVectorCollection;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Query input for code retrieval.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodeRetrievalQuery {
    /// Workspace root used to hydrate source snippets.
    pub workspace_root: PathBuf,
    /// Natural-language query.
    pub query: String,
    /// Optional focal files.
    pub focal_files: Vec<String>,
    /// Optional focal symbols.
    pub focal_symbols: Vec<String>,
    /// Dense candidate limit.
    pub dense_limit: usize,
    /// Final token budget.
    pub token_budget: usize,
}

/// Dense code chunk candidate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeChunkDocument {
    /// Stable chunk identifier.
    pub chunk_id: String,
    /// File path relative to the workspace.
    pub file_path: String,
    /// Optional symbol path.
    pub symbol_path: Option<String>,
    /// Display summary.
    pub summary: String,
    /// Hydrated source content.
    pub body_markdown: String,
    /// Prompt token cost.
    pub token_cost: usize,
    /// Optional language.
    pub language: Option<String>,
    /// Optional chunk type.
    pub chunk_type: Option<String>,
    /// Start line.
    pub start_line: usize,
    /// End line.
    pub end_line: usize,
}

impl RetrievalDocument for CodeChunkDocument {
    fn id(&self) -> &str {
        &self.chunk_id
    }

    fn title(&self) -> &str {
        &self.file_path
    }

    fn summary(&self) -> &str {
        &self.summary
    }

    fn body_markdown(&self) -> &str {
        &self.body_markdown
    }

    fn token_cost(&self) -> usize {
        self.token_cost
    }
}

/// Output of the dense code pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeRetrievalResult {
    /// All fused candidates before the final stage.
    pub fused_candidates: Vec<FusedCandidate<CodeChunkDocument>>,
    /// Shared final-stage output.
    pub final_result: UnifiedFinalStageResult<CodeChunkDocument>,
}

/// Dense code retrieval pipeline.
pub struct CodeRetrievalPipeline<R = crate::CandleCrossEncoder> {
    collection: Arc<dyn DenseVectorCollection>,
    embedder: Arc<dyn CodeEmbedder>,
    final_stage: UnifiedFinalStage<R>,
}

impl<R> CodeRetrievalPipeline<R>
where
    R: CrossEncoderScorer,
{
    /// Construct a new code pipeline from injected components.
    pub fn new(
        collection: Arc<dyn DenseVectorCollection>,
        embedder: Arc<dyn CodeEmbedder>,
        reranker: R,
    ) -> Self {
        Self {
            collection,
            embedder,
            final_stage: UnifiedFinalStage::new(reranker),
        }
    }

    /// Retrieve and rank code chunks.
    pub async fn retrieve(&self, query: &CodeRetrievalQuery) -> Result<CodeRetrievalResult> {
        let query_embedding = self
            .embedder
            .embed(&query.query)
            .await
            .map_err(|err| RagError::Retrieval(err.to_string()))?;
        let hits = self
            .collection
            .search(&query_embedding, query.dense_limit)
            .await
            .map_err(|err| RagError::Retrieval(err.to_string()))?;
        let fused_candidates = hits
            .into_iter()
            .enumerate()
            .filter_map(|(index, hit)| hydrate_candidate(&query.workspace_root, hit, index + 1))
            .filter(|candidate| {
                matches_filters(candidate, &query.focal_files, &query.focal_symbols)
            })
            .collect::<Vec<_>>();

        let final_result = self
            .final_stage
            .run(&query.query, &fused_candidates, query.token_budget)
            .await?;

        Ok(CodeRetrievalResult {
            fused_candidates,
            final_result,
        })
    }
}

fn hydrate_candidate(
    workspace_root: &Path,
    hit: openakta_indexing::DenseSearchResult,
    rank: usize,
) -> Option<FusedCandidate<CodeChunkDocument>> {
    let file_path = hit.payload.get("file_path")?.as_str()?.to_string();
    let symbol_path = hit
        .payload
        .get("symbol_path")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .filter(|value| !value.is_empty());
    let start_line = hit
        .payload
        .get("start_line")
        .and_then(as_usize)
        .unwrap_or(1);
    let end_line = hit
        .payload
        .get("end_line")
        .and_then(as_usize)
        .unwrap_or(start_line);
    let full_path = if Path::new(&file_path).is_absolute() {
        PathBuf::from(&file_path)
    } else {
        workspace_root.join(&file_path)
    };
    let content = read_line_span(&full_path, start_line, end_line);
    let token_cost = content.len() / 4;
    Some(FusedCandidate {
        document: CodeChunkDocument {
            chunk_id: hit
                .payload
                .get("chunk_id")
                .and_then(|value| value.as_str())
                .unwrap_or(&hit.id)
                .to_string(),
            file_path: file_path.clone(),
            symbol_path,
            summary: format!("{file_path}:{start_line}-{end_line}"),
            body_markdown: content,
            token_cost,
            language: hit
                .payload
                .get("language")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string()),
            chunk_type: hit
                .payload
                .get("chunk_type")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string()),
            start_line,
            end_line,
        },
        rrf_score: hit.score,
        dense_rank: Some(rank as u32),
        dense_score: Some(hit.score),
        bm25_rank: None,
        bm25_score: None,
    })
}

fn matches_filters(
    candidate: &FusedCandidate<CodeChunkDocument>,
    focal_files: &[String],
    focal_symbols: &[String],
) -> bool {
    let file_match = focal_files.is_empty()
        || focal_files
            .iter()
            .any(|file| candidate.document.file_path.contains(file));
    let symbol_match = focal_symbols.is_empty()
        || focal_symbols.iter().any(|symbol| {
            candidate
                .document
                .symbol_path
                .as_ref()
                .map(|path| path.contains(symbol))
                .unwrap_or(false)
        });
    file_match && symbol_match
}

fn read_line_span(path: &Path, start_line: usize, end_line: usize) -> String {
    let Ok(content) = std::fs::read_to_string(path) else {
        return String::new();
    };
    content
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line_no = index + 1;
            if line_no >= start_line && line_no <= end_line {
                Some(line.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn as_usize(value: &serde_json::Value) -> Option<usize> {
    value
        .as_u64()
        .map(|value| value as usize)
        .or_else(|| value.as_str().and_then(|value| value.parse::<usize>().ok()))
}
