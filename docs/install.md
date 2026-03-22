# Install and build

## Rust workspace

- Workspace definition: root `Cargo.toml`.
- **Lockfile**: `Cargo.lock` is committed; use `cargo … --locked` in CI-style flows.
- **MSRV**: see `[workspace.package]` / `rust-toolchain.toml`.

Standard checks from the repo root (aliases in `.cargo/config.toml`):

```bash
cargo fmt-check
cargo lint
cargo test-all
```

See [RUST_TOOLING_BASELINE.md](./RUST_TOOLING_BASELINE.md) and [../CONTRIBUTING.md](../CONTRIBUTING.md).

## Node / pnpm (desktop + TS SDKs)

```bash
pnpm install
```

Use the root `package.json` scripts and `pnpm --filter <package>` for individual packages.

## Protocol buffers

When `.proto` files change:

```bash
pnpm proto:gen
```

(from repository root; see `package.json`.)

## Desktop packaging

See [apps/desktop/README.md](../apps/desktop/README.md) and [ELECTRON-RUST-BUILD-GUIDE.md](./ELECTRON-RUST-BUILD-GUIDE.md).
