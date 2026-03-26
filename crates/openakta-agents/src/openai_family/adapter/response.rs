//! Response denormalization: SDK CreateChatCompletionResponse → internal ModelResponse.

use async_openai::types::CreateChatCompletionResponse;
use serde_json::{self, Value};

use crate::openai_family::error::TransportError;
use crate::provider::{ModelResponse, ModelToolCall, ProviderKind, ProviderUsage};

/// Denormalize SDK response to internal ModelResponse.
pub fn parse_sdk_response(
    sdk: &CreateChatCompletionResponse,
    provider: ProviderKind,
) -> Result<ModelResponse, TransportError> {
    let choice = sdk
        .choices
        .first()
        .ok_or_else(|| TransportError::NoChoicesInResponse)?;

    let content = choice
        .message
        .content
        .clone()
        .map(|c| c.to_string())
        .unwrap_or_default();
    let tool_calls = choice
        .message
        .tool_calls
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|call| ModelToolCall {
            id: call.id,
            name: call.function.name,
            arguments_json: call.function.arguments,
        })
        .collect();

    // Extract usage (handle optional and variant fields)
    let usage = sdk
        .usage
        .as_ref()
        .map(|u| ProviderUsage {
            input_tokens: u.prompt_tokens as usize,
            output_tokens: u.completion_tokens as usize,
            total_tokens: u.total_tokens as usize,
            cache_write_tokens: u
                .prompt_tokens_details
                .as_ref()
                .and_then(|d| d.cached_tokens)
                .map(|v| v as usize)
                .unwrap_or(0),
            cache_read_tokens: u
                .prompt_tokens_details
                .as_ref()
                .and_then(|d| d.cached_tokens)
                .map(|v| v as usize)
                .unwrap_or(0),
            uncached_input_tokens: u.prompt_tokens.saturating_sub(
                u.prompt_tokens_details
                    .as_ref()
                    .and_then(|d| d.cached_tokens)
                    .unwrap_or(0),
            ) as usize,
        })
        .unwrap_or_else(|| ProviderUsage::default());

    Ok(ModelResponse {
        id: Some(sdk.id.clone()),
        provider,
        content: content.clone(),
        output_text: content,
        tool_calls,
        usage,
        stop_reason: choice.finish_reason.as_ref().map(|r| format!("{:?}", r)),
        provider_request_id: Some(sdk.id.clone()),
        raw: serde_json::to_value(sdk).map_err(|e| TransportError::Serialization(e.to_string()))?,
    })
}

/// Parse a raw OpenAI-compatible chat-completions JSON payload.
pub fn parse_raw_chat_completion_response(
    value: &Value,
    provider: ProviderKind,
) -> Result<ModelResponse, TransportError> {
    let choice = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .ok_or(TransportError::NoChoicesInResponse)?;

    let message = choice.get("message").cloned().unwrap_or(Value::Null);
    let content = content_from_message(&message);
    let tool_calls = message
        .get("tool_calls")
        .and_then(Value::as_array)
        .map(|calls| {
            calls
                .iter()
                .filter_map(|call| {
                    let function = call.get("function")?;
                    Some(ModelToolCall {
                        id: call.get("id")?.as_str()?.to_string(),
                        name: function.get("name")?.as_str()?.to_string(),
                        arguments_json: function.get("arguments")?.as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let usage = value.get("usage").cloned().unwrap_or(Value::Null);
    let input_tokens = usage
        .get("prompt_tokens")
        .and_then(Value::as_u64)
        .unwrap_or_default() as usize;
    let output_tokens = usage
        .get("completion_tokens")
        .and_then(Value::as_u64)
        .unwrap_or_default() as usize;
    let total_tokens = usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .unwrap_or((input_tokens + output_tokens) as u64) as usize;
    let cache_read_tokens = usage
        .get("prompt_tokens_details")
        .and_then(|details| details.get("cached_tokens"))
        .and_then(Value::as_u64)
        .unwrap_or_default() as usize;

    Ok(ModelResponse {
        id: value.get("id").and_then(Value::as_str).map(str::to_string),
        provider,
        content: content.clone(),
        output_text: content,
        tool_calls,
        usage: ProviderUsage {
            input_tokens,
            output_tokens,
            total_tokens,
            cache_write_tokens: 0,
            cache_read_tokens,
            uncached_input_tokens: input_tokens.saturating_sub(cache_read_tokens),
        },
        stop_reason: choice
            .get("finish_reason")
            .and_then(Value::as_str)
            .map(str::to_string),
        provider_request_id: value.get("id").and_then(Value::as_str).map(str::to_string),
        raw: value.clone(),
    })
}

fn content_from_message(message: &Value) -> String {
    if let Some(content) = message.get("content").and_then(Value::as_str) {
        return content.to_string();
    }

    message
        .get("content")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| {
                    part.get("text")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                        .or_else(|| {
                            part.get("content")
                                .and_then(Value::as_str)
                                .map(str::to_string)
                        })
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}
