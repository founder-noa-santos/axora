# Catalog Registry

Sistema de registro de providers e modelos LLM para o OPENAKTA.

## Visão Geral

O sistema consome catálogos JSON estáticos hospedados (ex: GitHub Pages) para obter metadados de providers e modelos LLM. Isso permite:

1. **Atualizações sem deploy**: Novos providers/modelos são adicionados atualizando o JSON
2. **Validação cruzada**: Verifica consistência entre providers e modelos
3. **Capacidades efetivas**: Interseção das capacidades do provider e do modelo
4. **Caching local**: Cache com TTL para funcionamento offline

## Estrutura de Arquivos

```
data-artifacts/
├── providers/v1.json    # Metadados de providers
├── models/v1.json       # Catálogo de modelos
└── schemas/             # Schemas de validação (opcional)
    ├── provider-config.schema.json
    └── model-catalog.schema.json
```

## Configuração

No `axora.toml` (ou similar):

```toml
[remote_registry]
url = "https://openakta.github.io/data-artifacts"
poll_interval_secs = 86400  # 24 horas
http_timeout_secs = 10
```

## Uso no Código

```rust
use openakta_agents::catalog_registry::{CatalogRegistry, RegistryConfig};
use std::time::Duration;

// Configurar
let config = RegistryConfig {
    base_url: "https://openakta.github.io/data-artifacts".to_string(),
    timeout: Duration::from_secs(10),
    cache_ttl: Duration::from_secs(86400),
    allow_partial: true,
};

// Criar registry
let mut registry = CatalogRegistry::new(config);

// Obter snapshot (do cache ou fetch)
let snapshot = registry.get_registry().await?;

// Consultar provider
if let Some(provider) = snapshot.get_provider("openai") {
    println!("Provider: {}", provider.label);
    println!("Base URL: {}", provider.api.base_url);
}

// Listar modelos de um provider
let models = snapshot.list_models_for_provider("openai");
for model in models {
    println!("  - {}: {} tokens context", model.label, model.context_window_tokens);
}

// Verificar capacidades efetivas
let caps = snapshot.get_effective_capabilities("openai", "gpt-4o");
println!("Streaming: {}", caps.streaming);
println!("Tool calls: {}", caps.tool_calls);
println!("Accepts images: {}", caps.accepts_images);

// Resolver adapter hint
match snapshot.resolve_adapter_hint("openai") {
    AdapterHint::Supported { adapter_id, surface } => {
        println!("Use adapter: {} (surface: {:?})", adapter_id, surface);
    }
    AdapterHint::Unknown { reason } => {
        println!("Unknown adapter: {}", reason);
    }
    _ => {}
}
```

## Cross-Validation

O sistema valida automaticamente:

1. **IDs únicos**: Não pode haver providers ou modelos duplicados
2. **Referências válidas**: Todo modelo deve referenciar um provider existente
3. **Modelos suportados**: `supported_model_ids` deve referenciar modelos existentes
4. **Modelo padrão**: `default_model_id` deve existir e pertencer ao provider
5. **Capacidades**: Se provider desabilita uma feature, modelo não pode afirmar que suporta

## Versionamento

- **URL**: `v1.json`, `v2.json`, etc. para breaking changes
- **Schema**: Campo `schema_version` no JSON (semver)
- **Compatibilidade**: App aceita schema 1.x.x, rejeita 2.x.x

## Providers Suportados

### Diretos
- **OpenAI**: GPT-4o, GPT-4o-mini, o1, o3-mini
- **Anthropic**: Claude 3.5 Sonnet, Claude 3 Opus
- **DeepSeek**: deepseek-chat, deepseek-coder

### Self-Hosted
- **Ollama**: Llama, Qwen, CodeLlama, Mistral (local)

### Agregadores
- **OpenRouter**: Acesso a múltiplos providers via API unificada

## Adicionando Novo Provider

1. Adicionar entry em `providers/v1.json`
2. Adicionar modelos correspondentes em `models/v1.json`
3. Validar com `cargo test` (testes de integração)

Exemplo de novo provider:

```json
{
  "id": "novo-provider",
  "vendor_slug": "novo-vendor",
  "label": "Novo Provider",
  "description": "Descrição do provider",
  "status": "active",
  "provider_type": "direct",
  "api": {
    "base_url": "https://api.novoprovider.com/v1",
    "compatibility": {
      "family": "open_ai",
      "surface": "chat_completions",
      "strictness": "compatible"
    }
  },
  "authentication": {
    "scheme": "bearer",
    "env_var_hint": "NOVO_PROVIDER_API_KEY"
  },
  "capabilities": {
    "chat": true,
    "streaming": true,
    "embeddings": false,
    "tool_calls": true
  },
  "supported_model_ids": ["modelo-1", "modelo-2"]
}
```

## Diagnósticos

O registry mantém diagnósticos de validação:

```rust
let snapshot = registry.get_registry().await?;
println!("Warnings: {:?}", snapshot.diagnostics.warnings);
println!("Errors: {:?}", snapshot.diagnostics.errors);
println!("Providers válidos: {}", snapshot.diagnostics.providers_valid);
println!("Models válidos: {}", snapshot.diagnostics.models_valid);
```

## Cache

- Cache em memória com TTL configurável
- `get_registry()` retorna do cache se válido
- `refresh()` força re-download
- Cache é transparente para o usuário

## Tratamento de Erros

- **FetchFailed**: Problema de rede
- **ParseError**: JSON inválido
- **ValidationError**: Dados inconsistentes
- **CrossValidationError**: Referências quebradas
- **IncompatibleVersion**: Schema não suportado

Modo `allow_partial = true` aceita dados parciais com warnings.
