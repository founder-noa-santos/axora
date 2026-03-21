# Planner update directives

**Companion:** `IMPLEMENTATION-PLAN-V2.md`  
**Purpose:** Normative thresholds, registry fields, and integration hooks for the context compaction planner.  
**Date:** 2026-03-20  

References cited in rationale are indicative (industry practice / internal analysis); validate thresholds against OPENAKTA evals.

---

## Q1 — Dynamic budget thresholds (relative pressure)

### Definitions

- Let \( C(\text{model}) = \text{DynamicModelRegistry}[\text{model}].\text{max\_context\_window} \).
- Let \( U = \dfrac{\text{estimated\_prompt\_tokens}}{C(\text{model})} \).

### Pressure bands (per-model, from registry)

| Band | Condition | Behavior |
|------|-----------|----------|
| **Nominal** | \( U < 0.60 \) | No compaction pressure beyond normal scoring. |
| **Pre-summarization** | \( 0.60 \le U < 0.75 \) | Light tightening (e.g. mid-tier compression bias). |
| **Rolling summary** | \( U \ge 0.75 \) | **Trigger rolling summary** (onset leaves headroom before the hard wall). |
| **Aggressive eviction** | \( U \ge 0.90 \) | **Aggressive eviction** of low-importance / old material. |
| **Hard stop** | Planner MUST keep \( U \le 0.98 \) | Clamp retrieval + history to `hard_cap_tokens`; reject layouts that exceed it. |

**Rationale (summary):** Many teams profile quality at ~50%, 75%, and 90% of context capacity; 75% is a practical compaction onset; ~90% is a common point where latency/quality degrades and pruning is advised. See e.g. [Redis — LLM context windows](https://redis.io/blog/llm-context-windows/), [arXiv context-window optimization](https://arxiv.org/pdf/2509.21361.pdf).

### Concrete derived budgets per model

For each model in `DynamicModelRegistry`:

- `rolling_summary_start_tokens(model) = floor(0.75 * max_context_window(model))`
- `aggressive_eviction_start_tokens(model) = floor(0.90 * max_context_window(model))`
- `hard_cap_tokens(model) = floor(0.98 * max_context_window(model))`

If `max_context_window` is missing:

- Use `max_context_window(model) = 8192` as a **conservative default** and apply the **same percentages** (not a fixed 8K compaction cap in code paths—only the fallback divisor).

### Planner integration

- Wire these thresholds into **`PressureMonitor`** and **`ContextCompactor`**.
- All planner decisions MUST be expressed as **multiplicative fractions of `max_context_window`** from `DynamicModelRegistry`, never anonymous fixed-token constants except the **fallback window** above.

---

## Q2 — Cloud native prompt caching alignment

### Provider constraints

- Anthropic and OpenAI (and compat layers) typically require the **cacheable prefix** to meet a **minimum token length** (e.g. OpenAI often cites ≥1024 tokens for prompt caching; Anthropic thresholds vary by model/version). See e.g. [OpenAI community — prompt caching minimum](https://community.openai.com/t/why-does-prompt-caching-requires-at-least-1024-tokens/1363167).
- Cache hits require the **prefix token sequence to be identical** across calls; any change to the prefix invalidates the cache. (Provider docs and talks; treat as architectural constraint.)

### Registry fields (cloud models)

For each cloud model in `DynamicModelRegistry`:

- `min_cacheable_prefix_tokens(model)` ∈ {1024, 2048, 4096, …} (enum or u32; update per provider changelog).
- `supports_prompt_caching(model)` ∈ {true, false}.

### Definitions

- `prefix_tokens(model)` — token length of the **static prefix** (system, tools, repo map, global guidelines).
- `suffix_tokens` — token length of the **dynamic tail** (conversation history, latest user message, tool outputs).

### Rules

1. **Cache eligibility**
   - If `supports_prompt_caching(model) == false` → skip caching for that request.
   - Else if `prefix_tokens(model) < min_cacheable_prefix_tokens(model)` → skip caching for that request.
   - Else → mark prefix as **cacheable**.

2. **TOON + Blackboard V2 segmentation** (cache-capable models)
   - `ToonSerializer` MUST be able to emit two segments:
     - `cached_prefix_segment` — **immutable** for the mission (or until deliberate rebuild).
     - `live_suffix_segment` — **mutable** each turn.
   - Blackboard V2 stores `cached_prefix_segment` under a key:

     \[
     \text{cache\_key} = \text{hash}(\text{model\_id}, \text{provider\_instance\_id}, \text{cached\_prefix\_tokens})
     \]

   - The cached prefix is **never updated in place**; any semantic change ⇒ new `cache_key`.

3. **Immutable prefix constraint**

   **May include:** system messages; tool/skill declarations; static repo map; global context.  

   **Must exclude:** user turns; tool outputs; any content that changes within a mission.

4. **Request assembly**
   - **Cacheable:** use provider APIs/headers to reference the cached prefix for `cache_key`; send only `live_suffix_segment` as non-cached (per provider semantics).
   - **Non-cacheable:** standard TOON assembly; **do not** assert cache hits.

### Planner integration

- Extend **`DynamicModelRegistry`** and **CC7** tasks with `supports_prompt_caching` and `min_cacheable_prefix_tokens`.
- All cloud caching logic uses: Blackboard V2 `cached_prefix_segment`, `cache_key` as above, thresholds from registry.

---

## Q3 — Local KV cache VRAM lifecycle

### Background

- KV cache growth dominates VRAM for long contexts and concurrent sessions; offloading and explicit eviction are standard mitigations. See e.g. [BentoML — KV cache offloading](https://bentoml.com/llm/inference-optimization/kv-cache-offloading), [vLLM discussions on memory](https://github.com/vllm-project/vllm/issues/20987).

### Registry fields (local models)

- `local_kv_ttl_seconds(model)` — default suggestion: **180** s.
- `local_kv_vram_evict_threshold(model)` — VRAM **fraction**; default suggestion: **0.80**.

### Lifecycle states

- `Active` — KV in use in the ReAct loop.
- `Idle` — retained for possible continuation; no inflight call.
- `Purged` — freed.

### Policy: local → cloud escalation

`escalation_mode ∈ {one_way_to_cloud, hybrid}` at **mission** level (user config or router heuristic).

1. **One-way to cloud**
   - After the **final** local call for the mission:
     - Immediately transition local KV → **Purged**.
     - Optionally write a **compact text summary** of the local segment to semantic memory; **do not** retain KV blobs for resume.

2. **Hybrid**
   - After last local call **before** possible return:
     - Set KV → **Idle**, `last_used_at = now`.
   - A **KV manager** (periodic / event-driven):
     - If `now - last_used_at > local_kv_ttl_seconds(model)` → **Purge**.
     - If VRAM utilization ≥ `local_kv_vram_evict_threshold(model)` → **Purge all Idle** KV caches immediately (TTL ignored for Idle purge).

**Heuristic:** long interactive missions → `hybrid`; one-off escalation (e.g. final cloud summarization) → `one_way_to_cloud`.

### Planner integration

- **`latent_context.rs`:** store session **handles** plus `state` (`Active` | `Idle` | `Purged`) and `last_used_at`.
- **Mission router** sets `escalation_mode`.
- **ReAct coordinator:** on leaving the local lane for cloud, **notify KV manager**; do not embed VRAM policy in the model loop.

---

## Q4 — ACON distillation & fact extraction timing

### Architectural rule

\[
\text{Execution}(\text{FactExtractor} \circ \text{ACON\_Distiller}) \in \text{Asynchronous}(\text{ConsolidationWorker})
\]

The **ReAct loop MUST NOT block** on fact extraction or ACON distillation.

### During ReAct

- ReAct writes full turns and tool traces to **Blackboard V2** and episodic memory.
- ReAct **does not await** fact extraction or distillation.
- Retrieval during ReAct uses **only** already-materialized semantic memory + prior ACON guidelines.

### ConsolidationWorker

- Subscribes to Blackboard V2: completed turns; mission-end; failure signals.
- Batches **FactExtractor** and **ACON_Distiller** work; triggers on idle windows, mission completion, or failure patterns (ACON).
- Selects consolidation model via `DynamicModelRegistry`:
  - e.g. `role = consolidation_only` or `both`;
  - `consolidation_cost_weight` — prefer cheap, high-window models.

### Optional synchronous path (strictly bounded)

- **Mission completion only:** ReAct may call **lightweight** fact extraction **synchronously** if:

  \[
  \dfrac{\text{estimated\_tokens\_for\_fact\_extraction}}{C(\text{consolidation\_model})} \le 0.25
  \]

  and mission budget allows.

- **ACON** distillation stays **asynchronous** always.

### Registry fields (consolidation)

- Flag models **consolidation-eligible** and expose **cost / window** metadata so ConsolidationWorker does not contend with the primary ReAct model.

### Planner integration

- **CC6 / CC8:** schedule via **ConsolidationWorker** + blackboard events; coordinator **emits events**, does not **await** consolidation.
- Extend **`DynamicModelRegistry`** with consolidation roles and cost metadata.

---

## Change control

When provider minimum prefix sizes or default percentages change:

1. Update **model cards** / registry defaults.
2. Re-run compaction + caching integration tests.
3. Bump `PLANNER_UPDATE_DIRECTIVES.md` revision note at top.
