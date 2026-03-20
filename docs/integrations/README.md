# SDK Integration Guides

This folder documents adapter behavior for third-party telemetry and analytics targets.

## Scope

- [TypeScript](./typescript.md)
- [Python](./python.md)
- [Java](./java.md)
- [C#](./csharp.md)

## Adapter Rules

- The core Wide Event contract is authoritative.
- Adapters translate the canonical payload into the vendor-specific transport format.
- No adapter may mutate the finalized payload.
- Sink failures are swallowed by the core SDK.
