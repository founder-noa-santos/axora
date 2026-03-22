from __future__ import annotations

import os
from pathlib import Path

import pytest

from akta_docs.engine import lint_files
from akta_docs.models import AktaConfig, ChangelogCfg, LinterCfg, PathsCfg, ProjectCfg, ScaffoldCfg
from akta_docs.word_count import count_words

FIXTURES = Path(__file__).resolve().parent.parent.parent / "typescript" / "tests" / "fixtures"


def fixture_path(name: str) -> Path:
    path = FIXTURES / name
    if path.exists():
        return path

    if os.getenv("CI") and os.getenv("SKIP_FIXTURES") != "1":
        pytest.fail(f"missing shared TS fixture: {path}")

    pytest.skip(f"shared TS fixtures not present: {path}")


def strict_config() -> AktaConfig:
    return AktaConfig(
        schema_version="1.0.0",
        project=ProjectCfg(name="Test", slug="test"),
        paths=PathsCfg(docs_root="./akta-docs", include_globs=["**/*.md"], exclude_globs=[]),
        linter=LinterCfg(default_severity="error", rules={}),
        scaffold=ScaffoldCfg(),
        changelog=ChangelogCfg(),
    )


def test_count_words() -> None:
    assert count_words("a b c") == 3
    assert count_words("") == 0


def test_meta001_no_frontmatter() -> None:
    f = fixture_path("no-frontmatter.md")
    res = lint_files([str(f)], strict_config(), str(FIXTURES.parent))
    ids = [d.rule_id for d in res.diagnostics]
    assert "META-001" in ids


def test_compliant_no_errors() -> None:
    f = fixture_path("compliant.md")
    res = lint_files([str(f)], strict_config(), str(FIXTURES.parent))
    errors = [d for d in res.diagnostics if d.severity == "error"]
    assert errors == []


def test_struct008_short_section() -> None:
    f = fixture_path("short-section.md")
    res = lint_files([str(f)], strict_config(), str(FIXTURES.parent))
    assert any(d.rule_id == "STRUCT-008" for d in res.diagnostics)
