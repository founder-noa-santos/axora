# R4: WireProfile/ProviderKind Separation - Implementation Plan

**Status:** Planning Complete  
**Estimated Effort:** 1 Sprint (5-7 dias)  
**Risk Level:** Medium (refactor arquitetural em código crítico)  

---

## 1. EXECUTIVE SUMMARY

Separar `ProviderKind` (hoje usado para telemetria E transporte) em dois tipos distintos:
- **`WireProfile`** — define como construir requests HTTP (transporte)
- **`ProviderKind`** — identificador para telemetria/métricas

**Por que:** Hoje DeepSeek, Qwen, Moonshot usam `ProviderKind::OpenAi` no transporte, mas não conseguimos telemetria separada sem alterar código em múltiplos lugares.

---

## 2. DEFINIÇÃO DOS TIPOS

```rust
/// Wire protocol profile - drives request building and transport selection
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WireProfile {
    /// Anthropic Messages API v1
    AnthropicMessagesV1,
    /// OpenAI Chat Completions API
    OpenAiChatCompletions,
    /// OpenAI Responses API
    OpenAiResponses,
    /// Ollama Chat API (OpenAI-compatible)
    OllamaChat,
}

impl WireProfile {
    /// Derive telemetry kind from wire profile for backwards compatibility
    pub fn telemetry_kind(&self) -> ProviderKind {
        match self {
            WireProfile::AnthropicMessagesV1 => ProviderKind::Anthropic,
            WireProfile::OpenAiChatCompletions 
            | WireProfile::OpenAiResponses 
            | WireProfile::OllamaChat => ProviderKind::OpenAi,
        }
    }
}

/// Telemetry-only provider identifier
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderKind {
    Anthropic,
    OpenAi,
    DeepSeek,      // ← NOVO
    Qwen,          // ← NOVO
    Moonshot,      // ← NOVO
    Ollama,        // ← NOVO
}
```

---

## 3. ANÁLISE DE IMPACTO

### 3.1 Arquivos Modificados

| Arquivo | Linhas | Mudanças |
|---------|--------|----------|
| `provider.rs` | ~850 | Novo `WireProfile`, `ProviderKind` expandido |
| `provider_transport.rs` | ~1000 | Usar `WireProfile` em vez de `ProviderKind` para transporte |
| `routing/mod.rs` | ~400 | `request_provider()` retorna `WireProfile` |
| `coordinator/v2.rs` | ~2700 | Testes atualizados, novos providers |
| `agent.rs` | ~250 | Atualizar `AgentContext` |
| `prompt_assembly.rs` | ~200 | Atualizar `into_model_request` |
| `provider_registry.rs` | ~150 | Métodos de lookup atualizados |
| `bootstrap.rs` (core) | ~510 | Novos providers no teste |

**Total estimado:** ~200-300 linhas modificadas, ~50 novas linhas.

### 3.2 APIs Públicas Afetadas

```rust
// ANTES
pub fn request_provider(&self) -> ProviderKind;
pub fn build_body(request: &ModelRequest, provider: ProviderKind) -> Value;

// DEPOIS
pub fn request_provider(&self) -> WireProfile;
pub fn build_body(request: &ModelRequest, profile: WireProfile) -> Value;
```

---

## 4. IMPLEMENTATION PHASES

### Phase 1: Foundation (Dia 1-2)

**1.1 Criar `WireProfile`**
- Arquivo: `crates/openakta-agents/src/wire_profile.rs` (novo)
- Implementar enum + métodos `telemetry_kind()`, `from_provider_profile_id()`
- Testes unitários

**1.2 Expandir `ProviderKind`**
- Adicionar `DeepSeek`, `Qwen`, `Moonshot`, `Ollama`
- Implementar `Display` para logging
- Testes unitários

**1.3 Atualizar `ProviderProfileId`**
```rust
impl ProviderProfileId {
    pub fn wire_profile(&self) -> WireProfile;  // NOVO
    pub fn telemetry_kind(&self) -> ProviderKind;  // RENOMEADO de provider_kind()
}
```

### Phase 2: Transport Layer (Dia 2-3)

**2.1 Refatorar `provider_transport.rs`**
- `ProviderTransport::execute()` usar `WireProfile`
- `build_request()` usar `WireProfile`
- Atualizar `CloudModelRef` para ter ambos:
  ```rust
  pub struct CloudModelRef {
      pub instance_id: ProviderInstanceId,
      pub model: String,
      pub wire_profile: WireProfile,      // ← NOVO: para transporte
      pub telemetry_kind: ProviderKind,   // ← existente: para métricas
  }
  ```

**2.2 Refatorar `provider.rs`**
- `build_body()` aceitar `WireProfile`
- `prepare_request()` usar `WireProfile`
- `ModelRequest.provider` mudar de `ProviderKind` para `WireProfile`

### Phase 3: Routing Layer (Dia 3-4)

**3.1 Atualizar `routing/mod.rs`**
- `RoutedTarget::Cloud` e `RoutedTarget::Local` armazenar `WireProfile`
- `request_provider()` retornar `WireProfile`
- Remover `unwrap_or(ProviderKind::OpenAi)` hardcoded

**3.2 Atualizar consumidores**
- `coordinator/v2.rs`: `build_model_request()` usar `WireProfile`
- `agent.rs`: `AgentContext` atualizar field types
- `prompt_assembly.rs`: `into_model_request()` aceitar `WireProfile`

### Phase 4: Registry & Configuration (Dia 4)

**4.1 Atualizar `provider_registry.rs`**
- `provider_wire_profile()` — lookup por instance_id
- `provider_telemetry_kind()` — lookup por instance_id

**4.2 Atualizar configuração**
- `ProviderInstanceConfig` já tem `profile: ProviderProfileId`
- Adicionar campo opcional `telemetry_kind: Option<ProviderKind>` para override

**4.3 TOML config**
```toml
[providers.instances.deepseek-cloud]
profile = "open_ai_compatible"
telemetry_kind = "deepseek"  # opcional, default derivado do profile
base_url = "https://api.deepseek.com"
```

### Phase 5: Tests & Validation (Dia 5-7)

**5.1 Atualizar testes existentes**
- `coordinator/v2.rs`: ~20 testes atualizarem mocks
- `provider_transport.rs`: ~15 testes atualizarem assertions
- `provider.rs`: ~10 testes atualizarem fixtures

**5.2 Novos testes**
- `WireProfile::telemetry_kind()` mapping
- `ProviderKind` expandido em métricas
- Testes de integração DeepSeek/Qwen/Moonshot

**5.3 Validation checklist**
- [ ] `cargo test --package openakta-agents` passa
- [ ] `cargo test --package openakta-core` passa  
- [ ] `cargo clippy` sem warnings novos
- [ ] Exemplo TOML atualizado

---

## 5. MIGRATION STRATEGY

### Opção A: Big Bang (Recomendada)
- Todas as mudanças em um PR
- CI passa completamente
- Menor risco de estados intermediários inconsistentes

### Opção B: Gradual (Descartada)
- Manter `ProviderKind` como alias durante transição
- Adicionar `#[deprecated]` nos usos antigos
- Remover após 1 sprint

**Decisão:** Opção A — o refactor é pequeno o suficiente para ser atômico.

---

## 6. DETALHES TÉCNICOS

### 6.1 Mapping ProviderProfileId → WireProfile

```rust
impl ProviderProfileId {
    pub fn wire_profile(&self) -> WireProfile {
        match self {
            ProviderProfileId::AnthropicMessagesV1 => WireProfile::AnthropicMessagesV1,
            ProviderProfileId::OpenAiChatCompletions => WireProfile::OpenAiChatCompletions,
            ProviderProfileId::OpenAiCompatible => WireProfile::OllamaChat,
        }
    }
}
```

### 6.2 Request Building por WireProfile

```rust
fn build_body(profile: WireProfile, request: &ModelRequest, ...) -> Value {
    match profile {
        WireProfile::AnthropicMessagesV1 => build_anthropic_body(...),
        WireProfile::OpenAiChatCompletions 
        | WireProfile::OpenAiResponses 
        | WireProfile::OllamaChat => build_openai_body(...),
    }
}
```

### 6.3 Telemetry Enrichment

```rust
// Em CloudModelRef
impl CloudModelRef {
    pub fn for_instance(instance: &ResolvedProviderInstance) -> Self {
        let wire_profile = instance.profile.wire_profile();
        let telemetry_kind = instance.telemetry_kind_override
            .unwrap_or_else(|| wire_profile.telemetry_kind());
        
        Self {
            instance_id: instance.id.clone(),
            model: instance.default_model.clone().unwrap_or_default(),
            wire_profile,
            telemetry_kind,
        }
    }
}
```

---

## 7. RISCOS E MITIGAÇÕES

| Risco | Probabilidade | Impacto | Mitigação |
|-------|---------------|---------|-----------|
| Testes quebrados por mudança de tipo | Alta | Médio | CI completo antes de merge; script de fix automático |
| Conflitos com trabalho em paralelo | Média | Alto | Coordenar com time; fazer em sprint isolado |
| Runtime panic em edge cases | Baixa | Alto | Testes de integração abrangentes; feature flag opcional |
| Breaking change em consumers externos | Baixa | Alto | Verificar se há consumers fora dos crates openakta |

---

## 8. DEFINIÇÃO DE PRONTO

- [ ] `WireProfile` criado e testado
- [ ] `ProviderKind` expandido com novos providers
- [ ] Todos os usos de `ProviderKind` para transporte migrados para `WireProfile`
- [ ] `ProviderKind` usado apenas em telemetria/métricas
- [ ] Todos os testes passando
- [ ] Documentação atualizada (business-core, AGENTS.md)
- [ ] Exemplo TOML demonstrando novo provider (DeepSeek)
- [ ] PR revisado e aprovado

---

## 9. ESTIMATIVA

| Atividade | Dias |
|-----------|------|
| Phase 1: Foundation | 1.5 |
| Phase 2: Transport Layer | 1.5 |
| Phase 3: Routing Layer | 1.5 |
| Phase 4: Registry & Config | 1 |
| Phase 5: Tests & Validation | 1.5 |
| **Total** | **~7 dias** |
| **Buffer** | **+1 dia** |
| **Total com buffer** | **~8 dias (1 sprint)** |

---

## 10. PRÓXIMOS PASSOS

1. **Aprovação** — Checar com chefia se o plano está ok
2. **Scheduling** — Reservar sprint exclusivo (sem outras mudanças grandes)
3. **Branch** — Criar `feature/wire-profile-separation`
4. **Daily updates** — Reportar progresso no AGENTS.md

---

**Autor:** Implementation Agent  
**Data:** 2026-03-20  
**Versão:** 1.0
