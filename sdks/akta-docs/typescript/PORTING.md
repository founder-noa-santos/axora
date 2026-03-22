# Porting akta-docs-core to Python, Java, and C#

The TypeScript implementation in **this directory** (`sdks/akta-docs/typescript/`, npm package `akta-docs-core`) is the **reference** for behavior, rule IDs, thresholds, CLI argv, exit codes, and diagnostics ordering.

The **Python** port lives in [`../python/`](../python/) (package name on PyPI: `akta-docs`). It reuses the same fixtures from `typescript/tests/fixtures/` for parity tests.

The **Java** port lives in [`../java/`](../java/) (Maven, package `dev.openakta.aktadocs`). It uses the same shared fixtures via tests that resolve `../typescript/tests/fixtures/` relative to the Java module root.

The **C# / .NET** port lives in [`../csharp/`](../csharp/) (SDK-style project, namespace `OpenAkta.AktaDocs`, assembly `akta-docs`). Tests load fixtures from `typescript/tests/fixtures/` via paths relative to the test output directory.

## Relationship to `crates/openakta-docs`

The Rust crate at [`../../../crates/openakta-docs/`](../../../crates/openakta-docs/) is part of the OPENAKTA runtime workspace, not the portable `akta-docs` source of truth. Its current linter and config surface overlap in naming with `akta-docs-core`, but they do not provide bit-for-bit CLI or rule parity yet, so do not treat `openakta doc lint` as interchangeable with `akta-docs lint`.

## Shared artifacts

- [schemas/akta-config.schema.json](schemas/akta-config.schema.json) — JSON Schema for `.akta-config.yaml` (YAML parses to the same object shape).
- [schemas/changelog-entry.schema.json](schemas/changelog-entry.schema.json) — canonical changelog append payload.
- [tests/fixtures/](tests/fixtures/) — golden Markdown inputs; each port should reproduce the same diagnostics for the same files under the same config.

## Parity checklist

1. **CLI**: `init`, `lint`, `changelog append`, `create` with the same flags and defaults as `src/cli.ts`.
2. **Exit codes**: `0` success (no errors), `1` lint errors or warning cap exceeded, `2` config/IO/JSON errors.
3. **Diagnostics**: sort by `file`, `line`, `column`, `rule_id`; UTF-16 columns optional but document if different.
4. **Rules**: `META-001`–`META-004`, `META-QUICK`, `STRUCT-008`, `CONTENT-001` with identical semantics (including YAML `date` coerced from native date types).
5. **Changelog**: append at `<!-- akta-changelog-append -->`, atomic replace, same compact line format.

## Suggested stacks (from Phase 1)

| Language   | CLI            | Config        | Markdown        |
| ---------- | -------------- | ------------- | --------------- |
| Python     | Typer or Click | Pydantic v2   | markdown-it-py  |
| Java       | picocli        | Jackson + bean | commonmark-java |
| C# / .NET  | System.CommandLine | System.Text.Json | Markdig      |

## Cross-language tests

Copy `tests/fixtures/*.md` into each implementation’s test data directory and assert JSON-equal diagnostics for a frozen `AktaConfig` object matching `tests/linter.test.ts`’s `strictConfig()`.
