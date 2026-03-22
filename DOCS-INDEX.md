# OPENAKTA documentation index

**Last updated:** 2026-03-21  

---

## Hub

| Resource | Purpose |
|----------|---------|
| **[docs/README.md](./docs/README.md)** | Topic index (start here for navigation) |
| **[AGENTS.md](./AGENTS.md)** | Contributor and coding-agent conventions |
| **[CONTRIBUTING.md](./CONTRIBUTING.md)** | Rust/PR workflow and toolchain |
| **[business-core/](./business-core/)** | What the backend **actually** implements (source-of-truth for product claims) |

---

## Portable docs tooling

The portable `akta-docs` polyglot packages live under **`./sdks/akta-docs/`** (TypeScript reference plus Python, Java, and C# ports), alongside the existing logger SDK layout under `sdks/`. The Rust runtime-oriented docs commands live in `./crates/openakta-docs/` and `./crates/openakta-cli/` for `openakta`.

CI for this family (plus Rust fixture parity in `openakta-docs`) is **[`.github/workflows/akta-docs-ci.yml`](./.github/workflows/akta-docs-ci.yml)**.

Treat those as two distinct config and CLI surfaces for now. `akta-docs lint` is the portable parity target; `openakta doc lint` is the runtime-oriented Rust command and does not yet share the same rule IDs, config contract, or UX.

---

## Quick links

| Topic | Location |
|--------|----------|
| Getting started | [docs/getting-started.md](./docs/getting-started.md) |
| Install & build | [docs/install.md](./docs/install.md) |
| Configuration | [docs/configuration.md](./docs/configuration.md) |
| Contributing | [docs/contributing.md](./docs/contributing.md) |
| Architecture overview | [docs/architecture.md](./docs/architecture.md) |
| Active architecture (narrative) | [docs/active_architecture/README.md](./docs/active_architecture/README.md) |
| Architecture ledger & ADRs | [docs/ARCHITECTURE-LEDGER.md](./docs/ARCHITECTURE-LEDGER.md) |
| Desktop + Rust build | [docs/ELECTRON-RUST-BUILD-GUIDE.md](./docs/ELECTRON-RUST-BUILD-GUIDE.md) |
| Rust tooling baseline | [docs/RUST_TOOLING_BASELINE.md](./docs/RUST_TOOLING_BASELINE.md) |
| Wide event schema | [docs/wide-event-schema.md](./docs/wide-event-schema.md) |
| SDK examples | [docs/examples/](./docs/examples/) |
| Integration guides | [docs/integrations/](./docs/integrations/) |

---

## Repository layout (documentation-relevant)

```
aktacode/
├── docs/                    # User and contributor documentation
├── business-core/           # Business rules grounded in implementation
├── apps/desktop/            # Electron + Next.js shell
├── crates/                  # Rust workspace
├── sdks/                    # Language SDKs (logger family + akta-docs family)
├── integrations/            # Vendor adapters
└── proto/                   # Protocol buffers
```

---

## Deprecated internal trees

Historical sprint plans and research prompts were **removed** from this repository to reduce drift. Recover them from **git history** if needed (`git log --diff-filter=D --summary`).
