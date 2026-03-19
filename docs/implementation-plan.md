# AXORA Implementation Plan

## Status

The backend workspace remains intact. The desktop frontend has been reset and rebuilt around Electron + Next.js.

## Active implementation tracks

### Track 1: Desktop shell foundation

- [x] Remove legacy Tauri/Vite renderer shell
- [x] Create Electron main, preload, and Next.js renderer boundaries
- [x] Add Tailwind CSS v4 tokens and shadcn/ui component foundation
- [x] Create macOS-first desktop shell layout
- [x] Add typed shared contracts for preload and IPC

### Track 2: Desktop integration

- [x] Add local preference persistence in Electron main
- [x] Validate IPC payloads with shared schemas
- [ ] Connect Electron main to live Rust daemon capabilities
- [ ] Add typed mission, run, and workspace contracts backed by Rust services

### Track 3: Backend productization

- [ ] Harden daemon lifecycle management for desktop ownership
- [ ] Define launch, health, and shutdown semantics for Rust sidecar or embedded service mode
- [ ] Add packaging, signing, and release automation for desktop distribution

## Milestones

1. Desktop reset complete: Electron + Next shell boots and renders
2. Secure bridge complete: preload API replaces direct renderer/native coupling
3. Rust integration complete: main process brokers daemon and crate-backed features
4. Release readiness complete: signed desktop bundles and documented operations

## Non-goals

- Reviving the previous Tauri renderer
- Preserving compatibility layers for removed Vite/Tauri UI code
- Exposing Node or Electron primitives directly to React components
