//! Closure evaluation for local workflow authority.

use std::collections::{HashMap, HashSet};

use thiserror::Error;
use uuid::Uuid;

use crate::mol_flags::MolFeatureFlags;

#[derive(Debug, Clone)]
pub struct ClosureSnapshot {
    pub gates: Vec<(String, String)>,
    pub required_requirement_ids: Vec<Uuid>,
    pub claims: Vec<(Uuid, String)>,
    pub verification_runs: Vec<(Uuid, String)>,
    pub findings: Vec<(Uuid, String)>,
    pub handoffs: Vec<(Uuid, String)>,
    pub acceptance_checks: Vec<(Uuid, String)>,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ClosureEngineError {
    #[error(
        "closure snapshot is empty; at least one authoritative closure fact is required before closing"
    )]
    EmptyClosureBasis,

    #[error("closure gate {gate_type} is not satisfied (status={status})")]
    GateNotSatisfied { gate_type: String, status: String },

    #[error(
        "completion claim missing or not accepted for requirement {requirement_id} (need recorded or accepted)"
    )]
    ClaimMissingOrPending { requirement_id: Uuid },

    #[error("verification run {run_id} is not complete (status={status})")]
    VerificationRunIncomplete { run_id: Uuid, status: String },

    #[error("verification finding {finding_id} is still open")]
    FindingOpen { finding_id: Uuid },

    #[error("handoff contract {contract_id} is not complete (status={status})")]
    HandoffIncomplete { contract_id: Uuid, status: String },

    #[error("acceptance check {check_id} is not satisfied (status={status})")]
    AcceptanceNotSatisfied { check_id: Uuid, status: String },
}

fn gate_ok(status: &str) -> bool {
    matches!(status, "passed" | "waived")
}

fn claim_ok(status: &str) -> bool {
    matches!(status, "recorded" | "accepted")
}

fn verification_run_ok(status: &str) -> bool {
    status == "passed"
}

fn finding_ok(status: &str) -> bool {
    status != "open"
}

fn handoff_terminal(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "passed" | "accepted" | "completed" | "fulfilled" | "closed" | "waived"
    )
}

fn acceptance_ok(status: &str) -> bool {
    matches!(status, "passed" | "waived" | "skipped")
}

pub fn evaluate_closure(snapshot: &ClosureSnapshot) -> Result<(), ClosureEngineError> {
    let mol = MolFeatureFlags {
        closure_allow_open_findings: false,
        ..Default::default()
    };
    evaluate_closure_with_mol(snapshot, mol, true)
}

pub fn evaluate_closure_with_mol(
    snapshot: &ClosureSnapshot,
    mol: MolFeatureFlags,
    verification_required: bool,
) -> Result<(), ClosureEngineError> {
    if snapshot.gates.is_empty()
        && snapshot.required_requirement_ids.is_empty()
        && snapshot.claims.is_empty()
        && snapshot.verification_runs.is_empty()
        && snapshot.findings.is_empty()
        && snapshot.handoffs.is_empty()
        && snapshot.acceptance_checks.is_empty()
    {
        return Err(ClosureEngineError::EmptyClosureBasis);
    }

    for (gate_type, status) in &snapshot.gates {
        if !gate_ok(status) {
            return Err(ClosureEngineError::GateNotSatisfied {
                gate_type: gate_type.clone(),
                status: status.clone(),
            });
        }
    }

    let mut best: HashMap<Uuid, String> = HashMap::new();
    for (req_id, st) in &snapshot.claims {
        best.entry(*req_id)
            .and_modify(|cur| {
                if claim_rank(st.as_str()) > claim_rank(cur.as_str()) {
                    *cur = st.clone();
                }
            })
            .or_insert_with(|| st.clone());
    }

    for req_id in &snapshot.required_requirement_ids {
        let st = best.get(req_id).map(String::as_str).unwrap_or("");
        if !claim_ok(st) {
            return Err(ClosureEngineError::ClaimMissingOrPending {
                requirement_id: *req_id,
            });
        }
    }

    if verification_required {
        for (run_id, status) in &snapshot.verification_runs {
            if !verification_run_ok(status) {
                return Err(ClosureEngineError::VerificationRunIncomplete {
                    run_id: *run_id,
                    status: status.clone(),
                });
            }
        }
    }

    if !mol.closure_allow_open_findings {
        for (finding_id, status) in &snapshot.findings {
            if !finding_ok(status) {
                return Err(ClosureEngineError::FindingOpen {
                    finding_id: *finding_id,
                });
            }
        }
    }

    for (contract_id, status) in &snapshot.handoffs {
        if !handoff_terminal(status) {
            return Err(ClosureEngineError::HandoffIncomplete {
                contract_id: *contract_id,
                status: status.clone(),
            });
        }
    }

    for (check_id, status) in &snapshot.acceptance_checks {
        if !acceptance_ok(status) {
            return Err(ClosureEngineError::AcceptanceNotSatisfied {
                check_id: *check_id,
                status: status.clone(),
            });
        }
    }

    Ok(())
}

fn claim_rank(status: &str) -> u8 {
    match status {
        "accepted" => 3,
        "recorded" => 2,
        "pending" => 1,
        "rejected" => 0,
        _ => 1,
    }
}

pub fn dedup_ids(ids: Vec<Uuid>) -> Vec<Uuid> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for id in ids {
        if seen.insert(id) {
            out.push(id);
        }
    }
    out
}
