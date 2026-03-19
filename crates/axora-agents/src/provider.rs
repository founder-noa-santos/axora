//! Provider abstraction and model-bound payload compaction.

use crate::error::AgentError;
use crate::patch_protocol::ContextPack;
use crate::Result;
use axora_cache::{PrefixCache, PrefixCacheLookup, Schema, ToonSerializer};
use axora_proto as proto;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Supported provider backends.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderKind {
    /// Anthropic Messages API.
    Anthropic,
    /// OpenAI Responses-style API.
    OpenAi,
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
}

/// Typed request sent to any provider adapter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelRequest {
    /// Requested provider.
    pub provider: ProviderKind,
    /// Model identifier.
    pub model: String,
    /// System instructions.
    pub system_instructions: Vec<String>,
    /// Tool schemas encoded as JSON schema objects.
    pub tool_schemas: Vec<Value>,
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
    /// Target provider.
    pub provider: ProviderKind,
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
    /// Text output.
    pub output_text: String,
    /// Token usage.
    pub usage: ProviderUsage,
    /// Stop reason.
    pub stop_reason: Option<String>,
    /// Raw provider payload for auditing.
    pub raw: Value,
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
        let compact: ModelBoundaryPayloadCompact =
            serde_json::from_str(&json).map_err(|err| AgentError::Serialization(err.to_string()))?;
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

/// Anthropic provider adapter.
pub struct AnthropicProvider {
    prefix_cache: PrefixCache,
}

impl AnthropicProvider {
    /// Create a new Anthropic adapter.
    pub fn new(prefix_cache: PrefixCache) -> Self {
        Self { prefix_cache }
    }
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

impl ProviderClient for AnthropicProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Anthropic
    }

    fn prepare_request(&mut self, request: &ModelRequest) -> Result<PreparedProviderRequest> {
        prepare_request(self.kind(), &mut self.prefix_cache, request)
    }

    fn parse_response(&self, response: &Value) -> Result<ModelResponse> {
        parse_anthropic_response(response)
    }
}

impl ProviderClient for OpenAiProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::OpenAi
    }

    fn prepare_request(&mut self, request: &ModelRequest) -> Result<PreparedProviderRequest> {
        prepare_request(self.kind(), &mut self.prefix_cache, request)
    }

    fn parse_response(&self, response: &Value) -> Result<ModelResponse> {
        parse_openai_response(response)
    }
}

fn prepare_request(
    provider: ProviderKind,
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
        &format!("{provider:?}-{}", request.model),
        &static_prefix,
        prefix_tokens,
    );
    let toon_payload = request.payload.to_toon()?;
    let body = match provider {
        ProviderKind::Anthropic => build_anthropic_body(request, &toon_payload, &prefix_lookup),
        ProviderKind::OpenAi => build_openai_body(request, &toon_payload, &prefix_lookup),
    };

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
        provider,
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

fn build_anthropic_body(
    request: &ModelRequest,
    toon_payload: &str,
    prefix_lookup: &PrefixCacheLookup,
) -> Value {
    let mut system_blocks = request
        .system_instructions
        .iter()
        .map(|instruction| json!({"type": "text", "text": instruction}))
        .collect::<Vec<_>>();
    for invariant in &request.invariant_mission_context {
        system_blocks.push(json!({
            "type": "text",
            "text": invariant.to_string()
        }));
    }
    if prefix_lookup.eligible && !system_blocks.is_empty() {
        if let Some(last) = system_blocks.last_mut() {
            last["cache_control"] = anthropic_cache_control(request.cache_retention);
        }
    }

    let mut messages = vec![json!({
        "role": "user",
        "content": [{"type": "text", "text": toon_payload}]
    })];
    for message in &request.recent_messages {
        messages.push(json!({
            "role": message.role,
            "content": [{"type": "text", "text": message.content}]
        }));
    }

    let mut tools = request.tool_schemas.clone();
    if prefix_lookup.eligible && !tools.is_empty() {
        let last = tools.len() - 1;
        if let Some(tool) = tools.get_mut(last) {
            tool["cache_control"] = anthropic_cache_control(request.cache_retention);
        }
    }

    json!({
        "model": request.model,
        "max_tokens": request.max_output_tokens,
        "temperature": request.temperature,
        "stream": request.stream,
        "system": system_blocks,
        "tools": tools,
        "messages": messages
    })
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
        input.push(json!({
            "role": message.role,
            "content": [{"type": "input_text", "text": message.content}]
        }));
    }

    let mut body = json!({
        "model": request.model,
        "input": input,
        "stream": request.stream,
        "max_output_tokens": request.max_output_tokens,
        "tools": request.tool_schemas,
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

fn parse_anthropic_response(response: &Value) -> Result<ModelResponse> {
    let usage = response.get("usage").cloned().unwrap_or(Value::Null);
    let input_tokens = number_field(&usage, "input_tokens");
    let output_tokens = number_field(&usage, "output_tokens");
    let cache_write_tokens = number_field(&usage, "cache_creation_input_tokens");
    let cache_read_tokens = number_field(&usage, "cache_read_input_tokens");
    let content = response
        .get("content")
        .and_then(Value::as_array)
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|block| block.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    Ok(ModelResponse {
        id: response.get("id").and_then(Value::as_str).map(str::to_string),
        provider: ProviderKind::Anthropic,
        output_text: content,
        usage: ProviderUsage {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            cache_write_tokens,
            cache_read_tokens,
            uncached_input_tokens: input_tokens.saturating_sub(cache_read_tokens),
        },
        stop_reason: response
            .get("stop_reason")
            .and_then(Value::as_str)
            .map(str::to_string),
        raw: response.clone(),
    })
}

fn parse_openai_response(response: &Value) -> Result<ModelResponse> {
    let usage = response.get("usage").cloned().unwrap_or(Value::Null);
    let input_tokens = number_field(&usage, "input_tokens").max(number_field(&usage, "prompt_tokens"));
    let output_tokens =
        number_field(&usage, "output_tokens").max(number_field(&usage, "completion_tokens"));
    let cached_tokens = usage
        .get("input_tokens_details")
        .and_then(|details| details.get("cached_tokens"))
        .or_else(|| {
            usage.get("prompt_tokens_details")
                .and_then(|details| details.get("cached_tokens"))
        })
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let output_text = response
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

    Ok(ModelResponse {
        id: response.get("id").and_then(Value::as_str).map(str::to_string),
        provider: ProviderKind::OpenAi,
        output_text,
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
        raw: response.clone(),
    })
}

fn number_field(value: &Value, key: &str) -> usize {
    value.get(key).and_then(Value::as_u64).unwrap_or(0) as usize
}

fn anthropic_cache_control(cache_retention: CacheRetention) -> Value {
    match cache_retention {
        CacheRetention::ProviderDefault | CacheRetention::Short => {
            json!({"type": "ephemeral"})
        }
        CacheRetention::Extended => json!({"type": "ephemeral", "ttl": "1h"}),
    }
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

    fn sample_request(provider: ProviderKind) -> ModelRequest {
        ModelRequest {
            provider,
            model: "test-model".to_string(),
            system_instructions: vec!["Return unified diff only.".to_string()],
            tool_schemas: vec![json!({"name": "apply_patch", "input_schema": {"type": "object"}})],
            invariant_mission_context: vec![json!({"mission":"auth"})],
            payload: ModelBoundaryPayload::from_task_assignment(&sample_assignment(), None),
            recent_messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Focus only on login".to_string(),
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
    fn test_anthropic_request_marks_cache_breakpoint() {
        let mut provider = AnthropicProvider::new(PrefixCache::new(16));
        let prepared = provider.prepare_request(&sample_request(ProviderKind::Anthropic)).unwrap();
        assert_eq!(prepared.provider, ProviderKind::Anthropic);
        assert_eq!(prepared.cache_metrics.requests_eligible_for_caching, 1);
        assert!(prepared.body["system"][1]["cache_control"].is_object());
        assert_eq!(prepared.body["system"][1]["cache_control"]["ttl"], "1h");
    }

    #[test]
    fn test_openai_request_uses_prompt_cache_fields() {
        let mut provider = OpenAiProvider::new(PrefixCache::new(16));
        let prepared = provider.prepare_request(&sample_request(ProviderKind::OpenAi)).unwrap();
        assert_eq!(prepared.provider, ProviderKind::OpenAi);
        assert!(prepared.body.get("prompt_cache_key").is_some());
        assert!(prepared.body.get("prompt_cache_retention").is_some());
    }

    #[test]
    fn test_anthropic_usage_parsing() {
        let response = json!({
            "id": "msg_1",
            "content": [{"type": "text", "text": "--- a.rs\n+++ a.rs\n@@ -1,0 +1,1 @@\n+fn main() {}\n"}],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 1200,
                "output_tokens": 40,
                "cache_creation_input_tokens": 800,
                "cache_read_input_tokens": 0
            }
        });
        let parsed = parse_anthropic_response(&response).unwrap();
        assert_eq!(parsed.usage.cache_write_tokens, 800);
        assert_eq!(parsed.usage.input_tokens, 1200);
    }

    #[test]
    fn test_openai_usage_parsing() {
        let response = json!({
            "id": "resp_1",
            "output_text": "--- a.rs\n+++ a.rs\n@@ -1,0 +1,1 @@\n+fn main() {}\n",
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
    }
}
