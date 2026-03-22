//! Exhaustive SDET suite: validation, scoring hygiene, decision matrix, serde rigidity.

use serde::de::DeserializeOwned;

use crate::engine::{
    calculate_quality_score, evaluate_artifact, evaluate_full, ACCEPT_CONFIDENCE_THRESHOLD,
    CONFIDENCE_REJECT_BELOW, QUALITY_HARD_REJECT_BELOW, WEIGHT_SUM,
};
use crate::evaluate_verdict;
use crate::types::{
    Artifact, ArtifactEvaluationMatrix, ArtifactType, ContextSufficiency, ContradictionType,
    Decision, DimensionScores, EvaluationContext, EvaluationType, EvaluatorInput, IssueCounts,
    RiskLevel, UserIntent,
};
use crate::validate_evaluator_input;
use crate::EvaluatorInputError;

fn dimensions_uniform(level: f64) -> DimensionScores {
    DimensionScores {
        correctness: level,
        security: level,
        best_practices: level,
        performance: level,
        maintainability: level,
        completeness: level,
    }
}

fn matrix(
    dim_level: f64,
    confidence: f64,
    risk: RiskLevel,
    issues: IssueCounts,
    contradictions: usize,
    sufficiency: ContextSufficiency,
) -> ArtifactEvaluationMatrix {
    ArtifactEvaluationMatrix::new(
        dimensions_uniform(dim_level),
        confidence,
        risk,
        issues,
        contradictions,
        sufficiency,
    )
}

fn minimal_input(id: &str, content: &str, intent: &str) -> EvaluatorInput {
    EvaluatorInput {
        evaluation_id: id.to_string(),
        evaluation_type: EvaluationType::CodeReview,
        artifact: Artifact {
            artifact_type: ArtifactType::Code,
            content: content.to_string(),
            language: None,
            file_path: None,
            checksum: None,
        },
        context: EvaluationContext {
            user_intent: UserIntent {
                description: intent.to_string(),
                constraints: vec![],
                acceptance_criteria: vec![],
            },
            context_sufficiency: ContextSufficiency::Sufficient,
        },
    }
}

// --- EVAL-001 / 003 / 004 & try_new ---

#[test]
fn eval001_accepts_exact_pattern() {
    let ok = minimal_input("eval_0123456789abcdef", "fn main() {}", "intent");
    assert!(validate_evaluator_input(&ok).is_ok());
    assert!(EvaluatorInput::try_new(
        ok.evaluation_id.clone(),
        ok.evaluation_type,
        ok.artifact.clone(),
        ok.context.clone(),
    )
    .is_ok());
}

#[test]
fn eval001_rejects_wrong_prefix_length_charset() {
    for bad in [
        "eval_0123456789abcde",
        "eval_0123456789abcdef0",
        "eval_0123456789ABCD",
        "xeval_0123456789abcdef",
        "eval-0123456789abcdef",
        "'; DROP TABLE eval; --",
        "eval_\u{03b1}123456789abcdef",
    ] {
        let input = minimal_input(bad, "x", "i");
        assert_eq!(
            validate_evaluator_input(&input),
            Err(EvaluatorInputError::InvalidEvaluationId),
            "expected reject for id={bad:?}"
        );
        let try_res = EvaluatorInput::try_new(
            bad.to_string(),
            EvaluationType::CodeReview,
            input.artifact.clone(),
            input.context.clone(),
        );
        assert!(matches!(
            try_res,
            Err(EvaluatorInputError::InvalidEvaluationId)
        ));
    }
}

#[test]
fn eval003_empty_or_whitespace_only_artifact() {
    for content in ["", "   ", "\t\n", "\u{00a0}\u{2003}"] {
        let input = minimal_input("eval_0123456789abcdef", content, "ok");
        assert_eq!(
            validate_evaluator_input(&input),
            Err(EvaluatorInputError::EmptyArtifactContent)
        );
    }
}

#[test]
fn eval003_injection_like_content_passes_if_non_empty() {
    let payload = "'; DROP TABLE artifacts; --\nSELECT 1";
    let ok = minimal_input("eval_0123456789abcdef", payload, "intent");
    assert!(validate_evaluator_input(&ok).is_ok());
}

#[test]
fn eval004_empty_intent_description() {
    let input = minimal_input("eval_0123456789abcdef", "code", "  \n\t");
    assert_eq!(
        validate_evaluator_input(&input),
        Err(EvaluatorInputError::EmptyUserIntentDescription)
    );
}

#[test]
fn try_new_propagates_first_validation_failure() {
    let r = EvaluatorInput::try_new(
        "bad_id".to_string(),
        EvaluationType::CodeReview,
        Artifact {
            artifact_type: ArtifactType::Code,
            content: "x".to_string(),
            language: None,
            file_path: None,
            checksum: None,
        },
        EvaluationContext {
            user_intent: UserIntent {
                description: "ok".to_string(),
                constraints: vec![],
                acceptance_criteria: vec![],
            },
            context_sufficiency: ContextSufficiency::Sufficient,
        },
    );
    assert!(matches!(
        r,
        Err(EvaluatorInputError::InvalidEvaluationId)
    ));
}

// --- Floating-point scoring ---

#[test]
fn weight_sum_is_unity_within_fp_epsilon() {
    assert!(
        (WEIGHT_SUM - 1.0).abs() < 1e-12,
        "WEIGHT_SUM must be 1.0, got {WEIGHT_SUM:?}"
    );
}

#[test]
fn calculate_quality_score_never_panics_on_non_finite_inputs() {
    let nan = DimensionScores {
        correctness: f64::NAN,
        security: f64::INFINITY,
        best_practices: f64::NEG_INFINITY,
        performance: f64::NAN,
        maintainability: 0.5,
        completeness: -1.0,
    };
    let s = calculate_quality_score(&nan);
    assert!(s.is_finite());
    assert!((0.0..=1.0).contains(&s));
}

#[test]
fn calculate_quality_score_deterministic_bits() {
    let d = DimensionScores {
        correctness: 0.1,
        security: 0.2,
        best_practices: 0.3,
        performance: 0.4,
        maintainability: 0.5,
        completeness: 0.6,
    };
    let a = calculate_quality_score(&d);
    let b = calculate_quality_score(&d);
    assert_eq!(a.to_bits(), b.to_bits());
}

#[test]
fn calculate_quality_score_all_ones_and_all_zeros() {
    let ones = DimensionScores {
        correctness: 1.0,
        security: 1.0,
        best_practices: 1.0,
        performance: 1.0,
        maintainability: 1.0,
        completeness: 1.0,
    };
    assert_eq!(calculate_quality_score(&ones).to_bits(), 1.0f64.to_bits());

    let z = DimensionScores {
        correctness: 0.0,
        security: 0.0,
        best_practices: 0.0,
        performance: 0.0,
        maintainability: 0.0,
        completeness: 0.0,
    };
    assert_eq!(calculate_quality_score(&z).to_bits(), 0.0f64.to_bits());
}

// --- Decision matrix ---

#[test]
fn matrix_request_info_takes_precedence_over_reject_signals() {
    let m = matrix(
        0.9,
        0.0,
        RiskLevel::Critical,
        IssueCounts {
            critical: 99,
            high: 99,
            medium: 99,
            low: 0,
        },
        99,
        ContextSufficiency::Insufficient,
    );
    assert_eq!(evaluate_full(&m), Decision::RequestInfo);
}

#[test]
fn matrix_reject_invalid_confidence_nan_or_out_of_range() {
    let base = IssueCounts::default();
    let m_nan = matrix(0.9, f64::NAN, RiskLevel::Low, base, 0, ContextSufficiency::Sufficient);
    assert_eq!(evaluate_full(&m_nan), Decision::Reject);

    let m_inf = matrix(0.9, f64::INFINITY, RiskLevel::Low, base, 0, ContextSufficiency::Sufficient);
    assert_eq!(evaluate_full(&m_inf), Decision::Reject);

    let m_high = matrix(0.9, 1.01, RiskLevel::Low, base, 0, ContextSufficiency::Sufficient);
    assert_eq!(evaluate_full(&m_high), Decision::Reject);

    let m_neg = matrix(0.9, -0.01, RiskLevel::Low, base, 0, ContextSufficiency::Sufficient);
    assert_eq!(evaluate_full(&m_neg), Decision::Reject);
}

#[test]
fn matrix_confidence_boundary_reject_below() {
    let just_below = CONFIDENCE_REJECT_BELOW - f64::EPSILON;
    let m = matrix(
        0.9,
        just_below,
        RiskLevel::Low,
        IssueCounts::default(),
        0,
        ContextSufficiency::Sufficient,
    );
    assert_eq!(evaluate_full(&m), Decision::Reject);

    let at = CONFIDENCE_REJECT_BELOW;
    let m = matrix(
        0.9,
        at,
        RiskLevel::Low,
        IssueCounts::default(),
        0,
        ContextSufficiency::Sufficient,
    );
    assert_ne!(evaluate_full(&m), Decision::Reject);
}

#[test]
fn matrix_confidence_boundary_accept_threshold() {
    let below = ACCEPT_CONFIDENCE_THRESHOLD - f64::EPSILON;
    let m = matrix(
        0.9,
        below,
        RiskLevel::Low,
        IssueCounts::default(),
        0,
        ContextSufficiency::Sufficient,
    );
    assert_eq!(evaluate_full(&m), Decision::Flag);

    let at = ACCEPT_CONFIDENCE_THRESHOLD;
    let m = matrix(
        0.9,
        at,
        RiskLevel::Low,
        IssueCounts::default(),
        0,
        ContextSufficiency::Sufficient,
    );
    assert_eq!(evaluate_full(&m), Decision::Accept);
}

#[test]
fn matrix_critical_risk_rejects_even_if_zero_critical_issues() {
    let m = matrix(
        0.9,
        1.0,
        RiskLevel::Critical,
        IssueCounts::default(),
        0,
        ContextSufficiency::Sufficient,
    );
    assert_eq!(evaluate_full(&m), Decision::Reject);
}

#[test]
fn matrix_exhaustive_four_way_partition() {
    let cases: Vec<(ArtifactEvaluationMatrix, Decision)> = vec![
        (
            matrix(
                0.9,
                0.9,
                RiskLevel::Low,
                IssueCounts::default(),
                0,
                ContextSufficiency::Ambiguous,
            ),
            Decision::RequestInfo,
        ),
        (
            matrix(
                0.9,
                f64::NAN,
                RiskLevel::Low,
                IssueCounts::default(),
                0,
                ContextSufficiency::Sufficient,
            ),
            Decision::Reject,
        ),
        (
            matrix(
                0.9,
                0.9,
                RiskLevel::Low,
                IssueCounts {
                    critical: 1,
                    high: 0,
                    medium: 0,
                    low: 0,
                },
                0,
                ContextSufficiency::Sufficient,
            ),
            Decision::Reject,
        ),
        (
            matrix(
                0.9,
                0.9,
                RiskLevel::High,
                IssueCounts::default(),
                0,
                ContextSufficiency::Sufficient,
            ),
            Decision::Flag,
        ),
        (
            matrix(
                0.9,
                ACCEPT_CONFIDENCE_THRESHOLD,
                RiskLevel::Medium,
                IssueCounts {
                    critical: 0,
                    high: 2,
                    medium: 5,
                    low: 0,
                },
                0,
                ContextSufficiency::Sufficient,
            ),
            Decision::Accept,
        ),
        (
            matrix(
                0.9,
                0.65,
                RiskLevel::Low,
                IssueCounts::default(),
                0,
                ContextSufficiency::Sufficient,
            ),
            Decision::Flag,
        ),
    ];

    for (m, expected) in cases {
        let got = evaluate_full(&m);
        assert_eq!(
            got, expected,
            "matrix mismatch: confidence={} risk={:?} issues={:?}",
            m.confidence, m.risk_level, m.issues
        );
    }
}

#[test]
fn evaluate_artifact_matches_full_with_defaults() {
    let issues = IssueCounts {
        critical: 0,
        high: 3,
        medium: 0,
        low: 0,
    };
    let dims = dimensions_uniform(0.5);
    let a = evaluate_artifact(&dims, &issues, RiskLevel::Low, 0.95);
    let b = evaluate_full(&ArtifactEvaluationMatrix::new(
        dims,
        0.95,
        RiskLevel::Low,
        issues,
        0,
        ContextSufficiency::Sufficient,
    ));
    assert_eq!(a, b);
}

#[test]
fn matrix_rejects_low_aggregate_quality() {
    let m = matrix(
        0.2,
        0.99,
        RiskLevel::Low,
        IssueCounts::default(),
        0,
        ContextSufficiency::Sufficient,
    );
    assert!(
        calculate_quality_score(&m.dimensions) < QUALITY_HARD_REJECT_BELOW,
        "fixture should be below hard reject"
    );
    assert_eq!(evaluate_full(&m), Decision::Reject);
}

#[test]
fn evaluate_verdict_aligns_with_decision() {
    let m = matrix(
        0.9,
        0.9,
        RiskLevel::Low,
        IssueCounts::default(),
        0,
        ContextSufficiency::Sufficient,
    );
    let out = evaluate_verdict(&m, vec![]);
    assert_eq!(out.decision(), evaluate_full(&m));
    assert_eq!(out.confidence(), m.confidence);
}

// --- Serde rigidity ---

fn assert_rejects_unknown_enum<T: DeserializeOwned>(json: &str) {
    let r: Result<T, _> = serde_json::from_str(json);
    assert!(r.is_err(), "expected deserialize error for {json}");
}

#[test]
fn serde_rejects_unknown_evaluation_type() {
    let j = r#"{
        "evaluation_id": "eval_0123456789abcdef",
        "evaluation_type": "unknown_review",
        "artifact": { "artifact_type": "code", "content": "x" },
        "context": { "user_intent": { "description": "d" } }
    }"#;
    assert_rejects_unknown_enum::<EvaluatorInput>(j);
}

#[test]
fn serde_rejects_unknown_decision() {
    assert_rejects_unknown_enum::<Decision>(r#""MAYBE""#);
}

#[test]
fn serde_rejects_unknown_risk_level() {
    assert_rejects_unknown_enum::<RiskLevel>(r#""extreme""#);
}

#[test]
fn serde_rejects_unknown_contradiction_type() {
    assert_rejects_unknown_enum::<ContradictionType>(r#""speculative""#);
}

#[test]
fn serde_rejects_unknown_field_on_evaluator_input() {
    let j = r#"{
        "evaluation_id": "eval_0123456789abcdef",
        "evaluation_type": "code_review",
        "artifact": { "artifact_type": "code", "content": "x" },
        "context": { "user_intent": { "description": "d" } },
        "evil": true
    }"#;
    assert_rejects_unknown_enum::<EvaluatorInput>(j);
}

