//! MCP gRPC client for tool execution.

use crate::diagnostics::WideEvent;
use crate::react::Observation;
use openakta_proto::mcp::v1::tool_service_client::ToolServiceClient;
use openakta_proto::mcp::v1::{AuditEvent, CapabilityPolicy, ListToolsRequest, ToolCallRequest};
use prost_types::{value::Kind, Struct, Value};
use serde_json::{Map, Value as JsonValue};
use tonic::transport::{Channel, Endpoint};

/// MCP client wrapper used by agents.
#[derive(Clone)]
pub struct McpClient {
    endpoint: String,
}

impl McpClient {
    /// Create a new MCP client for the given endpoint.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    async fn connect(&self) -> Result<ToolServiceClient<Channel>, tonic::transport::Error> {
        let endpoint = Endpoint::from_shared(self.endpoint.clone())?;
        let channel = endpoint.connect().await?;
        Ok(ToolServiceClient::new(channel))
    }

    /// List available tools for the role.
    pub async fn list_tools(
        &self,
        agent_id: &str,
        role: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut client = self.connect().await?;
        let response = client
            .list_tools(ListToolsRequest {
                agent_id: agent_id.to_string(),
                role: role.to_string(),
            })
            .await?
            .into_inner();
        Ok(response.tools.into_iter().map(|tool| tool.name).collect())
    }

    /// Execute an MCP tool call and convert it into a ReAct observation.
    #[allow(clippy::too_many_arguments)]
    pub async fn call_tool(
        &self,
        request_id: &str,
        agent_id: &str,
        role: &str,
        tool_name: &str,
        workspace_root: &str,
        arguments: Struct,
        policy: Option<CapabilityPolicy>,
        mission_id: Option<&str>,
    ) -> Result<Observation, Box<dyn std::error::Error + Send + Sync>> {
        let mut client = self.connect().await?;
        let policy = policy.or_else(|| {
            Some(Self::default_policy(
                agent_id,
                role,
                tool_name,
                workspace_root,
            ))
        });
        let result = client
            .call_tool(ToolCallRequest {
                request_id: request_id.to_string(),
                agent_id: agent_id.to_string(),
                role: role.to_string(),
                tool_name: tool_name.to_string(),
                arguments: Some(arguments),
                policy,
                workspace_root: workspace_root.to_string(),
                mission_id: mission_id.unwrap_or("").to_string(),
            })
            .await?
            .into_inner();
        if result.success {
            if tool_name == "read_file" {
                if let Some(content) = result
                    .output
                    .as_ref()
                    .and_then(|output| output.fields.get("content"))
                    .and_then(|value| value.kind.as_ref())
                    .and_then(|kind| match kind {
                        Kind::StringValue(value) => Some(value.clone()),
                        _ => None,
                    })
                {
                    return Ok(Observation::success(JsonValue::String(content)));
                }
            }

            let mut payload = serde_json::json!({
                "stdout": result.stdout,
                "stderr": result.stderr,
                "exit_code": result.exit_code,
            });

            if let Some(output) = result.output {
                payload["output"] = proto_struct_to_json(output);
            }

            Ok(Observation::success(payload))
        } else {
            let diagnostic = WideEvent::tool_failure(
                tool_name,
                request_id,
                workspace_root,
                result.exit_code,
                &result.stdout,
                &result.stderr,
                result.audit_event.as_ref().map(audit_event_to_json),
            )
            .to_toon()?;
            Ok(Observation::failure_with_result(
                &result.stderr,
                serde_json::json!({
                    "stdout": result.stdout,
                    "stderr": result.stderr,
                    "exit_code": result.exit_code,
                    "diagnostic_toon": diagnostic,
                    "tool_name": tool_name,
                }),
            ))
        }
    }

    /// Build a string-only protobuf struct.
    pub fn string_args(entries: &[(&str, String)]) -> Struct {
        Struct {
            fields: entries
                .iter()
                .map(|(key, value)| {
                    (
                        (*key).to_string(),
                        Value {
                            kind: Some(Kind::StringValue(value.clone())),
                        },
                    )
                })
                .collect(),
        }
    }

    fn default_policy(
        agent_id: &str,
        role: &str,
        tool_name: &str,
        workspace_root: &str,
    ) -> CapabilityPolicy {
        CapabilityPolicy {
            agent_id: agent_id.to_string(),
            role: role.to_string(),
            allowed_actions: vec![tool_name.to_string()],
            allowed_scope_patterns: vec![workspace_root.to_string()],
            denied_scope_patterns: vec![
                format!("{workspace_root}/.git"),
                format!("{workspace_root}/target"),
            ],
            max_execution_seconds: 30,
        }
    }
}

fn proto_struct_to_json(value: Struct) -> JsonValue {
    JsonValue::Object(
        value
            .fields
            .into_iter()
            .map(|(key, value)| (key, proto_value_to_json(value)))
            .collect::<Map<String, JsonValue>>(),
    )
}

fn proto_value_to_json(value: Value) -> JsonValue {
    match value.kind {
        Some(Kind::NullValue(_)) | None => JsonValue::Null,
        Some(Kind::NumberValue(number)) => serde_json::json!(number),
        Some(Kind::StringValue(string)) => JsonValue::String(string),
        Some(Kind::BoolValue(boolean)) => serde_json::json!(boolean),
        Some(Kind::StructValue(struct_value)) => proto_struct_to_json(struct_value),
        Some(Kind::ListValue(list_value)) => JsonValue::Array(
            list_value
                .values
                .into_iter()
                .map(proto_value_to_json)
                .collect(),
        ),
    }
}

fn audit_event_to_json(event: &AuditEvent) -> JsonValue {
    serde_json::json!({
        "event_id": event.event_id,
        "request_id": event.request_id,
        "agent_id": event.agent_id,
        "role": event.role,
        "tool_name": event.tool_name,
        "action": event.action,
        "scope": event.scope,
        "allowed": event.allowed,
        "detail": event.detail,
        "created_at": event.created_at.as_ref().map(|ts| ts.seconds),
    })
}
