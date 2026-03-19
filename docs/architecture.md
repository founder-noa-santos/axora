# AXORA Architecture

## Overview

AXORA is split into two clear layers:

1. A Rust workspace that owns domain logic, storage, indexing, orchestration, and daemon behavior.
2. A desktop application that owns local shell behavior, windowing, renderer composition, and future native capability brokering.

## Current desktop topology

```text
┌──────────────────────────────────────────────────────────────────────┐
│                           Electron Main                              │
│  BrowserWindow lifecycle · IPC handlers · native integration layer   │
└──────────────────────────────┬───────────────────────────────────────┘
                               │
                      contextBridge + IPC only
                               │
┌──────────────────────────────▼───────────────────────────────────────┐
│                           Electron Preload                           │
│         Minimal typed bridge exposed as window.axoraDesktop          │
└──────────────────────────────┬───────────────────────────────────────┘
                               │
┌──────────────────────────────▼───────────────────────────────────────┐
│                     Next.js Renderer (App Router)                    │
│ React components · Tailwind v4 tokens · shadcn/ui · Lucide icons     │
└──────────────────────────────┬───────────────────────────────────────┘
                               │
                future daemon / crate integration owned by main
                               │
┌──────────────────────────────▼───────────────────────────────────────┐
│                          Rust Workspace                               │
│ axora-core · axora-daemon · axora-storage · axora-* domain crates    │
└──────────────────────────────────────────────────────────────────────┘
```

## Boundary rules

- React components do not import Node.js or Electron APIs.
- The preload script exposes a small typed contract for desktop state and preferences.
- IPC payloads are validated in the main process using shared Zod schemas.
- Future Rust capabilities must be implemented behind Electron main, not inside the renderer.

## Repository layout

### Desktop app

- [apps/desktop/app](/Users/noasantos/Fluri/axora/apps/desktop/app): Next.js App Router entrypoints
- [apps/desktop/components](/Users/noasantos/Fluri/axora/apps/desktop/components): shell layout and UI primitives
- [apps/desktop/electron/main](/Users/noasantos/Fluri/axora/apps/desktop/electron/main): window bootstrap, IPC registration, local persistence
- [apps/desktop/electron/preload](/Users/noasantos/Fluri/axora/apps/desktop/electron/preload): typed desktop bridge
- [apps/desktop/shared/contracts](/Users/noasantos/Fluri/axora/apps/desktop/shared/contracts): shared schemas and API contracts

### Rust workspace

- [crates/axora-core](/Users/noasantos/Fluri/axora/crates/axora-core): core orchestration logic
- [crates/axora-daemon](/Users/noasantos/Fluri/axora/crates/axora-daemon): daemon executable
- [crates/axora-storage](/Users/noasantos/Fluri/axora/crates/axora-storage): persistence
- [crates/axora-proto](/Users/noasantos/Fluri/axora/crates/axora-proto): shared protocol types

## Renderer strategy

The renderer is intentionally static-first:

- Next.js uses App Router for composition and file-system structure.
- The initial shell exports to static assets for Electron production loading.
- Server components are used only where they simplify structure; privileged work stays outside the renderer.

## UI system

- Tailwind CSS v4 tokens are centralized in [apps/desktop/styles/tokens.css](/Users/noasantos/Fluri/axora/apps/desktop/styles/tokens.css).
- shadcn/ui components are kept local and edited in-repo.
- Lucide React is the single icon family and should remain the only default icon set.
- The shell is macOS-first: hidden inset title bar, restrained dark surfaces, dense but calm information layout.
