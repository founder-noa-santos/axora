//! Map `work.v1` protobuf messages to [`crate::work_management`] view types.
#![allow(clippy::result_large_err)] // `ApiError` carries large `tonic::Status`; matches crate patterns

use chrono::{DateTime, Utc};
use openakta_proto::work::v1::{
    AcceptanceCheck, ClosureClaim, ClosureGate, ClosureReport, GetClosureReportResponse,
    GetRequirementGraphResponse, HandoffContract, ListPersonaAssignmentsResponse, Persona,
    PersonaAssignment, Requirement, RequirementCoverage, RequirementEdge, VerificationFinding,
};

use crate::error::{ApiError, Result};
use crate::work_management::{
    AcceptanceCheckView, ClosureClaimView, ClosureGateView, ClosureReportView, HandoffContractView,
    PersonaAssignmentView, PersonaAssignmentsListView, PersonaView, RequirementCoverageView,
    RequirementEdgeView, RequirementGraphView, RequirementView, VerificationFindingView,
};

fn parse_uuid(raw: &str, field: &str) -> Result<uuid::Uuid> {
    if raw.is_empty() {
        return Err(ApiError::Internal(format!("missing UUID for {field}")));
    }
    uuid::Uuid::parse_str(raw).map_err(|e| ApiError::Internal(format!("{field}: {e}")))
}

fn parse_opt_uuid(raw: &str) -> Result<Option<uuid::Uuid>> {
    if raw.is_empty() {
        return Ok(None);
    }
    Ok(Some(uuid::Uuid::parse_str(raw).map_err(|e| {
        ApiError::Internal(format!("optional UUID: {e}"))
    })?))
}

fn parse_rfc3339(raw: &str, field: &str) -> Result<DateTime<Utc>> {
    if raw.is_empty() {
        return Err(ApiError::Internal(format!("missing timestamp for {field}")));
    }
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| ApiError::Internal(format!("{field}: {e}")))
}

fn parse_json_opt(raw: &str) -> Result<Option<serde_json::Value>> {
    if raw.trim().is_empty() {
        return Ok(None);
    }
    serde_json::from_str(raw)
        .map(Some)
        .map_err(|e| ApiError::Internal(format!("invalid JSON payload: {e}")))
}

pub(crate) fn requirement_graph_from_response(
    resp: GetRequirementGraphResponse,
) -> Result<RequirementGraphView> {
    let mut requirements = Vec::with_capacity(resp.requirements.len());
    for r in resp.requirements {
        requirements.push(requirement_from_proto(r)?);
    }
    let mut requirement_edges = Vec::with_capacity(resp.edges.len());
    for e in resp.edges {
        requirement_edges.push(requirement_edge_from_proto(e)?);
    }
    let mut acceptance_checks = Vec::with_capacity(resp.acceptance_checks.len());
    for c in resp.acceptance_checks {
        acceptance_checks.push(acceptance_check_from_proto(c)?);
    }
    let mut requirement_coverage = Vec::with_capacity(resp.coverage.len());
    for c in resp.coverage {
        requirement_coverage.push(requirement_coverage_from_proto(c)?);
    }
    let mut handoff_contracts = Vec::with_capacity(resp.handoff_contracts.len());
    for h in resp.handoff_contracts {
        handoff_contracts.push(handoff_contract_from_proto(h)?);
    }
    Ok(RequirementGraphView {
        requirements,
        requirement_edges,
        acceptance_checks,
        requirement_coverage,
        handoff_contracts,
    })
}

pub(crate) fn closure_report_from_response(
    resp: GetClosureReportResponse,
) -> Result<ClosureReportView> {
    let report = resp
        .report
        .ok_or_else(|| ApiError::Internal("GetClosureReport: empty report".to_string()))?;
    closure_report_from_proto(report)
}

fn closure_report_from_proto(report: ClosureReport) -> Result<ClosureReportView> {
    let workspace_id = parse_uuid(&report.workspace_id, "closure_report.workspace_id")?;
    let story_id = parse_opt_uuid(&report.story_id)?;
    let prepared_story_id = parse_opt_uuid(&report.prepared_story_id)?;

    let mut requirements = Vec::with_capacity(report.requirements.len());
    for r in report.requirements {
        requirements.push(requirement_from_proto(r)?);
    }
    let mut closure_claims = Vec::with_capacity(report.closure_claims.len());
    for c in report.closure_claims {
        closure_claims.push(closure_claim_from_proto(c)?);
    }
    let mut closure_gates = Vec::with_capacity(report.closure_gates.len());
    for g in report.closure_gates {
        closure_gates.push(closure_gate_from_proto(g)?);
    }
    let mut verification_findings = Vec::with_capacity(report.verification_findings.len());
    for f in report.verification_findings {
        verification_findings.push(verification_finding_from_proto(f)?);
    }

    Ok(ClosureReportView {
        workspace_id,
        story_id,
        prepared_story_id,
        requirements,
        closure_claims,
        closure_gates,
        verification_findings,
    })
}

pub(crate) fn persona_assignments_from_response(
    resp: ListPersonaAssignmentsResponse,
) -> Result<PersonaAssignmentsListView> {
    let mut personas = Vec::with_capacity(resp.items.len());
    for p in resp.items {
        personas.push(persona_from_proto(p)?);
    }
    let mut assignments = Vec::with_capacity(resp.assignments.len());
    for a in resp.assignments {
        assignments.push(persona_assignment_from_proto(a)?);
    }
    Ok(PersonaAssignmentsListView {
        personas,
        assignments,
    })
}

fn requirement_from_proto(r: Requirement) -> Result<RequirementView> {
    Ok(RequirementView {
        id: parse_uuid(&r.id, "requirement.id")?,
        workspace_id: parse_uuid(&r.workspace_id, "requirement.workspace_id")?,
        story_id: parse_opt_uuid(&r.story_id)?,
        prepared_story_id: parse_opt_uuid(&r.prepared_story_id)?,
        plan_version_id: parse_opt_uuid(&r.plan_version_id)?,
        parent_requirement_id: parse_opt_uuid(&r.parent_requirement_id)?,
        title: r.title,
        statement: r.statement,
        kind: r.kind,
        criticality: r.criticality,
        source: r.source,
        ambiguity_state: r.ambiguity_state,
        owner_persona_id: if r.owner_persona_id.is_empty() {
            None
        } else {
            Some(r.owner_persona_id)
        },
        status: r.status,
        created_at: parse_rfc3339(&r.created_at, "requirement.created_at")?,
        updated_at: parse_rfc3339(&r.updated_at, "requirement.updated_at")?,
    })
}

fn requirement_edge_from_proto(e: RequirementEdge) -> Result<RequirementEdgeView> {
    Ok(RequirementEdgeView {
        id: parse_uuid(&e.id, "requirement_edge.id")?,
        workspace_id: parse_uuid(&e.workspace_id, "requirement_edge.workspace_id")?,
        requirement_id: parse_uuid(&e.requirement_id, "requirement_edge.requirement_id")?,
        related_requirement_id: parse_uuid(
            &e.related_requirement_id,
            "requirement_edge.related_requirement_id",
        )?,
        edge_type: e.edge_type,
        created_at: parse_rfc3339(&e.created_at, "requirement_edge.created_at")?,
    })
}

fn acceptance_check_from_proto(c: AcceptanceCheck) -> Result<AcceptanceCheckView> {
    Ok(AcceptanceCheckView {
        id: parse_uuid(&c.id, "acceptance_check.id")?,
        workspace_id: parse_uuid(&c.workspace_id, "acceptance_check.workspace_id")?,
        requirement_id: parse_uuid(&c.requirement_id, "acceptance_check.requirement_id")?,
        check_kind: c.check_kind,
        title: c.title,
        status: c.status,
        evidence_required: c.evidence_required,
        created_at: parse_rfc3339(&c.created_at, "acceptance_check.created_at")?,
        updated_at: parse_rfc3339(&c.updated_at, "acceptance_check.updated_at")?,
    })
}

fn requirement_coverage_from_proto(c: RequirementCoverage) -> Result<RequirementCoverageView> {
    Ok(RequirementCoverageView {
        id: parse_uuid(&c.id, "requirement_coverage.id")?,
        workspace_id: parse_uuid(&c.workspace_id, "requirement_coverage.workspace_id")?,
        requirement_id: parse_uuid(&c.requirement_id, "requirement_coverage.requirement_id")?,
        work_item_id: parse_uuid(&c.work_item_id, "requirement_coverage.work_item_id")?,
        coverage_kind: c.coverage_kind,
        status: c.status,
        created_at: parse_rfc3339(&c.created_at, "requirement_coverage.created_at")?,
        updated_at: parse_rfc3339(&c.updated_at, "requirement_coverage.updated_at")?,
    })
}

fn handoff_contract_from_proto(h: HandoffContract) -> Result<HandoffContractView> {
    Ok(HandoffContractView {
        id: parse_uuid(&h.id, "handoff_contract.id")?,
        workspace_id: parse_uuid(&h.workspace_id, "handoff_contract.workspace_id")?,
        prepared_story_id: parse_opt_uuid(&h.prepared_story_id)?,
        from_work_item_id: parse_opt_uuid(&h.from_work_item_id)?,
        to_work_item_id: parse_opt_uuid(&h.to_work_item_id)?,
        contract_kind: h.contract_kind,
        expected_artifact_json: parse_json_opt(&h.expected_artifact_json)?,
        acceptance_signal_json: parse_json_opt(&h.acceptance_signal_json)?,
        status: h.status,
        created_at: parse_rfc3339(&h.created_at, "handoff_contract.created_at")?,
        updated_at: parse_rfc3339(&h.updated_at, "handoff_contract.updated_at")?,
    })
}

fn closure_claim_from_proto(c: ClosureClaim) -> Result<ClosureClaimView> {
    Ok(ClosureClaimView {
        id: parse_uuid(&c.id, "closure_claim.id")?,
        workspace_id: parse_uuid(&c.workspace_id, "closure_claim.workspace_id")?,
        work_item_id: parse_opt_uuid(&c.work_item_id)?,
        requirement_id: parse_opt_uuid(&c.requirement_id)?,
        claim_type: c.claim_type,
        status: c.status,
        claimed_by_persona_id: if c.claimed_by_persona_id.is_empty() {
            None
        } else {
            Some(c.claimed_by_persona_id)
        },
        claim_json: parse_json_opt(&c.claim_json)?,
        created_at: parse_rfc3339(&c.created_at, "closure_claim.created_at")?,
        updated_at: parse_rfc3339(&c.updated_at, "closure_claim.updated_at")?,
    })
}

fn closure_gate_from_proto(g: ClosureGate) -> Result<ClosureGateView> {
    Ok(ClosureGateView {
        id: parse_uuid(&g.id, "closure_gate.id")?,
        workspace_id: parse_uuid(&g.workspace_id, "closure_gate.workspace_id")?,
        story_id: parse_opt_uuid(&g.story_id)?,
        prepared_story_id: parse_opt_uuid(&g.prepared_story_id)?,
        gate_type: g.gate_type,
        status: g.status,
        decided_by_persona_id: if g.decided_by_persona_id.is_empty() {
            None
        } else {
            Some(g.decided_by_persona_id)
        },
        rationale_md: if g.rationale_md.is_empty() {
            None
        } else {
            Some(g.rationale_md)
        },
        created_at: parse_rfc3339(&g.created_at, "closure_gate.created_at")?,
        updated_at: parse_rfc3339(&g.updated_at, "closure_gate.updated_at")?,
    })
}

fn verification_finding_from_proto(f: VerificationFinding) -> Result<VerificationFindingView> {
    Ok(VerificationFindingView {
        id: parse_uuid(&f.id, "verification_finding.id")?,
        workspace_id: parse_uuid(&f.workspace_id, "verification_finding.workspace_id")?,
        verification_run_id: parse_uuid(
            &f.verification_run_id,
            "verification_finding.verification_run_id",
        )?,
        requirement_id: parse_opt_uuid(&f.requirement_id)?,
        severity: f.severity,
        finding_type: f.finding_type,
        title: f.title,
        detail_md: if f.detail_md.is_empty() {
            None
        } else {
            Some(f.detail_md)
        },
        status: f.status,
        created_at: parse_rfc3339(&f.created_at, "verification_finding.created_at")?,
        updated_at: parse_rfc3339(&f.updated_at, "verification_finding.updated_at")?,
    })
}

fn persona_from_proto(p: Persona) -> Result<PersonaView> {
    Ok(PersonaView {
        id: p.id,
        workspace_id: parse_uuid(&p.workspace_id, "persona.workspace_id")?,
        display_name: p.display_name,
        accountability_md: p.accountability_md,
        tool_scope_json: parse_json_opt(&p.tool_scope_json)?,
        memory_scope_json: parse_json_opt(&p.memory_scope_json)?,
        autonomy_policy_json: parse_json_opt(&p.autonomy_policy_json)?,
        active: p.active,
        created_at: parse_rfc3339(&p.created_at, "persona.created_at")?,
        updated_at: parse_rfc3339(&p.updated_at, "persona.updated_at")?,
    })
}

fn persona_assignment_from_proto(a: PersonaAssignment) -> Result<PersonaAssignmentView> {
    Ok(PersonaAssignmentView {
        id: parse_uuid(&a.id, "persona_assignment.id")?,
        workspace_id: parse_uuid(&a.workspace_id, "persona_assignment.workspace_id")?,
        persona_id: a.persona_id,
        subject_type: a.subject_type,
        subject_id: parse_opt_uuid(&a.subject_id)?,
        assignment_role: a.assignment_role,
        status: a.status,
        created_at: parse_rfc3339(&a.created_at, "persona_assignment.created_at")?,
        updated_at: parse_rfc3339(&a.updated_at, "persona_assignment.updated_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use openakta_proto::work::v1::GetRequirementGraphResponse;

    #[test]
    fn requirement_graph_empty_roundtrip() {
        let v = requirement_graph_from_response(GetRequirementGraphResponse {
            requirements: vec![],
            edges: vec![],
            acceptance_checks: vec![],
            coverage: vec![],
            handoff_contracts: vec![],
        })
        .unwrap();
        assert!(v.requirements.is_empty());
        assert!(v.requirement_edges.is_empty());
    }

    #[test]
    fn persona_assignments_empty() {
        let v = persona_assignments_from_response(ListPersonaAssignmentsResponse {
            items: vec![],
            assignments: vec![],
        })
        .unwrap();
        assert!(v.personas.is_empty());
        assert!(v.assignments.is_empty());
    }
}
