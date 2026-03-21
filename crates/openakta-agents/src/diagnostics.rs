//! Canonical Wide Event diagnostics encoded with TOON.

use crate::error::AgentError;
use crate::Result;
use chrono::Utc;
use openakta_cache::{Schema, ToonSerializer};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// Structured error payload for a wide event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WideEventError {
    /// Error type.
    pub r#type: Option<String>,
    /// Error message.
    pub message: Option<String>,
    /// Error stack or detail payload.
    pub stack: Option<String>,
}

impl WideEventError {
    /// Build a new error payload.
    pub fn new(
        r#type: impl Into<Option<String>>,
        message: impl Into<Option<String>>,
        stack: impl Into<Option<String>>,
    ) -> Self {
        Self {
            r#type: r#type.into(),
            message: message.into(),
            stack: stack.into(),
        }
    }
}

/// SDK metadata for a wide event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WideEventMeta {
    /// SDK version.
    pub sdk_version: String,
    /// SDK language.
    pub sdk_language: String,
}

impl Default for WideEventMeta {
    fn default() -> Self {
        Self {
            sdk_version: env!("CARGO_PKG_VERSION").to_string(),
            sdk_language: "rust".to_string(),
        }
    }
}

/// Canonical wide event payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WideEvent {
    /// Unique event ID.
    pub event_id: String,
    /// Service name.
    pub service: String,
    /// Runtime environment.
    pub environment: String,
    /// Start timestamp in UTC.
    pub timestamp_start: String,
    /// End timestamp in UTC.
    pub timestamp_end: String,
    /// Duration in milliseconds.
    pub duration_ms: f64,
    /// Log level.
    pub level: String,
    /// Operation name.
    pub operation: String,
    /// Final status.
    pub status: String,
    /// Open-ended context.
    pub context: Map<String, Value>,
    /// Structured error.
    pub error: WideEventError,
    /// SDK metadata.
    pub meta: WideEventMeta,
}

impl WideEvent {
    /// Create a failed MCP tool event.
    pub fn tool_failure(
        tool_name: &str,
        request_id: &str,
        workspace_root: &str,
        exit_code: i32,
        stdout: &str,
        stderr: &str,
        audit_event: Option<Value>,
    ) -> Self {
        let now = Utc::now().to_rfc3339();
        let status = if exit_code == -1 {
            "timeout"
        } else if exit_code == 130 {
            "cancelled"
        } else {
            "error"
        };
        let mut context = Map::new();
        context.insert("tool_name".to_string(), json!(tool_name));
        context.insert("request_id".to_string(), json!(request_id));
        context.insert("workspace_root".to_string(), json!(workspace_root));
        context.insert("exit_code".to_string(), json!(exit_code));
        context.insert("stdout".to_string(), json!(stdout));
        context.insert("stderr".to_string(), json!(stderr));
        if let Some(audit_event) = audit_event {
            context.insert("audit_event".to_string(), audit_event);
        }

        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            service: "openakta-agents".to_string(),
            environment: "development".to_string(),
            timestamp_start: now.clone(),
            timestamp_end: now,
            duration_ms: 0.0,
            level: "error".to_string(),
            operation: format!("mcp.tool.{tool_name}"),
            status: status.to_string(),
            context,
            error: WideEventError::new(
                Some("ToolExecutionFailed".to_string()),
                Some(stderr.to_string()),
                Some(format!("stdout={stdout}\nstderr={stderr}")),
            ),
            meta: WideEventMeta::default(),
        }
    }

    /// Create a failed validation or coordinator event.
    pub fn workflow_failure(operation: &str, message: &str, context: Map<String, Value>) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            service: "openakta-agents".to_string(),
            environment: "development".to_string(),
            timestamp_start: now.clone(),
            timestamp_end: now,
            duration_ms: 0.0,
            level: "error".to_string(),
            operation: operation.to_string(),
            status: "error".to_string(),
            context,
            error: WideEventError::new(
                Some("WorkflowFailure".to_string()),
                Some(message.to_string()),
                None,
            ),
            meta: WideEventMeta::default(),
        }
    }

    fn schema() -> Schema {
        let mut schema = Schema::new();
        for field in [
            "event_id",
            "service",
            "environment",
            "timestamp_start",
            "timestamp_end",
            "duration_ms",
            "level",
            "operation",
            "status",
            "context",
            "error",
            "meta",
            "type",
            "message",
            "stack",
            "sdk_version",
            "sdk_language",
            "tool_name",
            "request_id",
            "workspace_root",
            "exit_code",
            "stdout",
            "stderr",
            "audit_event",
        ] {
            schema.add_field(field);
        }
        schema
    }

    /// Encode the payload with the fixed TOON schema.
    pub fn to_toon(&self) -> Result<String> {
        let json = serde_json::to_string(self)
            .map_err(|err| AgentError::Serialization(err.to_string()))?;
        ToonSerializer::new(Self::schema())
            .encode(&json)
            .map_err(|err| AgentError::Serialization(err.to_string()).into())
    }
}
