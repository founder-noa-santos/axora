# HITL Async Flow — Suspend-and-Wake (Blackboard V2 + gRPC)

This document specifies the **event-driven** Human-in-the-Loop (HITL) path required by the Dual-Thread ReAct architecture. The Actor thread **never blocks** waiting for a human; it **yields** after emitting state and returning the tool observation.

## Invariants (MetaGlyph mapping)

| MetaGlyph intent | Rust/runtime mapping |
|------------------|----------------------|
| \( \text{Tool}_{request\_user\_input} \circ \text{Execution}_{Actor} \Rightarrow \text{Emit}(Q \rightarrow \text{BB}_{V2}) \cap \text{State} = \text{Pending\_HITL} \cap \text{Yield} \) | `RequestUserInputTool::execute` completes when `MissionHitlGate::raise_question` finishes: mission lifecycle → `PendingAnswer`, optional `SharedBlackboard::publish` for `hitl_question:{id}`, bump `watch` version. MCP returns **immediately** with `question_id` (no `oneshot` wait in the tool). |
| \( \text{Pending\_HITL} \Rightarrow \text{Stream}(Q) \in gRPC \) | `MissionHitlGate` publishes `Message` (type `QUESTION`) on a shared `broadcast::Sender<Message>`. `CollectiveService::stream_messages` **subscribes** and filters by `StreamMessagesRequest.agent_id` vs `Message.recipient_id` / payload fields. |
| \( \text{Ingest}(A) \in \text{RPC}_{SubmitHitlAnswer} \Rightarrow \text{Update}(\text{BB}_{V2}) \circ \text{PubSub} \Rightarrow \text{Wake}(\text{Planner}) \) | `SubmitHitlAnswer` / `SendMessage(ANSWER)` calls `MissionHitlGate::submit_answer`. On success, **`SharedBlackboard::publish`** stores `hitl_answer:{question_id}` and increments version. Planner’s `watch` on `SharedBlackboard` **fires** → `planner_interrupt_loop` sends `InterruptSignal::ContextUpdate` → parked planner/actor loop resumes non-blockingly. |

## Core structs

### Mission lifecycle (authoritative)

- Source: `openakta_proto::collective::v1::MissionLifecycleState` (prost).
- Gate: `MissionHitlGate` (`crates/openakta-agents/src/hitl.rs`) — `HashMap<mission_id, MissionRecord>`.

### Blackboard V2

- `SharedBlackboard` (`crates/openakta-agents/src/memory.rs`):
  - `version: u64`
  - `version_tx: watch::Sender<u64>` — **`subscribe_version()`** for planner-side waits.
  - `publish(MemoryEntry, accessible_by)` — bumps version on every mutation.

### Message bus (UI / external clients)

- `tokio::sync::broadcast::Sender<collective.v1::Message>` — **fan-out**; multiple `stream_messages` subscribers.
- HITL gate holds a clone of the sender; `CollectiveServer` holds the same sender for subscriptions.

### gRPC surface

- `CollectiveService::StreamMessages` — filtered fan-out.
- `CollectiveService::SubmitHitlAnswer` — **authoritative** answer path (preferred over raw `SendMessage` for answers).
- `SendMessage` with `MESSAGE_TYPE_ANSWER` — when `hitl_gate` is configured, **forwards** to `submit_answer` (no duplicate bus send; gate publishes).

## Actor yield (non-blocking)

1. **Planning thread** proposes `Action { tool_name: "request_user_input", ... }`.
2. **Acting thread** runs `ToolSet::execute` → MCP `CallTool` → `RequestUserInputTool::execute`.
3. Tool calls `gate.raise_question(envelope, mission_id)`:
   - Validates envelope, updates `MissionRecord` to `PendingAnswer`, persists checkpoint, **publishes question** to `broadcast`, returns `Ok(question_id)`.
4. MCP returns `ToolCallResult` **success** with structured output; Actor sends `ActionExecution` to planner **without awaiting** any human channel.

If a **local** integration still needs a blocking CLI wait, use `register_answer_waiter` (`oneshot`) **outside** the Actor hot path — not inside the default MCP tool.

## Planner wake (event-driven)

1. Human client calls **`SubmitHitlAnswer`** with `AnswerEnvelope`.
2. Server `submit_answer(answer)`:
   - Validates, moves mission `PendingAnswer` → `Running`, persists, publishes **Answer** `Message` on bus.
3. Server calls **`SharedBlackboard::publish`** with a `MemoryEntry` id `hitl_answer:{question_id}` (content: JSON or TOON summary for planner consumption).
4. **`version_tx.send`** unblocks any `planner_interrupt_loop` / `watch::Receiver` waiting on blackboard.
5. `DualThreadReactAgent` already uses `InterruptSignal::ContextUpdate` on version change (`react.rs`) to resume planning **without polling**.

## Trusted metadata

- `ToolCallRequest.mission_id` (proto) — set by coordinator/runtime on MCP calls, **not** parsed from model tool args for security-of-binding.
- `StreamMessagesRequest.agent_id` — must match `QuestionEnvelope.session_id` (orchestrator-assigned session) for delivery scoping.
- `build_answer_message` sets `recipient_id` to the **same `session_id`** as the question so answer streams are targetable.

## Failure / rollback

- `raise_question`: if checkpoint I/O or bus publish fails after in-memory transition, **`rollback_raise`** restores `MissionRecord` and re-persists; avoids zombie `pending_answer` without a visible question.

## File map

| Concern | Location |
|---------|----------|
| Gate + checkpoints | `crates/openakta-agents/src/hitl.rs` |
| Blackboard + watch | `crates/openakta-agents/src/memory.rs` |
| gRPC collective + bus | `crates/openakta-core/src/server.rs` |
| Embedded MCP + shared gate | `crates/openakta-core/src/bootstrap.rs` |
| Daemon MCP + collective | `crates/openakta-daemon/src/main.rs` |
| MCP tool | `crates/openakta-mcp-server/src/lib.rs` (`RequestUserInputTool`) |
| Planner interrupt on BB | `crates/openakta-agents/src/react.rs` |
