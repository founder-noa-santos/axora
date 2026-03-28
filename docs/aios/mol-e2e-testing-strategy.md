# MOL end-to-end testing strategy

**Audience:** contributors and agents implementing Mission Operating Layer (MOL) tests.  
**Scope:** how to run and extend tests that exercise story intake → preparation → execution → closure paths across the Rust crates, without assuming a single “browser E2E” harness yet.

Related: [mission-operating-layer.md](mission-operating-layer.md), [story-preparation-flow.md](story-preparation-flow.md), [requirement-closure-engine.md](requirement-closure-engine.md). Risk checklist: `REPORT_AI_WORK_MANAGEMENT_ORCHESTRATION.md` (repo root).

---

## 1. What “E2E” means here

MOL spans **three logical layers**, but only one of them owns workflow authority:

| Layer | Role | Persistence in tests |
|--------|------|------------------------|
| **Daemon** (`aktacode/crates/openakta-daemon`) | Authoritative workflow commands, read models, orchestration | **In-memory read models** in unit tests; **SQLite** `.openakta/work-management.db` at runtime |
| **API** (`openakta-api`) | Optional cloud infrastructure and legacy compatibility shell | **PostgreSQL** for users, quotas, billing, and provider-facing infra only |
| **Agents** (`openakta-agents`) | Coordinator execution | Usually exercised via coordinator/unit tests, not full cloud stack |

A **full MOL E2E** is local-first: scripted transitions on **one workspace + story + prepared story** through capture → preparation states → compiled plan → closure, with **rejection** asserts on illegal transitions in the daemon-owned workflow path.

---

## 2. Fast tests (no Docker)

Run from the **`aktacode/`** workspace root unless noted.

| Goal | Command |
|------|---------|
| Daemon: compile plan from read model (profiles, prepared story, raw items) | `cargo test -p openakta-daemon compile_work_plan` |
| Daemon: full `work_plan_compiler` module tests | `cargo test -p openakta-daemon work_plan_compiler` |
| Daemon: workflow authority tests | `cargo test -p openakta-daemon work_management_service` |
| API client: config / flags | `cargo test -p openakta-api-client` |
| Workspace default (all crates) | `cargo test-all` (alias: `test --workspace --all-features --locked`) |

These tests use **fixed UUIDs** only inside helper data (see §4). They do **not** open Postgres.

---

## 3. Hosted API integration tests (`openakta-api`)

The **`openakta-api/tests/integration_api.rs`** harness:

- Uses **`DATABASE_URL`, `REDIS_URL`, `QDRANT_URL`, `QDRANT_API_KEY`** if all are set; otherwise starts **testcontainers** (Postgres 15, Redis 7, Qdrant) and sets env vars.
- Runs **`openakta_api::db::run_migrations`** for hosted-user, quota, and provider infrastructure tables.
- Uses **`serial_test::serial`** for tests that share global state.

Typical invocation (from `openakta-api/`, Docker required if env not provided):

```bash
cd openakta-api
cargo test --test integration_api -- --test-threads=1
```

Use **`--ignored`** only for tests explicitly marked with `#[ignore = "...reason..."]` (see `tests/sim_provider_service.rs`). The repo disallows bare `#[ignore]` (see `scripts/qa/verify_test_taxonomy.sh`).

Hosted API tests are no longer the place to prove MOL authority. Use them only for:

- compatibility responses on legacy workflow routes
- billing, quota, entitlement, and provider infrastructure behavior
- OAuth/webhook ingress that must stay server-side

---

## 4. Minimal stable IDs and payloads

For **deterministic** unit tests, align with the daemon’s `sample_read_model()` in  
`aktacode/crates/openakta-daemon/src/background/work_plan_compiler.rs` (`#[cfg(test)]`):

| Concept | Example UUID (fixed in tests) |
|---------|------------------------------|
| Workspace | `aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa` |
| Story | `bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb` |
| Prepared story | `cccccccc-cccc-cccc-cccc-cccccccccccc` |
| Work item | `dddddddd-dddd-dddd-dddd-dddddddddddd` |
| Requirement | `eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee` |

Integration tests that insert rows should generate **new** UUIDs per run or use namespaced constants documented in the test module to avoid collisions under parallel execution—prefer **`serial`** when sharing one DB snapshot.

---

## 5. SQLite vs Postgres

| Store | When to rely on it in tests |
|-------|-----------------------------|
| **SQLite** | Authoritative daemon workflow behavior in `work_mirror.rs`; use this for workflow commands, read models, idempotency, and closure-state persistence. |
| **Postgres** | Hosted API infrastructure tests only: users, quotas, billing, entitlements, provider-facing services, and compatibility routes. |

Do not reintroduce the idea that Postgres is authoritative for workflow.

---

## 6. Optional filters and CI

There is **no** dedicated `mol` Cargo feature yet. To run a subset by name:

```bash
# aktacode workspace
cargo test -p openakta-daemon mol

# openakta-api — adjust filter to match test names you add
cargo test -p openakta-api work_management
```

If CI adds a **`mol`** job or test name filter (plan A15), this document should be updated with the exact workflow path.

---

## 7. Checklist before claiming an MOL test is “done”

1. `cargo fmt` on touched crates.
2. `cargo clippy …` per `aktacode/docs/RUST_TOOLING_BASELINE.md` if you changed production code.
3. Scope-appropriate tests: **`cargo test -p <crate>`** for each crate you edited.
4. For hosted API changes: confirm the server is still not reclaiming workflow authority.

---

## 8. Full lifecycle E2E (ABC5)

The full lifecycle lane must move to daemon-owned tests. Hosted API integration tests may assert that legacy workflow endpoints reject canonical workflow operations.

Recommended local authority coverage today:

```bash
cargo test -p openakta-daemon work_management_service
```

If a hosted API test mentions MOL, it must be about compatibility or refusal semantics, not server-owned workflow behavior.
