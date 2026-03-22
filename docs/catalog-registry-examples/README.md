# Catalog registry

Registry of LLM providers and models for OPENAKTA.

## Overview

The system consumes static JSON catalogs hosted remotely (e.g. GitHub Pages) for provider and model metadata. This enables:

1. **Updates without deploy**: add providers/models by updating JSON
2. **Cross-validation**: consistency between providers and models
3. **Effective capabilities**: intersection of provider and model capabilities
4. **Local caching**: TTL-backed cache for offline use

## File layout

```
data-artifacts/
├── providers/v1.json    # Provider metadata
├── models/v1.json       # Model catalog
└── schemas/             # Optional validation schemas
    ├── provider-config.schema.json
    └── model-catalog.schema.json
```

## Configuration

In `axora.toml` (or equivalent):

```toml
[remote_registry]
url = "https://openakta.github.io/data-artifacts"
poll_interval_secs = 86400  # 24 hours
http_timeout_secs = 10
```

## Usage in code

```rust
use openakta_agents::catalog_registry::{CatalogRegistry, RegistryConfig};
use std::time::Duration;

// Configure
let config = RegistryConfig {
    base_url: "https://openakta.github.io/data-artifacts".to_string(),
    timeout: Duration::from_secs(10),
    cache_ttl: Duration::from_secs(86400),
    allow_partial: true,
};

// Create registry
let mut registry = CatalogRegistry::new(config);

// Get snapshot (from cache or fetch)
let snapshot = registry.get_registry().await?;

// Look up provider
if let Some(provider) = snapshot.get_provider("openai") {
    println!("Provider: {}", provider.label);
    println!("Base URL: {}", provider.api.base_url);
}

// List models for a provider
let models = snapshot.list_models_for_provider("openai");
for model in models {
    println!("  - {}: {} context tokens", model.label, model.context_window_tokens);
}

// Effective capabilities
let caps = snapshot.get_effective_capabilities("openai", "gpt-4o");
println!("Streaming: {}", caps.streaming);
println!("Tool calls: {}", caps.tool_calls);
println!("Accepts images: {}", caps.accepts_images);

// Resolve adapter hint
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

## Cross-validation

The system validates:

1. **Unique IDs**: no duplicate providers or models
2. **Valid references**: every model references an existing provider
3. **Supported models**: `supported_model_ids` must reference existing models
4. **Default model**: `default_model_id` must exist and belong to the provider
5. **Capabilities**: if a provider disables a feature, the model cannot claim it

## Versioning

- **URL**: `v1.json`, `v2.json`, etc. for breaking changes
- **Schema**: `schema_version` field in JSON (semver)
- **Compatibility**: app accepts schema 1.x.x, rejects 2.x.x

## Supported providers

### Direct
- **OpenAI**: GPT-4o, GPT-4o-mini, o1, o3-mini
- **Anthropic**: Claude 3.5 Sonnet, Claude 3 Opus
- **DeepSeek**: deepseek-chat, deepseek-coder

### Self-hosted
- **Ollama**: Llama, Qwen, CodeLlama, Mistral (local)

### Aggregators
- **OpenRouter**: multiple providers through one API

## Adding a provider

1. Add an entry to `providers/v1.json`
2. Add corresponding models to `models/v1.json`
3. Validate with `cargo test` (integration tests)

Example provider entry:

```json
{
  "id": "new-provider",
  "vendor_slug": "new-vendor",
  "label": "New Provider",
  "description": "Provider description",
  "status": "active",
  "provider_type": "direct",
  "api": {
    "base_url": "https://api.newprovider.com/v1",
    "compatibility": {
      "family": "open_ai",
      "surface": "chat_completions",
      "strictness": "compatible"
    }
  },
  "authentication": {
    "scheme": "bearer",
    "env_var_hint": "NEW_PROVIDER_API_KEY"
  },
  "capabilities": {
    "chat": true,
    "streaming": true,
    "embeddings": false,
    "tool_calls": true
  },
  "supported_model_ids": ["model-1", "model-2"]
}
```

## Diagnostics

The registry keeps validation diagnostics:

```rust
let snapshot = registry.get_registry().await?;
println!("Warnings: {:?}", snapshot.diagnostics.warnings);
println!("Errors: {:?}", snapshot.diagnostics.errors);
println!("Valid providers: {}", snapshot.diagnostics.providers_valid);
println!("Valid models: {}", snapshot.diagnostics.models_valid);
```

## Cache

- In-memory cache with configurable TTL
- `get_registry()` returns from cache when valid
- `refresh()` forces re-download
- Cache is transparent to callers

## Errors

- **FetchFailed**: network issue
- **ParseError**: invalid JSON
- **ValidationError**: inconsistent data
- **CrossValidationError**: broken references
- **IncompatibleVersion**: unsupported schema

With `allow_partial = true`, partial data is accepted with warnings.
