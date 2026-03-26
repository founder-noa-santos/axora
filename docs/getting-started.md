# Getting started

OPENAKTA is a **multi-agent coding system** with a Rust runtime and a **macOS-first** desktop shell (Electron + Next.js).

## Prerequisites

- **Rust**: toolchain matching MSRV in the repo root (`rust-toolchain.toml`, `Cargo.toml`).
- **Node**: see root `package.json` `engines` and `packageManager` (pnpm).
- **API keys**: e.g. OpenAI as needed for your provider setup (see [configuration.md](./configuration.md)).

## CLI quick start

From the repository root:

```bash
export OPENAI_API_KEY=...
cargo run -p openakta-cli -- do "add JWT auth"
```

The runtime bootstraps a local `.openakta/` directory, SQLite, semantic memory, default skills, and the MCP tool boundary where configured.

## Desktop shell

```bash
pnpm install
pnpm --filter @openakta/desktop dev
```

See [install.md](./install.md) for toolchain details and [ELECTRON-RUST-BUILD-GUIDE.md](./ELECTRON-RUST-BUILD-GUIDE.md) for the Electron + Rust layout.

## Where to read next

- [architecture.md](./architecture.md) — system overview  
- [`active_architecture/README.md`](./active_architecture/README.md) — detailed narrative  
- [../business-core/README.md](../business-core/README.md) — what the backend actually implements today  
