//! Legacy work item fence for MOL-managed fields.

use serde_json::Value;
use uuid::Uuid;

use crate::mol_flags::MolFeatureFlags;
use crate::transition::MolError;

pub fn check_legacy_create_work_item(
    flags: MolFeatureFlags,
    prepared_story_id: Option<Uuid>,
    owner_persona_id: Option<&String>,
    requirement_slice_json: Option<&Value>,
    handoff_contract_state: Option<&String>,
    claim_state: Option<&String>,
) -> Result<(), MolError> {
    if !flags.strict_legacy_fence || prepared_story_id.is_none() {
        return Ok(());
    }
    let fields = collect_fenced_fields_present(
        owner_persona_id.map(String::as_str),
        requirement_slice_json,
        handoff_contract_state.map(String::as_str),
        claim_state.map(String::as_str),
    );
    if fields.is_empty() {
        Ok(())
    } else {
        Err(MolError::LegacyFenceViolation { fields })
    }
}

pub fn check_legacy_patch_work_item(
    flags: MolFeatureFlags,
    effective_prepared_story_id: Option<Uuid>,
    owner_persona_id: Option<&String>,
    requirement_slice_json: Option<&Value>,
    handoff_contract_state: Option<&String>,
    claim_state: Option<&String>,
) -> Result<(), MolError> {
    if !flags.strict_legacy_fence || effective_prepared_story_id.is_none() {
        return Ok(());
    }
    let fields = collect_fenced_fields_present(
        owner_persona_id.map(String::as_str),
        requirement_slice_json,
        handoff_contract_state.map(String::as_str),
        claim_state.map(String::as_str),
    );
    if fields.is_empty() {
        Ok(())
    } else {
        Err(MolError::LegacyFenceViolation { fields })
    }
}

fn collect_fenced_fields_present(
    owner_persona_id: Option<&str>,
    requirement_slice_json: Option<&Value>,
    handoff_contract_state: Option<&str>,
    claim_state: Option<&str>,
) -> String {
    let mut names: Vec<&'static str> = Vec::new();
    if owner_persona_id.is_some() {
        names.push("owner_persona_id");
    }
    if requirement_slice_json.is_some() {
        names.push("requirement_slice_json");
    }
    if handoff_contract_state.is_some() {
        names.push("handoff_contract_state");
    }
    if claim_state.is_some() {
        names.push("claim_state");
    }
    names.join(", ")
}
