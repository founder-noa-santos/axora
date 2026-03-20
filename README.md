# AXORA MVP

AXORA is a batteries-included multi-agent coding system with a Rust runtime and a macOS-first desktop shell.

## Quick start

```bash
export ANTHROPIC_API_KEY=...
cargo run -p axora-cli -- do "add JWT auth"
```

AXORA now bootstraps its local runtime automatically for the mission path:

- infers the workspace from the current directory
- creates a local `.axora/` runtime directory
- initializes SQLite, semantic memory, and the default skill library
- starts the native MCP tool boundary
- boots the default Base Squad inside `CoordinatorV2`

## Current desktop architecture

The desktop app in [apps/desktop](/Users/noasantos/Fluri/axora/apps/desktop) now uses:

- Electron for the native shell
- Next.js App Router for the renderer
- React + TypeScript for UI code
- Tailwind CSS v4 for styling tokens and utilities
- shadcn/ui primitives for foundational components
- Lucide React as the only icon system

The renderer is isolated from privileged APIs. Native capabilities are exposed only through a typed preload bridge and IPC handlers owned by Electron main.

## Project structure

```text
axora/
├── apps/
│   └── desktop/          # Electron + Next.js desktop app
├── crates/              # Rust workspace crates
├── sdks/                # Language SDKs
├── integrations/        # Vendor adapters for SDKs
├── docs/                # Architecture docs and ADRs
├── planning/            # Historical planning material
└── proto/               # Protocol buffer schemas
```

## Desktop shell

```bash
pnpm install
pnpm --filter @axora/desktop dev
```

Useful commands:

```bash
pnpm --filter @axora/desktop lint
pnpm --filter @axora/desktop typecheck
pnpm --filter @axora/desktop test
pnpm --filter @axora/desktop build
pnpm --filter @axora/desktop package
cargo test --workspace
```

## SDKs

The AXORA diagnostics SDKs live under `sdks/` and `integrations/`.

- Canonical schema: [docs/wide-event-schema.md](/Users/noasantos/Fluri/axora/docs/wide-event-schema.md)
- Usage examples: [docs/examples/](/Users/noasantos/Fluri/axora/docs/examples/)
- Integration guides: [docs/integrations/](/Users/noasantos/Fluri/axora/docs/integrations/)

TypeScript packages are part of the pnpm workspace, so they can be built with:

```bash
pnpm build
pnpm test
pnpm lint
pnpm typecheck
```

## Documentation

- Architecture overview: [docs/architecture.md](/Users/noasantos/Fluri/axora/docs/architecture.md)
- Implementation status and ledger: [docs/ARCHITECTURE-LEDGER.md](/Users/noasantos/Fluri/axora/docs/ARCHITECTURE-LEDGER.md)
- Desktop build and runtime guide: [docs/ELECTRON-RUST-BUILD-GUIDE.md](/Users/noasantos/Fluri/axora/docs/ELECTRON-RUST-BUILD-GUIDE.md)
- Desktop and Rust integration plan: [docs/ELECTRON-RUST-MIGRATION-PLAN.md](/Users/noasantos/Fluri/axora/docs/ELECTRON-RUST-MIGRATION-PLAN.md)
- Batteries-included analysis: [BATTERIES_INCLUDED_ANALYSIS.md](/Users/noasantos/Fluri/axora/BATTERIES_INCLUDED_ANALYSIS.md)

## License

MIT OR Apache-2.0
