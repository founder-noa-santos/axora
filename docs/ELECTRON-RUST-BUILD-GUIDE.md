# Electron + Rust Build Guide

**Date:** 2026-03-17  
**Scope:** Local development, build, package, and future Rust ownership

## Desktop app commands

All desktop commands run from the repo root:

```bash
pnpm --filter @openakta/desktop dev
pnpm --filter @openakta/desktop lint
pnpm --filter @openakta/desktop typecheck
pnpm --filter @openakta/desktop test
pnpm --filter @openakta/desktop build
pnpm --filter @openakta/desktop package
```

## What each command does

- `dev`: starts Next.js, watches Electron main/preload bundles, then launches Electron
- `build`: produces static renderer assets in `out/` and bundled Electron files in `dist-electron/`
- `package`: runs `electron-builder` using [apps/desktop/electron-builder.yml](/Users/noasantos/Fluri/openakta/apps/desktop/electron-builder.yml)

## Production artifacts

- Renderer export: [apps/desktop/out](/Users/noasantos/Fluri/openakta/apps/desktop/out)
- Electron bundles: [apps/desktop/dist-electron](/Users/noasantos/Fluri/openakta/apps/desktop/dist-electron)
- Packaged app output: [apps/desktop/release](/Users/noasantos/Fluri/openakta/apps/desktop/release)

## macOS notes

- The window uses `titleBarStyle: "hiddenInset"` and a dark vibrancy-first shell.
- Packaging currently targets macOS `dmg` and `zip`.
- Icons are sourced from [apps/desktop/build/icons](/Users/noasantos/Fluri/openakta/apps/desktop/build/icons).

## Future Rust build ownership

When Rust integration lands:

1. Build or locate the daemon/binary from Electron main or a coordinated release step.
2. Keep Rust startup and health checks in the desktop shell, not the renderer.
3. Package Rust binaries as Electron resources or install them through the main process bootstrap.

## Recommended CI checks

```bash
pnpm install
pnpm --filter @openakta/desktop lint
pnpm --filter @openakta/desktop typecheck
pnpm --filter @openakta/desktop test
pnpm --filter @openakta/desktop build
cargo test --workspace
```
