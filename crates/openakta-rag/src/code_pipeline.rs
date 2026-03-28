//! Hybrid code retrieval pipeline with dense, sparse, and structural channels.

use crate::contract::{
    build_candidate_scores, build_channel_stats, stage_stat, RetrievalContract,
    RetrievalDiagnosticsData,
};
use crate::error::RagError;
use crate::final_stage::{
    FusedCandidate, RetrievalDocument, UnifiedFinalStage, UnifiedFinalStageResult,
};
use crate::reranker::CrossEncoderScorer;
use crate::{
    RankedHit, ReciprocalRankFusion, Result, StructuralCodeRetrievalRequest,
    StructuralCodeRetriever,
};
use openakta_embeddings::CodeEmbedder;
use openakta_indexing::{
    DenseSearchResult, DenseVectorCollection, IncrementalIndexer, Language, ParserRegistry,
    SCIPIndex, SparseCodeHit, TantivyCodeIndex,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

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
    /// Sparse candidate limit.
    pub sparse_limit: usize,
    /// Shared candidate limit across channels.
    pub candidate_limit: usize,
    /// Final token budget.
    pub token_budget: usize,
}

/// Dense/sparse/structural code chunk candidate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeChunkDocument {
    pub chunk_id: String,
    pub file_path: String,
    pub symbol_path: Option<String>,
    pub summary: String,
    pub body_markdown: String,
    pub token_cost: usize,
    pub language: Option<String>,
    pub chunk_type: Option<String>,
    pub start_line: usize,
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

/// Output of the hybrid code pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeRetrievalResult {
    pub fused_candidates: Vec<FusedCandidate<CodeChunkDocument>>,
    pub final_result: UnifiedFinalStageResult<CodeChunkDocument>,
    pub diagnostics: RetrievalDiagnosticsData,
}

/// Hybrid code retrieval pipeline.
pub struct CodeRetrievalPipeline<R = crate::OpenaktaReranker> {
    workspace_root: PathBuf,
    sparse_index: Arc<TantivyCodeIndex>,
    indexer: Mutex<IncrementalIndexer>,
    dense_collection: Arc<dyn DenseVectorCollection>,
    embedder: Arc<dyn CodeEmbedder>,
    final_stage: UnifiedFinalStage<R>,
    fusion: ReciprocalRankFusion,
}

impl<R> CodeRetrievalPipeline<R>
where
    R: CrossEncoderScorer,
{
    /// Construct a new code pipeline from injected components.
    pub fn new(
        workspace_root: PathBuf,
        state_path: PathBuf,
        dense_collection: Arc<dyn DenseVectorCollection>,
        sparse_index: Arc<TantivyCodeIndex>,
        embedder: Arc<dyn CodeEmbedder>,
        reranker: R,
    ) -> Result<Self> {
        let indexer = IncrementalIndexer::new(
            &workspace_root,
            &state_path,
            embedder.clone(),
            dense_collection.clone(),
            sparse_index.clone(),
        )
        .map_err(|err| RagError::Retrieval(err.to_string()))?;
        Ok(Self {
            workspace_root,
            sparse_index,
            indexer: Mutex::new(indexer),
            dense_collection,
            embedder,
            final_stage: UnifiedFinalStage::new(reranker),
            fusion: ReciprocalRankFusion::default(),
        })
    }

    /// Synchronize the code indexes with the workspace before retrieval.
    pub async fn sync_if_needed(&self) -> Result<()> {
        self.indexer
            .lock()
            .await
            .index()
            .await
            .map(|_| ())
            .map_err(|err| RagError::Retrieval(err.to_string()).into())
    }

    /// Retrieve and rank code chunks.
    pub async fn retrieve(&self, query: &CodeRetrievalQuery) -> Result<CodeRetrievalResult> {
        self.sync_if_needed().await?;

        let candidate_limit = non_zero(query.candidate_limit, 32);
        let dense_limit = non_zero(query.dense_limit, candidate_limit);
        let sparse_limit = non_zero(query.sparse_limit, candidate_limit);

        let dense_started = std::time::Instant::now();
        let dense_hits = self.search_dense(&query.query, dense_limit).await?;
        let dense_stage = stage_stat("dense_search", dense_started, 1, dense_hits.len(), false);

        let sparse_started = std::time::Instant::now();
        let (sparse_hits, sparse_degraded) = match self
            .sparse_index
            .search(&query.query, sparse_limit)
        {
            Ok(hits) => (hits, false),
            Err(err) => {
                tracing::warn!(target: "openakta_rag", error = %err, "sparse code search degraded");
                (Vec::new(), true)
            }
        };
        let sparse_stage = stage_stat(
            "sparse_search",
            sparse_started,
            1,
            sparse_hits.len(),
            sparse_degraded,
        );

        let structural_started = std::time::Instant::now();
        let (structural_candidates, structural_degraded) =
            self.search_structural(query, candidate_limit);
        let structural_stage = stage_stat(
            "structural_search",
            structural_started,
            usize::from(!query.focal_files.is_empty() || !query.focal_symbols.is_empty()),
            structural_candidates.len(),
            structural_degraded,
        );

        let fusion_started = std::time::Instant::now();
        let fused_candidates = self.fuse_candidates(
            query,
            &dense_hits,
            &sparse_hits,
            &structural_candidates,
            candidate_limit,
        )?;
        let fusion_stage = stage_stat(
            "fusion",
            fusion_started,
            dense_hits.len() + sparse_hits.len() + structural_candidates.len(),
            fused_candidates.len(),
            false,
        );

        let rerank_started = std::time::Instant::now();
        let mut final_result = self
            .final_stage
            .run(&query.query, &fused_candidates, query.token_budget)
            .await?;
        order_selected_documents(query, &mut final_result.selection.selected_documents);
        let rerank_stage = stage_stat(
            "rerank_select",
            rerank_started,
            fused_candidates.len(),
            final_result.selection.selected_documents.len(),
            false,
        );

        let diagnostics = RetrievalDiagnosticsData {
            contract: RetrievalContract::v1(["dense", "sparse", "structural"]),
            channel_stats: build_channel_stats(&fused_candidates),
            fused_candidates: fused_candidates.len(),
            accept_count: final_result.memgas.accept_set.len(),
            reject_count: final_result.memgas.reject_set.len(),
            selected_count: final_result.selection.selected_documents.len(),
            used_tokens: final_result.selection.used_tokens,
            memgas_converged: final_result.memgas.converged,
            memgas_degenerate: final_result.memgas.degenerate,
            candidate_scores: build_candidate_scores(
                &fused_candidates,
                &final_result.memgas,
                &final_result.selection,
            ),
            stage_stats: vec![
                dense_stage,
                sparse_stage,
                structural_stage,
                fusion_stage,
                rerank_stage,
            ],
            degraded_mode: sparse_degraded || structural_degraded,
        };

        Ok(CodeRetrievalResult {
            fused_candidates,
            final_result,
            diagnostics,
        })
    }

    async fn search_dense(&self, query: &str, limit: usize) -> Result<Vec<DenseSearchResult>> {
        let query_embedding = self
            .embedder
            .embed(query)
            .await
            .map_err(|err| RagError::Retrieval(err.to_string()))?;
        self.dense_collection
            .search(&query_embedding, limit)
            .await
            .map_err(|err| RagError::Retrieval(err.to_string()).into())
    }

    fn search_structural(
        &self,
        query: &CodeRetrievalQuery,
        limit: usize,
    ) -> (Vec<CodeChunkDocument>, bool) {
        if query.focal_files.is_empty() && query.focal_symbols.is_empty() {
            return (Vec::new(), false);
        }

        let Some(language) = query
            .focal_files
            .first()
            .and_then(|file| detect_language(file))
            .or_else(|| {
                query
                    .focal_symbols
                    .first()
                    .and_then(|_| detect_language_from_workspace(&self.workspace_root))
            })
        else {
            return (Vec::new(), true);
        };

        let registry = ParserRegistry::new();
        let scip = match registry.parse(language, &self.workspace_root) {
            Ok(index) => index,
            Err(err) => {
                tracing::warn!(target: "openakta_rag", error = %err, "structural parse degraded");
                return (Vec::new(), true);
            }
        };
        let documents = load_documents(&self.workspace_root, &scip);
        let retriever = match StructuralCodeRetriever::from_scip(
            &scip,
            documents,
            crate::StructuralCodeRetrievalConfig {
                token_budget: query.token_budget,
                max_documents: limit.max(1),
            },
        ) {
            Ok(retriever) => retriever,
            Err(err) => {
                tracing::warn!(target: "openakta_rag", error = %err, "structural retriever degraded");
                return (Vec::new(), true);
            }
        };

        match retriever.retrieve(&StructuralCodeRetrievalRequest {
            task_id: "code-retrieval".to_string(),
            query: query.query.clone(),
            focal_file: query.focal_files.first().cloned(),
            focal_symbol: query.focal_symbols.first().cloned(),
        }) {
            Ok(result) => (
                result
                    .documents
                    .into_iter()
                    .enumerate()
                    .map(|(index, document)| CodeChunkDocument {
                        chunk_id: format!("structural:{}:{}", document.file_path, index),
                        file_path: document.file_path,
                        symbol_path: document.symbols.first().cloned(),
                        summary: "structural_context".to_string(),
                        body_markdown: document.content,
                        token_cost: document.token_count,
                        language: None,
                        chunk_type: Some(if document.direct_dependency {
                            "structural_direct".to_string()
                        } else {
                            "structural_context".to_string()
                        }),
                        start_line: 1,
                        end_line: 1,
                    })
                    .collect(),
                false,
            ),
            Err(err) => {
                tracing::warn!(target: "openakta_rag", error = %err, "structural retrieval degraded");
                (Vec::new(), true)
            }
        }
    }

    fn fuse_candidates(
        &self,
        query: &CodeRetrievalQuery,
        dense_hits: &[DenseSearchResult],
        sparse_hits: &[SparseCodeHit],
        structural_candidates: &[CodeChunkDocument],
        candidate_limit: usize,
    ) -> Result<Vec<FusedCandidate<CodeChunkDocument>>> {
        let dense_ranked = dense_hits
            .iter()
            .enumerate()
            .map(|(index, hit)| RankedHit {
                document_id: dense_hit_id(hit),
                rank: (index + 1) as u32,
                score: hit.score,
                source: "dense".to_string(),
            })
            .collect::<Vec<_>>();
        let sparse_ranked = sparse_hits
            .iter()
            .map(|hit| RankedHit {
                document_id: hit.chunk_id.clone(),
                rank: hit.rank,
                score: hit.score,
                source: "sparse".to_string(),
            })
            .collect::<Vec<_>>();
        let structural_ranked = structural_candidates
            .iter()
            .enumerate()
            .map(|(index, document)| RankedHit {
                document_id: document.chunk_id.clone(),
                rank: (index + 1) as u32,
                score: 1.0 / (index + 1) as f32,
                source: "structural".to_string(),
            })
            .collect::<Vec<_>>();

        let fused = self.fusion.fuse(&[
            dense_ranked.clone(),
            sparse_ranked.clone(),
            structural_ranked.clone(),
        ]);

        let dense_scores = dense_hits
            .iter()
            .enumerate()
            .map(|(index, hit)| {
                let id = dense_hit_id(hit);
                (
                    id,
                    (
                        (index + 1) as u32,
                        hit.score,
                        hydrate_dense_candidate(&query.workspace_root, hit),
                    ),
                )
            })
            .collect::<HashMap<_, _>>();
        let sparse_scores = sparse_hits
            .iter()
            .filter_map(|hit| {
                self.sparse_index
                    .get_document(&hit.chunk_id)
                    .ok()
                    .flatten()
                    .map(|document| {
                        (
                            hit.chunk_id.clone(),
                            (
                                hit.rank,
                                hit.score,
                                hydrate_sparse_candidate(&query.workspace_root, &document),
                            ),
                        )
                    })
            })
            .collect::<HashMap<_, _>>();
        let structural_scores = structural_candidates
            .iter()
            .enumerate()
            .map(|(index, document)| {
                (
                    document.chunk_id.clone(),
                    (
                        (index + 1) as u32,
                        1.0 / (index + 1) as f32,
                        document.clone(),
                    ),
                )
            })
            .collect::<HashMap<_, _>>();

        let mut candidates = Vec::new();
        let mut seen = HashSet::new();
        for rank in fused.into_iter().take(candidate_limit.max(1) * 3) {
            if !seen.insert(rank.document_id.clone()) {
                continue;
            }

            let dense = dense_scores.get(&rank.document_id);
            let sparse = sparse_scores.get(&rank.document_id);
            let structural = structural_scores.get(&rank.document_id);
            let document = dense
                .map(|(_, _, document)| document.clone())
                .or_else(|| sparse.map(|(_, _, document)| document.clone()))
                .or_else(|| structural.map(|(_, _, document)| document.clone()));
            let Some(document) = document else {
                continue;
            };

            candidates.push(FusedCandidate {
                document,
                rrf_score: rank.score,
                dense_rank: dense.map(|(rank, _, _)| *rank),
                dense_score: dense.map(|(_, score, _)| *score),
                bm25_rank: sparse.map(|(rank, _, _)| *rank),
                bm25_score: sparse.map(|(_, score, _)| *score),
                structural_rank: structural.map(|(rank, _, _)| *rank),
                structural_score: structural.map(|(_, score, _)| *score),
            });
        }

        Ok(candidates)
    }
}

fn dense_hit_id(hit: &DenseSearchResult) -> String {
    hit.payload
        .get("chunk_id")
        .and_then(|value| value.as_str())
        .unwrap_or(&hit.id)
        .to_string()
}

fn hydrate_dense_candidate(workspace_root: &Path, hit: &DenseSearchResult) -> CodeChunkDocument {
    let file_path = hit
        .payload
        .get("file_path")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
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
    let body_markdown = read_line_span(&full_path, start_line, end_line);
    let token_cost = hit
        .payload
        .get("token_cost")
        .and_then(as_usize)
        .unwrap_or_else(|| estimate_tokens(&body_markdown));

    CodeChunkDocument {
        chunk_id: dense_hit_id(hit),
        file_path: file_path.clone(),
        symbol_path,
        summary: hit
            .payload
            .get("summary")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .unwrap_or_else(|| format!("{file_path}:{start_line}-{end_line}")),
        body_markdown,
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
    }
}

fn hydrate_sparse_candidate(
    workspace_root: &Path,
    document: &openakta_indexing::CodeIndexDocument,
) -> CodeChunkDocument {
    let full_path = if Path::new(&document.file_path).is_absolute() {
        PathBuf::from(&document.file_path)
    } else {
        workspace_root.join(&document.file_path)
    };
    let body_markdown = if document.body_markdown.is_empty() {
        read_line_span(&full_path, document.start_line, document.end_line)
    } else {
        document.body_markdown.clone()
    };
    CodeChunkDocument {
        chunk_id: document.chunk_id.clone(),
        file_path: document.file_path.clone(),
        symbol_path: document.symbol_path.clone(),
        summary: document.summary.clone(),
        body_markdown,
        token_cost: document
            .token_cost
            .max(estimate_tokens(&document.body_markdown)),
        language: document.language.clone(),
        chunk_type: document.chunk_type.clone(),
        start_line: document.start_line,
        end_line: document.end_line,
    }
}

fn order_selected_documents(
    query: &CodeRetrievalQuery,
    selected: &mut [crate::RerankedCandidate<CodeChunkDocument>],
) {
    selected.sort_by(|left, right| {
        direct_match_score(&right.accepted.candidate.document, query)
            .cmp(&direct_match_score(
                &left.accepted.candidate.document,
                query,
            ))
            .then_with(|| {
                right
                    .cross_score
                    .partial_cmp(&left.cross_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                right
                    .accepted
                    .candidate
                    .rrf_score
                    .partial_cmp(&left.accepted.candidate.rrf_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.token_cost.cmp(&right.token_cost))
    });
}

fn direct_match_score(document: &CodeChunkDocument, query: &CodeRetrievalQuery) -> u8 {
    let file_match = query
        .focal_files
        .iter()
        .any(|file| document.file_path.contains(file));
    let symbol_match = query.focal_symbols.iter().any(|symbol| {
        document
            .symbol_path
            .as_ref()
            .map(|path| path.contains(symbol))
            .unwrap_or(false)
    });
    let structural_match = document
        .chunk_type
        .as_deref()
        .unwrap_or_default()
        .starts_with("structural");

    match (file_match, symbol_match, structural_match) {
        (_, true, true) => 4,
        (_, true, false) => 3,
        (true, _, true) => 2,
        (true, _, false) => 1,
        _ => 0,
    }
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

fn estimate_tokens(content: &str) -> usize {
    content.len().saturating_add(3) / 4
}

fn non_zero(value: usize, default: usize) -> usize {
    if value == 0 {
        default
    } else {
        value
    }
}

fn detect_language(file_path: &str) -> Option<Language> {
    if file_path.ends_with(".rs") {
        Some(Language::Rust)
    } else if file_path.ends_with(".ts")
        || file_path.ends_with(".tsx")
        || file_path.ends_with(".js")
        || file_path.ends_with(".jsx")
    {
        Some(Language::TypeScript)
    } else if file_path.ends_with(".py") {
        Some(Language::Python)
    } else if file_path.ends_with(".go") {
        Some(Language::Go)
    } else {
        None
    }
}

fn detect_language_from_workspace(workspace_root: &Path) -> Option<Language> {
    let mut observed = None;
    if let Ok(entries) = std::fs::read_dir(workspace_root) {
        for entry in entries.flatten() {
            if let Some(name) = entry.path().to_str() {
                observed = detect_language(name);
                if observed.is_some() {
                    break;
                }
            }
        }
    }
    observed
}

fn load_documents(workspace_root: &Path, scip: &SCIPIndex) -> HashMap<String, String> {
    let mut documents = HashMap::new();
    for file_path in scip
        .occurrences
        .iter()
        .map(|occurrence| occurrence.file_path.clone())
    {
        if documents.contains_key(&file_path) {
            continue;
        }
        let full_path = workspace_root.join(&file_path);
        if let Ok(content) = std::fs::read_to_string(full_path) {
            documents.insert(file_path, content);
        }
    }
    documents
}
