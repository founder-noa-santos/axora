# Rust tooling baseline (OPENAKTA)

**Status:** Active  
**Last updated:** 2026-03-20  

This document is the canonical description of Rust formatting, linting, Cargo usage, and CI for the OPENAKTA workspace. It complements [CONTRIBUTING.md](../CONTRIBUTING.md).

---

## 1. Repository assessment (historical)

**Before this baseline:**

- No `rustfmt.toml` / `clippy.toml` / `.cargo/config.toml`.
- `.gitignore` excluded `Cargo.lock` even though the workspace ships binaries (`openakta-cli`, `openakta-daemon`).
- No dedicated GitHub Actions workflow for Rust; SDK workflows existed separately.
- Several crates used `#![warn(missing_docs)]`, which makes `cargo clippy -- -D warnings` effectively require a fully documented public API.
- `cargo fmt --check` was not clean; Clippy with `-D warnings` surfaced real issues (dead code, API smells, test drift vs protos).

**After this baseline:** formatting and Clippy are CI-enforced; lockfile is committed; aliases and docs describe the standard workflow.

---

## 2. Proposed standards (now enforced)

| Area | Standard |
|------|-----------|
| **Format** | `rustfmt`, stable options, `newline_style = "Unix"` in `rustfmt.toml`. |
| **Lint** | `clippy` with `-D warnings` on `--workspace --all-targets --all-features` and `--locked`. |
| **MSRV** | `1.88`, declared in `[workspace.package]` and mirrored in `clippy.toml` (matches current `Cargo.lock` / `time` crate requirements). |
| **Lockfile** | `Cargo.lock` committed; CI uses `--locked`. |
| **Workspace lints** | `[workspace.lints.rust]` with `unsafe_op_in_unsafe_fn = "warn"`; each member has `[lints] workspace = true`. |
| **Docs on public API** | Not denied in CI (see tradeoffs). Prefer documenting new exports. |
| **Release builds** | `[profile.release] strip = "debuginfo"` for smaller artifacts without exotic LTO settings. |

---

## 3. File changes (reference)

| Path | Role |
|------|------|
| `rustfmt.toml` | Minimal rustfmt overrides. |
| `clippy.toml` | `msrv = "1.88"`. |
| `.cargo/config.toml` | Aliases: `fmt-check`, `lint`, `check-all`, `test-all`. |
| `.github/workflows/rust-ci.yml` | `fmt --check`, `clippy`, `test` on Rust path changes. |
| `Cargo.toml` | `[workspace.lints]`, `[profile.release]`, comments. |
| `crates/*/Cargo.toml` | `[lints] workspace = true` per member. |
| `.gitignore` | Stop ignoring `Cargo.lock`. |
| `CONTRIBUTING.md` | Contributor-facing commands and expectations. |
| `README.md` | Short Rust section + links. |
| `docs/RUST_TOOLING_BASELINE.md` | This report. |

**Code hygiene (summary):** Numerous Clippy-driven fixes across `openakta-indexing`, `openakta-cache`, `openakta-memory`, `openakta-agents`, `openakta-mcp-server`, `openakta-core`, benches, and tests (e.g. `FromStr` for enums, `Display` for diffs/schemas, removal of dead helpers, proto bench struct updates). Integration tests that are intentionally low-signal use crate-level `#![allow(...)]` where called out in the diff.

---

## 4. Commands and workflow

```bash
# Day to day
cargo fmt --all
cargo check -p openakta-cli

# Pre-push / CI parity
cargo fmt-check
cargo lint
cargo test-all
```

---

## 5. CI strategy

Workflow: **`.github/workflows/rust-ci.yml`**

- **Concurrency:** New pushes cancel in-progress runs for the same ref (faster feedback on stacked commits).
- **Triggers:** `push` to `main` and `pull_request` when Rust-related paths change (workspace, crates, proto, Cargo files, this workflow).
- **Jobs:**
  - **`msrv`:** `cargo check --workspace --all-targets --all-features --locked` on the MSRV toolchain (parallel with `quality`).
  - **`quality`:** `fmt` → `clippy -D warnings` → `cargo test` (default suite; excludes `#[ignore]` slow tests in `openakta-agents`).
  - **`slow-tests`:** On **`push` to `main` only**, runs `cargo test ... -- --ignored` after `quality` succeeds (keeps PR CI fast while still exercising slow tests on the default branch).
- **Environment:** `RUSTFLAGS=-Dwarnings` on compile steps.
- **Caching:** `rust-cache` with separate `prefix-key` for MSRV vs main test cache where useful.

---

## 6. Rationale and tradeoffs

**`Cargo.lock` committed**  
Required for deterministic CI and binary releases. Library-only workspaces sometimes omit the lockfile; this repo is not library-only.

**`missing_docs` not denied in CI**  
Previous `#![warn(missing_docs)]` attributes caused hundreds of warnings under `-D warnings`. Policy is now: workspace-level `missing_docs` stays at default *allow*; optional future step is `[workspace.lints.rust] missing_docs = "warn"` *after* a documentation pass, or crate-scoped `warn` only for stable public crates.

**Targeted `#[allow(clippy::…)]` / test-only `#![allow(...)]`**  
Used where the alternative is large structural refactors unrelated to correctness (e.g. `arc_with_non_send_sync` for `Arc` around rusqlite-backed stores, `items_after_test_module` when `mod tests` is not at file end, legacy integration tests). Each is documented in code or confined to test crates.

**`clippy.toml` only sets MSRV**  
Avoids a large deny-list/allow-list; `-D warnings` remains the main gate.

**Rejected / deferred**

- **Pedantic / nursery Clippy groups globally:** high churn, debated value for this codebase stage.
- **`[profile.release] lto = "fat"` / `codegen-units = 1`:** strong compile-time cost; not enabled without profiling data.
- **Third-party formatters or extra Rust tools:** unnecessary given rustfmt + Clippy + Cargo.

---

## 7. Follow-up recommendations (high value only)

1. **Optional:** Add a scheduled weekly `cargo audit` / `cargo deny` job (not enabled by default to keep CI minimal).  
2. **Re-enable `missing_docs` gradually:** Start with leaf crates or run `cargo doc --document-private-items` in CI as a non-blocking job.  
3. **`cargo deny` / supply-chain:** Consider [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) later for licenses and advisories; not added now to keep the baseline minimal.  
4. **Split long-running tests:** Some tests exceed 60s; consider marking with `#[ignore]` for default `cargo test` or lowering sleeps for CI speed.

---

## 8. Verification log (maintainers)

Recorded when this baseline landed:

- `cargo fmt --all -- --check` — pass  
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` — pass  
- `cargo test --workspace --all-features --locked` — run locally before merge (full suite is long-running)

Update this section when MSRV or CI steps change.
