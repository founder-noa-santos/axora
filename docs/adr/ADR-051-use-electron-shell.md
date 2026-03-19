# ADR-051: Use Electron as the Desktop Shell

**Date:** 2026-03-17  
**Status:** Accepted

## Context

The previous desktop shell was intentionally discarded as part of the frontend reset. The new shell needed:

- mature desktop windowing
- explicit preload isolation
- stable IPC patterns
- a broad ecosystem for future macOS-first native features

## Decision

Adopt Electron as the desktop shell for `apps/desktop`.

## Why

- Electron gives direct control over secure `BrowserWindow` configuration
- preload, `contextIsolation`, and IPC patterns are explicit and well understood
- the shell can own future Rust daemon lifecycle and native integrations without leaking them into React
- packaging and distribution tooling are mature

## Consequences

- Desktop-native concerns live in Electron main, not the renderer
- The renderer must remain isolated from Node.js and Electron internals
- Packaging moves to `electron-builder`
