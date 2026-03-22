# akta-docs (Python)

Python port of **akta-docs-core** — same CLI (`akta-docs`), rules, and schemas as the [TypeScript reference](../typescript/).

## Install

```bash
cd sdks/akta-docs/python
pip install -e .
# or after publish: pip install akta-docs
```

## Development

Use a virtualenv (recommended on macOS with PEP 668). Full check: **ruff** (lint + format check), **pytest**, and **`python -m build`** (sdist/wheel):

```bash
python3 -m venv .venv && . .venv/bin/activate
pip install -e ".[dev]"
ruff check src tests && ruff format --check src tests
python -m pytest
python -m build
```

## Parity

- Config: `.akta-config.yaml` (Pydantic validation)
- Rules: `META-001`–`META-004`, `META-QUICK`, `STRUCT-008`, `CONTENT-001`
- Fixtures: reuse `../typescript/tests/fixtures/` in tests

The Rust workspace command `openakta doc lint` is not the same product surface as this portable port. Use the TypeScript package under `../typescript/` as the parity reference until the Rust runtime linter is explicitly aligned.

See [PORTING.md](../typescript/PORTING.md) in the TS package.

## License

MIT
