# Contributing to OPENAKTA

## Rust workspace

This repository is a Cargo workspace (see root `Cargo.toml`). Binaries such as `openakta-cli` and `openakta-daemon` are part of the workspace; **`Cargo.lock` is committed** for reproducible builds and CI (`--locked`).

### Toolchain

- **Edition:** 2021  
- **MSRV:** `1.88` (see `[workspace.package]` in the root `Cargo.toml`). CI runs a parallel `msrv` job with that toolchain; day-to-day development typically uses stable Rust.

### Standard commands

From the repository root:

| Command | Purpose |
|--------|---------|
| `cargo fmt --all` | Format all crates. |
| `cargo fmt-check` | Alias: `fmt --all -- --check` (same as CI). |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Full Clippy (warnings are errors). |
| `cargo lint` | Same as the line above (project alias). |
| `cargo check-all` | `check` across workspace, all targets/features, locked. |
| `cargo test-all` | Full workspace tests with all features, locked (skips `#[ignore]` slow tests). |
| `cargo test-slow` | Run only ignored/slow tests (`-- --ignored`). |
| `cargo build --release -p openakta-cli` | Optimized CLI build. |

### Before opening a PR (Rust changes)

1. `cargo fmt-check`  
2. `cargo lint`  
3. `cargo test-all`  

Or rely on **Rust CI** (`.github/workflows/rust-ci.yml`), which runs the same gates on push/PR when Rust paths change.

### Conventions

- **Formatting:** `rustfmt` with project `rustfmt.toml` (minimal; Unix newlines enforced).  
- **Linting:** Clippy with `-D warnings` on the full workspace including tests and benches.  
- **API docs:** Workspace does not currently deny `missing_docs` in CI; prefer documenting new public API anyway. See `docs/RUST_TOOLING_BASELINE.md`.  
- **TypeScript / desktop:** See `apps/desktop/README.md` and root `README.md` for pnpm workflows.

## License

By contributing, you agree that your contributions are licensed under the same terms as the project (MIT OR Apache-2.0).
