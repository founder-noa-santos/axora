//! Retrieval benchmark portfolio and near-term IR metrics.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

pub const RETRIEVAL_BENCHMARK_PORTFOLIO_V1: &str = "retrieval_benchmark_portfolio_v1";

/// Benchmark case grouping for repo scale.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RepoSizeBucket {
    Small,
    Medium,
    Large,
}

impl RepoSizeBucket {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }
}

/// Benchmark case grouping for user intent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalTaskClass {
    BugFix,
    FeatureWork,
    Refactor,
    TestAuthoring,
    Navigation,
    Explanation,
}

impl RetrievalTaskClass {
    fn as_str(&self) -> &'static str {
        match self {
            Self::BugFix => "bug_fix",
            Self::FeatureWork => "feature_work",
            Self::Refactor => "refactor",
            Self::TestAuthoring => "test_authoring",
            Self::Navigation => "navigation",
            Self::Explanation => "explanation",
        }
    }
}

/// Relevant document annotation for a benchmark case.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RelevantDocument {
    pub document_id: String,
    #[serde(default = "default_relevance")]
    pub relevance: f32,
}

fn default_relevance() -> f32 {
    1.0
}

/// Retrieval benchmark case suitable for a generalist code product.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalBenchmarkCase {
    pub case_id: String,
    pub codebase_id: String,
    pub language: String,
    pub repo_size: RepoSizeBucket,
    pub task_class: RetrievalTaskClass,
    pub query: String,
    pub relevant_documents: Vec<RelevantDocument>,
}

/// Serializable portfolio manifest for cross-codebase evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkPortfolio {
    pub version: String,
    pub cases: Vec<RetrievalBenchmarkCase>,
}

impl BenchmarkPortfolio {
    pub fn v1(cases: Vec<RetrievalBenchmarkCase>) -> Self {
        Self {
            version: RETRIEVAL_BENCHMARK_PORTFOLIO_V1.to_string(),
            cases,
        }
    }
}

/// Ordered retrieval output from a single benchmark run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievedDocumentRank {
    pub document_id: String,
    pub rank: usize,
    pub score: f32,
}

/// Runtime result captured for a benchmark case.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalBenchmarkRun {
    pub case_id: String,
    pub retrieved_documents: Vec<RetrievedDocumentRank>,
    #[serde(default)]
    pub selected_document_ids: Vec<String>,
    pub mission_success: Option<bool>,
}

/// Aggregate IR metrics used for near-term retrieval tuning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalMetricSummary {
    pub success_at_k: f32,
    pub recall_at_k: f32,
    pub mrr: f32,
    pub ndcg_at_k: f32,
}

impl Default for RetrievalMetricSummary {
    fn default() -> Self {
        Self {
            success_at_k: 0.0,
            recall_at_k: 0.0,
            mrr: 0.0,
            ndcg_at_k: 0.0,
        }
    }
}

/// Minimal mission-linked outcome signals for retrieval experiments.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MissionLinkageSummary {
    pub selected_evidence_rate: f32,
    pub mission_success_rate: f32,
    pub mission_success_with_selected_evidence_rate: f32,
    pub observed_mission_count: usize,
}

impl Default for MissionLinkageSummary {
    fn default() -> Self {
        Self {
            selected_evidence_rate: 0.0,
            mission_success_rate: 0.0,
            mission_success_with_selected_evidence_rate: 0.0,
            observed_mission_count: 0,
        }
    }
}

/// Case-level report used to debug retrieval regressions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalCaseReport {
    pub case_id: String,
    pub codebase_id: String,
    pub language: String,
    pub repo_size: RepoSizeBucket,
    pub task_class: RetrievalTaskClass,
    pub metrics: RetrievalMetricSummary,
    pub mission: MissionLinkageSummary,
}

/// Slice report for language, repo-size, task-class, or codebase rollups.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalSliceReport {
    pub dimension: String,
    pub value: String,
    pub case_count: usize,
    pub codebase_count: usize,
    pub metrics: RetrievalMetricSummary,
    pub mission: MissionLinkageSummary,
}

/// Full benchmark portfolio report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalPortfolioReport {
    pub k: usize,
    pub case_count: usize,
    pub overall: RetrievalMetricSummary,
    pub mission: MissionLinkageSummary,
    pub cases: Vec<RetrievalCaseReport>,
    pub slices: Vec<RetrievalSliceReport>,
}

/// Evaluate a single benchmark case against a retrieval run.
pub fn evaluate_retrieval_case(
    case: &RetrievalBenchmarkCase,
    run: &RetrievalBenchmarkRun,
    k: usize,
) -> RetrievalCaseReport {
    let ordered = ordered_unique_documents(&run.retrieved_documents);
    let relevant = case
        .relevant_documents
        .iter()
        .map(|doc| (doc.document_id.as_str(), doc.relevance))
        .collect::<HashMap<_, _>>();

    let top_k = ordered.iter().take(k.max(1)).collect::<Vec<_>>();
    let hits = top_k
        .iter()
        .filter(|doc| relevant.contains_key(doc.document_id.as_str()))
        .count();
    let relevant_count = case.relevant_documents.len().max(1);
    let success_at_k = if hits > 0 { 1.0 } else { 0.0 };
    let recall_at_k = hits as f32 / relevant_count as f32;
    let mrr = ordered
        .iter()
        .find(|doc| relevant.contains_key(doc.document_id.as_str()))
        .map(|doc| 1.0 / doc.rank.max(1) as f32)
        .unwrap_or(0.0);
    let ndcg_at_k = normalized_dcg(&top_k, &case.relevant_documents, &relevant);
    let selected_evidence = run
        .selected_document_ids
        .iter()
        .any(|id| relevant.contains_key(id.as_str()));
    let observed_mission_count = usize::from(run.mission_success.is_some());
    let mission_success = run.mission_success.unwrap_or(false);

    RetrievalCaseReport {
        case_id: case.case_id.clone(),
        codebase_id: case.codebase_id.clone(),
        language: case.language.clone(),
        repo_size: case.repo_size.clone(),
        task_class: case.task_class.clone(),
        metrics: RetrievalMetricSummary {
            success_at_k,
            recall_at_k,
            mrr,
            ndcg_at_k,
        },
        mission: MissionLinkageSummary {
            selected_evidence_rate: if selected_evidence { 1.0 } else { 0.0 },
            mission_success_rate: if mission_success && observed_mission_count > 0 {
                1.0
            } else {
                0.0
            },
            mission_success_with_selected_evidence_rate: if mission_success
                && selected_evidence
                && observed_mission_count > 0
            {
                1.0
            } else {
                0.0
            },
            observed_mission_count,
        },
    }
}

/// Evaluate a benchmark portfolio and emit cross-codebase slice rollups.
pub fn evaluate_retrieval_portfolio(
    portfolio: &BenchmarkPortfolio,
    runs: &[RetrievalBenchmarkRun],
    k: usize,
) -> RetrievalPortfolioReport {
    let run_map = runs
        .iter()
        .map(|run| (run.case_id.as_str(), run))
        .collect::<HashMap<_, _>>();
    let cases = portfolio
        .cases
        .iter()
        .filter_map(|case| {
            run_map
                .get(case.case_id.as_str())
                .map(|run| evaluate_retrieval_case(case, run, k))
        })
        .collect::<Vec<_>>();

    RetrievalPortfolioReport {
        k,
        case_count: cases.len(),
        overall: aggregate_metrics(&cases),
        mission: aggregate_mission(&cases),
        slices: aggregate_slices(&cases),
        cases,
    }
}

fn ordered_unique_documents(documents: &[RetrievedDocumentRank]) -> Vec<RetrievedDocumentRank> {
    let mut ordered = documents.to_vec();
    ordered.sort_by_key(|doc| doc.rank);

    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for document in ordered {
        if seen.insert(document.document_id.clone()) {
            deduped.push(document);
        }
    }
    deduped
}

fn normalized_dcg(
    top_k: &[&RetrievedDocumentRank],
    relevant_documents: &[RelevantDocument],
    relevance: &HashMap<&str, f32>,
) -> f32 {
    let dcg = top_k
        .iter()
        .enumerate()
        .map(|(index, document)| {
            let gain = relevance
                .get(document.document_id.as_str())
                .copied()
                .unwrap_or(0.0);
            discounted_gain(gain, index)
        })
        .sum::<f32>();
    let mut ideal = relevant_documents.to_vec();
    ideal.sort_by(|left, right| {
        right
            .relevance
            .partial_cmp(&left.relevance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let idcg = ideal
        .iter()
        .take(top_k.len())
        .enumerate()
        .map(|(index, document)| discounted_gain(document.relevance, index))
        .sum::<f32>();

    if idcg > 0.0 {
        dcg / idcg
    } else {
        0.0
    }
}

fn discounted_gain(relevance: f32, zero_based_rank: usize) -> f32 {
    let denominator = (zero_based_rank as f32 + 2.0).log2();
    if denominator > 0.0 {
        relevance / denominator
    } else {
        0.0
    }
}

fn aggregate_metrics(cases: &[RetrievalCaseReport]) -> RetrievalMetricSummary {
    if cases.is_empty() {
        return RetrievalMetricSummary::default();
    }

    let count = cases.len() as f32;
    RetrievalMetricSummary {
        success_at_k: cases
            .iter()
            .map(|case| case.metrics.success_at_k)
            .sum::<f32>()
            / count,
        recall_at_k: cases
            .iter()
            .map(|case| case.metrics.recall_at_k)
            .sum::<f32>()
            / count,
        mrr: cases.iter().map(|case| case.metrics.mrr).sum::<f32>() / count,
        ndcg_at_k: cases.iter().map(|case| case.metrics.ndcg_at_k).sum::<f32>() / count,
    }
}

fn aggregate_mission(cases: &[RetrievalCaseReport]) -> MissionLinkageSummary {
    if cases.is_empty() {
        return MissionLinkageSummary::default();
    }

    let case_count = cases.len() as f32;
    let observed_mission_count: usize = cases
        .iter()
        .map(|case| case.mission.observed_mission_count)
        .sum();
    let mission_denominator = observed_mission_count.max(1) as f32;

    MissionLinkageSummary {
        selected_evidence_rate: cases
            .iter()
            .map(|case| case.mission.selected_evidence_rate)
            .sum::<f32>()
            / case_count,
        mission_success_rate: cases
            .iter()
            .map(|case| case.mission.mission_success_rate)
            .sum::<f32>()
            / mission_denominator,
        mission_success_with_selected_evidence_rate: cases
            .iter()
            .map(|case| case.mission.mission_success_with_selected_evidence_rate)
            .sum::<f32>()
            / mission_denominator,
        observed_mission_count,
    }
}

fn aggregate_slices(cases: &[RetrievalCaseReport]) -> Vec<RetrievalSliceReport> {
    let mut groups: BTreeMap<(String, String), Vec<&RetrievalCaseReport>> = BTreeMap::new();

    for case in cases {
        groups
            .entry(("language".to_string(), case.language.clone()))
            .or_default()
            .push(case);
        groups
            .entry(("repo_size".to_string(), case.repo_size.as_str().to_string()))
            .or_default()
            .push(case);
        groups
            .entry((
                "task_class".to_string(),
                case.task_class.as_str().to_string(),
            ))
            .or_default()
            .push(case);
        groups
            .entry(("codebase".to_string(), case.codebase_id.clone()))
            .or_default()
            .push(case);
    }

    groups
        .into_iter()
        .map(|((dimension, value), grouped)| {
            let grouped_cases = grouped.into_iter().cloned().collect::<Vec<_>>();
            let codebase_count = grouped_cases
                .iter()
                .map(|case| case.codebase_id.as_str())
                .collect::<HashSet<_>>()
                .len();
            RetrievalSliceReport {
                dimension,
                value,
                case_count: grouped_cases.len(),
                codebase_count,
                metrics: aggregate_metrics(&grouped_cases),
                mission: aggregate_mission(&grouped_cases),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrieval_case_metrics_capture_hits_and_mission_linkage() {
        let case = RetrievalBenchmarkCase {
            case_id: "case-1".to_string(),
            codebase_id: "repo-a".to_string(),
            language: "rust".to_string(),
            repo_size: RepoSizeBucket::Medium,
            task_class: RetrievalTaskClass::BugFix,
            query: "fix auth timeout".to_string(),
            relevant_documents: vec![
                RelevantDocument {
                    document_id: "src/auth.rs#handle".to_string(),
                    relevance: 2.0,
                },
                RelevantDocument {
                    document_id: "src/config.rs#timeouts".to_string(),
                    relevance: 1.0,
                },
            ],
        };
        let run = RetrievalBenchmarkRun {
            case_id: case.case_id.clone(),
            retrieved_documents: vec![
                RetrievedDocumentRank {
                    document_id: "src/readme.md".to_string(),
                    rank: 1,
                    score: 0.4,
                },
                RetrievedDocumentRank {
                    document_id: "src/auth.rs#handle".to_string(),
                    rank: 2,
                    score: 0.9,
                },
            ],
            selected_document_ids: vec!["src/auth.rs#handle".to_string()],
            mission_success: Some(true),
        };

        let report = evaluate_retrieval_case(&case, &run, 3);
        assert_eq!(report.metrics.success_at_k, 1.0);
        assert_eq!(report.metrics.recall_at_k, 0.5);
        assert_eq!(report.metrics.mrr, 0.5);
        assert!(report.metrics.ndcg_at_k > 0.0);
        assert_eq!(report.mission.selected_evidence_rate, 1.0);
        assert_eq!(report.mission.mission_success_rate, 1.0);
        assert_eq!(
            report.mission.mission_success_with_selected_evidence_rate,
            1.0
        );
    }

    #[test]
    fn portfolio_report_rolls_up_cross_codebase_slices() {
        let portfolio = BenchmarkPortfolio::v1(vec![
            RetrievalBenchmarkCase {
                case_id: "case-1".to_string(),
                codebase_id: "repo-a".to_string(),
                language: "rust".to_string(),
                repo_size: RepoSizeBucket::Small,
                task_class: RetrievalTaskClass::Navigation,
                query: "find auth handler".to_string(),
                relevant_documents: vec![RelevantDocument {
                    document_id: "src/auth.rs#handler".to_string(),
                    relevance: 1.0,
                }],
            },
            RetrievalBenchmarkCase {
                case_id: "case-2".to_string(),
                codebase_id: "repo-b".to_string(),
                language: "python".to_string(),
                repo_size: RepoSizeBucket::Large,
                task_class: RetrievalTaskClass::BugFix,
                query: "trace payment retry".to_string(),
                relevant_documents: vec![RelevantDocument {
                    document_id: "payments/retry.py#retry".to_string(),
                    relevance: 1.0,
                }],
            },
        ]);
        let runs = vec![
            RetrievalBenchmarkRun {
                case_id: "case-1".to_string(),
                retrieved_documents: vec![RetrievedDocumentRank {
                    document_id: "src/auth.rs#handler".to_string(),
                    rank: 1,
                    score: 1.0,
                }],
                selected_document_ids: vec!["src/auth.rs#handler".to_string()],
                mission_success: Some(true),
            },
            RetrievalBenchmarkRun {
                case_id: "case-2".to_string(),
                retrieved_documents: vec![RetrievedDocumentRank {
                    document_id: "other.py#helper".to_string(),
                    rank: 1,
                    score: 0.2,
                }],
                selected_document_ids: Vec::new(),
                mission_success: Some(false),
            },
        ];

        let report = evaluate_retrieval_portfolio(&portfolio, &runs, 5);
        assert_eq!(report.case_count, 2);
        assert_eq!(
            report
                .slices
                .iter()
                .filter(|slice| slice.dimension == "language")
                .count(),
            2
        );
        assert_eq!(
            report
                .slices
                .iter()
                .filter(|slice| slice.dimension == "codebase")
                .count(),
            2
        );
        assert!(report.overall.success_at_k > 0.0);
    }
}
