//! Shared final-stage reranking and budgeted selection.

use crate::reranker::{CrossEncoderScorer, RerankDocument};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Document contract accepted by the shared final stage.
pub trait RetrievalDocument: Clone + Send + Sync {
    /// Stable identifier.
    fn id(&self) -> &str;
    /// Human-readable title.
    fn title(&self) -> &str;
    /// Compact summary.
    fn summary(&self) -> &str;
    /// Body content used by the cross-encoder.
    fn body_markdown(&self) -> &str;
    /// Prompt token cost.
    fn token_cost(&self) -> usize;
}

/// Candidate after dense or hybrid retrieval.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FusedCandidate<D> {
    /// Candidate document.
    pub document: D,
    /// Reciprocal rank fusion score.
    pub rrf_score: f32,
    /// Dense rank, if present.
    pub dense_rank: Option<u32>,
    /// Dense score, if present.
    pub dense_score: Option<f32>,
    /// BM25 rank, if present.
    pub bm25_rank: Option<u32>,
    /// BM25 score, if present.
    pub bm25_score: Option<f32>,
}

/// Candidate accepted by MemGAS.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AcceptedCandidate<D> {
    /// Base fused candidate.
    pub candidate: FusedCandidate<D>,
    /// Posterior probability of belonging to the high-mean component.
    pub accept_posterior: f32,
}

/// Candidate after cross-encoder scoring.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RerankedCandidate<D> {
    /// Base accepted candidate.
    pub accepted: AcceptedCandidate<D>,
    /// Local reranker score.
    pub cross_score: f32,
    /// Prompt token cost.
    pub token_cost: usize,
}

/// Selection result from the knapsack stage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectionResult<D> {
    /// Chosen items.
    pub selected_documents: Vec<RerankedCandidate<D>>,
    /// Accepted but budget-discarded items.
    pub discarded_by_budget: Vec<RerankedCandidate<D>>,
    /// Tokens used.
    pub used_tokens: usize,
    /// Objective score.
    pub objective_score: f32,
}

/// Result from the MemGAS classifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemgasResult<D> {
    /// Accepted candidates.
    pub accept_set: Vec<AcceptedCandidate<D>>,
    /// Rejected candidates.
    pub reject_set: Vec<AcceptedCandidate<D>>,
    /// Gaussian means.
    pub component_means: [f32; 2],
    /// Gaussian variances.
    pub component_variances: [f32; 2],
    /// Whether EM converged.
    pub converged: bool,
    /// Whether a degenerate fallback path was used.
    pub degenerate: bool,
}

/// Final-stage output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnifiedFinalStageResult<D> {
    /// MemGAS split.
    pub memgas: MemgasResult<D>,
    /// Final budgeted selection.
    pub selection: SelectionResult<D>,
}

/// MemGAS classifier contract.
pub trait MemgasClassifier<D> {
    /// Split fused candidates into accept/reject sets.
    fn classify(&self, candidates: &[FusedCandidate<D>]) -> MemgasResult<D>;
}

/// Default two-component GMM classifier over RRF scores.
#[derive(Debug, Clone)]
pub struct GaussianMemgasClassifier {
    /// Maximum EM iterations.
    pub max_iterations: usize,
    /// Convergence epsilon.
    pub epsilon: f32,
    /// Variance floor.
    pub variance_floor: f32,
}

impl Default for GaussianMemgasClassifier {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            epsilon: 1e-4,
            variance_floor: 1e-6,
        }
    }
}

impl<D> MemgasClassifier<D> for GaussianMemgasClassifier
where
    D: Clone,
{
    fn classify(&self, candidates: &[FusedCandidate<D>]) -> MemgasResult<D> {
        if candidates.len() < 3 {
            let accepted = candidates
                .iter()
                .cloned()
                .map(|candidate| AcceptedCandidate {
                    candidate,
                    accept_posterior: 1.0,
                })
                .collect::<Vec<_>>();
            return MemgasResult {
                accept_set: accepted,
                reject_set: Vec::new(),
                component_means: [0.0, 0.0],
                component_variances: [1.0, 1.0],
                converged: false,
                degenerate: true,
            };
        }

        let standardized = standardize_scores(candidates);
        let mut means = initial_quantiles(&standardized);
        let mut variances = [1.0f32, 1.0f32];
        let mut priors = [0.5f32, 0.5f32];
        let mut converged = false;

        for _ in 0..self.max_iterations {
            let responsibilities = standardized
                .iter()
                .map(|score| posterior(*score, means, variances, priors))
                .collect::<Vec<_>>();

            let previous_means = means;
            for component in 0..2 {
                let weight_sum = responsibilities.iter().map(|resp| resp[component]).sum::<f32>();
                if weight_sum <= self.variance_floor {
                    continue;
                }
                means[component] = responsibilities
                    .iter()
                    .zip(standardized.iter())
                    .map(|(resp, score)| resp[component] * score)
                    .sum::<f32>()
                    / weight_sum;
                variances[component] = (responsibilities
                    .iter()
                    .zip(standardized.iter())
                    .map(|(resp, score)| resp[component] * (score - means[component]).powi(2))
                    .sum::<f32>()
                    / weight_sum)
                    .max(self.variance_floor);
                priors[component] = (weight_sum / standardized.len() as f32).max(self.variance_floor);
            }

            if (means[0] - previous_means[0]).abs() < self.epsilon
                && (means[1] - previous_means[1]).abs() < self.epsilon
            {
                converged = true;
                break;
            }
        }

        if (means[0] - means[1]).abs() < self.epsilon {
            let accepted = candidates
                .iter()
                .cloned()
                .map(|candidate| AcceptedCandidate {
                    candidate,
                    accept_posterior: 1.0,
                })
                .collect::<Vec<_>>();
            return MemgasResult {
                accept_set: accepted,
                reject_set: Vec::new(),
                component_means: means,
                component_variances: variances,
                converged,
                degenerate: true,
            };
        }

        let accept_component = if means[0] >= means[1] { 0 } else { 1 };
        let mut accept_set = Vec::new();
        let mut reject_set = Vec::new();
        for (candidate, score) in candidates.iter().cloned().zip(standardized.iter().copied()) {
            let resp = posterior(score, means, variances, priors);
            let accepted = AcceptedCandidate {
                candidate,
                accept_posterior: resp[accept_component],
            };
            if accepted.accept_posterior >= 0.5 {
                accept_set.push(accepted);
            } else {
                reject_set.push(accepted);
            }
        }

        MemgasResult {
            accept_set,
            reject_set,
            component_means: means,
            component_variances: variances,
            converged,
            degenerate: false,
        }
    }
}

/// Budgeted selection contract.
pub trait BudgetedSelector<D> {
    /// Select the best subset for the provided token budget.
    fn select(&self, items: &[RerankedCandidate<D>], budget_tokens: usize) -> SelectionResult<D>;
}

/// Exact 0/1 knapsack selector.
#[derive(Debug, Default, Clone)]
pub struct KnapsackBudgetedSelector;

impl<D> BudgetedSelector<D> for KnapsackBudgetedSelector
where
    D: Clone + RetrievalDocument,
{
    fn select(&self, items: &[RerankedCandidate<D>], budget_tokens: usize) -> SelectionResult<D> {
        let mut dp = vec![vec![0.0f32; budget_tokens + 1]; items.len() + 1];
        let mut keep = vec![vec![false; budget_tokens + 1]; items.len() + 1];

        for (index, item) in items.iter().enumerate() {
            let weight = item.token_cost.min(budget_tokens + 1);
            for budget in 0..=budget_tokens {
                let skip = dp[index][budget];
                let take = if weight <= budget {
                    dp[index][budget - weight] + item.cross_score
                } else {
                    f32::NEG_INFINITY
                };
                if take > skip {
                    dp[index + 1][budget] = take;
                    keep[index + 1][budget] = true;
                } else {
                    dp[index + 1][budget] = skip;
                }
            }
        }

        let mut selected = Vec::new();
        let mut budget = budget_tokens;
        for index in (1..=items.len()).rev() {
            if keep[index][budget] {
                let item = items[index - 1].clone();
                budget = budget.saturating_sub(item.token_cost);
                selected.push(item);
            }
        }
        selected.reverse();

        let selected_ids = selected
            .iter()
            .map(|item| item.accepted.candidate.document.id().to_string())
            .collect::<HashSet<_>>();
        let discarded_by_budget = items
            .iter()
            .filter(|item| !selected_ids.contains(item.accepted.candidate.document.id()))
            .cloned()
            .collect::<Vec<_>>();

        SelectionResult {
            selected_documents: selected,
            discarded_by_budget,
            used_tokens: items
                .iter()
                .filter(|item| selected_ids.contains(item.accepted.candidate.document.id()))
                .map(|item| item.token_cost)
                .sum(),
            objective_score: items
                .iter()
                .filter(|item| selected_ids.contains(item.accepted.candidate.document.id()))
                .map(|item| item.cross_score)
                .sum(),
        }
    }
}

/// Shared rerank + budget stage for multiple retrieval domains.
pub struct UnifiedFinalStage<R = crate::reranker::CandleCrossEncoder> {
    classifier: GaussianMemgasClassifier,
    selector: KnapsackBudgetedSelector,
    reranker: R,
}

impl<R> UnifiedFinalStage<R> {
    /// Construct a new final stage from an injected reranker.
    pub fn new(reranker: R) -> Self {
        Self {
            classifier: GaussianMemgasClassifier::default(),
            selector: KnapsackBudgetedSelector,
            reranker,
        }
    }
}

impl<R> UnifiedFinalStage<R>
where
    R: CrossEncoderScorer,
{
    /// Execute MemGAS, cross-encoder reranking, and knapsack selection.
    pub async fn run<D>(
        &self,
        query: &str,
        candidates: &[FusedCandidate<D>],
        budget_tokens: usize,
    ) -> Result<UnifiedFinalStageResult<D>>
    where
        D: RetrievalDocument + Serialize + for<'de> Deserialize<'de>,
    {
        let memgas = self.classifier.classify(candidates);
        let rerank_docs = memgas
            .accept_set
            .iter()
            .map(|accepted| RerankDocument {
                id: accepted.candidate.document.id().to_string(),
                title: accepted.candidate.document.title().to_string(),
                summary: accepted.candidate.document.summary().to_string(),
                body_markdown: accepted.candidate.document.body_markdown().to_string(),
            })
            .collect::<Vec<_>>();
        let cross_scores = self.reranker.score_pairs(query, &rerank_docs).await?;
        let reranked = memgas
            .accept_set
            .iter()
            .cloned()
            .zip(cross_scores.into_iter())
            .map(|(accepted, cross_score)| RerankedCandidate {
                token_cost: accepted.candidate.document.token_cost(),
                accepted,
                cross_score,
            })
            .collect::<Vec<_>>();
        let selection = self.selector.select(&reranked, budget_tokens);
        Ok(UnifiedFinalStageResult { memgas, selection })
    }
}

fn standardize_scores<D>(candidates: &[FusedCandidate<D>]) -> Vec<f32> {
    let scores = candidates
        .iter()
        .map(|candidate| candidate.rrf_score)
        .collect::<Vec<_>>();
    let mean = scores.iter().sum::<f32>() / scores.len() as f32;
    let variance = scores
        .iter()
        .map(|score| (score - mean).powi(2))
        .sum::<f32>()
        / scores.len() as f32;
    let stddev = variance.sqrt().max(1e-6);
    scores
        .into_iter()
        .map(|score| (score - mean) / stddev)
        .collect()
}

fn initial_quantiles(values: &[f32]) -> [f32; 2] {
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| {
        left.partial_cmp(right)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let q25 = sorted[((sorted.len() as f32 * 0.25).floor() as usize).min(sorted.len() - 1)];
    let q75 = sorted[((sorted.len() as f32 * 0.75).floor() as usize).min(sorted.len() - 1)];
    [q25, q75]
}

fn posterior(score: f32, means: [f32; 2], variances: [f32; 2], priors: [f32; 2]) -> [f32; 2] {
    let mut probs = [0.0f32; 2];
    for component in 0..2 {
        let variance = variances[component].max(1e-6);
        let coefficient = 1.0 / (2.0 * std::f32::consts::PI * variance).sqrt();
        let exponent = -((score - means[component]).powi(2)) / (2.0 * variance);
        probs[component] = priors[component] * coefficient * exponent.exp();
    }
    let total = probs[0] + probs[1];
    if total <= 1e-6 {
        [0.5, 0.5]
    } else {
        [probs[0] / total, probs[1] / total]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestDoc {
        id: String,
        title: String,
        summary: String,
        body: String,
        token_cost: usize,
    }

    impl RetrievalDocument for TestDoc {
        fn id(&self) -> &str {
            &self.id
        }
        fn title(&self) -> &str {
            &self.title
        }
        fn summary(&self) -> &str {
            &self.summary
        }
        fn body_markdown(&self) -> &str {
            &self.body
        }
        fn token_cost(&self) -> usize {
            self.token_cost
        }
    }

    struct MockScorer;

    #[async_trait]
    impl CrossEncoderScorer for MockScorer {
        async fn score_pairs(&self, _query: &str, docs: &[RerankDocument]) -> Result<Vec<f32>> {
            Ok(docs
                .iter()
                .map(|doc| if doc.id.contains("keep") { 10.0 } else { 1.0 })
                .collect())
        }
    }

    #[tokio::test]
    async fn unified_final_stage_handles_mixed_documents() {
        let stage = UnifiedFinalStage::new(MockScorer);
        let candidates = vec![
            FusedCandidate {
                document: TestDoc {
                    id: "skill-keep".to_string(),
                    title: "Skill".to_string(),
                    summary: "Summary".to_string(),
                    body: "Skill content".to_string(),
                    token_cost: 20,
                },
                rrf_score: 1.0,
                dense_rank: Some(1),
                dense_score: Some(0.9),
                bm25_rank: Some(1),
                bm25_score: Some(7.0),
            },
            FusedCandidate {
                document: TestDoc {
                    id: "code-drop".to_string(),
                    title: "Code".to_string(),
                    summary: "Summary".to_string(),
                    body: "Code content".to_string(),
                    token_cost: 40,
                },
                rrf_score: 0.8,
                dense_rank: Some(2),
                dense_score: Some(0.8),
                bm25_rank: None,
                bm25_score: None,
            },
            FusedCandidate {
                document: TestDoc {
                    id: "skill-drop".to_string(),
                    title: "Skill 2".to_string(),
                    summary: "Summary".to_string(),
                    body: "Other content".to_string(),
                    token_cost: 60,
                },
                rrf_score: 0.7,
                dense_rank: Some(3),
                dense_score: Some(0.7),
                bm25_rank: None,
                bm25_score: None,
            },
        ];

        let result = stage.run("repair cargo", &candidates, 30).await.unwrap();
        assert_eq!(result.selection.selected_documents.len(), 1);
        assert_eq!(
            result.selection.selected_documents[0]
                .accepted
                .candidate
                .document
                .id(),
            "skill-keep"
        );
    }
}
