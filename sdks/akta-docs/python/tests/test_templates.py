from __future__ import annotations

import tempfile
from pathlib import Path

from akta_docs.engine import lint_files
from akta_docs.models import AktaConfig, ChangelogCfg, LinterCfg, PathsCfg, ProjectCfg, ScaffoldCfg
from akta_docs.templates import write_template


def strict_config() -> AktaConfig:
    return AktaConfig(
        schema_version="1.0.0",
        project=ProjectCfg(name="Test", slug="test"),
        paths=PathsCfg(docs_root="./akta-docs", include_globs=["**/*.md"], exclude_globs=[]),
        linter=LinterCfg(default_severity="error", rules={}),
        scaffold=ScaffoldCfg(),
        changelog=ChangelogCfg(),
    )


def test_adr_template_passes_lint() -> None:
    with tempfile.TemporaryDirectory() as d:
        out = Path(d) / "adr.md"
        write_template(
            "adr",
            out,
            title="Use Postgres",
            slug="postgres",
            doc_id="test.postgres.adr",
            date="2025-03-21",
        )
        res = lint_files([str(out)], strict_config(), d)
        errors = [x for x in res.diagnostics if x.severity == "error"]
        assert errors == []
