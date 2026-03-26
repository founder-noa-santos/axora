//! Shared parsing of prepared story `execution_card_json` for gRPC and mission compilation.

use openakta_proto::work::v1::{ExecutionCard, ExecutionTaskOutline};
use serde_json::Value;
use uuid::Uuid;

/// Maps API `execution_card_json` into a typed [`ExecutionCard`] (schema v1). Unknown keys may live in `extension_json` later.
pub(crate) fn proto_execution_card_from_json(
    value: &Value,
    primary_execution_profile: &str,
) -> ExecutionCard {
    let mut card = ExecutionCard {
        schema_version: 1,
        primary_execution_profile: primary_execution_profile.to_string(),
        ..Default::default()
    };
    let Some(obj) = value.as_object() else {
        return card;
    };
    if let Some(s) = obj.get("story_summary").and_then(|x| x.as_str()) {
        card.story_summary = s.to_string();
    }
    if let Some(s) = obj
        .get("primary_execution_profile")
        .and_then(|x| x.as_str())
    {
        card.primary_execution_profile = s.to_string();
    }
    if let Some(p) = obj.get("policy_json") {
        card.policy_json = p.to_string();
    }
    if let Some(ext) = obj.get("extension_json") {
        card.extension_json = ext.to_string();
    }
    if let Some(arr) = obj.get("tasks").and_then(|x| x.as_array()) {
        for t in arr {
            let Some(to) = t.as_object() else {
                continue;
            };
            card.tasks.push(ExecutionTaskOutline {
                work_item_id: to
                    .get("work_item_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                title: to
                    .get("title")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                execution_profile: to
                    .get("execution_profile")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                wave_rank: to
                    .get("wave_rank")
                    .and_then(|x| x.as_i64())
                    .and_then(|n| i32::try_from(n).ok())
                    .unwrap_or(0),
                wave_label: to
                    .get("wave_label")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                parent_work_item_id: to
                    .get("parent_work_item_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
            });
        }
    }
    card
}

/// Task rows from `execution_card_json` suitable for ordering and field overrides at compile time.
/// Skips entries whose `work_item_id` is missing or not a valid UUID (same keys as [`ExecutionTaskOutline`]).
pub(crate) fn execution_compile_task_outlines(json: Option<&Value>) -> Vec<ExecutionTaskOutline> {
    let Some(value) = json else {
        return Vec::new();
    };
    let Some(obj) = value.as_object() else {
        return Vec::new();
    };
    let Some(arr) = obj.get("tasks").and_then(|x| x.as_array()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for t in arr {
        let Some(to) = t.as_object() else {
            continue;
        };
        let Some(wid) = to.get("work_item_id").and_then(|x| x.as_str()) else {
            continue;
        };
        if Uuid::parse_str(wid).is_err() {
            continue;
        }
        out.push(ExecutionTaskOutline {
            work_item_id: wid.to_string(),
            title: to
                .get("title")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            execution_profile: to
                .get("execution_profile")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            wave_rank: to
                .get("wave_rank")
                .and_then(|x| x.as_i64())
                .and_then(|n| i32::try_from(n).ok())
                .unwrap_or(0),
            wave_label: to
                .get("wave_label")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            parent_work_item_id: to
                .get("parent_work_item_id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
        });
    }
    out
}
