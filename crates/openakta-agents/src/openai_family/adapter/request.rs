//! Request normalization: internal ModelRequest → SDK CreateChatCompletionRequest.

use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessage,
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestToolMessage, ChatCompletionRequestUserMessage, ChatCompletionTool,
    ChatCompletionToolType, CreateChatCompletionRequest, CreateChatCompletionRequestArgs,
    FunctionCall, FunctionObject,
};

use crate::openai_family::capabilities::ResolvedCapabilities;
use crate::openai_family::error::TransportError;
use crate::provider::{ModelBoundaryPayloadType, ModelRequest, ModelToolSchema};

/// Normalize internal ModelRequest to SDK CreateChatCompletionRequest.
pub fn build_sdk_request(
    internal: &ModelRequest,
    capabilities: &ResolvedCapabilities,
) -> Result<CreateChatCompletionRequest, TransportError> {
    // Validate capabilities BEFORE building request
    validate_capabilities(internal, capabilities)?;

    let mut builder = CreateChatCompletionRequestArgs::default();

    // REQUIRED: model
    builder.model(&internal.model);

    // REQUIRED: messages (system + user + assistant)
    builder.messages(build_messages(internal)?);

    // OPTIONAL: max_tokens (capability-gated)
    if internal.max_output_tokens > 0 {
        if internal.max_output_tokens > capabilities.max_output_tokens {
            return Err(TransportError::MaxTokensExceeded {
                requested: internal.max_output_tokens,
                max: capabilities.max_output_tokens,
            });
        }
        builder.max_tokens(internal.max_output_tokens);
    }

    // OPTIONAL: temperature (validated range)
    if let Some(temp) = internal.temperature {
        if temp < 0.0 || temp > 2.0 {
            return Err(TransportError::InvalidTemperature {
                value: temp,
                min: 0.0,
                max: 2.0,
            });
        }
        builder.temperature(temp);
    }

    // OPTIONAL: tools (capability-gated)
    if !internal.tool_schemas.is_empty() {
        if !capabilities.supports_tools {
            return Err(TransportError::CapabilityNotSupported(
                "tool calling not supported by this provider".into(),
            ));
        }
        builder.tools(build_tools(internal.tool_schemas.clone())?);
    }

    // OPTIONAL: response_format (capability-gated)
    // Note: ResponseFormat API varies by version; skipping for now
    // Can be added when we verify the exact API for v0.28.0

    // OPTIONAL: stream
    builder.stream(internal.stream);

    builder
        .build()
        .map_err(|err| TransportError::SdkRequestBuild(err.to_string()))
}

fn build_messages(
    request: &ModelRequest,
) -> Result<Vec<ChatCompletionRequestMessage>, TransportError> {
    let mut messages = Vec::new();

    // System instructions → system messages
    for instruction in &request.system_instructions {
        messages.push(
            ChatCompletionRequestSystemMessage {
                content: instruction.clone().into(),
                name: None,
            }
            .into(),
        );
    }

    for invariant in &request.invariant_mission_context {
        messages.push(
            ChatCompletionRequestSystemMessage {
                content: serde_json::to_string(invariant)
                    .map_err(|err| TransportError::Serialization(err.to_string()))?
                    .into(),
                name: None,
            }
            .into(),
        );
    }

    // User message with TOON payload
    messages.push(
        ChatCompletionRequestUserMessage {
            content: request
                .payload
                .to_toon()
                .map_err(|e| TransportError::Serialization(e.to_string()))?
                .into(),
            name: None,
        }
        .into(),
    );

    // Recent messages (user/assistant)
    for msg in &request.recent_messages {
        match msg.role.as_str() {
            "user" => messages.push(
                ChatCompletionRequestUserMessage {
                    content: msg.content.clone().into(),
                    name: None,
                }
                .into(),
            ),
            "assistant" => messages.push(
                #[allow(deprecated)]
                ChatCompletionRequestAssistantMessage {
                    content: (!msg.content.is_empty()).then(|| msg.content.clone().into()),
                    name: None,
                    tool_calls: (!msg.tool_calls.is_empty())
                        .then(|| msg.tool_calls.iter().map(to_sdk_tool_call).collect()),
                    refusal: None,
                    audio: None,
                    function_call: None,
                }
                .into(),
            ),
            "tool" => messages.push(
                ChatCompletionRequestToolMessage {
                    content: msg.content.clone().into(),
                    tool_call_id: msg.tool_call_id.clone().ok_or_else(|| {
                        TransportError::InvalidMessageRole(
                            "tool message missing tool_call_id".to_string(),
                        )
                    })?,
                }
                .into(),
            ),
            _ => {
                return Err(TransportError::InvalidMessageRole(msg.role.clone()));
            }
        }
    }

    Ok(messages)
}

fn build_tools(schemas: Vec<ModelToolSchema>) -> Result<Vec<ChatCompletionTool>, TransportError> {
    schemas
        .into_iter()
        .map(|schema| {
            Ok(ChatCompletionTool {
                r#type: ChatCompletionToolType::Function,
                function: FunctionObject {
                    name: schema.name.into(),
                    description: Some(schema.description),
                    parameters: Some(schema.parameters),
                    strict: Some(schema.strict),
                },
            })
        })
        .collect()
}

fn to_sdk_tool_call(call: &crate::provider::ModelToolCall) -> ChatCompletionMessageToolCall {
    ChatCompletionMessageToolCall {
        id: call.id.clone(),
        r#type: ChatCompletionToolType::Function,
        function: FunctionCall {
            name: call.name.clone(),
            arguments: call.arguments_json.clone(),
        },
    }
}

fn validate_capabilities(
    request: &ModelRequest,
    capabilities: &ResolvedCapabilities,
) -> Result<(), TransportError> {
    // Fail-fast: tools
    if !request.tool_schemas.is_empty() && !capabilities.supports_tools {
        return Err(TransportError::CapabilityNotSupported(
            "tool calling not supported".into(),
        ));
    }

    // Fail-fast: JSON mode
    if matches!(
        request.payload.payload_type,
        ModelBoundaryPayloadType::Retrieval
    ) && !capabilities.supports_json_mode
    {
        return Err(TransportError::CapabilityNotSupported(
            "JSON mode not supported".into(),
        ));
    }

    // Fail-fast: streaming
    if request.stream && !capabilities.supports_streaming {
        return Err(TransportError::CapabilityNotSupported(
            "streaming not supported".into(),
        ));
    }

    // Fail-fast: max tokens
    if request.max_output_tokens > capabilities.max_output_tokens {
        return Err(TransportError::MaxTokensExceeded {
            requested: request.max_output_tokens,
            max: capabilities.max_output_tokens,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{
        CacheRetention, ChatMessage, ModelBoundaryPayload, ModelBoundaryPayloadType,
    };
    use crate::wire_profile::WireProfile;
    use serde_json::json;

    fn capabilities() -> ResolvedCapabilities {
        ResolvedCapabilities {
            supports_tools: true,
            supports_json_mode: true,
            supports_streaming: true,
            supports_vision: false,
            supports_prompt_cache: false,
            max_context_window: 128_000,
            max_output_tokens: 8_192,
        }
    }

    fn request_with_invariant() -> ModelRequest {
        ModelRequest {
            provider: WireProfile::OpenAiChatCompletions,
            model: "openai/gpt-5.4".to_string(),
            system_instructions: vec!["system-a".to_string()],
            tool_schemas: Vec::new(),
            invariant_mission_context: vec![json!({"workspace_context":"File: package.json"})],
            payload: ModelBoundaryPayload {
                payload_type: ModelBoundaryPayloadType::TaskExecution,
                task_id: "task-1".to_string(),
                title: "Inspect manifests".to_string(),
                description: "Inspect manifests".to_string(),
                task_type: "Retrieval".to_string(),
                target_files: vec!["package.json".to_string()],
                target_symbols: Vec::new(),
                context_spans: Vec::new(),
                context_pack: None,
            },
            recent_messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Read package.json".to_string(),
                name: None,
                tool_call_id: None,
                tool_calls: Vec::new(),
            }],
            max_output_tokens: 512,
            temperature: Some(0.0),
            stream: false,
            cache_retention: CacheRetention::Short,
        }
    }

    #[test]
    fn build_messages_serializes_invariant_context_before_payload() {
        let messages = build_messages(&request_with_invariant()).unwrap();
        let value = serde_json::to_value(&messages).unwrap();
        let rendered = value.to_string();

        assert!(rendered.contains("system-a"));
        assert!(rendered.contains("workspace_context"));
        assert!(rendered.contains("File: package.json"));
        assert!(rendered.contains("task-1"));
        assert!(rendered.contains("Read package.json"));
    }

    #[test]
    fn build_sdk_request_keeps_invariant_system_messages() {
        let request = build_sdk_request(&request_with_invariant(), &capabilities()).unwrap();
        let value = serde_json::to_value(&request).unwrap().to_string();

        assert!(value.contains("workspace_context"));
        assert!(value.contains("File: package.json"));
        assert!(value.contains("task-1"));
    }
}
