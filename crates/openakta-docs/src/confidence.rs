//! Hybrid confidence scoring for drift reports (Plan 5).
//!
//! Produces a [`ConfidenceReconcileDecision`] from a [`DriftReport`](crate::drift::DriftReport)
//! using deterministic heuristics only (no LLM self-confidence).
//!
//! Auto-apply thresholds are **aligned** with [`openakta_evaluator::ACCEPT_CONFIDENCE_THRESHOLD`]:
//! the daemon SQLite job queue will not emit auto-changelog/git paths unless the scored
//! confidence meets the same lower bound as the Evaluator Agent accept gate.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use openakta_evaluator::ACCEPT_CONFIDENCE_THRESHOLD;
use serde::{Deserialize, Serialize};

use crate::drift::{DriftDomain, DriftKind, DriftReport, DriftSeverity, InconsistencyFlag};

/// Risk tier derived from documentation path conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentRiskProfile {
    Low,
    Medium,
    High,
}

/// Reconciliation outcome for drift-based documentation updates (confidence layer).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceReconcileDecision {
    /// No durable doc mutation; drift absent or deemed ignorable under policy.
    Noop { reason: String },
    /// Safe to apply deterministic changelog append plus optional index-only git commit.
    UpdateRequired {
        score: f64,
        /// Per-doc targets; caller may split into multiple commits.
        target_docs: Vec<std::path::PathBuf>,
    },
    /// Insufficient confidence or policy forbids auto apply.
    ReviewRequired { score: f64, report_summary: String },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfidenceBreakdown {
    pub base: f64,
    pub after_severity: f64,
    pub after_domain: f64,
    pub after_kind_penalties: f64,
    pub after_ambiguity: f64,
    pub after_risk_profile: f64,
    pub penalties: Vec<String>,
}

/// Tunable thresholds for [`ConfidenceScorer`].
#[derive(Debug, Clone)]
pub struct ConfidenceScorerConfig {
    pub auto_update_min: f64,
    pub review_below: f64,
    /// Max raw score after risk ceiling for HighRisk docs.
    pub high_risk_auto_ceiling: f64,
    /// Minimum score to auto-apply when [`DocumentRiskProfile`] is [`High`](DocumentRiskProfile::High).
    pub high_risk_auto_min: f64,
    pub business_rule_penalty: f64,
    pub critical_severity_penalty: f64,
    /// Penalty when multiple distinct fingerprints reference the same doc path.
    pub ambiguity_penalty_per_extra_fingerprint: f64,
    pub rule_ids_penalty: f64,
}

impl Default for ConfidenceScorerConfig {
    fn default() -> Self {
        Self {
            auto_update_min: ACCEPT_CONFIDENCE_THRESHOLD,
            review_below: 0.55,
            high_risk_auto_ceiling: 0.91,
            high_risk_auto_min: f64::max(0.92, ACCEPT_CONFIDENCE_THRESHOLD),
            business_rule_penalty: 0.18,
            critical_severity_penalty: 0.12,
            ambiguity_penalty_per_extra_fingerprint: 0.04,
            rule_ids_penalty: 0.03,
        }
    }
}

/// Deterministic drift confidence evaluation.
pub struct ConfidenceScorer {
    cfg: ConfidenceScorerConfig,
}

impl ConfidenceScorer {
    pub fn new(cfg: ConfidenceScorerConfig) -> Self {
        Self { cfg }
    }

    pub fn with_defaults() -> Self {
        Self::new(ConfidenceScorerConfig::default())
    }

    /// Map repository-relative doc paths to risk tier.
    pub fn risk_for_doc_path(&self, doc_path: &Path) -> DocumentRiskProfile {
        let s = doc_path.to_string_lossy();
        if s.contains("03-business-logic") {
            DocumentRiskProfile::High
        } else if s.contains("06-technical") {
            DocumentRiskProfile::Medium
        } else {
            DocumentRiskProfile::Low
        }
    }

    fn ambiguity_penalty(flags: &[InconsistencyFlag], cfg: &ConfidenceScorerConfig) -> f64 {
        let mut by_doc: HashMap<String, HashSet<&str>> = HashMap::new();
        for flag in flags {
            let key = flag.doc_path.display().to_string();
            by_doc
                .entry(key)
                .or_default()
                .insert(flag.fingerprint.as_str());
        }
        let mut pen = 0.0;
        for fps in by_doc.values() {
            if fps.len() > 1 {
                pen += (fps.len() - 1) as f64 * cfg.ambiguity_penalty_per_extra_fingerprint;
            }
        }
        pen
    }

    fn score_flags(
        &self,
        report: &DriftReport,
        risk: DocumentRiskProfile,
    ) -> (f64, ConfidenceBreakdown) {
        let mut b = ConfidenceBreakdown::default();
        let mut penalties = Vec::new();

        if report.total_flags == 0 {
            b.base = 1.0;
            return (1.0, b);
        }

        let crit = report.critical_flags as f64;
        let warn = report.warning_flags as f64;
        let info = report.info_flags as f64;
        let mass = crit * 1.0 + warn * 0.55 + info * 0.2;
        let mut s = (1.0 / (1.0 + mass * 0.35)).min(1.0).max(0.0);
        b.after_severity = s;

        if report.business_rule_flags > 0 {
            s -= self.cfg.business_rule_penalty;
            penalties.push("business_rule_domain".into());
        }
        b.after_domain = s;

        for flag in &report.flags {
            let pen = match flag.kind {
                DriftKind::SignatureMismatch | DriftKind::StructuralDrift => 0.08,
                DriftKind::MissingSymbol | DriftKind::MissingRuleBinding => 0.10,
                DriftKind::DeadCodeReference => 0.04,
            };
            s -= pen;
            if matches!(flag.severity, DriftSeverity::Critical) {
                s -= self.cfg.critical_severity_penalty;
            }
            if matches!(flag.domain, DriftDomain::BusinessRule) {
                s -= 0.05;
            }
            if !flag.rule_ids.is_empty() {
                s -= self.cfg.rule_ids_penalty;
            }
        }
        s = s.max(0.0);
        b.after_kind_penalties = s;

        let amb = Self::ambiguity_penalty(&report.flags, &self.cfg);
        s = (s - amb).max(0.0);
        b.after_ambiguity = s;
        if amb > 0.0 {
            penalties.push("multi_fingerprint_per_doc".into());
        }

        s = match risk {
            DocumentRiskProfile::Low => s,
            DocumentRiskProfile::Medium => s * 0.95,
            DocumentRiskProfile::High => s.min(self.cfg.high_risk_auto_ceiling),
        };
        b.after_risk_profile = s;
        b.penalties = penalties;
        (s, b)
    }

    /// Policy: drift kinds that may be auto-reconciled via changelog-only updates.
    fn auto_allowed_kinds(report: &DriftReport) -> bool {
        report.flags.iter().all(|f| {
            matches!(
                f.kind,
                DriftKind::DeadCodeReference | DriftKind::MissingRuleBinding
            ) || (matches!(f.kind, DriftKind::SignatureMismatch)
                && matches!(f.severity, DriftSeverity::Info | DriftSeverity::Warning))
        })
    }

    /// Primary doc path for routing: first flag's doc, or `fallback`.
    pub fn primary_doc_path<'a>(&self, report: &'a DriftReport, fallback: &'a Path) -> &'a Path {
        report
            .flags
            .first()
            .map(|f| f.doc_path.as_path())
            .unwrap_or(fallback)
    }

    pub fn decide(
        &self,
        report: &DriftReport,
        primary_doc: &Path,
    ) -> (ConfidenceReconcileDecision, ConfidenceBreakdown) {
        let risk = self.risk_for_doc_path(primary_doc);
        let (score, breakdown) = self.score_flags(report, risk);

        if report.total_flags == 0 {
            return (
                ConfidenceReconcileDecision::Noop {
                    reason: "no_drift".into(),
                },
                breakdown,
            );
        }

        if score < self.cfg.review_below || !Self::auto_allowed_kinds(report) {
            return (
                ConfidenceReconcileDecision::ReviewRequired {
                    score,
                    report_summary: format!(
                        "flags={} highest={:?}",
                        report.total_flags, report.highest_severity
                    ),
                },
                breakdown,
            );
        }

        if score >= self.cfg.auto_update_min {
            if matches!(risk, DocumentRiskProfile::High) && score < self.cfg.high_risk_auto_min {
                return (
                    ConfidenceReconcileDecision::ReviewRequired {
                        score,
                        report_summary: "below_high_risk_auto_min".into(),
                    },
                    breakdown,
                );
            }
            return (
                ConfidenceReconcileDecision::UpdateRequired {
                    score,
                    target_docs: vec![primary_doc.to_path_buf()],
                },
                breakdown,
            );
        }

        (
            ConfidenceReconcileDecision::ReviewRequired {
                score,
                report_summary: "below_auto_threshold".into(),
            },
            breakdown,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sample_flag(
        kind: DriftKind,
        severity: DriftSeverity,
        domain: DriftDomain,
        doc: PathBuf,
        fingerprint: &str,
    ) -> InconsistencyFlag {
        InconsistencyFlag {
            domain,
            kind,
            severity,
            message: "test".into(),
            doc_path: doc,
            code_path: None,
            symbol_name: None,
            rule_ids: vec![],
            fingerprint: fingerprint.into(),
        }
    }

    #[test]
    fn noop_when_no_flags() {
        let s = ConfidenceScorer::with_defaults();
        let report = DriftReport::default();
        let (d, _) = s.decide(&report, Path::new("akta-docs/x.md"));
        assert!(matches!(d, ConfidenceReconcileDecision::Noop { .. }));
    }

    #[test]
    fn review_when_critical_missing_symbol() {
        let s = ConfidenceScorer::with_defaults();
        let doc = PathBuf::from("akta-docs/06-technical/api.md");
        let report = DriftReport {
            total_flags: 1,
            api_surface_flags: 1,
            business_rule_flags: 0,
            code_reference_flags: 0,
            critical_flags: 1,
            warning_flags: 0,
            info_flags: 0,
            highest_severity: Some(DriftSeverity::Critical),
            flags: vec![sample_flag(
                DriftKind::MissingSymbol,
                DriftSeverity::Critical,
                DriftDomain::ApiSurface,
                doc,
                "fp1",
            )],
        };
        let (d, _) = s.decide(
            &report,
            s.primary_doc_path(&report, Path::new("akta-docs/fallback.md")),
        );
        assert!(matches!(
            d,
            ConfidenceReconcileDecision::ReviewRequired { .. }
        ));
    }

    #[test]
    fn dead_code_reference_can_route_to_update_when_mild() {
        let s = ConfidenceScorer::with_defaults();
        let doc = PathBuf::from("akta-docs/06-technical/x.md");
        let report = DriftReport {
            total_flags: 1,
            api_surface_flags: 0,
            business_rule_flags: 0,
            code_reference_flags: 1,
            critical_flags: 0,
            warning_flags: 0,
            info_flags: 1,
            highest_severity: Some(DriftSeverity::Info),
            flags: vec![sample_flag(
                DriftKind::DeadCodeReference,
                DriftSeverity::Info,
                DriftDomain::CodeReference,
                doc,
                "fp1",
            )],
        };
        let (d, _) = s.decide(
            &report,
            s.primary_doc_path(&report, Path::new("akta-docs/fallback.md")),
        );
        assert!(matches!(
            d,
            ConfidenceReconcileDecision::UpdateRequired { .. }
        ));
    }

    #[test]
    fn two_fingerprints_same_doc_increases_ambiguity_and_can_force_review() {
        let mut cfg = ConfidenceScorerConfig::default();
        // Keep this scenario strict: ambiguity must not auto-apply even when the global floor is 0.7.
        cfg.auto_update_min = 0.85;
        let s = ConfidenceScorer::new(cfg);
        let doc = PathBuf::from("akta-docs/06-technical/x.md");
        let report = DriftReport {
            total_flags: 2,
            api_surface_flags: 0,
            business_rule_flags: 0,
            code_reference_flags: 2,
            critical_flags: 0,
            warning_flags: 0,
            info_flags: 2,
            highest_severity: Some(DriftSeverity::Info),
            flags: vec![
                sample_flag(
                    DriftKind::DeadCodeReference,
                    DriftSeverity::Info,
                    DriftDomain::CodeReference,
                    doc.clone(),
                    "fp-a",
                ),
                sample_flag(
                    DriftKind::DeadCodeReference,
                    DriftSeverity::Info,
                    DriftDomain::CodeReference,
                    doc,
                    "fp-b",
                ),
            ],
        };
        let (_, b) = s.decide(&report, Path::new("akta-docs/06-technical/x.md"));
        assert!(b
            .penalties
            .contains(&"multi_fingerprint_per_doc".to_string()));
        let (d, _) = s.decide(&report, Path::new("akta-docs/06-technical/x.md"));
        assert!(matches!(
            d,
            ConfidenceReconcileDecision::ReviewRequired { .. }
        ));
    }
}
