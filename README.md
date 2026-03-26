# OPENAKTA MVP

**Repository:** [github.com/openakta/aktacode](https://github.com/openakta/aktacode)

OPENAKTA is a batteries-included multi-agent coding system with a Rust runtime and a macOS-first desktop shell.

## Quick start

```bash
export OPENAI_API_KEY=...
cargo run -p openakta-cli -- do "add JWT auth"
```

OPENAKTA bootstraps its local runtime automatically for the mission path:

- infers the workspace from the current directory
- creates a local `.openakta/` runtime directory
- initializes SQLite, semantic memory, and the default skill library
- starts the native MCP tool boundary
- boots the default Base Squad inside `CoordinatorV2`

## Mission Operating Layer (MOL)

**Today:** The hosted work-management stack persists Mission Operating Layer data in Postgres ([`../openakta-api/migrations/0005_mission_operating_layer.sql`](../openakta-api/migrations/0005_mission_operating_layer.sql)) and exposes it through gRPC work-management RPCs ([`../openakta-api/src/work_management.rs`](../openakta-api/src/work_management.rs), [`proto/work/v1/work.proto`](./proto/work/v1/work.proto)). The local daemon mirrors read models and pending commands in SQLite at `.openakta/work-management.db` ([`crates/openakta-daemon/src/background/work_mirror.rs`](./crates/openakta-daemon/src/background/work_mirror.rs)) and runs compilation / coordination against that mirror plus API calls.

**Target:** End-to-end **hard gates** so story preparation, closure, and verification invariants cannot be satisfied only by rich data—they must hold on every read/write path (see product roadmap: “MOL hard gates”). Until those gates are complete, some flows remain **best-effort** or **legacy-compatible**.

**Legacy paths (still in play):** Work can still be created or advanced via **raw work items** and daemon execution without a fully prepared story; the compiler and coordinator may synthesize or ignore MOL fields depending on context. Treat docs under `docs/aios/` as the **intended** model; enforcement depth is documented per topic there.

For a fuller picture: [mission-operating-layer.md](./docs/aios/mission-operating-layer.md), [01_CORE_ARCHITECTURE.md](./docs/active_architecture/01_CORE_ARCHITECTURE.md).

## Current desktop architecture

The desktop app in [apps/desktop](./apps/desktop) uses:

- Electron for the native shell
- Next.js App Router for the renderer
- React + TypeScript for UI code
- Tailwind CSS v4 for styling tokens and utilities
- shadcn/ui primitives for foundational components
- Lucide React as the only icon system

The renderer is isolated from privileged APIs. Native capabilities are exposed only through a typed preload bridge and IPC handlers owned by Electron main.

## Project structure

```text
openakta/
├── apps/
│   └── desktop/          # Electron + Next.js desktop app
├── crates/               # Rust workspace crates
├── sdks/                 # Language SDKs
├── integrations/         # Vendor adapters for SDKs
├── docs/                 # Documentation (see docs/README.md)
├── business-core/        # Business rules grounded in code
└── proto/                # Protocol buffer schemas
```

## Desktop shell

```bash
pnpm install
pnpm --filter @openakta/desktop dev
```

Useful commands:

```bash
pnpm --filter @openakta/desktop lint
pnpm --filter @openakta/desktop typecheck
pnpm --filter @openakta/desktop test
pnpm --filter @openakta/desktop build
pnpm --filter @openakta/desktop package
cargo test --workspace
```

## Rust workspace

- **MSRV:** `1.94` (see root `Cargo.toml` and `rust-toolchain.toml`; enforced by CI).  
- **Lockfile:** `Cargo.lock` is tracked; use `cargo … --locked` in CI and release flows.  
- **Quick validation:** `cargo fmt-check && cargo lint && cargo test-all` (aliases from `.cargo/config.toml`).  
- **Details:** [CONTRIBUTING.md](./CONTRIBUTING.md) and [docs/RUST_TOOLING_BASELINE.md](./docs/RUST_TOOLING_BASELINE.md).

## SDKs

The OPENAKTA diagnostics SDKs live under `sdks/` and `integrations/`.

- Canonical schema: [docs/wide-event-schema.md](./docs/wide-event-schema.md)
- Usage examples: [docs/examples/](./docs/examples/)
- Integration guides: [docs/integrations/](./docs/integrations/)

TypeScript packages are part of the pnpm workspace:

```bash
pnpm build
pnpm test
pnpm lint
pnpm typecheck
```

## Documentation

- **Index:** [DOCS-INDEX.md](./DOCS-INDEX.md) and [docs/README.md](./docs/README.md)
- Architecture overview: [docs/architecture.md](./docs/architecture.md)
- Implementation status and ledger: [docs/ARCHITECTURE-LEDGER.md](./docs/ARCHITECTURE-LEDGER.md)
- Desktop build and runtime: [docs/ELECTRON-RUST-BUILD-GUIDE.md](./docs/ELECTRON-RUST-BUILD-GUIDE.md)

## License

MIT OR Apache-2.0
