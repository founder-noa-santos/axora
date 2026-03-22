# Protocol policy

## Core principle

**Anthropic/Claude uses the native protocol. Everyone else uses OpenAI-compatible.**

```
┌─────────────────────────────────────────────────────────────┐
│  Anthropic/Claude  →  anthropic_messages_v1  (native)       │
│  All others        →  open_ai_chat_completions (default)    │
└─────────────────────────────────────────────────────────────┘
```

## Why?

### Anthropic Messages API
- ✅ Prompt caching (lower cost)
- ✅ Native PDF input
- ✅ 200k context tuned for Claude
- ⚠️ Less third-party tooling

### OpenAI Chat Completions
- ✅ De facto standard since 2023
- ✅ Universal tooling (LangChain, LiteLLM, etc.)
- ✅ Plenty of documentation
- ✅ Supported by all listed providers

## Mapping

| Provider | Wire profile | Rationale |
|----------|--------------|------------|
| **Anthropic** | `AnthropicMessagesV1` | Only provider worth a native wire protocol |
| **OpenAI** | `OpenAiChatCompletions` | Original standard |
| **DeepSeek** | `OpenAiChatCompletions` | OpenAI-compatible only |
| **Qwen/Alibaba** | `OpenAiChatCompletions` | OpenAI-compatible only |
| **Moonshot** | `OpenAiChatCompletions` | OpenAI-compatible only |
| **Kimi** | `OpenAiChatCompletions` | OpenAI-compatible only |
| **Gemini** | `OpenAiChatCompletions` | OpenAI-compatible is more stable |
| **Mistral** | `OpenAiChatCompletions` | OpenAI-compatible only |
| **Ollama** | `OpenAiChatCompletions` | OpenAI-compatible |
| **OpenRouter** | `OpenAiChatCompletions` | Core product surface |
| **Groq** | `OpenAiChatCompletions` | OpenAI-compatible only |
| **Together** | `OpenAiChatCompletions` | OpenAI-compatible only |
| **Fireworks** | `OpenAiChatCompletions` | OpenAI-compatible only |
| **Perplexity** | `OpenAiChatCompletions` | OpenAI-compatible only |

## Configuration

```toml
# Anthropic — only provider using native wire
[providers.instances.anthropic]
profile = "anthropic_messages_v1"
base_url = "https://api.anthropic.com"
api_key_file = ".secrets/anthropic.key"

# Everyone else uses open_ai_compatible
[providers.instances.deepseek]
profile = "open_ai_compatible"
base_url = "https://api.deepseek.com"
api_key_file = ".secrets/deepseek.key"

[providers.instances.qwen]
profile = "open_ai_compatible"
base_url = "https://dashscope.aliyuncs.com/compatible-mode/v1"
api_key_file = ".secrets/qwen.key"

[providers.instances.moonshot]
profile = "open_ai_compatible"
base_url = "https://api.moonshot.cn/v1"
api_key_file = ".secrets/moonshot.key"

[providers.instances.ollama]
profile = "open_ai_compatible"
base_url = "http://localhost:11434"
is_local = true
```

## Implementation

The code chooses automatically:

```rust
match wire_profile {
    WireProfile::AnthropicMessagesV1 => build_anthropic_body(request),
    WireProfile::OpenAiChatCompletions => build_openai_body(request),
}
```

That is the whole rule.
