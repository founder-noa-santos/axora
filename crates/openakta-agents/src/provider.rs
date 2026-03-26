//! Provider abstraction and model-bound payload compaction.

use crate::error::AgentError;
use crate::patch_protocol::ContextPack;
use crate::Result;
use openakta_cache::{PrefixCache, PrefixCacheLookup, Schema, ToonSerializer};
use openakta_proto as proto;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Telemetry-only provider identifier for metrics and logging.
///
/// This enum identifies the provider for telemetry purposes only.
/// For transport/wire protocol selection, use `WireProfile` instead.
///
/// ## Anthropic Removal Note
///
/// Anthropic support has been intentionally removed from aktacode.
/// Future provider integrations (including Anthropic) must be implemented
/// behind openakta-api, not directly in aktacode.
///
/// aktacode currently supports only OpenAI and OpenAI-compatible providers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderKind {
    /// OpenAI API (including compatible providers).
    OpenAi,
    /// DeepSeek API (OpenAI-compatible).
    DeepSeek,
    /// Qwen/DashScope API (OpenAI-compatible).
    Qwen,
    /// Moonshot API (OpenAI-compatible).
    Moonshot,
    /// Ollama local API (OpenAI-compatible).
    Ollama,
}

impl ProviderKind {
    /// Get the display name for this provider.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::OpenAi => "openai",
            ProviderKind::DeepSeek => "deepseek",
            ProviderKind::Qwen => "qwen",
            ProviderKind::Moonshot => "moonshot",
            ProviderKind::Ollama => "ollama",
        }
    }
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Cache scope for prompt segments.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PromptCacheScope {
    /// System instructions.
    System,
    /// Tool schemas.
    Tools,
    /// Invariant mission context.
    InvariantContext,
    /// Dynamic task payload.
    Dynamic,
}

/// OpenAI cache retention preference.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CacheRetention {
    /// Provider default retention.
    ProviderDefault,
    /// Short-lived cache entry.
    Short,
    /// Longer-lived cache entry.
    Extended,
}

/// Model-bound payload type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelBoundaryPayloadType {
    /// Task assignment and context.
    TaskExecution,
    /// Retrieval-only payload.
    Retrieval,
}

/// Prompt segment metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptSegment {
    /// Segment scope.
    pub scope: PromptCacheScope,
    /// Raw content.
    pub content: String,
    /// Whether this segment is eligible for provider caching.
    pub cacheable: bool,
}

/// Shared chat message model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    /// Message role.
    pub role: String,
    /// Message text content.
    pub content: String,
    /// Optional sender/tool name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Tool call id for tool-result messages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Tool calls emitted by the assistant for loop continuation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ModelToolCall>,
}

/// Structured tool schema exposed to the model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelToolSchema {
    /// Tool name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON schema parameters contract.
    pub parameters: Value,
    /// Whether provider-side strict schema validation is preferred.
    #[serde(default)]
    pub strict: bool,
    /// Canonical tool kind label for runtime/UI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_kind: Option<String>,
    /// Whether the tool is read only.
    #[serde(default)]
    pub read_only: bool,
    /// Whether the tool mutates workspace or state.
    #[serde(default)]
    pub mutating: bool,
    /// Whether approval is required before execution.
    #[serde(default)]
    pub requires_approval: bool,
    /// Preferred renderer for UI traces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_renderer: Option<String>,
}

/// Structured model-emitted tool call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelToolCall {
    /// Provider-generated call id.
    pub id: String,
    /// Canonical tool name.
    pub name: String,
    /// Raw JSON argument payload as a string.
    pub arguments_json: String,
}

/// Typed request sent to any provider adapter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelRequest {
    /// Wire protocol profile for request building.
    pub provider: crate::wire_profile::WireProfile,
    /// Model identifier.
    pub model: String,
    /// System instructions.
    pub system_instructions: Vec<String>,
    /// Tool schemas exposed to the model.
    pub tool_schemas: Vec<ModelToolSchema>,
    /// Invariant mission context eligible for caching.
    pub invariant_mission_context: Vec<Value>,
    /// Dynamic model-bound payload.
    pub payload: ModelBoundaryPayload,
    /// Recent messages not eligible for prefix caching.
    pub recent_messages: Vec<ChatMessage>,
    /// Maximum output tokens.
    pub max_output_tokens: u32,
    /// Optional temperature.
    pub temperature: Option<f32>,
    /// Whether the provider should stream responses.
    pub stream: bool,
    /// Preferred prompt cache retention for providers that support it.
    pub cache_retention: CacheRetention,
}

/// Shared provider usage accounting.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderUsage {
    /// Input tokens reported by the provider.
    pub input_tokens: usize,
    /// Output tokens reported by the provider.
    pub output_tokens: usize,
    /// Total tokens reported by the provider.
    pub total_tokens: usize,
    /// Tokens written into provider-side prompt cache.
    pub cache_write_tokens: usize,
    /// Tokens read from provider-side prompt cache.
    pub cache_read_tokens: usize,
    /// Input tokens billed without cache benefit.
    pub uncached_input_tokens: usize,
}

/// Cache metrics aggregated at the provider builder layer.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheMetrics {
    /// Number of requests with a deterministic cacheable prefix.
    pub requests_eligible_for_caching: usize,
    /// Tokens written into cache.
    pub cache_write_tokens: usize,
    /// Tokens read from cache.
    pub cache_read_tokens: usize,
    /// Input tokens that remained uncached.
    pub uncached_input_tokens: usize,
    /// Effective tokens saved by prompt caching.
    pub effective_tokens_saved: usize,
    /// Local prefix token count.
    pub prefix_tokens: usize,
    /// Local prefix cache key.
    pub prefix_cache_key: Option<String>,
    /// Cold request latency in milliseconds when measured.
    pub cold_latency_ms: Option<u64>,
    /// Warm request latency in milliseconds when measured.
    pub warm_latency_ms: Option<u64>,
}

impl CacheMetrics {
    /// Record a latency sample pair.
    pub fn with_latency(mut self, cold_ms: u64, warm_ms: u64) -> Self {
        self.cold_latency_ms = Some(cold_ms);
        self.warm_latency_ms = Some(warm_ms);
        self
    }

    /// Return the latency delta when both measurements are present.
    pub fn latency_delta_ms(&self) -> Option<i64> {
        match (self.cold_latency_ms, self.warm_latency_ms) {
            (Some(cold), Some(warm)) => Some(cold as i64 - warm as i64),
            _ => None,
        }
    }
}

/// Prepared provider request body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreparedProviderRequest {
    /// Wire protocol profile used for request building.
    pub wire_profile: crate::wire_profile::WireProfile,
    /// Final request body.
    pub body: Value,
    /// TOON payload sent to the model boundary.
    pub toon_payload: String,
    /// Prompt segments used to build the request.
    pub prompt_segments: Vec<PromptSegment>,
    /// Local prefix cache lookup result.
    pub prefix_lookup: PrefixCacheLookup,
    /// Cache metrics derived during request construction.
    pub cache_metrics: CacheMetrics,
}

/// Shared non-streamed response model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelResponse {
    /// Provider-specific response identifier.
    pub id: Option<String>,
    /// Provider kind.
    pub provider: ProviderKind,
    /// Final assistant text content.
    pub content: String,
    /// Compatibility mirror of [`Self::content`] for older call sites.
    pub output_text: String,
    /// Structured tool calls returned by the model.
    #[serde(default)]
    pub tool_calls: Vec<ModelToolCall>,
    /// Token usage.
    pub usage: ProviderUsage,
    /// Stop reason.
    pub stop_reason: Option<String>,
    /// Provider request id/correlation id when available.
    pub provider_request_id: Option<String>,
    /// Raw provider payload for auditing.
    pub raw: Value,
}

impl ModelResponse {
    /// Return the final assistant text content.
    pub fn assistant_text(&self) -> &str {
        &self.content
    }
}

/// Shared streamed response event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelResponseChunk {
    /// Provider kind.
    pub provider: ProviderKind,
    /// Delta content.
    pub delta: String,
    /// Whether the stream is complete.
    pub done: bool,
}

/// Typed payload converted to TOON at the model boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelBoundaryPayload {
    /// Payload type.
    pub payload_type: ModelBoundaryPayloadType,
    /// Task identifier.
    pub task_id: String,
    /// Task title.
    pub title: String,
    /// Task description.
    pub description: String,
    /// Task type.
    pub task_type: String,
    /// Target files.
    pub target_files: Vec<String>,
    /// Target symbols.
    pub target_symbols: Vec<String>,
    /// Context spans rendered for the model.
    pub context_spans: Vec<String>,
    /// Optional compact context pack.
    pub context_pack: Option<ContextPack>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ModelBoundaryPayloadCompact {
    payload_type: ModelBoundaryPayloadType,
    task_id: String,
    title: String,
    description: String,
    task_type: String,
    target_files: String,
    target_symbols: String,
    context_spans: String,
    context_pack: Option<String>,
}

impl ModelBoundaryPayload {
    /// Build a model payload from a typed task assignment and optional context pack.
    pub fn from_task_assignment(
        assignment: &proto::TaskAssignment,
        context_pack: Option<ContextPack>,
    ) -> Self {
        let proto_pack = assignment.context_pack.as_ref();
        let target_files = if assignment.target_files.is_empty() {
            proto_pack
                .map(|pack| pack.target_files.clone())
                .unwrap_or_default()
        } else {
            assignment.target_files.clone()
        };
        let target_symbols = if assignment.target_symbols.is_empty() {
            proto_pack
                .map(|pack| pack.symbols.clone())
                .unwrap_or_default()
        } else {
            assignment.target_symbols.clone()
        };
        let context_spans = proto_pack
            .map(|pack| {
                pack.spans
                    .iter()
                    .map(|span| {
                        format!(
                            "{}:{}-{}:{}",
                            span.file_path, span.start_line, span.end_line, span.symbol_path
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Self {
            payload_type: ModelBoundaryPayloadType::TaskExecution,
            task_id: assignment.task_id.clone(),
            title: assignment.title.clone(),
            description: assignment.description.clone(),
            task_type: proto::TaskPayloadType::try_from(assignment.task_type)
                .map(|task_type| format!("{task_type:?}"))
                .unwrap_or_else(|_| "TASK_PAYLOAD_TYPE_UNSPECIFIED".to_string()),
            target_files,
            target_symbols,
            context_spans,
            context_pack,
        }
    }

    fn into_compact(self) -> ModelBoundaryPayloadCompact {
        ModelBoundaryPayloadCompact {
            payload_type: self.payload_type,
            task_id: self.task_id,
            title: self.title,
            description: self.description,
            task_type: self.task_type,
            target_files: self.target_files.join("\n"),
            target_symbols: self.target_symbols.join("\n"),
            context_spans: self.context_spans.join("\n"),
            context_pack: self
                .context_pack
                .map(|pack| serde_json::to_string(&pack).unwrap_or_default()),
        }
    }

    fn from_compact(compact: ModelBoundaryPayloadCompact) -> Result<Self> {
        Ok(Self {
            payload_type: compact.payload_type,
            task_id: compact.task_id,
            title: compact.title,
            description: compact.description,
            task_type: compact.task_type,
            target_files: split_compact_lines(&compact.target_files),
            target_symbols: split_compact_lines(&compact.target_symbols),
            context_spans: split_compact_lines(&compact.context_spans),
            context_pack: compact
                .context_pack
                .filter(|value| !value.trim().is_empty())
                .map(|value| {
                    serde_json::from_str(&value)
                        .map_err(|err| AgentError::Serialization(err.to_string()))
                })
                .transpose()?,
        })
    }

    fn schema() -> Schema {
        let mut schema = Schema::new();
        for field in [
            "payload_type",
            "task_id",
            "title",
            "description",
            "task_type",
            "target_files",
            "target_symbols",
            "context_spans",
            "context_pack",
        ] {
            schema.add_field(field);
        }
        schema
    }

    /// Serialize this payload to TOON.
    pub fn to_toon(&self) -> Result<String> {
        let json = serde_json::to_string(&self.clone().into_compact())
            .map_err(|err| AgentError::Serialization(err.to_string()))?;
        ToonSerializer::new(Self::schema())
            .encode(&json)
            .map_err(|err| AgentError::Serialization(err.to_string()).into())
    }

    /// Decode a TOON payload into the typed model-bound payload.
    pub fn from_toon(toon: &str) -> Result<Self> {
        let serializer = ToonSerializer::new(Self::schema());
        let json = serializer
            .decode(toon)
            .map_err(|err| AgentError::Serialization(err.to_string()))?;
        let compact: ModelBoundaryPayloadCompact = serde_json::from_str(&json)
            .map_err(|err| AgentError::Serialization(err.to_string()))?;
        Self::from_compact(compact)
    }
}

fn split_compact_lines(value: &str) -> Vec<String> {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// Shared provider client behavior.
pub trait ProviderClient {
    /// Provider kind handled by this client.
    fn kind(&self) -> ProviderKind;

    /// Build a provider request from the shared request model.
    fn prepare_request(&mut self, request: &ModelRequest) -> Result<PreparedProviderRequest>;

    /// Parse a provider JSON response into the shared response model.
    fn parse_response(&self, response: &Value) -> Result<ModelResponse>;
}

/// OpenAI provider adapter.
pub struct OpenAiProvider {
    prefix_cache: PrefixCache,
}

impl OpenAiProvider {
    /// Create a new OpenAI adapter.
    pub fn new(prefix_cache: PrefixCache) -> Self {
        Self { prefix_cache }
    }
}

impl ProviderClient for OpenAiProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::OpenAi
    }

    fn prepare_request(&mut self, request: &ModelRequest) -> Result<PreparedProviderRequest> {
        prepare_request(request.provider, &mut self.prefix_cache, request)
    }

    fn parse_response(&self, response: &Value) -> Result<ModelResponse> {
        parse_openai_response(response)
    }
}

fn prepare_request(
    wire_profile: crate::wire_profile::WireProfile,
    prefix_cache: &mut PrefixCache,
    request: &ModelRequest,
) -> Result<PreparedProviderRequest> {
    let segments = build_segments(request)?;
    let static_prefix = segments
        .iter()
        .filter(|segment| segment.cacheable)
        .map(|segment| segment.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let prefix_tokens = estimate_tokens(&static_prefix);
    let prefix_lookup = prefix_cache.lookup_or_insert(
        &format!("{}-{}", wire_profile, request.model),
        &static_prefix,
        prefix_tokens,
    );
    let toon_payload = request.payload.to_toon()?;
    // All providers now use OpenAI-compatible format
    let body = build_openai_body(request, &toon_payload, &prefix_lookup);

    let cache_metrics = CacheMetrics {
        requests_eligible_for_caching: usize::from(prefix_lookup.eligible),
        cache_write_tokens: usize::from(!prefix_lookup.hit) * prefix_tokens,
        cache_read_tokens: usize::from(prefix_lookup.hit) * prefix_tokens,
        uncached_input_tokens: estimate_tokens(&toon_payload)
            + request
                .recent_messages
                .iter()
                .map(|message| estimate_tokens(&message.content))
                .sum::<usize>(),
        effective_tokens_saved: usize::from(prefix_lookup.hit) * prefix_tokens,
        prefix_tokens,
        prefix_cache_key: Some(prefix_lookup.cache_key.clone()),
        cold_latency_ms: None,
        warm_latency_ms: None,
    };

    Ok(PreparedProviderRequest {
        wire_profile,
        body,
        toon_payload,
        prompt_segments: segments,
        prefix_lookup,
        cache_metrics,
    })
}

fn build_segments(request: &ModelRequest) -> Result<Vec<PromptSegment>> {
    let mut segments = Vec::new();
    for instruction in &request.system_instructions {
        segments.push(PromptSegment {
            scope: PromptCacheScope::System,
            content: instruction.clone(),
            cacheable: true,
        });
    }
    for tool in &request.tool_schemas {
        segments.push(PromptSegment {
            scope: PromptCacheScope::Tools,
            content: serde_json::to_string(tool)
                .map_err(|err| AgentError::Serialization(err.to_string()))?,
            cacheable: true,
        });
    }
    for invariant in &request.invariant_mission_context {
        segments.push(PromptSegment {
            scope: PromptCacheScope::InvariantContext,
            content: serde_json::to_string(invariant)
                .map_err(|err| AgentError::Serialization(err.to_string()))?,
            cacheable: true,
        });
    }
    for message in &request.recent_messages {
        segments.push(PromptSegment {
            scope: PromptCacheScope::Dynamic,
            content: message.content.clone(),
            cacheable: false,
        });
    }
    Ok(segments)
}

fn build_openai_body(
    request: &ModelRequest,
    toon_payload: &str,
    prefix_lookup: &PrefixCacheLookup,
) -> Value {
    let mut input = request
        .system_instructions
        .iter()
        .map(|instruction| {
            json!({
                "role": "system",
                "content": [{"type": "input_text", "text": instruction}]
            })
        })
        .collect::<Vec<_>>();

    if !request.invariant_mission_context.is_empty() {
        input.push(json!({
            "role": "system",
            "content": [{"type": "input_text", "text": Value::Array(request.invariant_mission_context.clone()).to_string()}]
        }));
    }

    input.push(json!({
        "role": "user",
        "content": [{"type": "input_text", "text": toon_payload}]
    }));

    for message in &request.recent_messages {
        let mut item = json!({
            "role": message.role,
            "content": [{"type": "input_text", "text": message.content}]
        });
        if let Some(name) = &message.name {
            item["name"] = json!(name);
        }
        if let Some(tool_call_id) = &message.tool_call_id {
            item["tool_call_id"] = json!(tool_call_id);
        }
        if !message.tool_calls.is_empty() {
            item["tool_calls"] = json!(message.tool_calls);
        }
        input.push(item);
    }

    let tools = request
        .tool_schemas
        .iter()
        .map(|tool| serde_json::to_value(tool).unwrap_or_else(|_| Value::Null))
        .collect::<Vec<_>>();

    let mut body = json!({
        "model": request.model,
        "input": input,
        "stream": request.stream,
        "max_output_tokens": request.max_output_tokens,
        "tools": tools,
    });

    if prefix_lookup.eligible {
        body["prompt_cache_key"] = json!(prefix_lookup.cache_key);
        body["prompt_cache_retention"] = json!(match request.cache_retention {
            CacheRetention::ProviderDefault => "provider_default",
            CacheRetention::Short => "short",
            CacheRetention::Extended => "extended",
        });
    }

    if let Some(temperature) = request.temperature {
        body["temperature"] = json!(temperature);
    }

    body
}

fn parse_openai_response(response: &Value) -> Result<ModelResponse> {
    let usage = response.get("usage").cloned().unwrap_or(Value::Null);
    let input_tokens =
        number_field(&usage, "input_tokens").max(number_field(&usage, "prompt_tokens"));
    let output_tokens =
        number_field(&usage, "output_tokens").max(number_field(&usage, "completion_tokens"));
    let cached_tokens = usage
        .get("input_tokens_details")
        .and_then(|details| details.get("cached_tokens"))
        .or_else(|| {
            usage
                .get("prompt_tokens_details")
                .and_then(|details| details.get("cached_tokens"))
        })
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let content = response
        .get("output_text")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            response
                .get("output")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(|item| item.get("content"))
                .and_then(Value::as_array)
                .and_then(|blocks| blocks.first())
                .and_then(|block| block.get("text"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_default();
    let tool_calls = parse_tool_calls(response);
    let provider_request_id = response
        .get("request_id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            response
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string)
        });

    Ok(ModelResponse {
        id: response
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string),
        provider: ProviderKind::OpenAi,
        content: content.clone(),
        output_text: content,
        tool_calls,
        usage: ProviderUsage {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            cache_write_tokens: 0,
            cache_read_tokens: cached_tokens,
            uncached_input_tokens: input_tokens.saturating_sub(cached_tokens),
        },
        stop_reason: response
            .get("stop_reason")
            .and_then(Value::as_str)
            .map(str::to_string),
        provider_request_id,
        raw: response.clone(),
    })
}

fn parse_tool_calls(response: &Value) -> Vec<ModelToolCall> {
    if let Some(tool_calls) = response.get("tool_calls").and_then(Value::as_array) {
        return tool_calls
            .iter()
            .filter_map(parse_tool_call_value)
            .collect();
    }

    response
        .get("output")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter(|item| item.get("type").and_then(Value::as_str) == Some("function_call"))
                .filter_map(parse_tool_call_value)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_tool_call_value(value: &Value) -> Option<ModelToolCall> {
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let name = value
        .get("name")
        .or_else(|| value.get("tool_name"))
        .or_else(|| {
            value
                .get("function")
                .and_then(|function| function.get("name"))
        })
        .and_then(Value::as_str)?
        .to_string();
    let arguments_json = value
        .get("arguments")
        .or_else(|| value.get("arguments_json"))
        .or_else(|| {
            value
                .get("function")
                .and_then(|function| function.get("arguments"))
        })
        .map(|arguments| match arguments {
            Value::String(raw) => raw.clone(),
            other => other.to_string(),
        })
        .unwrap_or_else(|| "{}".to_string());

    Some(ModelToolCall {
        id: if id.is_empty() {
            format!("call_{}", uuid::Uuid::new_v4())
        } else {
            id
        },
        name,
        arguments_json,
    })
}

fn number_field(value: &Value, key: &str) -> usize {
    value.get(key).and_then(Value::as_u64).unwrap_or(0) as usize
}

fn estimate_tokens(content: &str) -> usize {
    content.len() / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_assignment() -> proto::TaskAssignment {
        proto::TaskAssignment {
            task_id: "task-1".to_string(),
            title: "Implement auth patch".to_string(),
            description: "Update src/auth.rs".to_string(),
            task_type: proto::TaskPayloadType::CodeModification as i32,
            target_files: vec!["src/auth.rs".to_string()],
            target_symbols: vec!["auth::login".to_string()],
            token_budget: 1200,
            context_pack: Some(proto::ContextPack {
                id: "ctx-1".to_string(),
                task_id: "task-1".to_string(),
                target_files: vec!["src/auth.rs".to_string()],
                symbols: vec!["auth::login".to_string()],
                spans: vec![proto::ContextSpan {
                    file_path: "src/auth.rs".to_string(),
                    start_line: 10,
                    end_line: 24,
                    symbol_path: "auth::login".to_string(),
                }],
                toon_payload: String::new(),
                base_revision: "rev-1".to_string(),
                meta_glyph_commands: vec![],
                compression_mode: proto::CompressionMode::Toon as i32,
                latent_context: Vec::new(),
                latent_context_handle: String::new(),
                cryptographic_signature: Vec::new(),
                audit_correlation_id: String::new(),
            }),
        }
    }

    fn sample_request(wire_profile: crate::wire_profile::WireProfile) -> ModelRequest {
        ModelRequest {
            provider: wire_profile,
            model: "test-model".to_string(),
            system_instructions: vec!["Return unified diff only.".to_string()],
            tool_schemas: vec![ModelToolSchema {
                name: "apply_patch".to_string(),
                description: "Apply a patch".to_string(),
                parameters: json!({"type": "object"}),
                strict: false,
                tool_kind: Some("command".to_string()),
                read_only: false,
                mutating: true,
                requires_approval: true,
                ui_renderer: Some("tool_call".to_string()),
            }],
            invariant_mission_context: vec![json!({"mission":"auth"})],
            payload: ModelBoundaryPayload::from_task_assignment(&sample_assignment(), None),
            recent_messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Focus only on login".to_string(),
                name: None,
                tool_call_id: None,
                tool_calls: Vec::new(),
            }],
            max_output_tokens: 512,
            temperature: Some(0.1),
            stream: false,
            cache_retention: CacheRetention::Extended,
        }
    }

    #[test]
    fn test_model_boundary_toon_roundtrip() {
        let payload = ModelBoundaryPayload::from_task_assignment(&sample_assignment(), None);
        let toon = payload.to_toon().unwrap();
        let decoded = ModelBoundaryPayload::from_toon(&toon).unwrap();
        assert_eq!(decoded.task_id, "task-1");
        assert_eq!(decoded.target_files, vec!["src/auth.rs".to_string()]);
    }

    #[test]
    fn test_openai_request_uses_prompt_cache_fields() {
        let mut provider = OpenAiProvider::new(PrefixCache::new(16));
        let prepared = provider
            .prepare_request(&sample_request(
                crate::wire_profile::WireProfile::OpenAiChatCompletions,
            ))
            .unwrap();
        assert_eq!(
            prepared.wire_profile,
            crate::wire_profile::WireProfile::OpenAiChatCompletions
        );
        assert!(prepared.body.get("prompt_cache_key").is_some());
        assert!(prepared.body.get("prompt_cache_retention").is_some());
    }

    #[test]
    fn test_openai_usage_parsing() {
        let response = json!({
            "id": "resp_1",
            "output_text": "--- a.rs\n+++ a.rs\n@@ -1,0 +1,1 @@\n+fn main() {}\n",
            "tool_calls": [{
                "id": "call_1",
                "name": "read_file",
                "arguments": "{\"path\":\"src/main.rs\"}"
            }],
            "usage": {
                "input_tokens": 1000,
                "output_tokens": 30,
                "input_tokens_details": {
                    "cached_tokens": 700
                }
            }
        });
        let parsed = parse_openai_response(&response).unwrap();
        assert_eq!(parsed.usage.cache_read_tokens, 700);
        assert_eq!(parsed.usage.uncached_input_tokens, 300);
        assert_eq!(parsed.tool_calls.len(), 1);
        assert_eq!(parsed.tool_calls[0].name, "read_file");
    }
}
