# MOL end-to-end testing strategy

**Audience:** contributors and agents implementing Mission Operating Layer (MOL) tests.  
**Scope:** how to run and extend tests that exercise story intake → preparation → execution → closure paths across the Rust crates, without assuming a single “browser E2E” harness yet.

Related: [mission-operating-layer.md](mission-operating-layer.md), [story-preparation-flow.md](story-preparation-flow.md), [requirement-closure-engine.md](requirement-closure-engine.md). Risk checklist: `REPORT_AI_WORK_MANAGEMENT_ORCHESTRATION.md` (repo root).

---

## 1. What “E2E” means here

MOL spans **three logical layers**:

| Layer | Role | Persistence in tests |
|--------|------|------------------------|
| **API** (`openakta-api`) | Authoritative gRPC/HTTP work management | **PostgreSQL** (migrations `0003`–`0005` include MOL tables) |
| **Daemon** (`aktacode/crates/openakta-daemon`) | Plan compilation, mirror sync | **In-memory read models** in unit tests; **SQLite** `.openakta/work-management.db` at runtime |
| **Agents** (`openakta-agents`) | Coordinator execution | Usually exercised via coordinator/unit tests, not full cloud stack |

A **full MOL E2E** (target: plan item ABC5) is: scripted transitions on **one workspace + story + prepared story** through capture → preparation states → compiled plan → (future) closure, with **rejection** asserts on illegal transitions. Until that integration test exists, “E2E strategy” means **coordinating the right `cargo test` targets and fixtures** across these layers.

---

## 2. Fast tests (no Docker)

Run from the **`aktacode/`** workspace root unless noted.

| Goal | Command |
|------|---------|
| Daemon: compile plan from read model (profiles, prepared story, raw items) | `cargo test -p openakta-daemon compile_work_plan` |
| Daemon: full `work_plan_compiler` module tests | `cargo test -p openakta-daemon work_plan_compiler` |
| API client: config / flags | `cargo test -p openakta-api-client` |
| Workspace default (all crates) | `cargo test-all` (alias: `test --workspace --all-features --locked`) |

Run from **`openakta-api/`** (standalone crate, not in the `aktacode` workspace):

| Goal | Command |
|------|---------|
| Work management **validators** only (waves, tracker, run state, preparation status, persona id) | `cargo test -p openakta-api --lib work_management::tests` |

These tests use **fixed UUIDs** only inside helper data (see §4). They do **not** open Postgres.

---

## 3. Postgres-backed integration tests (`openakta-api`)

The **`openakta-api/tests/integration_api.rs`** harness:

- Uses **`DATABASE_URL`, `REDIS_URL`, `QDRANT_URL`, `QDRANT_API_KEY`** if all are set; otherwise starts **testcontainers** (Postgres 15, Redis 7, Qdrant) and sets env vars.
- Runs **`openakta_api::db::run_migrations`**, so MOL schema from `0005_mission_operating_layer.sql` is present.
- Uses **`serial_test::serial`** for tests that share global state.

Typical invocation (from `openakta-api/`, Docker required if env not provided):

```bash
cd openakta-api
cargo test --test integration_api -- --test-threads=1
```

Use **`--ignored`** only for tests explicitly marked with `#[ignore = "...reason..."]` (see `tests/sim_provider_service.rs`). The repo disallows bare `#[ignore]` (see `scripts/qa/verify_test_taxonomy.sh`).

**New MOL RPC or HTTP coverage** should either extend `integration_api` patterns (JWT harness, `PgPool` seeds) or add a **dedicated** `tests/` file with the same hygiene (no runtime self-skip in default lanes; simulation-style files use a `sim_*` prefix per taxonomy script).

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
| **Postgres** | Any test that must prove **API** invariants, migrations, or RPC handlers against real SQL. |
| **SQLite** | Daemon **mirror** behavior (`work_mirror.rs`); local-only. E2E that must assert mirror sync should either use a **temp file** DB in a test harness (future) or document manual steps; the default automated lane is Postgres + in-memory compiler read models. |

Do not treat SQLite mirror state as authoritative; see mission-operating-layer doc.

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
4. For API changes: confirm no alternate public path bypasses the same invariant (see MOL plan handoff protocol).

---

## 8. Full lifecycle E2E (ABC5)

Implemented: **`openakta-api/tests/integration_api.rs`**, test `mol_e2e_captured_to_closed`. It drives **`captured → … → closed`** on story preparation (HTTP commands) with **rejection** asserts (`INVALID_STORY_INTAKE_CAPTURE`, `INVALID_STORY_PREPARATION_TRANSITION`). It uses the same Postgres + testcontainers + JWT harness as the other `integration_api` tests; per-run workspace / story / prepared_story UUIDs are documented in the test’s module comment.

Run (from `openakta-api/`, Docker required unless all of `DATABASE_URL`, `REDIS_URL`, `QDRANT_URL`, `QDRANT_API_KEY` are set):

```bash
cargo test --test integration_api mol_e2e_captured_to_closed -- --test-threads=1
```

For broader MOL coverage, combine with §2 (`cargo test -p openakta-api --lib work_management::tests`, daemon compiler tests, etc.).
