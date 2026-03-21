# Política de Protocolos

## Princípio Fundamental

**Anthropic/Claude usa protocolo nativo. Todo mundo else usa OpenAI-compatible.**

```
┌─────────────────────────────────────────────────────────────┐
│  Anthropic/Claude  →  anthropic_messages_v1  (nativo)      │
│  Todos os outros   →  open_ai_chat_completions (padrão)    │
└─────────────────────────────────────────────────────────────┘
```

## Por que?

### Anthropic Messages API
- ✅ Prompt caching (mais barato)
- ✅ PDF input nativo
- ✅ 200k context otimizado
- ⚠️ Menos tooling disponível

### OpenAI Chat Completions
- ✅ Padrão de fato desde 2023
- ✅ Tooling universal (LangChain, LiteLLM, etc.)
- ✅ Documentação abundante
- ✅ Todos os providers oferecem

## Mapeamento

| Provider | Wire Profile | Por que |
|----------|-------------|---------|
| **Anthropic** | `AnthropicMessagesV1` | É o único que vale a pena ter protocolo nativo |
| **OpenAI** | `OpenAiChatCompletions` | O padrão original |
| **DeepSeek** | `OpenAiChatCompletions` | Só oferecem OpenAI-compatible |
| **Qwen/Alibaba** | `OpenAiChatCompletions` | Só oferecem OpenAI-compatible |
| **Moonshot** | `OpenAiChatCompletions` | Só oferecem OpenAI-compatible |
| **Kimi** | `OpenAiChatCompletions` | Só oferecem OpenAI-compatible |
| **Gemini** | `OpenAiChatCompletions` | OpenAI-compatible é mais estável |
| **Mistral** | `OpenAiChatCompletions` | Só oferecem OpenAI-compatible |
| **Ollama** | `OpenAiChatCompletions` | É OpenAI-compatible |
| **OpenRouter** | `OpenAiChatCompletions` | É o produto deles |
| **Groq** | `OpenAiChatCompletions` | Só oferecem OpenAI-compatible |
| **Together** | `OpenAiChatCompletions` | Só oferecem OpenAI-compatible |
| **Fireworks** | `OpenAiChatCompletions` | Só oferecem OpenAI-compatible |
| **Perplexity** | `OpenAiChatCompletions` | Só oferecem OpenAI-compatible |

## Configuração

```toml
# Anthropic - único que usa nativo
[providers.instances.anthropic]
profile = "anthropic_messages_v1"
base_url = "https://api.anthropic.com"
api_key_file = ".secrets/anthropic.key"

# Todos os outros usam open_ai_compatible
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

## Implementação

O código decide automaticamente:

```rust
match wire_profile {
    WireProfile::AnthropicMessagesV1 => build_anthropic_body(request),
    WireProfile::OpenAiChatCompletions => build_openai_body(request),
}
```

Simples assim.
