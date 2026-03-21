# Context Compacting Implementation Plan V2

**Baseline:** Revises `IMPLEMENTATION-PLAN.md` (2026-03-18)  
**Version:** 2.1  
**Date:** 2026-03-20  
**Status:** Authoritative architecture (local-first, multi-model heterogeneous)  
**Priority:** CRITICAL  
**Owner:** Agent B (Storage/Context Specialist)  

**Normative planner thresholds & registry fields:** [`PLANNER_UPDATE_DIRECTIVES.md`](./PLANNER_UPDATE_DIRECTIVES.md) (relative pressure \(U\), cloud cache segmentation, local KV VRAM lifecycle, async consolidation). This V2 plan **must stay consistent** with that document.

---

## Formal design constraints (MetaGlyph)

Interpretation for this document:

| Symbol | Meaning |
|--------|---------|
| $\Rightarrow$ | entails / design commitment |
| $\cap$ | conjunction (must hold together) |
| $\neg$ | explicit descope |
| $\rightarrow$ | refinement or implementation path |
| $\cup$ | inclusive merge of concerns |

**Q1 — State sync (Phase 1):**  
$\text{Architecture}(\text{OPENAKTA}) \Rightarrow \text{Local\_Single\_Process} \cap \neg(\text{Distributed\_Cluster})$  
$\text{State\_Sync} \rightarrow \text{Blackboard\_V2} \Rightarrow \text{Official\_Standard} \cap \neg(\text{CRDT} \mid \text{NATS} \mid \text{json-patch})$

**Q2 — Budgeting (CC3, CC5):**  
$\text{Budget}(\text{Compactor}) \Rightarrow f(\text{Model\_Metadata\_Registry}) \cap \neg(\text{Static\_8K\_Limit})$  
Implementation name: **`DynamicModelRegistry`** — see **PLANNER_UPDATE_DIRECTIVES Q1** for \(U = \hat{t}/C(\text{model})\) bands (0.60 / 0.75 / 0.90 / 0.98) and derived `rolling_summary_start_tokens`, `aggressive_eviction_start_tokens`, `hard_cap_tokens`. May **evolve from** `ProviderRegistry` (`crates/openakta-agents/src/provider_registry.rs`) plus model cards.

**Q3 — CC7 pivot:**  
$\text{MultiModel}(\text{Execution}) \Rightarrow \neg(\text{Universal\_Latent\_Space})$  
$\text{Optimization}_{CC7} \rightarrow \text{Native\_Prompt\_Caching}(\text{Cloud}) \oplus \text{Persistent\_KV\_Cache}(\text{Local})$  
Normative detail: **PLANNER_UPDATE_DIRECTIVES Q2** (cacheable prefix, `cache_key`, TOON segments) **and Q3** (local KV states, TTL, VRAM purge, `escalation_mode`).

**Q4 — CC6 & CC8:**  
$\text{Status}(\text{CC6} \mid \text{CC8}) \rightarrow \text{Active\_Development}$ with **plain semantic text** in Qdrant and $\text{Execution}(\text{FactExtractor} \circ \text{ACON}) \in \text{Async}(\text{ConsolidationWorker})$ per **PLANNER_UPDATE_DIRECTIVES Q4**.

---

## Goal

Deliver context compaction for **long-running, multi-turn orchestration** across **heterogeneous models** (local: Ollama / Candle-style stacks; cloud: Anthropic, OpenAI, others) with **60–80% effective cost reduction** (tokens + repeated compute) while preserving **>95% task success** on benchmark suites.

Compaction must be **local-first**: one coordinator process, no cluster-wide blackboard or distributed patch bus.

---

## Current state analysis

### What we have (approved baseline)

| Component | Status | Location | Notes |
|-----------|--------|----------|--------|
| **Blackboard V2** (official CC1 ∪ CC2) | Complete | `crates/openakta-cache/src/blackboard/v2.rs` (+ `v2_versioning.rs`, `v2_pubsub.rs`) | Optimistic concurrency, versioned KV, in-process subscribers, **JSON-shaped diff payloads** on `Update`. **Not** CRDT / multi-replica. |
| Context compactor | Partial | `crates/openakta-cache/src/compactor.rs`, `compactor/hierarchical_memory.rs`, `compactor/rolling_summary.rs`, `compactor/importance_scorer.rs` | **Gap:** budget still **not** wired to `DynamicModelRegistry` (see CC3/CC5). |
| TOON | Complete (layout) | `crates/openakta-cache/src/toon.rs`, `docs/TOON.md` | Schema + serializer; file split optional. |
| Unified diff (files) | Complete | `crates/openakta-cache/src/diff.rs` | Source patches; unrelated to blackboard state transport. |
| Latent context store | Minimal | `crates/openakta-cache/src/latent_context.rs` | Opaque blobs + handles; **to be repurposed** for CC7 provider cache artifacts. |
| Semantic memory core | Partial | `crates/openakta-memory/src/semantic_store.rs` | Embeddings + store; **fact extraction pipeline** still open (CC6). |
| Heterogeneous providers | Partial | `crates/openakta-agents/src/provider_registry.rs`, `provider_transport.rs` | Lanes for cloud/local; **model metadata surface** for compaction still to unify under `DynamicModelRegistry`. |

### Descoped — explicitly $\neg$ in V2

| Item | Reason |
|------|--------|
| `yrs` / Yjs / **CRDT** blackboard | Conflicts with $\text{Local\_Single\_Process}$ standard; Blackboard V2 is authoritative. |
| **NATS JetStream** / Redis pub-sub for blackboard | Distributed cluster pattern; replaced by in-proc `PubSubHub`. |
| **RFC 6902 `json-patch`** as interchange | Not required; blackboard publishes **domain JSON diffs** (`Update.diff`). |
| **ICAE / universal cross-model latent compilation** | No shared latent space across provider families; mathematically misaligned with Q3. |

### Remaining build (active development)

| Track | Priority | Notes |
|-------|----------|--------|
| CC3 + CC5 — registry-aware budgeting | CRITICAL | `DynamicModelRegistry` drives `max_context`, pressure thresholds, summary triggers. |
| CC4 — TOON | Maintenance | Hardening, benchmarks; already shipped. |
| CC6 — Facts → Qdrant as **plain text** | HIGH | Extractors write human-readable strings for any model. |
| CC7 — Cloud prompt caching ∪ local KV blobs | MEDIUM | Via `latent_context.rs` + provider adapters. |
| CC8 — ACON distilled guidelines as **plain text** | MEDIUM | Same retrieval plane as CC6. |
| CC9 — Benchmarks + validation doc | MEDIUM | No CRDT/NATS benches; include multi-model budget scenarios. |

---

## Phase breakdown

### Phase 1: Foundation — Blackboard V2 as CC1 ∪ CC2 (complete)

**Goal:** Single-process shared state with optimistic concurrency and compact diffs. **No further greenfield work** for alternate sync stacks.

#### Sprint CC1–CC2 (merged): Blackboard V2 standard

**Owner:** Agent B  
**Priority:** Baseline **DONE** — retain tests and documentation only.

**Normative behavior (already implemented):**

- Versioned writes; stale-version rejection.
- Subscribers receive `Update` with `old_value` / `new_value` / `diff` / `size_reduction()`.
- In-process `PubSubHub`; no external message broker.

**Optional hardening tasks (non-blocking):**

1. [ ] Document blackboard diff semantics in `docs/` (shape of `diff`, when full value vs delta).
2. [ ] Stress tests for concurrent writers **in one process** (not multi-replica).

**Deliverables:** None new — **`blackboard/v2.rs`** is the deliverable.

**Success criteria:** Already satisfied by existing `tests/blackboard_v2.rs` and integration patterns.

---

### Phase 2: Compaction engines (registry-aware)

**Goal:** TOON + hierarchical memory + rolling summary operate under **per-model** context ceilings and pressure curves.

#### Sprint CC3: Hierarchical memory + dynamic budget

**Owner:** Agent B  
**Duration:** ~2–3 days (delta on existing code)  
**Priority:** CRITICAL

**Relative pressure (normative):** Let \(C(\text{model}) = \text{max\_context\_window}\) from registry and \(U = \text{estimated\_prompt\_tokens} / C(\text{model})\). Implement bands from **PLANNER_UPDATE_DIRECTIVES Q1**: Nominal \(U<0.60\); pre-summarization \(0.60\le U<0.75\); **rolling summary at** \(U\ge 0.75\); **aggressive eviction at** \(U\ge 0.90\); **never plan** \(U>0.98\) — clamp to `hard_cap_tokens = floor(0.98 * C)`. If `max_context_window` missing, use **8192** only as fallback for \(C\) when computing percentages (not a magic compaction constant elsewhere).

**Tasks:**

1. [ ] Extend **`DynamicModelRegistry`** with `max_context_window` (and fields in § **DynamicModelRegistry schema** below).
2. [ ] Plumb **`ContextCompactor`** / **`CompactorConfig`** so effective caps and eviction are **fractions of** `max_context_window`, matching Q1 derived token breakpoints.
3. [ ] Keep **`HierarchicalMemory`** in `compactor/hierarchical_memory.rs`; escalate mid/old summarization aggressiveness with **Pre-summarization** and **Aggressive** bands.
4. [ ] Optional: inject **retrieved plain-text facts** (already in store) with lower eviction priority than user **Decision** entries.
5. [ ] Tests: two models, different \(C\) → same \(U\) triggers at different absolute token counts.

**Deliverables:**

- Registry + integration at `openakta-cache` / `openakta-agents` boundary.
- Updated `compactor.rs`, `pressure_monitor.rs` hooks + tests.

**Success criteria:**

- [ ] All thresholds expressed as **multipliers of** `max_context_window` (+ documented fallback 8192).
- [ ] Hard cap enforced at 0.98 × \(C\).
- [ ] No JSON Patch dependency for memory updates.

---

#### Sprint CC4: TOON serializer

**Owner:** Agent B  
**Priority:** HIGH — **largely complete**

**Tasks:**

1. [ ] Regression tests, token benchmarks, optional module split (`toon/` submodules).
2. [ ] **Cache-capable assembly (CC7/Q2):** API to serialize into **`cached_prefix_segment`** (immutable) vs **`live_suffix_segment`** (mutable), per **PLANNER_UPDATE_DIRECTIVES Q2**. Immutable prefix excludes user turns, tool outputs, and any mission-volatile content.

**Deliverables:** `toon.rs` updates + `benches/token_savings.rs`; blackboard stores prefix under `cache_key = hash(model_id, provider_instance_id, cached_prefix_tokens)`.

---

#### Sprint CC5: Rolling summary + pressure monitor

**Owner:** Agent B  
**Duration:** ~2 days  
**Priority:** HIGH

**Tasks:**

1. [ ] Implement **`PressureMonitor`** with bands **Nominal | PreSummarization | RollingSummary | AggressiveEviction** mapped **exactly** to Q1 \(U\) thresholds (0.60 / 0.75 / 0.90); expose current \(U\) and `hard_cap_tokens` for clamping.
2. [ ] **Rolling summary** engages at **RollingSummary** band (\(U \ge 0.75\)); **aggressive eviction** at \(U \ge 0.90\); pre-summarization tightens mid-tier earlier in **PreSummarization** band.
3. [ ] **`RollingSummary`**: keep last *N* full turns where *N* may scale with registry or yield to **token budget** under `hard_cap_tokens`.
4. [ ] Tests: identical transcript, two \(C(\text{model})\) → rolling summary starts at `floor(0.75*C)` tokens; clamp at `floor(0.98*C)`.

**Deliverables:**

- `crates/openakta-cache/src/compactor/pressure_monitor.rs` (or equivalent)
- Updated `rolling_summary.rs`, `compactor.rs`

**Success criteria:**

- [ ] Enum bands align with **PLANNER_UPDATE_DIRECTIVES Q1** (no ad-hoc pressure names).
- [ ] **Decision**-tagged entries still favored via importance scorer under eviction.

---

#### Sprint CC6: Semantic memory + fact extractor (plain text)

**Owner:** Agent B  
**Duration:** ~3 days  
**Priority:** HIGH — **Active development**

**Execution model (normative):** Fact extraction runs in **`ConsolidationWorker`** (async). ReAct **writes** turns to Blackboard V2 / episodic store and **does not await** extraction. ReAct **reads** only facts already in semantic memory. Optional **sync** lightweight extraction **at mission end only** if \(\hat{t}_{\text{extract}}/C(\text{consolidation\_model}) \le 0.25\) and budget allows — **PLANNER_UPDATE_DIRECTIVES Q4**.

**Tasks:**

1. [ ] Implement **`fact_extractor`** (`crates/openakta-memory/src/fact_extractor.rs`): declarative **plain UTF-8** sentences.
2. [ ] **ConsolidationWorker:** subscribe to blackboard (completed turns, mission-end, failures); batch extraction on idle / completion.
3. [ ] Upsert into **`SemanticMemory`** / Qdrant; embeddings over the **same** human-readable string returned at retrieval.
4. [ ] Registry: **`DynamicModelRegistry`** flags consolidation-eligible models + cost metadata (Q4).
5. [ ] Tests: worker-driven path does not block ReAct; retrieval is model-agnostic text.

**Deliverables:**

- `fact_extractor.rs`, `consolidation_worker` module (crate TBD)
- Event wiring from Blackboard V2

**Success criteria:**

- [ ] Q4 async rule satisfied; sync path only on mission end and only if inequality holds.
- [ ] Precision@k within agreed tolerance on internal RAG eval harness.

---

### Phase 3: Provider-native optimization + ACON + benchmarks

#### Sprint CC7: Native prompt caching (cloud) ∪ persistent KV / session cache (local)

**Owner:** Agent B  
**Duration:** ~4–5 days  
**Priority:** MEDIUM

$\neg$ **ICAE / universal LatentCompiler** — **descoped.**

**Cloud (Q2):** Registry fields `supports_prompt_caching`, `min_cacheable_prefix_tokens`. Eligibility: prefix length ≥ minimum; prefix **bit-stable** across calls. Use Blackboard V2 **`cached_prefix_segment`** + `cache_key`; request assembly via provider cache APIs for cacheable calls only.

**Local (Q3):** Registry `local_kv_ttl_seconds` (default 180), `local_kv_vram_evict_threshold` (default 0.80). KV states **`Active` / `Idle` / `Purged`**. Mission **`escalation_mode`**: `one_way_to_cloud` (purge KV after last local call; optional text summary only) vs `hybrid` (Idle + TTL purge + VRAM purge of all Idle). **ReAct notifies KV manager** when leaving local lane; policy not embedded in the tool loop.

**Tasks:**

1. [ ] Cloud: implement Q2 gating + TOON two-segment output; store immutable prefix in blackboard under `cache_key`.
2. [ ] Local: KV manager + `latent_context.rs` extended with **handle**, `state`, `last_used_at`, escalation mode.
3. [ ] Document provider matrix (`docs/CONTEXT-CACHING-PROVIDERS.md`).
4. [ ] Tests: mock provider — prefix mutation ⇒ new `cache_key`; mock VRAM — Idle purged at threshold.

**Deliverables:**

- Extended `latent_context.rs`, provider adapters, KV manager
- `docs/CONTEXT-CACHING-PROVIDERS.md`

**Success criteria:**

- [ ] Cloud: no cache attempt when `supports_prompt_caching == false` or prefix &lt; minimum.
- [ ] Local: VRAM/TTL policy matches Q3; no cross-model latent tensors.

---

#### Sprint CC8: ACON integration (guidelines as plain semantic text)

**Owner:** Agent B  
**Duration:** ~3 days  
**Priority:** MEDIUM — **Active development**

**Timing (normative):** ACON distillation is **always asynchronous** via **ConsolidationWorker** (Q4). ReAct never awaits distillation; injection at compaction uses **already distilled** guideline text from the store.

**Tasks:**

1. [ ] Failure-driven **guidelines** + distiller output → **plain text** bullets; store in Qdrant like CC6 facts.
2. [ ] Worker batches distillation on failure patterns + mission-end signals; model pick via registry consolidation flags / cost.
3. [ ] Compaction injects guidelines + facts; respects \(U\) bands and `hard_cap_tokens`.

**Deliverables:**

- `crates/openakta-agents/src/acon/` (or shared with memory crate) + ConsolidationWorker hooks

**Success criteria:**

- [ ] Q4: zero blocking distillation in ReAct hot path.
- [ ] Replay eval shows task success gains; batch distillation &lt;1s soft target where applicable.

---

#### Sprint CC9: Performance benchmarking

**Owner:** Agent A (Documentation) + Agent B  
**Duration:** ~2 days  
**Priority:** MEDIUM

**Tasks:**

1. [ ] Criterion benches: blackboard v2 updates, compaction with **multiple registry profiles**, TOON round-trip.
2. [ ] Measure effective compression ratio pre/post compactor for long transcripts.
3. [ ] Document cloud **cache hit** economics separately from local **prefix reuse**.
4. [ ] Author **`docs/CONTEXT-COMPACTING-VALIDATION.md`**.

**Deliverables:**

- `crates/openakta-cache/benches/context_compacting_v2.rs` (or extend `token_savings.rs`)
- `docs/CONTEXT-COMPACTING-VALIDATION.md`

**Success criteria:**

- [ ] Reproducible `cargo bench` / documented methodology.
- [ ] Explicit separation: **semantic** compaction metrics vs **provider cache** metrics.

---

## Success metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Compression ratio (semantic compaction) | 10:1 – 30:1 equivalent on long transcripts | Tokens before / after compactor at fixed quality gate |
| Task success | >95% vs baseline compact policy | Mission replay / internal eval |
| Relative pressure | Q1 bands honored (0.60 / 0.75 / 0.90 / 0.98) | \(U=\hat{t}/C(\text{model})\) from registry |
| Context ceiling | Per-model `max_context_window`; fallback 8192 for \(C\) only | Never a silent fixed 8K compaction limit |
| Cloud cost | 60–80% reduction where cacheable prefixes dominate | $ / 1M tokens with cache on vs off |
| Local efficiency | KV lifecycle Q3; reduced redundant prefill | VRAM / TTL metrics + wall time |
| Orchestration latency | P95 compaction + inject < agreed SLO | Tracing |
| Consolidation | ReAct never blocks on CC6/CC8 | Worker queue depth / await counts |

---

## DynamicModelRegistry schema (consolidated)

Fields referenced across **PLANNER_UPDATE_DIRECTIVES** and this plan (extend over time):

| Field | Applies | Purpose |
|-------|---------|---------|
| `max_context_window` | All | \(C(\text{model})\); fallback 8192 if absent |
| `supports_prompt_caching` | Cloud | Gate API cache path |
| `min_cacheable_prefix_tokens` | Cloud | Eligibility vs static prefix length |
| `local_kv_ttl_seconds` | Local | Idle purge (default 180) |
| `local_kv_vram_evict_threshold` | Local | Purge all Idle when VRAM ≥ fraction (default 0.80) |
| `consolidation_eligible`, `consolidation_cost_weight`, `role` | Worker | Pick cheap consolidation model for CC6/CC8 |

---

## Dependencies

### Internal dependency graph (V2)

| Sprint | Depends on | Blocked by |
|--------|------------|------------|
| CC1–CC2 | — | — (complete) |
| CC3 | CC1–CC2 (patterns), **DynamicModelRegistry** design | — |
| CC4 | — | — |
| CC5 | CC3, CC4 (optional TOON in summaries) | — |
| CC6 | Qdrant path stability (`openakta-memory`) | — |
| CC7 | CC3 (budget awareness), provider adapters | Provider API secrets / config |
| CC8 | CC5, CC6 | — |
| CC9 | CC3–CC8 material enough to benchmark | — |

### External dependencies

| Dependency | Purpose | V2 status |
|------------|---------|-----------|
| `yrs` / Yjs | CRDT | **Descoped** $\neg$ |
| `json-patch` | RFC 6902 | **Descoped** $\neg$ |
| `async-nats` | Cluster bus | **Not required** for compaction; remove from mental model $\neg$ |
| Qdrant / `qdrant-client` | Vector store for **plain-text** facts & guidelines | **In use** / extend |
| Anthropic / OpenAI SDKs | Prompt caching controls | **Integrate** per CC7 |
| Ollama (or local runner) | Session / KV where documented | **Integrate** per CC7 |

---

## Timeline (indicative)

| Window | Scope |
|--------|--------|
| Week 1 | `DynamicModelRegistry` + CC3/CC5 integration |
| Week 2 | CC6 fact extractor + Qdrant plain-text verification |
| Week 3 | CC7 provider caching + `latent_context` repurposing |
| Week 4 | CC8 ACON text artifacts + CC9 benches + validation doc |

---

## Definition of done

- [ ] Blackboard V2 remains the only state-sync standard; no CRDT/NATS/json-patch creep in compaction scope.
- [ ] **`PressureMonitor` + `ContextCompactor`** implement **PLANNER_UPDATE_DIRECTIVES Q1** exactly (relative \(U\), no hardcoded caps except fallback \(C\)).
- [ ] **CC4/CC7:** TOON **two-segment** + Blackboard `cache_key` per **Q2** where providers support caching.
- [ ] **CC7 local:** KV states, TTL, VRAM purge, `escalation_mode` per **Q3**; `latent_context` holds handles + metadata.
- [ ] **CC6/CC8:** **ConsolidationWorker** async path per **Q4**; optional mission-end sync extraction only within 0.25 × \(C\) rule.
- [ ] CC6/CC8 artifacts are **universally readable** plain text in retrieval.
- [ ] CC9 validation doc and benches merged; CI runs applicable benches or documents manual runbook.

---

**Supersedes:** Legacy CC1/CC2/ICAE narrative in `IMPLEMENTATION-PLAN.md` for architectural purposes. Keep V1 file for historical audit trail; **V2.1 + PLANNER_UPDATE_DIRECTIVES** are the working contract.
