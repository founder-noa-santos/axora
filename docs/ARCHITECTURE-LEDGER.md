# Architecture Ledger (Detailed)

**Last Updated:** 2026-03-17  
**Maintained By:** Architect Agent  
**Status:** Active

## Current desktop baseline

- Shell: Electron
- Renderer: Next.js App Router
- UI: React + TypeScript + Tailwind CSS v4 + shadcn/ui + Lucide React
- Boundary: preload bridge + IPC only
- Backend path: future Rust integration owned by Electron main

## Recent changes

### 2026-03-21

- **Root `AGENTS.md` rewritten** to a Codex-style contributor/coding-agent guide (Rust and desktop workflows, source-of-truth order, commands). Long sprint-history tables and metrics that previously lived in `AGENTS.md` were removed from that file to reduce drift and duplication. Recover pre-revision content from git history if needed (`git log -- AGENTS.md`).
- **Repository cleanup:** Removed `planning/`, `research/`, and `.backup-outdated/`; removed audit/plan Markdown at repo root and obsolete docs (`docs/*` migration/HITL/benchmark drafts). Added `docs/README.md` plus `getting-started.md`, `install.md`, `configuration.md`, `contributing.md` as a topic-based doc hub. Updated `DOCS-INDEX.md`, `README.md`, and cross-references accordingly.

### 2026-03-17

- Replaced the previous Tauri/Vite desktop frontend with an Electron + Next.js shell in `apps/desktop`
- Removed legacy renderer architecture, tests, docs, and packaging config tied to Tauri/Vite
- Added a minimal typed preload bridge with schema-validated IPC handlers
- Established a macOS-first shell UI system with centralized Tailwind v4 tokens and local shadcn/ui primitives
- Superseded prior frontend shell assumptions in architecture docs and ADRs

## Active ADRs

| ADR | Title | Status |
|-----|-------|--------|
| ADR-042 | Graph-Based Workflow | Active |
| ADR-043 | Sliding-Window Semaphores | Active |
| ADR-044 | Atomic Checkout | Active |
| ADR-045 | Repository Map | Active |
| ADR-046 | AGENTS.md Ledger | Active |
| ADR-050 | Use shadcn/ui for Desktop Components | Active, updated for Electron + Next |
| ADR-051 | Use Electron as Desktop Shell | Active |
| ADR-052 | Use Next.js App Router as Renderer | Active |
| ADR-053 | Enforce Preload + IPC Boundary | Active |

## Superseded assumptions

- Tauri is no longer the active desktop shell architecture.
- Vite is no longer the active desktop renderer build system.
- Direct Tauri API usage in frontend code is no longer allowed.

Historical planning materials were removed from the tree; use git history if you need old sprint docs.
