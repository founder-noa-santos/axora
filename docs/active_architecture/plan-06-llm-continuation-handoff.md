# Plan 6 (SSOT / LivingDocs Review Queue) — LLM Continuation Handoff

**Purpose:** This document is a **standalone prompt and context bundle** for another engineer or LLM to continue OPENAKTA **Plan 6: Conflict Resolution (SSOT) and Notifications UI** until the system is **production-grade** end-to-end. Paste the **“Master prompt”** section into your agent’s system or first user message when resuming work.

**Repository root (this monorepo):** `aktacode/` (workspace members include `crates/openakta-daemon`, `crates/openakta-docs`, `apps/desktop`, `proto/`).

**Canonical architecture spec:** [`plan-06-ssot-conflict-resolution-ui-spec.md`](./plan-06-ssot-conflict-resolution-ui-spec.md)

---

## Master prompt (copy for the next LLM)

You are continuing **OPENAKTA Plan 6** in the **aktacode** Rust/Next.js/Electron monorepo. The product is **local-first**; the **LivingDocs** pipeline (Plan 5) writes drift reports and **human review** rows into a **SQLite queue DB**. The desktop shell must **never** open that DB directly; it must use **daemon gRPC** only.

### Your mission

1. **Close all gaps** between the current implementation and **production-level** behavior described in `plan-06-ssot-conflict-resolution-ui-spec.md` and this handoff.
2. **Option A (UPDATE_DOC):** After `SubmitResolution`, the runtime must **actually** run (or verifiably enqueue) the **TOON JSON changelog append** / doc-update path using existing Plan 5 types (`ToonChangelogPayload`, `append_changelog_entry`, `write_external_changelog_file`, etc. in `openakta-docs` and the processor in `openakta-daemon`).
3. **Option B (UPDATE_CODE):** Dispatch a **CodeModification**-style mission to **CoordinatorV2** via the existing **gRPC/MCP/collective** boundary, and return a real **`patch_receipt_id`** (or equivalent ID) in `SubmitResolutionResponse`, aligned with `collective.v1` patch concepts.
4. **Frontend:** Wire **`apps/desktop`** to the daemon with a **typed gRPC client** (or IPC wrapper): polling `GetPendingReviewCount` / `ListPendingReviews`, load `GetReviewDetail`, submit `SubmitResolution` with a **UUID** `client_resolution_id`. Implement **non-intrusive** notifications (badge + optional toast) and the **Review queue** surface per the spec.
5. **Quality bar:** Add **tests** that prove idempotency, workspace isolation, failure modes, and (where possible) integration with coordinator/TOON **without** flaking in CI. Run `cargo test -p openakta-daemon` and `npm test` in `apps/desktop` before claiming completion.

### Hard constraints

- **Do not** add cloud dependencies for this feature unless explicitly specified.
- **Bind** services to **localhost**; document any future auth token for IPC.
- **Preserve** SQLite as the source of truth for pending reviews; UI reads only via API.
- **Match** proto contracts in `proto/livingdocs/v1/review.proto`; regenerate `openakta-proto` if protos change.

### Verification commands (must pass)

```bash
cd aktacode
cargo build -p openakta-daemon -p openakta-proto
cargo test -p openakta-daemon
cd apps/desktop && npm test && npm run typecheck
```

---

## Current implementation snapshot (what already exists)

### Data & persistence

- **Queue DB path:** `SqliteJobQueue::path_for_workspace(workspace_root)` → `{workspace_root}/.openakta/livingdocs-queue.db` (same file as `LivingDocsEngine` in `background/engine.rs`).
- **Tables:** `livingdocs_reconcile_reviews`, `livingdocs_drift_reports`, `livingdocs_drift_flags`, `livingdocs_confidence_audit`, `livingdocs_resolution_dedupe` (idempotency for `SubmitResolution`).
- **Review rows:** Created by Plan 5 / `enqueue_reconcile_review` when confidence routing yields **ReviewRequired**.

### gRPC (implemented)

- **Proto:** `aktacode/proto/livingdocs/v1/review.proto` — service `LivingDocsReviewService`.
- **Rust crate:** `openakta-proto` — module `livingdocs::v1`.
- **Server implementation:** `crates/openakta-daemon/src/background/livingdocs_review_service.rs` (`LivingDocsReviewGrpc`).
- **Registration:** `openakta-daemon` **`main.rs`** adds `LivingDocsReviewServiceServer` to the **same tonic server** as MCP (`GraphRetrievalService`, `ToolService`) on **`config.mcp_server_address()`** (not necessarily the same port as `config.port` — **verify** `CoreConfig::mcp_server_address()` when wiring clients).

### Resolution behavior (partial)

- **`SqliteJobQueue::submit_resolution`:** Updates review to **approved**, writes **dedupe** row, logs structured message. **Does not** yet invoke TOON append or CoordinatorV2.
- **`SubmitResolutionResponse`:** `patch_receipt_id` and `toon_changelog_entry_id` are currently **unset** on success.

### Desktop UI (partial)

- **Components:** `apps/desktop/components/review/` — `ReviewQueueBadge`, `DriftDiffPanel`, `ConflictResolverModal`, `ReviewQueueDemo` (demo wiring in `workspace-canvas.tsx`).
- **Tests:** `components/review/review-queue-badge.test.tsx`; global RTL **`cleanup`** in `apps/desktop/tests/setup.ts`.

### Automated tests (daemon)

- **`queue.rs`:** Includes tests for jobs, drift persistence, reconcile enqueue idempotency, confidence audit, **`submit_resolution`** (OK / duplicate / conflict), **`drift_report_by_report_id`**.
- **`livingdocs_review_service.rs`:** Async tests invoking the **tonic trait** directly (list/count/detail/submit, workspace ACL, not found).

---

## Problems, gaps, and risks (must be addressed for “production level”)

### 1. Option A / Option B are not end-to-end

- **Symptom:** Resolving a review **only flips DB state**; documentation and code are **not** automatically reconciled per product intent.
- **Required work:** Thread `SsotChoice` from `SubmitResolution` into:
  - **UPDATE_DOC:** Call or enqueue the same code paths `LivingDocsProcessor` uses for **changelog / TOON** when confidence says update (see `processor.rs`, `changelog.rs`, `try_autocommit_changelog`, `write_external_changelog_file`).
  - **UPDATE_CODE:** Build inputs for **CoordinatorV2** (workspace root, target files from drift flags, constraints from docs), run mission, capture **patch receipt** ID, handle failures without corrupting queue state.

### 2. Architectural split: gRPC handler vs engine thread

- **Symptom:** `LivingDocsEngine` runs **`LivingDocsProcessor`** in a **blocking thread**; **`SubmitResolution`** runs in the **async gRPC** task. There is **no shared job channel** yet for “resolve_review” work.
- **Risk:** Duplicate SQLite writers, race on same DB, or deadlocks if not designed (WAL helps but is not sufficient for application-level races).
- **Required work:** Define a **single owner** for mutating follow-up work: e.g. enqueue a **new job kind** (`ResolveSsotReview`) processed by the engine, or an **`tokio::sync::mpsc`** bridge from gRPC to engine (careful with thread boundaries). Document the chosen design in the architecture spec.

### 3. Review status semantics vs spec

- **Symptom:** `submit_resolution` sets status to **`approved`** for all successful resolutions; spec mentions **`resolved_with_doc_update`** / **`resolved_with_code_update`** style outcomes.
- **Required work:** Extend `ReconcileReviewStatus` (or `notes` / metadata columns) so analytics and UI can distinguish Option A vs B; migrate schema if needed; update tests.

### 4. `expected_excerpt` / `actual_excerpt` quality

- **Symptom:** `ReviewDriftFlagView.expected_excerpt` may be **empty** because `StoredDriftFlag` does not persist full doc expectation blobs—only processed drift message/metadata.
- **Required work:** Either extend persistence (new columns or JSON) or derive excerpts deterministically from doc snapshots and AST summaries.

### 5. Path equality for workspace ACL

- **Symptom:** `paths_equal` in `livingdocs_review_service.rs` uses **`Path::components()`** equality—**symlinks**, trailing slashes, or **canonical** differences may cause **false “permission denied”** or **false match**.
- **Required work:** Use a documented normalization (`std::fs::canonicalize` where appropriate, or consistent `display()` string rules) and add tests for edge cases.

### 6. Idempotency vs HTTP “409”

- **Symptom:** Spec mentions **409 conflict**; gRPC returns **`SubmitResolutionResponse`** with `ResolutionOutcome` enum instead of HTTP status.
- **Required work:** Align API docs; if JSON/IPC is added later, map **CONFLICT** → 409 consistently.

### 7. Duplicate `client_resolution_id` ordering

- **Behavior:** Dedupe is checked **before** loading the review; **duplicate** returns the **same** `server_resolution_id` even if review is already resolved (by design for idempotency). Confirm this matches product expectations and document.

### 8. No tonic-level integration test

- **Symptom:** Tests call **`LivingDocsReviewService` trait** methods directly, not **`tonic::transport::Server` + client**.
- **Required work:** Optional but valuable: one **`#[tokio::test]`** with an **in-memory or ephemeral port** server + `LivingDocsReviewServiceClient` to validate **wire serialization** and **metadata** (e.g. timeouts, message size).

### 9. Frontend not connected to daemon

- **Symptom:** UI is **mock / demo**; no gRPC from Electron/renderer.
- **Required work:** Add **preload** IPC or **grpc-web** (if applicable) or **native tonic** in a sidecar—**choose one** transport, document in spec, implement **typed client** and **error surfaces** (network down, daemon missing).

### 10. Security and abuse

- **Symptom:** Localhost-only reduces risk; still no **auth token** for gRPC.
- **Required work:** If the shell exposes the endpoint beyond localhost in the future, add **shared secret** or **Unix socket** + permissions.

### 11. `SubscribeReviewQueue` (optional v2)

- **Not implemented.** Polling is acceptable for v1; streaming is listed in proto roadmap in the spec.

---

## File map (edit these when extending)

| Area | Path |
|------|------|
| Proto | `aktacode/proto/livingdocs/v1/review.proto` |
| Proto build | `crates/openakta-proto/build.rs`, `crates/openakta-proto/src/lib.rs` |
| Queue / SQLite | `crates/openakta-daemon/src/background/queue.rs` |
| LivingDocs engine | `crates/openakta-daemon/src/background/engine.rs`, `processor.rs` |
| gRPC service | `crates/openakta-daemon/src/background/livingdocs_review_service.rs` |
| Daemon entry | `crates/openakta-daemon/src/main.rs` |
| TOON / changelog | `crates/openakta-docs/src/changelog.rs`, confidence in `openakta-docs` |
| Coordinator | `crates/openakta-agents/.../coordinator/v2.rs` (verify exact path) |
| Desktop UI | `apps/desktop/components/review/*`, `apps/desktop/lib/*` |
| Architecture index | `aktacode/docs/active_architecture/README.md` |

---

## Suggested implementation order (next PRs)

1. **Schema / status** — distinguish resolution kinds; migrations in `SqliteJobQueue::init_schema`.
2. **Engine integration** — single pipeline for post-resolution work; avoid double-writes.
3. **Option A** — wire TOON/changelog append; fill `toon_changelog_entry_id` when known.
4. **Option B** — wire CoordinatorV2 + patch receipt; fill `patch_receipt_id`.
5. **Excerpts** — improve drift flag storage or derivation.
6. **Desktop** — real client + badge polling + resolver wired to live data.
7. **Hardening** — path canonicalization, optional tonic e2e test, load/error testing.

---

## Definition of done (production level)

- [ ] User can **see accurate pending count** and **list** for their workspace via gRPC only.
- [ ] User can **open detail**, see **expected vs actual** with **rule IDs**, and **must** pick SSOT before submit.
- [ ] **Option A** results in a **verifiable doc/changelog outcome** (or clear job ID / audit reference).
- [ ] **Option B** results in a **traceable patch receipt** and safe failure behavior.
- [ ] **Idempotency** and **workspace isolation** covered by automated tests.
- [ ] **Observability:** structured logs include `review_id`, `report_id`, `choice`, `outcome`.
- [ ] **CI green:** Rust + desktop tests; no new warnings that the repo treats as errors.

---

*Generated for continuity of Plan 6; update this file when major milestones land.*
