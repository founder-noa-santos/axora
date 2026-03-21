# Electron + Rust Integration Plan

**Date:** 2026-03-17  
**Status:** Active

## Goal

Keep the new Electron + Next.js frontend stable while integrating existing Rust crates behind Electron main.

## Current state

- Renderer: Next.js App Router with static export for production loading
- Shell: Electron main process with secure preload bridge
- Bridge: typed contracts in [apps/desktop/shared/contracts/desktop.ts](/Users/noasantos/Fluri/openakta/apps/desktop/shared/contracts/desktop.ts)
- Rust: existing crates remain outside the desktop renderer and are not imported directly by UI code

## Integration principle

Rust integration must not leak transport details into React.

The renderer should only call service methods shaped like:

```ts
desktopService.runMission(input)
desktopService.searchWorkspace(query)
desktopService.getMissionStream(id)
```

Whether those capabilities come from:

- Electron-owned child process management
- direct Node-side bindings
- a local daemon over sockets
- another internal transport

is a main-process concern.

## Recommended rollout

### Stage 1: Stable contract surface

- Keep extending `shared/contracts` with Zod schemas and exported TypeScript types.
- Add new preload methods only after main process handlers exist.
- Treat every channel as versioned application surface.

### Stage 2: Main-process Rust ownership

- Start or attach to the Rust daemon from Electron main.
- Keep process lifecycle, environment, and failure handling out of React.
- Translate daemon or crate errors into typed IPC error envelopes.

### Stage 3: Renderer service migration

- Replace shell placeholders with service calls in `lib/services`.
- Keep components dumb: render data and UI state only.
- Avoid storing transport-specific state inside components.

## Initial capability map

| Capability | Renderer call site | Main-process owner | Rust target |
|------------|--------------------|--------------------|-------------|
| App metadata | `desktopClient.getInfo()` | Electron main | none |
| Preferences | `desktopClient.getPreferences()` | Electron main | none |
| Mission execution | future service | Electron main | `openakta-core`, `openakta-daemon` |
| Task/run history | future service | Electron main | `openakta-storage` |
| Repository search/index | future service | Electron main | `openakta-indexing`, `openakta-rag` |

## Constraints

- No direct Rust bindings inside the renderer
- No `ipcRenderer` exposure beyond the typed bridge
- No fake success paths for unimplemented Rust capabilities
- No transport-specific UI assumptions
