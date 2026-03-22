# OPENAKTA — agent and contributor instructions

This file tells **coding agents** and **human contributors** how to work safely and consistently in this repository. It is modeled after upstream projects that separate **how to edit code** from **product roadmaps**.

For architecture baseline, ADR summaries, and narrative docs, use:

- [`docs/ARCHITECTURE-LEDGER.md`](./docs/ARCHITECTURE-LEDGER.md) — architecture baseline and ADRs  
- [`docs/active_architecture/`](./docs/active_architecture/) — current system narrative  
- [`docs/README.md`](./docs/README.md) — documentation map  
- [`business-core/`](./business-core/) — **what the backend actually implements today** (code wins over aspirational docs)

---

## Repository map

| Area | Path | Notes |
|------|------|--------|
| Rust workspace | `crates/` | Crates are prefixed `openakta-*` (e.g. `openakta-core`, `openakta-agents`) |
| Desktop shell | `apps/desktop/` | Electron + Next.js App Router; privileged APIs only in main/preload |
| Protocol | `proto/` | Protobuf sources; regenerate with root `pnpm proto:gen` (see `package.json`) |
| SDKs | `sdks/`, `integrations/` | TypeScript/Python/Java/C# packages; see per-package `package.json` |
| Business truth | `business-core/` | Documented behavior grounded in implementation |

---

## Source of truth

1. **Runtime code** in `crates/`, `proto/`, and `apps/desktop/` (as applicable).  
2. **Tests** that exercise that behavior.  
3. **Example config** (`openakta.example.toml`) when backed by parsers in code.  
4. Markdown under `docs/` and `business-core/` — **only when consistent with (1)**.  
5. Older planning files — **may be stale**; never implement from them without verifying code.

Do **not** assume production billing, multi-tenant SaaS, or account systems exist because a doc mentions them — see `business-core/README.md`.

---

## Rust (`crates/`)

- **Naming:** Workspace crates use the `openakta-*` prefix (e.g. `openakta-cli`, `openakta-agents`). Internal module names stay lowercase/snake_case.
- **MSRV:** Declared in root `Cargo.toml` / `rust-toolchain.toml`; CI enforces it (`cargo check` with the MSRV toolchain). Match that version when reasoning about language features.
- **Formatting:** `rustfmt` per root `rustfmt.toml`. After substantive Rust edits, run `cargo fmt --all` (or `cargo fmt-check` before PRs).
- **Clippy:** CI runs with `-D warnings` on the full workspace (`cargo lint` alias in `.cargo/config.toml`). Fix warnings; do not silence them without a documented reason.
- **`format!`:** Inline variables in the format string when possible ([`uninlined_format_args`](https://rust-lang.github.io/rust-clippy/master/index.html#uninlined_format_args)).
- **Control flow:** Prefer collapsing nested `if` where Clippy suggests ([`collapsible_if`](https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if)).
- **Closures:** Prefer method references over redundant closures where Clippy suggests ([`redundant_closure_for_method_calls`](https://rust-lang.github.io/rust-clippy/master/index.html#redundant_closure_for_method_calls)).
- **API shape:** Avoid `foo(false)` / `bar(None)` at public boundaries when enums, newtypes, or named methods would document intent better.
- **`match`:** Prefer exhaustive matches; avoid catch-all arms unless unavoidable.
- **Tests:** Prefer `assert_eq!` on whole values (structs, enums) when equality is meaningful; avoid mutating global environment in tests — pass config or handles explicitly.
- **Modules:** Prefer new modules over unbounded growth of a single file. If a file is large and touch-heavy, add new behavior in a submodule unless there is a strong reason not to.
- **Transport vs telemetry:** Do not conflate `WireProfile` (how HTTP requests are built) with `ProviderKind` (telemetry/metrics). See coordinator/provider code and `business-core/` for the current split.

### Commands (from repo root)

Defined in `.cargo/config.toml` and aligned with `.github/workflows/rust-ci.yml`:

| Command | Purpose |
|--------|---------|
| `cargo fmt-check` | Format check (CI) |
| `cargo lint` | Clippy workspace, all targets/features, `-D warnings`, `--locked` |
| `cargo test-all` | Full test run excluding `#[ignore]` slow tests |
| `cargo test-slow` | Only `#[ignore]` / slow tests |

**Before a Rust PR:** `cargo fmt-check` → `cargo lint` → `cargo test-all` (or at least tests for touched crates).

**Scoped testing:** After changes, prefer `cargo test -p <crate>` for the crate you edited before running the full workspace.

---

## Protocol buffers

- **Do not** hand-edit generated Rust (or other) output from `buf`/`prost` unless the repo explicitly treats those files as hand-maintained (they usually are not).
- After `.proto` changes, run `pnpm proto:gen` from the repo root and commit generated outputs if the project expects them tracked.

---

## Desktop (`apps/desktop/`)

- **Stack:** Electron main + Next.js (App Router) renderer + React + TypeScript + Tailwind v4 + shadcn/ui (see root `README.md`).
- **Security:** Renderer code must not assume access to Node/OS APIs — use the **preload + IPC** surface only; keep new IPC typed and validated.
- **Lint / format:** Use the app-local ESLint and Prettier configs (`eslint.config.mjs`, `prettier.config.mjs`). Registry-generated or vendored UI under `components/ai-elements/**` may intentionally relax specific rules — do not “fix” those files to satisfy stricter rules without an explicit request.
- **Scripts:** `pnpm --filter @openakta/desktop lint|typecheck|test|build` from the repo root after `pnpm install`.

---

## Documentation and config

- When behavior or public config **changes**, update:
  - `docs/active_architecture/` if it affects the architecture narrative  
  - `openakta.example.toml` (and any schema docs) for user-visible config  
  - `business-core/` when the **implemented** business boundary changes  
- Navigation index: [`DOCS-INDEX.md`](./DOCS-INDEX.md).

---

## Multi-agent / product context (for agents)

- **Coordinator / workers:** Orchestration concepts live in code (`openakta-agents` and related crates) and in `docs/active_architecture/` — not in this file’s bullet list.
- **Token / concurrency targets** in older tables were planning aids; **enforce what the code actually checks**, not a number from a historical sprint doc.

---

## What this file is not

- It is **not** a product roadmap or task tracker — use issue tracker / team process for status.  
- It is **not** a substitute for `business-core/` or integration tests when deciding what the product does in production.

---

## License and contributions

See [`CONTRIBUTING.md`](./CONTRIBUTING.md). By contributing, you agree your contributions follow the project license (MIT OR Apache-2.0).
