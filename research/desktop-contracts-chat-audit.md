# Auditoria: contratos TS, mapper e chat (desktop)

**Escopo:** `apps/desktop/shared/contracts`, `lib/chat-message-mapper`, tipos de mensagem, componentes `ai-elements` relevantes (message, reasoning, tool, checkpoint) e chat em `components/chat/`.

**Data:** 2025-03-21

## Resumo executivo

Os contratos em `shared/contracts` descrevem um modelo de chat **rico** (papéis `user` | `assistant` | `tool` | `system`, raciocínio, ferramentas, checkpoint, versões, streaming). O **mock** (`lib/app-state.tsx`) só produz mensagens **mínimas** (`user` | `assistant`, texto + timestamp). O **mapper** projeta tudo para “mensagem finalizada” e **não preenche** raciocínio, ferramentas, checkpoint nem versões. Os componentes de chat **consomem** esses campos — no fluxo atual, grande parte do contrato é **código morto em termos de dados reais**. `MessageSchema` **não é validado em runtime** (só tipos TypeScript). `ShellStateSchema` / `ShellStateV2` está **isolado** do IPC real (`desktop.ts` + `electron/main/ipc.ts`).

## Metodologia

- Ficheiros analisados: `shared/contracts/*.ts`, `lib/chat-message-mapper.ts`, `lib/app-state.tsx`, `components/chat/*`, `components/ai-elements/{message,reasoning,tool,checkpoint}.tsx`, `electron/main/ipc.ts`, `tests/contracts.test.ts`.
- Proto / Rust (inferência por nomes dentro de `aktacode` apenas): `proto/collective/v1/core.proto` (`Message`, `MessageType`), `crates/openakta-agents` (`ChatMessage` role+content, `ModelRequest` com `stream`), `communication.rs` (`AgentMessage`).

## Achados

### Campos nunca preenchidos / sempre vazios no mock

| Campo / área | Contrato | Comportamento real (mock + mapper) |
|--------------|----------|-------------------------------------|
| `reasoning` | opcional, objeto completo | **Sempre ausente** — `threadMessageToContract` não define. |
| `toolCalls` | opcional array | **Sempre ausente**; papéis `tool` nunca vêm do mock. |
| `checkpointId` | opcional | **Sempre ausente** — `ChatCheckpoint` nunca aparece no mock. |
| `versions` | default `[]` | Mapper força `[]`; UI não usa ramos/versões. |
| `isStreaming` / `isComplete` (nível mensagem) | defaults Zod | Mapper fixa **`isStreaming: false`**, **`isComplete: true`** para **todas** as mensagens. |

**Mapper (`lib/chat-message-mapper.ts`):**

- Define `isStreaming: false`, `isComplete: true`, `versions: []`.
- Não mapeia `reasoning`, `toolCalls`, `checkpointId`.

**Mock (`lib/app-state.tsx`):**

- `Message`: apenas `id`, `role: "user" | "assistant"`, `content`, `timestamp`.

### Duplicação semântica e estados difíceis de raciocinar

- **Streaming em dois níveis:** `Message.isStreaming` / `Message.isComplete` **e** `reasoning.isStreaming` / `reasoning.isComplete`. A UI em `ChatAssistantMessage` esconde o corpo enquanto só há raciocínio a correr (`reasoningBlocksBody`), mas o **schema Zod não proíbe** combinações incoerentes (ex.: `message.isStreaming === true` com `isComplete === true`, ou `reasoning.isStreaming && reasoning.isComplete` simultâneos).
- **`tool` role vs `assistant` + `toolCalls`:** O contrato permite ferramentas em mensagem `assistant` **e** papel `tool` que renderiza só `toolCalls`. Pode haver `role: "tool"` com `toolCalls` vazio → **render vazio** (`ChatMessage` não usa `content` para `tool`).

### Streaming / raciocínio / tool calls vs estado representável

- **Mock:** não há fase “a responder”, chunks, nem ferramentas — o contrato **promete** mais do que `AppMessage` + mapper **podem** expressar.
- **`ChatAssistantMessage`:** depende de `reasoning`, `isStreaming`, `checkpointId`, `toolCalls` para UX completa; com dados atuais prevalece o corpo Markdown (`MessageResponse`).
- **`Reasoning` (ai-elements):** `duration` no componente está em **segundos**; o contrato usa **`durationMs`** — conversão em `ChatAssistantMessage` (`Math.ceil(reasoning.durationMs / 1000)`); coerência é **convenção**, não garantida pelo tipo.

### Falhas silenciosas / defaults que mascaram problemas

- **`MessageSchema`:** não há `parse` em runtime — só `export type Message`. Objetos inválidos ao Zod **não falham** em runtime.
- **`ChatMessage`:** `default` em `switch` → `return null` para papel inválido — falha **silenciosa**.
- **`ChatToolApproval`:** `onApprove` / `onDeny` opcionais — botões podem **não fazer nada**.
- **`ChatToolCall` `toolCallToUiState`:** `default` do `switch` → `"input-available"` para qualquer valor inesperado de `status`.

### Shell / desktop / IPC

- **`desktop.ts` `shellStateSchema`:** `daemon.endpoint` como `string | null`.
- **`shell-state.ts` `ShellStateSchema`:** `endpoint` como `z.string().url().optional()` — **modelos diferentes** para o mesmo conceito.
- **`ShellStateSchema` / `ShellStateV2`:** sem consumidores no código além da definição; IPC usa `ShellState` minimal de `desktop.ts`.
- **`ipc.ts` (stubs `openakta:*`):** `missionSubmit` / `missionStatus` como `unknown` — contrato nominal apenas.

### Alinhamento com proto / Rust (inferência local)

- **`collective.v1.Message`:** `MessageType` e payloads opcionais — **não** papéis de chat `user`/`assistant` como no contrato TS.
- **`ChatMessage` (Rust, provider):** `role` + `content` — subconjunto do contrato TS; sem `reasoning`/`tool_calls` no mesmo struct.
- O contrato TS de mensagem de UI está **orientado ao produto desktop / AI SDK**, não espelhado literalmente num único proto de “chat UI” na pasta `proto/` atual.

## Drift contratual (UI vs intenção documentada)

| Fonte | Intenção | Drift |
|-------|----------|-------|
| `shell-state.ts` (comentário) | `ShellState` agregado do renderer, distinto de `desktop:get-shell-state` | `ShellStateV2` **não ligado** ao renderer nem ao IPC. |
| `ipc.ts` (comentário) | Stubs `openakta:*`, sem handlers nesta fase | Tipos descrevem **futuro**; sem validação em runtime. |
| `ChatToolApproval.tsx` (comentário) | Fallback vs `Confirmation` + `ToolUIPart` | Aprovação **não ligada** a estado se callbacks ausentes. |
| Docs / proto collective | Mensagens de agente, `MessageType`, etc. | Domínio **paralelo** ao chat UI; exige camada de tradução explícita. |

## Riscos (prioridade subjetiva)

1. **Alto:** Contrato de mensagem **não validado** + mapper **simplificado** → regressões só com dados reais ou testes de integração.
2. **Médio:** Duplicação `isStreaming`/`isComplete` (mensagem vs reasoning) sem invariantes.
3. **Médio:** `ShellStateSchema` não usado vs `ShellState` no IPC — duas narrativas, uma **inativa**.
4. **Baixo:** Papel `tool` com `toolCalls` vazio; `null` em `ChatMessage` para role inválido.

## Referências de código

- `apps/desktop/shared/contracts/message.ts` — `MessageSchema`, `ToolCallSchema`
- `apps/desktop/lib/chat-message-mapper.ts` — `threadMessageToContract`
- `apps/desktop/lib/app-state.tsx` — `Message`, mock, `sendMessage`
- `apps/desktop/components/chat/ChatMessage.tsx` — `switch` por `role`
- `apps/desktop/components/chat/ChatAssistantMessage.tsx` — reasoning, checkpoint, tools
- `apps/desktop/components/chat/ChatToolCall.tsx` — `toolCallToUiState`
- `apps/desktop/shared/contracts/desktop.ts` vs `shell-state.ts` — dois modelos de shell
