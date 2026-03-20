# SDK Examples

This folder contains consumer-facing usage snippets for the AXORA Wide Event SDKs.

## Included Examples

- [TypeScript](./typescript.md)
- [Python](./python.md)
- [Java](./java.md)
- [C#](./csharp.md)

## Canonical Example Shape

```text
Logger -> startEvent() -> appendContext() -> emit()
Logger -> trace() -> automatic emit on success or failure
```
