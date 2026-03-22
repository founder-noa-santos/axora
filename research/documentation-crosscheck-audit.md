# Auditoria: documentação vs código (NATS, gRPC, notificações, SSOT)

**Data:** 2026-03-21  
**Escopo analisado:** `docs/**`, `README`, `business-core/`, `active_architecture/` — cruzado com código em `aktacode/`.  
**Nota:** `codex/` não foi usado como fonte técnica do produto.

---

## Resumo executivo

Existe um núcleo de documentação **alinhado ao código** (`business-core/15`, `11`, `12` + `proto/collective/v1/core.proto` + `openakta-daemon`): **gRPC** (Collective com streaming de mensagens), **blackboard V2**, **ausência de canal de notificação a utilizador final** (email/SMS) no backend.

Em paralelo, vários documentos descrevem **NATS JetStream**, **assinaturas criptográficas end-to-end** e um **`agent_message.proto`** como se fossem realidade implementada — **não correspondem** ao estado atual do repositório. Isto é a principal fonte de **erro operacional** (configurar NATS, ligar cliente ao porto errado, assumir SSOT único).

---

## Evidências no código (factos)

### NATS / async-nats

- `async-nats` está no workspace (`Cargo.toml`, `openakta-agents/Cargo.toml`) mas **não há** `use async_nats` / chamadas em ficheiros `.rs`.
- `crates/openakta-agents/src/communication.rs`: módulo e struct dizem "NATS"; implementação é **in-process** (`HashMap`, handlers). Comentários: *"In a full implementation, this would use NATS"*.

### Contratos de mensagem

- Contrato **wire** do collective: `proto/collective/v1/core.proto` (`Message`, `StreamMessages`, `SendMessage`, tipos de patch/HITL, etc.).
- **Não existe** `openakta/proto/agent_message.proto` (referenciado em `docs/architecture-communication.md`).
- `AgentMessage` em Rust (`communication.rs`) é **serde**, com variantes/campos **diferentes** dos exemplos do doc longo de comunicação.

### Dois servidores gRPC / portas

- **Collective:** `CoreConfig::port`, default **50051** (CLI `openakta-daemon --port`).
- **MCP (Tool + GraphRetrieval) + LivingDocsReview:** `mcp_port`, default **50061** (`mcp_server_address()`).
- `crates/openakta-daemon/src/main.rs`: `LivingDocsReviewService` regista no **mesmo** `tonic::transport::Server` que Tool/GraphRetrieval — endereço MCP, **não** o do Collective.

### Assinaturas

- Documentação longa descreve **ed25519** e assinatura de mensagens.
- Em `transport.rs` / `provider.rs`: `cryptographic_signature: Vec::new()` — **placeholder**, não assinatura real.

### Plan 6 / LivingDocs

- `proto/livingdocs/v1/review.proto` + `LivingDocsReviewGrpc` em `openakta-daemon` — **implementados**.
- Spec `plan-06` menciona nome tipo `LivingDocsReviewServiceImpl`; código usa **`LivingDocsReviewGrpc`** (diferença só de nomenclatura).

### Caminhos

- `docs/architecture.md` continha paths absolutos para outro workspace (`/Users/.../Fluri/openakta/...`) — **inválidos** para clones genéricos.

---

## Inconsistências de alto risco operacional

| Risco | Onde na doc | Realidade no código |
|--------|-------------|---------------------|
| Assumir NATS em produção | `docs/architecture-communication.md`, narrativa NATS em `active_architecture/01_CORE_ARCHITECTURE.md` | Sem uso de cliente NATS; bus interno em memória + gRPC |
| Ligar UI/cliente ao porto errado | `docs/configuration.md` sem `port` vs `mcp_port` | Dois binds; review/LivingDocs no **50061** (MCP), Collective no **50051** |
| Gerar integração a partir do proto errado | `architecture-communication.md` + exemplo `agent_message.proto` | Usar `collective/v1/core.proto` e `livingdocs/v1/review.proto` |
| Política de segurança baseada em assinatura de mensagens | `architecture-communication.md` | Assinatura não implementada no pipeline atual |
| Onboarding com paths quebrados | `docs/architecture.md` (paths absolutos alheios) | Usar paths relativos ao repo `aktacode/` |

---

## Mitos a corrigir na documentação

1. **Mito:** O backbone entre agentes é NATS JetStream (DLQ, scaling, etc.).  
   **Realidade:** sem NATS ativo; coordenação relevante via **gRPC + blackboard + helpers in-process**.

2. **Mito:** “Exactly-once” no transporte como na tabela comparativa do doc de comunicação.  
   **Realidade:** não aplicável ao stack atual; onde há streaming, a semântica é a dos **serviços gRPC**, não JetStream.

3. **Mito:** Existe `agent_message.proto` canónico como no desenho.  
   **Realidade:** ficheiro **inexistente**; contratos oficiais são **`core.proto`** (collective) e **`review.proto`** (LivingDocs).

4. **Mito:** Um único endpoint “gRPC do daemon” para tudo.  
   **Realidade:** **dois** serviços em portos distintos (Collective vs MCP+LivingsDocs).

5. **Mito:** “Notificações OPENAKTA” = email/push SaaS.  
   **Realidade:** `business-core/15` — sinais **máquina-a-máquina**; Plan 6 — **badge/toast in-app** para fila de review. Não confundir com `docs/business_rules/PAY-*` (marcado como não implementado em `business-core/12`).

6. **Mito:** `docs/active_architecture/01_CORE_ARCHITECTURE.md` é SSOT absoluto.  
   **Realidade:** `business-core/11` e `active_architecture/README.md` dizem para **validar em código**; `01` mistura implementado, planeado e pseudo-código.

7. **Mito:** Dual-thread ReAct com ficheiro `worker.rs` citado.  
   **Realidade:** path **`crates/openakta-agents/src/worker.rs`** referenciado em `01` — **não existe** (ex.: `worker_pool.rs`).

---

## O que está bem alinhado (para referência)

- `business-core/15-notifications-and-communication-model.md` — modelo interno, sem email/SMS.
- `business-core/11-current-source-of-truth-map.md` — mapa de paths úteis.
- `business-core/12-deprecated-conflicting-or-stale-material.md` — avisa sobre `docs/business_rules/`, pagamentos, etc.
- `docs/active_architecture/plan-06-ssot-conflict-resolution-ui-spec.md` — fluxo gRPC + SQLite autoritário + UI sem ler DB diretamente (coerente com `main.rs`).

---

## Recomendações documentais (checklist)

- [ ] Marcar `architecture-communication.md` como **research / futuro** ou arquivar; remover claims de “já implementado”.
- [ ] Em `01_CORE_ARCHITECTURE.md`: isolar NATS em secção **Future**; corrigir paths de ficheiros; alinhar claim de SSOT com `business-core/11`.
- [ ] Em `docs/configuration.md`: documentar **`port` (50051)** vs **`mcp_port` (50061)** e qual cliente usa qual.
- [ ] Em `docs/architecture.md`: **paths relativos** ao repositório.
- [ ] Em `communication.rs` (código): ajustar naming/comentários para não sugerir NATS em produção hoje.
- [ ] `docs/README.md`: etiqueta clara para docs **não normativos** até revisão cruzada.

---

## Ficheiros tocados pela análise (memória de trabalho)

| Área | Ficheiros |
|------|-----------|
| Docs longos | `docs/architecture-communication.md`, `docs/architecture.md`, `docs/configuration.md`, `docs/README.md`, `docs/active_architecture/01_CORE_ARCHITECTURE.md`, `docs/active_architecture/README.md`, `docs/active_architecture/plan-06-*.md` |
| Business | `business-core/11`, `12`, `15` |
| Código | `crates/openakta-daemon/src/main.rs`, `crates/openakta-core/src/config.rs`, `crates/openakta-agents/src/communication.rs`, `proto/collective/v1/core.proto`, `proto/livingdocs/v1/review.proto` |

---

*Documento gerado a partir de auditoria cruzada documentação ↔ código; atualizar após mudanças relevantes em transporte ou portos.*
