# OPENAKTA Desktop

Electron owns the desktop shell. Next.js owns the renderer. React components talk only to the typed preload bridge exposed as `window.openaktaDesktop`.

## Local development

```bash
pnpm install
pnpm --filter @openakta/desktop dev
```

This starts:

- Next.js on `http://127.0.0.1:4000`
- a watched Electron main/preload bundle in `dist-electron/`
- Electron with `contextIsolation` enabled and `nodeIntegration` disabled

## Quality checks

```bash
pnpm --filter @openakta/desktop lint
pnpm --filter @openakta/desktop typecheck
pnpm --filter @openakta/desktop test
pnpm --filter @openakta/desktop build
pnpm --filter @openakta/desktop package
```

## Structure

```text
apps/desktop/
├── app/                  # Next.js App Router renderer
├── components/           # Shell layout and UI primitives
├── electron/
│   ├── main/             # BrowserWindow and IPC handlers
│   └── preload/          # Typed bridge exposed to the renderer
├── lib/                  # Renderer services, metrics, utilities
├── shared/               # Typed contracts shared across boundaries
├── styles/               # Centralized design tokens
└── tests/                # Renderer and contract tests
```

## Future Rust integration

Rust stays outside the renderer. Future integration should land behind Electron main IPC handlers or a sidecar owned by the main process. The renderer should continue consuming stable contracts from `shared/contracts`.
