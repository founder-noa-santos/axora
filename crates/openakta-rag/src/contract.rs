//! Shared Retrieval Contract v1 types and helpers.

use crate::final_stage::{FusedCandidate, MemgasResult, RetrievalDocument, SelectionResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

pub const RETRIEVAL_CONTRACT_VERSION: &str = "retrieval_contract_v1";
pub const EMBEDDING_SCHEMA_VERSION: &str = "embedding_schema_v1";
pub const CHUNK_SCHEMA_VERSION: &str = "chunk_schema_v1";
pub const PAYLOAD_SCHEMA_VERSION: &str = "payload_schema_v1";
pub const DIAGNOSTICS_SCHEMA_VERSION: &str = "diagnostics_schema_v1";
pub const FUSION_POLICY_V1: &str = "rrf_v1";
pub const RERANK_POLICY_V1: &str = "cross_encoder_v1";
pub const SELECTION_POLICY_V1: &str = "budgeted_knapsack_v1";

/// Versioned retrieval contract metadata surfaced in diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetrievalContract {
    pub contract_version: String,
    pub embedding_schema_version: String,
    pub chunk_schema_version: String,
    pub payload_schema_version: String,
    pub candidate_channels: Vec<String>,
    pub fusion_policy: String,
    pub rerank_policy: String,
    pub selection_policy: String,
    pub diagnostics_schema_version: String,
}

impl RetrievalContract {
    pub fn v1(channels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            contract_version: RETRIEVAL_CONTRACT_VERSION.to_string(),
            embedding_schema_version: EMBEDDING_SCHEMA_VERSION.to_string(),
            chunk_schema_version: CHUNK_SCHEMA_VERSION.to_string(),
            payload_schema_version: PAYLOAD_SCHEMA_VERSION.to_string(),
            candidate_channels: channels.into_iter().map(Into::into).collect(),
            fusion_policy: FUSION_POLICY_V1.to_string(),
            rerank_policy: RERANK_POLICY_V1.to_string(),
            selection_policy: SELECTION_POLICY_V1.to_string(),
            diagnostics_schema_version: DIAGNOSTICS_SCHEMA_VERSION.to_string(),
        }
    }
}

/// Per-channel rank and score for a candidate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalChannelScore {
    pub channel: String,
    pub rank: u32,
    pub score: f32,
}

/// Diagnostics for a single fused candidate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalCandidateScore {
    pub document_id: String,
    pub title: String,
    pub channel_scores: Vec<RetrievalChannelScore>,
    pub fusion_score: f32,
    pub accept_posterior: f32,
    pub cross_score: f32,
    pub token_cost: usize,
    pub selected: bool,
}

/// Per-channel candidate generation summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetrievalChannelStat {
    pub channel: String,
    pub hits: u32,
    pub participated: bool,
}

/// Per-stage timing and throughput summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetrievalStageStat {
    pub stage: String,
    pub latency_ms: u64,
    pub input_count: u32,
    pub output_count: u32,
    pub degraded: bool,
}

/// Shared diagnostics payload for retrieval responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalDiagnosticsData {
    pub contract: RetrievalContract,
    pub channel_stats: Vec<RetrievalChannelStat>,
    pub fused_candidates: usize,
    pub accept_count: usize,
    pub reject_count: usize,
    pub selected_count: usize,
    pub used_tokens: usize,
    pub memgas_converged: bool,
    pub memgas_degenerate: bool,
    pub candidate_scores: Vec<RetrievalCandidateScore>,
    pub stage_stats: Vec<RetrievalStageStat>,
    pub degraded_mode: bool,
}

pub fn stage_stat(
    stage: impl Into<String>,
    started_at: Instant,
    input_count: usize,
    output_count: usize,
    degraded: bool,
) -> RetrievalStageStat {
    RetrievalStageStat {
        stage: stage.into(),
        latency_ms: started_at.elapsed().as_millis().min(u64::MAX as u128) as u64,
        input_count: input_count.min(u32::MAX as usize) as u32,
        output_count: output_count.min(u32::MAX as usize) as u32,
        degraded,
    }
}

pub fn build_channel_stats<D>(fused: &[FusedCandidate<D>]) -> Vec<RetrievalChannelStat> {
    let mut dense_hits = 0u32;
    let mut sparse_hits = 0u32;
    let mut structural_hits = 0u32;
    for candidate in fused {
        if candidate.dense_rank.is_some() {
            dense_hits += 1;
        }
        if candidate.bm25_rank.is_some() {
            sparse_hits += 1;
        }
        if candidate.structural_rank.is_some() {
            structural_hits += 1;
        }
    }

    vec![
        RetrievalChannelStat {
            channel: "dense".to_string(),
            hits: dense_hits,
            participated: dense_hits > 0,
        },
        RetrievalChannelStat {
            channel: "sparse".to_string(),
            hits: sparse_hits,
            participated: sparse_hits > 0,
        },
        RetrievalChannelStat {
            channel: "structural".to_string(),
            hits: structural_hits,
            participated: structural_hits > 0,
        },
    ]
}

pub fn build_candidate_scores<D>(
    fused: &[FusedCandidate<D>],
    memgas: &MemgasResult<D>,
    selection: &SelectionResult<D>,
) -> Vec<RetrievalCandidateScore>
where
    D: RetrievalDocument,
{
    let selected_ids = selection
        .selected_documents
        .iter()
        .map(|item| item.accepted.candidate.document.id().to_string())
        .collect::<HashSet<_>>();
    let accepted_scores = memgas
        .accept_set
        .iter()
        .chain(memgas.reject_set.iter())
        .map(|item| {
            (
                item.candidate.document.id().to_string(),
                item.accept_posterior,
            )
        })
        .collect::<HashMap<_, _>>();
    let rerank_scores = selection
        .selected_documents
        .iter()
        .chain(selection.discarded_by_budget.iter())
        .map(|item| {
            (
                item.accepted.candidate.document.id().to_string(),
                (item.cross_score, item.token_cost),
            )
        })
        .collect::<HashMap<_, _>>();

    fused
        .iter()
        .map(|candidate| {
            let (cross_score, token_cost) = rerank_scores
                .get(candidate.document.id())
                .copied()
                .unwrap_or((0.0, candidate.document.token_cost()));
            RetrievalCandidateScore {
                document_id: candidate.document.id().to_string(),
                title: candidate.document.title().to_string(),
                channel_scores: channel_scores(candidate),
                fusion_score: candidate.rrf_score,
                accept_posterior: accepted_scores
                    .get(candidate.document.id())
                    .copied()
                    .unwrap_or(0.0),
                cross_score,
                token_cost,
                selected: selected_ids.contains(candidate.document.id()),
            }
        })
        .collect()
}

fn channel_scores<D>(candidate: &FusedCandidate<D>) -> Vec<RetrievalChannelScore> {
    let mut scores = Vec::new();
    if let (Some(rank), Some(score)) = (candidate.dense_rank, candidate.dense_score) {
        scores.push(RetrievalChannelScore {
            channel: "dense".to_string(),
            rank,
            score,
        });
    }
    if let (Some(rank), Some(score)) = (candidate.bm25_rank, candidate.bm25_score) {
        scores.push(RetrievalChannelScore {
            channel: "sparse".to_string(),
            rank,
            score,
        });
    }
    if let (Some(rank), Some(score)) = (candidate.structural_rank, candidate.structural_score) {
        scores.push(RetrievalChannelScore {
            channel: "structural".to_string(),
            rank,
            score,
        });
    }
    scores
}
