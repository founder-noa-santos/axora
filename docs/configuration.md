# Configuration

## User-facing config

- **Example / template**: `openakta.example.toml` (repository root).
- **Typical user path** (when using XDG-style layout): `~/.config/openakta/openakta.toml` — copy from the example and adjust.

## Providers and models

- Wire vs telemetry identifiers (`WireProfile`, `ProviderKind`) are documented in code and in [`business-core/`](../business-core/).
- Catalog-style examples (hosted JSON, protocol policy) live under [`catalog-registry-examples/`](./catalog-registry-examples/).

## Desktop

- Renderer must not access privileged APIs directly; configuration flows through **Electron main** and the **preload** IPC surface (see ADRs under [`adr/`](./adr/)).

## Diagnostics / SDKs

- Canonical wide-event payload: [wide-event-schema.md](./wide-event-schema.md).
