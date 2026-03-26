# Protocol policy

## Core principle

**OpenAI-compatible protocol for all providers.**

```
┌─────────────────────────────────────────────────────────────┐
│  All providers       →  open_ai_chat_completions (default)  │
└─────────────────────────────────────────────────────────────┘
```

## Why?

### OpenAI Chat Completions
- ✅ De facto standard since 2023
- ✅ Universal tooling (LangChain, LiteLLM, etc.)
- ✅ Plenty of documentation
- ✅ Supported by all providers
- ✅ Single code path, simpler maintenance

## Mapping

| Provider | Wire profile | Rationale |
|----------|--------------|------------|
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
# OpenAI — uses native OpenAI protocol
[providers.instances.openai]
profile = "open_ai_compatible"
base_url = "https://api.openai.com/v1"
api_key_file = ".secrets/openai.key"

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

The code uses a single path:

```rust
match wire_profile {
    WireProfile::OpenAiChatCompletions => build_openai_body(request),
}
```

That is the whole rule.
