# openakta-docs

Rust documentation tooling for the `openakta` workspace.

## Warning

This crate is not the source of truth for portable `akta-docs` CLI parity. The current reference implementation for portable rule IDs, config shape, CLI argv, exit codes, and shared fixture behavior lives in [`../../sdks/akta-docs/typescript/`](../../sdks/akta-docs/typescript/) (npm package name remains `akta-docs-core`).

`openakta doc lint` and `akta-docs lint` should be treated as different products until their linter semantics and config contracts are explicitly unified.
