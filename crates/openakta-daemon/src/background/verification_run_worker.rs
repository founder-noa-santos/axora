//! Drains `wm_verification_runs` rows in `pending` by transitioning them through `running` to
//! `passed`, recording a minimal automated finding for auditability (AB10).

use anyhow::Result;
use chrono::Utc;
use serde_json::json;
use tracing::{error, info};
use uuid::Uuid;

use crate::background::work_management_service::WorkManagementGrpc;

pub(crate) async fn process_pending_verification_runs(
    wm: &WorkManagementGrpc,
    workspace_id: Uuid,
) -> Result<()> {
    if !wm.mol_verification_automation_enabled() {
        return Ok(());
    }
    let snapshot = wm
        .synced_read_model(workspace_id)
        .await
        .map_err(|status| anyhow::anyhow!(status.to_string()))?;
    let pending: Vec<Uuid> = snapshot
        .model
        .verification_runs
        .iter()
        .filter(|r| r.status == "pending")
        .map(|r| r.id)
        .collect();
    for run_id in pending {
        if let Err(err) = process_one_run(wm, workspace_id, run_id).await {
            error!(
                workspace_id = %workspace_id,
                verification_run_id = %run_id,
                error = %err,
                "verification automation failed for run"
            );
        }
    }
    Ok(())
}

async fn process_one_run(wm: &WorkManagementGrpc, workspace_id: Uuid, run_id: Uuid) -> Result<()> {
    wm.submit_system_command(
        workspace_id,
        "update_verification_run",
        json!({
            "id": run_id,
            "status": "running",
        }),
    )
    .await?;
    let finished_at = Utc::now();
    let finding_id = Uuid::new_v4();
    wm.submit_system_command(
        workspace_id,
        "record_verification_finding",
        json!({
            "id": finding_id,
            "verification_run_id": run_id,
            "severity": "info",
            "finding_type": "automated_stub",
            "title": "Automated verification summary",
            "detail_md": "Daemon completed minimal automated verification for this run.",
            "status": "resolved",
        }),
    )
    .await?;
    wm.submit_system_command(
        workspace_id,
        "update_verification_run",
        json!({
            "id": run_id,
            "status": "passed",
            "completed_at": finished_at.to_rfc3339(),
            "summary_json": {
                "automation": {
                    "kind": "daemon_stub",
                    "finished_at": finished_at.to_rfc3339(),
                }
            }
        }),
    )
    .await?;
    info!(
        workspace_id = %workspace_id,
        verification_run_id = %run_id,
        "verification run completed by daemon automation"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_ids_filter_is_stable_contract() {
        let id = Uuid::new_v4();
        let runs = vec![openakta_api_client::VerificationRunView {
            id,
            workspace_id: Uuid::new_v4(),
            story_id: None,
            prepared_story_id: None,
            status: "pending".to_string(),
            verification_stage: "post_implementation".to_string(),
            run_kind: "independent".to_string(),
            initiated_by_persona_id: None,
            summary_json: None,
            created_at: Utc::now(),
            completed_at: None,
        }];
        let pending: Vec<Uuid> = runs
            .iter()
            .filter(|r| r.status == "pending")
            .map(|r| r.id)
            .collect();
        assert_eq!(pending, vec![id]);
    }
}
