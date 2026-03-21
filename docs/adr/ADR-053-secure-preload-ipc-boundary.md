# ADR-053: Enforce a Secure Preload and IPC Boundary

**Date:** 2026-03-17  
**Status:** Accepted

## Context

The frontend reset must not recreate an unsafe renderer that can reach privileged APIs directly.

## Decision

Enforce:

- `contextIsolation: true`
- `nodeIntegration: false`
- a preload bridge that exposes only explicit typed methods
- IPC payload validation at the main-process boundary

## Why

- it prevents React components from acquiring filesystem or process access
- it keeps desktop capabilities discoverable and auditable
- it creates a stable seam for future Rust-backed features

## Consequences

- React code must consume services built on `window.openaktaDesktop`
- Electron main owns all native, process, and future Rust integration logic
- raw `ipcRenderer` exposure is prohibited
