#![allow(dead_code)]

use openakta_agents::provider::ChatMessage;
use openakta_agents::{
    CacheRetention, ModelBoundaryPayload, ModelBoundaryPayloadType, ModelRequest, ModelResponse,
    ModelToolCall, ProviderKind, ProviderUsage, WireProfile,
};
use openakta_proto::provider_v1 as proto;
use serde_json::json;

pub fn create_internal_request() -> ModelRequest {
    ModelRequest {
        provider: WireProfile::OpenAiChatCompletions,
        model: "gpt-4o-mini".to_string(),
        system_instructions: vec!["You are a benchmark harness.".to_string()],
        tool_schemas: vec![],
        invariant_mission_context: vec![],
        payload: ModelBoundaryPayload {
            payload_type: ModelBoundaryPayloadType::TaskExecution,
            task_id: "benchmark-task".to_string(),
            title: "Benchmark task".to_string(),
            description: "Exercise provider request/response conversion".to_string(),
            task_type: "GENERAL".to_string(),
            target_files: vec![],
            target_symbols: vec![],
            context_spans: vec![],
            context_pack: None,
        },
        recent_messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "This is a benchmark test message. ".repeat(10),
            name: None,
            tool_call_id: None,
            tool_calls: Vec::new(),
        }],
        max_output_tokens: 100,
        temperature: Some(0.7),
        stream: false,
        cache_retention: CacheRetention::ProviderDefault,
    }
}

pub fn build_provider_request(
    request_id: &str,
    tenant_id: &str,
    model_request: &ModelRequest,
) -> proto::ProviderRequest {
    proto::ProviderRequest {
        request_id: request_id.to_string(),
        tenant_id: tenant_id.to_string(),
        model: model_request.model.clone(),
        model_hint: None,
        system_prompt: model_request.system_instructions.join("\n"),
        messages: model_request
            .recent_messages
            .iter()
            .map(|message| proto::ChatMessage {
                role: message.role.clone(),
                content: Some(message.content.clone()),
                name: message.name.clone(),
                content_parts: vec![],
                tool_call: message.tool_calls.first().map(|tool_call| proto::ToolCall {
                    id: tool_call.id.clone(),
                    name: tool_call.name.clone(),
                    arguments: tool_call.arguments_json.clone(),
                }),
                tool_call_id: message.tool_call_id.clone(),
            })
            .collect(),
        tools: vec![],
        tool_choice: proto::ToolChoice::Auto as i32,
        max_tokens: Some(model_request.max_output_tokens),
        temperature: model_request.temperature,
        top_p: None,
        stop_sequences: vec![],
        frequency_penalty: None,
        presence_penalty: None,
        stream: model_request.stream,
        provider_extensions: std::collections::HashMap::new(),
        required_capabilities: vec![],
        execution_strategy: proto::ExecutionStrategy::HostedOnly as i32,
    }
}

pub fn create_test_request() -> proto::ProviderRequest {
    let model_request = create_internal_request();
    build_provider_request("benchmark-request", "benchmark-tenant", &model_request)
}

pub fn create_proto_response() -> proto::ProviderResponse {
    proto::ProviderResponse {
        request_id: "benchmark-request".to_string(),
        response_id: "benchmark-response".to_string(),
        model: "gpt-4o-mini".to_string(),
        provider: "openai".to_string(),
        content: "This is a benchmark test response. ".repeat(20),
        tool_calls: vec![],
        stop_reason: proto::StopReason::Stop as i32,
        stop_sequence: None,
        usage: Some(proto::TokenUsage {
            input_tokens: 50,
            output_tokens: 100,
            total_tokens: 150,
            cache_write_tokens: Some(0),
            cache_read_tokens: Some(0),
            cost_usd: None,
        }),
        provider_metadata: None,
        latency: None,
        warnings: vec![],
    }
}

pub fn build_model_response(proto_response: &proto::ProviderResponse) -> ModelResponse {
    let usage = proto_response.usage.as_ref();
    let input_tokens = usage.map(|usage| usage.input_tokens as usize).unwrap_or(0);
    let cache_read_tokens = usage.and_then(|usage| usage.cache_read_tokens).unwrap_or(0) as usize;

    ModelResponse {
        id: Some(proto_response.response_id.clone()),
        provider: ProviderKind::OpenAi,
        content: proto_response.content.clone(),
        output_text: proto_response.content.clone(),
        tool_calls: proto_response
            .tool_calls
            .iter()
            .map(|tool_call| ModelToolCall {
                id: tool_call.id.clone(),
                name: tool_call.name.clone(),
                arguments_json: tool_call.arguments.clone(),
            })
            .collect(),
        usage: ProviderUsage {
            input_tokens,
            output_tokens: usage.map(|usage| usage.output_tokens as usize).unwrap_or(0),
            total_tokens: usage.map(|usage| usage.total_tokens as usize).unwrap_or(0),
            cache_write_tokens: usage
                .and_then(|usage| usage.cache_write_tokens)
                .unwrap_or(0) as usize,
            cache_read_tokens,
            uncached_input_tokens: input_tokens.saturating_sub(cache_read_tokens),
        },
        stop_reason: stop_reason_label(proto_response.stop_reason),
        provider_request_id: Some(proto_response.request_id.clone()),
        raw: json!({
            "request_id": proto_response.request_id,
            "response_id": proto_response.response_id,
            "provider": proto_response.provider,
            "model": proto_response.model,
            "content": proto_response.content,
        }),
    }
}

pub fn stop_reason_label(stop_reason: i32) -> Option<String> {
    match proto::StopReason::try_from(stop_reason).ok()? {
        proto::StopReason::Unspecified => None,
        proto::StopReason::Stop => Some("stop".to_string()),
        proto::StopReason::Length => Some("length".to_string()),
        proto::StopReason::ToolCall => Some("tool_call".to_string()),
        proto::StopReason::ContentFilter => Some("content_filter".to_string()),
        proto::StopReason::Error => Some("error".to_string()),
    }
}
