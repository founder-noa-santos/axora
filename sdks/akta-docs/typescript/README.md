# akta-docs-core

OPENAKTA documentation tooling: scaffold a standard doc tree, lint AI-oriented Markdown, append changelog entries without rewriting files, and generate compliant templates.

## Install

```bash
npm install -g akta-docs-core
# or
npx akta-docs-core --help
```

## Commands

- `akta-docs init` — create `akta-docs/` layout and `.akta-config.yaml`
- `akta-docs lint` — run META/STRUCT/CONTENT rules (ESLint-style output or `--format json`)
- `akta-docs changelog append --file <path>` — append from JSON payload (stdin or `--payload`)
- `akta-docs create <kind> <path>` — write a template (`adr`, `business_rule`, …)

## Config and schemas

See `schemas/` for JSON Schema. Normative rule IDs: `META-001`–`META-004`, `META-QUICK`, `STRUCT-008`, `CONTENT-001`.

## Development

From `sdks/akta-docs/typescript`:

```bash
npm ci
npm run verify   # lint (tsc --noEmit) + test + build
```

## Porting

See [PORTING.md](PORTING.md) for Python, Java, and .NET parity with this reference implementation.

The Rust crate at [`../../../crates/openakta-docs/`](../../../crates/openakta-docs/) is a separate runtime-facing implementation. It currently shares some rule-name vocabulary with `akta-docs-core` but is not a drop-in replacement for portable `akta-docs` CLI behavior.

## License

MIT
