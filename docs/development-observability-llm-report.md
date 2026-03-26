# Development Observability Report

## Audience

This document is written for another LLM that will implement development-mode observability for OPENAKTA.

The goal is not generic “better logging”. The goal is to remove the current black-box operator experience during mission execution and replace it with a complete, inspectable action stream that can drive both:

1. development logs in terminal / local files
2. live UI updates in desktop

This report is grounded in the current repository state.

## Problem Statement

Today, OPENAKTA can run missions, call models, and execute some tools, but the operator experience is still mostly opaque during execution.

What a developer sees today is typically:

- sparse `tracing` lines such as `mission started` and `mission completed`
- occasional warnings or errors
- very little visibility into what the model actually did between those points

What a developer needs instead:

- when the model requested a tool
- which tool it requested
- what file / symbol / query it targeted
- whether the request was approved / denied
- whether the request ran through MCP or another executor
- whether it succeeded or failed
- a short result preview
- how that action maps to the UI

This is especially important in development mode. The developer should not have to guess whether the model is stuck, reasoning, retrieving, reading files, or waiting on a tool.

## Relevant Existing Pieces

OPENAKTA already has several partial observability systems. The main problem is fragmentation.

### 1. Plain `tracing` line logs

Current tracing initialization in [`crates/openakta-core/src/lib.rs`](/Users/noasantos/Documents/openakta/aktacode/crates/openakta-core/src/lib.rs) is a basic formatted subscriber:

- env-filter driven
- thread ids enabled
- plain line-oriented output
- no structured JSON sink
- no explicit span hierarchy for mission -> task -> tool -> provider call

The coordinator emits coarse lifecycle lines in [`crates/openakta-agents/src/coordinator/v2.rs`](/Users/noasantos/Documents/openakta/aktacode/crates/openakta-agents/src/coordinator/v2.rs):

- `mission started`
- `mission completed`

These are useful but insufficient. They do not explain what happened between start and finish.

### 2. `WideEvent`

There is already a canonical wide-event diagnostic payload in [`crates/openakta-agents/src/diagnostics.rs`](/Users/noasantos/Documents/openakta/aktacode/crates/openakta-agents/src/diagnostics.rs) and the schema is documented in [`docs/wide-event-schema.md`](/Users/noasantos/Documents/openakta/aktacode/docs/wide-event-schema.md).

This is aligned with the general “wide event / canonical log line” idea described by the user-referenced [Logging Sucks](https://loggingsucks.com/) article:

- structured, context-rich events
- one authoritative event per logical operation
- context first, not vague strings

However, in the current repo this system is not the main development observability backbone.

Current limitations:

- mostly failure-oriented
- often emitted only after something already failed
- TOON-encoded for downstream handling, which is not ideal as the primary development log format
- not used as the canonical live execution stream for model and tool actions

### 3. `ExecutionTraceEvent`

There is now a dedicated execution trace type in [`crates/openakta-agents/src/execution_trace.rs`](/Users/noasantos/Documents/openakta/aktacode/crates/openakta-agents/src/execution_trace.rs).

It already models:

- `requested`
- `approved`
- `started`
- `progress`
- `completed`
- `failed`
- `denied`

And carries useful fields:

- `mission_id`
- `task_id`
- `turn_id`
- `agent_id`
- `provider_request_id`
- `tool_call_id`
- `tool_kind`
- `tool_name`
- `args_preview`
- `result_preview`
- `error`
- `read_only`
- `mutating`
- `requires_approval`

This is the strongest existing foundation for the operator-visible action stream.

Current limitation:

- it is mostly accumulated in memory inside the coordinator
- it is not yet the canonical output for dev logs
- it is not yet streamed to the desktop UI from the runtime
- it currently covers tool lifecycle better than provider/retrieval/reasoning lifecycle

### 4. MCP audit events

The MCP service already emits audit events in [`proto/mcp/v1/mcp.proto`](/Users/noasantos/Documents/openakta/aktacode/proto/mcp/v1/mcp.proto) and [`crates/openakta-mcp-server/src/lib.rs`](/Users/noasantos/Documents/openakta/aktacode/crates/openakta-mcp-server/src/lib.rs).

Strengths:

- there is a real `StreamAudit` RPC
- audit events include mission/task/turn/tool-call correlation fields
- they include status, phase, read-only / mutating flags, approval, previews, and error

Current limitation:

- MCP audit is only one slice of execution
- it is not unified with coordinator/provider events
- the desktop does not consume it as the main chat execution source

### 5. Tool registry and tool loop

The new tool registry in [`crates/openakta-agents/src/tool_registry.rs`](/Users/noasantos/Documents/openakta/aktacode/crates/openakta-agents/src/tool_registry.rs) and the coordinator loop in [`crates/openakta-agents/src/coordinator/v2.rs`](/Users/noasantos/Documents/openakta/aktacode/crates/openakta-agents/src/coordinator/v2.rs) are important because they centralize the points where observability should attach.

This is where the system already knows:

- which tool the model asked for
- whether the tool is read-only or mutating
- whether approval is required
- what arguments were passed
- whether the call succeeded

This is the correct place to emit canonical action events.

### 6. Desktop trace rendering

The desktop message contract in [`apps/desktop/shared/contracts/message.ts`](/Users/noasantos/Documents/openakta/aktacode/apps/desktop/shared/contracts/message.ts) already supports `toolCalls`.

The desktop mapper in [`apps/desktop/lib/chat-message-mapper.ts`](/Users/noasantos/Documents/openakta/aktacode/apps/desktop/lib/chat-message-mapper.ts) can derive UI tool calls from trace items.

The assistant message UI in [`apps/desktop/components/chat/ChatAssistantMessage.tsx`](/Users/noasantos/Documents/openakta/aktacode/apps/desktop/components/chat/ChatAssistantMessage.tsx) and [`apps/desktop/components/chat/ChatToolCall.tsx`](/Users/noasantos/Documents/openakta/aktacode/apps/desktop/components/chat/ChatToolCall.tsx) can render tool activity.

This is good.

Current limitation:

- desktop state is still mock-driven in [`apps/desktop/lib/app-state.tsx`](/Users/noasantos/Documents/openakta/aktacode/apps/desktop/lib/app-state.tsx)
- `generateMockTrace()` is still producing fake execution trace
- no real runtime stream is connected to the chat UI

### 7. Inter-agent progress transport

The internal transport in [`crates/openakta-agents/src/transport.rs`](/Users/noasantos/Documents/openakta/aktacode/crates/openakta-agents/src/transport.rs) and bus in [`crates/openakta-agents/src/communication.rs`](/Users/noasantos/Documents/openakta/aktacode/crates/openakta-agents/src/communication.rs) support typed progress updates and workflow transitions.

Current limitation:

- these are workflow/task oriented
- they are not a complete action-level event stream for provider calls, retrieval, file reads, shell commands, MCP calls, approvals, or result previews

## Current Diagnosis

The repository is not missing observability primitives. It is missing a single source of truth.

Current state can be summarized like this:

- plain tracing logs exist but are too sparse
- `WideEvent` exists but is mainly failure-centric and not the live action stream
- `ExecutionTraceEvent` exists and is close to the right model, but is not yet the system-wide canonical stream
- MCP audit exists, but only for MCP
- desktop rendering exists, but is fed by mocks instead of runtime events

The implementation task is therefore not “invent logging”. It is:

1. unify event production
2. define sinks for dev mode
3. wire the same event stream into desktop

## Target Outcome

The target development-mode experience should be:

### Terminal / local dev

When a mission runs, the operator should see a compact but informative live sequence such as:

- mission created
- task assigned
- provider request started
- provider response received
- tool requested: `read_file(path=apps/auth/package.json)`
- tool started
- tool completed
- tool result preview: `read 128 lines`
- provider resumed
- mission completed

This should not be vague line spam. It should be structured, correlated, and queryable.

### Desktop UI

The desktop should render the same underlying stream as:

- pending tool request
- running tool
- completed tool with preview
- failed tool with error
- approval request
- denied action

The UI should not infer execution from assistant prose.

## Design Principles

Another model implementing this should follow these principles.

### Principle 1: Use a canonical action event stream

`ExecutionTraceEvent` should become the primary runtime event contract for development-mode execution visibility.

Do not build a second competing trace type for UI.
Do not let terminal logs, MCP audit, and desktop state diverge semantically.

Instead:

- extend `ExecutionTraceEvent` if needed
- map MCP audit into it
- map provider call lifecycle into it
- map retrieval into it
- map approvals into it
- map desktop UI from it

### Principle 2: Keep `WideEvent` for summary / export, not as the only live stream

`WideEvent` is valuable, but it should not replace per-action trace events.

Recommended division:

- `ExecutionTraceEvent`: live step-by-step action stream
- `WideEvent`: summary / rollup / failure snapshot / export-friendly canonical log line

This is consistent with the “wide event” idea from [Logging Sucks](https://loggingsucks.com/): rich context matters, but for OPENAKTA the operator also needs intra-mission progress, not just one final summary line.

### Principle 3: Development logs must prefer readability over compression

Do not make TOON the main human-facing log format in development mode.

TOON is useful for LLM boundaries and compact transport, but development logs should expose:

- JSON or rich text
- named fields
- previews
- correlations

TOON can remain an auxiliary payload when needed.

### Principle 4: Correlation IDs are mandatory

Every event emitted in this system should be traceable through:

- session
- mission
- task
- turn
- agent
- provider request
- tool call

If an event lacks enough correlation to be joined to the rest of the story, it is not sufficient.

### Principle 5: Log actions, not implementation trivia

Do not fill the stream with low-signal lines such as:

- “total models in registry: 3”
- “entered function X”
- “building request”

unless they are emitted as rich structured summaries with actual operator value.

Prefer high-value action events:

- provider request started
- provider request failed with decode/classification details
- tool requested
- tool started
- tool completed
- retrieval returned N documents / N skills
- approval requested
- approval resolved

## Recommended Event Model

Treat `ExecutionTraceEvent` as the canonical action envelope, but extend it to cover more than tools.

### Recommended `kind` space

Do not overload everything into `tool_kind`.
Instead, keep a broader notion of execution item kind:

- `provider_request`
- `tool_call`
- `retrieval`
- `approval`
- `agent_assignment`
- `agent_result`
- `mission`
- `task`

If changing the existing struct is too disruptive, add one of:

- `event_kind`
- or `action_kind`

while preserving current tool fields for compatibility.

### Recommended required fields

At minimum, development-mode events should expose:

- `event_id`
- `timestamp`
- `phase`
- `event_kind`
- `display_name`
- `mission_id`
- `task_id`
- `turn_id`
- `agent_id`
- `provider_request_id`
- `tool_call_id`
- `target_path` or `target_symbol` or `query`
- `args_preview`
- `result_preview`
- `error`
- `duration_ms`
- `read_only`
- `mutating`
- `requires_approval`

### Recommended provider events

For each model invocation, emit:

- `provider_request.requested`
- `provider_request.started`
- `provider_request.completed`
- `provider_request.failed`

With fields like:

- model
- provider
- request mode
- tool count exposed
- message count
- streaming enabled
- stop reason
- response tool call count
- usage preview

### Recommended retrieval events

When retrieval runs, emit:

- query preview
- focal files / symbols
- result count
- token budget used
- diagnostics summary

This is more useful than only returning retrieval diagnostics hidden inside payloads.

### Recommended tool events

For tool calls, the existing lifecycle is good. Preserve:

- requested
- approved
- started
- progress
- completed
- failed
- denied

But improve the previews:

- `args_preview` should be compact and human-readable
- `result_preview` should be concise and operator-friendly
- file reads should say file path + line count / byte count
- command executions should say command + exit code + short stdout/stderr preview

## Recommended Sinks

Development mode should emit the same canonical events to multiple sinks.

### Sink 1: terminal pretty logs

Purpose:

- immediate local operator visibility

Format:

- concise, single-event summaries
- grouped by mission/task
- readable in real time

Example style:

- `[mission] started`
- `[provider] started model=openai/gpt-5.4`
- `[tool] requested read_file path=apps/auth/package.json`
- `[tool] completed read_file preview="read 78 lines"`

### Sink 2: JSONL event file

Purpose:

- postmortem inspection
- deterministic debugging
- replay into UI or dev tooling

Recommended path:

- `workspace/.openakta/logs/execution-<timestamp>.jsonl`

Each line should be one serialized canonical event.

This is the easiest way to support:

- local inspection
- diffing behavior across runs
- future timeline viewers

### Sink 3: in-process broadcast stream

Purpose:

- live desktop rendering
- future CLI TUI or inspector

Implementation options:

- reuse and extend the in-process bus
- or add a dedicated execution trace broadcast channel

The important requirement is: desktop should subscribe to real runtime trace data, not mock trace generation.

### Sink 4: optional wide-event summary

Purpose:

- one summary event per request / mission / failure
- external ingestion later if desired

This can use the existing `WideEvent` model.

## Recommended Implementation Strategy

Another model should implement this in phases.

### Phase 1: make `ExecutionTraceEvent` the canonical dev stream

Tasks:

1. Emit coordinator-level events for:
   - mission start / complete
   - task assignment
   - provider request start / complete / fail
2. Keep existing tool trace emission
3. Add event sink abstraction in `openakta-agents`
4. Write every event to:
   - in-memory mission trace
   - terminal pretty log
   - JSONL file in `.openakta/logs`

Do not block on desktop integration first.

### Phase 2: bridge MCP audit into canonical trace

Tasks:

1. Normalize MCP `AuditEvent` into canonical execution events
2. Ensure MCP events and coordinator events share correlation ids
3. Remove duplicate semantics where possible

Target:

- MCP is no longer “a separate logging system”
- it becomes one producer feeding the same execution story

### Phase 3: desktop real trace ingestion

Tasks:

1. Replace `generateMockTrace()` in [`apps/desktop/lib/app-state.tsx`](/Users/noasantos/Documents/openakta/aktacode/apps/desktop/lib/app-state.tsx)
2. Add a real runtime subscription path
3. Feed chat tool panels from runtime trace events
4. Show provider / retrieval / approval events, not only tool calls

Target:

- the desktop timeline reflects reality
- the chat no longer infers execution from mock or assistant text

### Phase 4: rich summaries and quality pass

Tasks:

1. Add `WideEvent` rollups per mission / task / failure
2. improve preview rendering
3. reduce noisy low-value log lines
4. add tests for event emission completeness

## Concrete Repo-Specific Changes Recommended

### `crates/openakta-core/src/lib.rs`

Current issue:

- tracing setup is minimal

Recommended:

- support dev-mode pretty sink and JSONL sink
- optionally support JSON structured logs behind env/config
- establish root span / mission correlation support

### `crates/openakta-agents/src/coordinator/v2.rs`

Current issue:

- this is the right place for canonical emission, but only part of the lifecycle is instrumented

Recommended:

- centralize event emission helpers
- emit provider lifecycle events, not just tool events
- ensure all task outcomes produce correlated trace and summary events

### `crates/openakta-agents/src/diagnostics.rs`

Current issue:

- failure-centric and TOON-oriented

Recommended:

- keep `WideEvent`, but add rollup builders from canonical trace
- do not use it as the only live debugging format

### `crates/openakta-agents/src/mcp_client.rs`

Current issue:

- failures map into `WideEvent`, but success path is not first-class in the dev stream

Recommended:

- ensure both success and failure produce canonical trace events
- preserve audit payload, but do not require consumers to understand MCP-specific semantics

### `crates/openakta-mcp-server/src/lib.rs`

Current issue:

- audit streaming exists, but it is not treated as one producer in a broader observability architecture

Recommended:

- keep audit emission
- align phases/status naming with canonical execution events
- ensure result previews are concise and useful

### `apps/desktop/lib/app-state.tsx`

Current issue:

- still mock-driven

Recommended:

- remove mock trace generation for real runtime mode
- store trace as authoritative runtime data
- support incremental append / update by `event_id` or `tool_call_id`

### `apps/desktop/lib/chat-message-mapper.ts`

Current issue:

- maps only a subset of execution semantics into `toolCalls`

Recommended:

- preserve tool call derivation
- add support for non-tool execution items in side panels / timelines

## What Not To Do

Another model should avoid these traps.

### Do not spam one log line per tiny internal step

This recreates the problem described by [Logging Sucks](https://loggingsucks.com/): too many weak lines, not enough context.

### Do not encode the primary operator-facing stream as TOON

TOON is good for LLM efficiency, not for human-first development observability.

### Do not create a UI-only event model

The desktop should consume the runtime event model, not a second invented schema.

### Do not keep MCP audit, `ExecutionTraceEvent`, `WideEvent`, and desktop state semantically independent

Unify them around one execution story.

### Do not rely on assistant message text to infer tool activity

The UI must be driven by action events, not prose.

## Acceptance Criteria

The implementation should be considered complete for development mode only when all of the following are true.

### Terminal

- running `openakta do ...` shows live action progress beyond mission start/end
- provider request start/end is visible
- each tool request is visible
- success and failure paths show previews and correlation ids

### JSONL / local artifacts

- each run writes a replayable execution event file under `.openakta/logs/`
- events contain enough fields to reconstruct the mission story

### UI

- desktop tool panels are fed by real runtime events
- mock trace generation is removed from the runtime path
- approval / denied / failed / completed states render correctly

### Completeness

- no file read, shell command, retrieval call, MCP call, or approval flow can happen silently
- every executable action has start and terminal events

### Quality

- low-signal line logs are reduced
- operator-facing events contain meaningful previews
- mission/task/provider/tool correlation is consistent across sinks

## Final Recommendation

The repo already contains the right primitives. The highest-leverage implementation is not to add more logging statements. It is to promote `ExecutionTraceEvent` into the single development-mode action stream, then fan it out to:

- terminal pretty logs
- JSONL persistence
- desktop live updates
- `WideEvent` summary records

The desktop trace UI should become a consumer of the same runtime truth that the developer sees in logs.

That is the shortest path from “black box with sparse lines” to “inspectable agent runtime”.
