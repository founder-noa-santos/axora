from __future__ import annotations

import re
from datetime import date
from pathlib import Path

import yaml

from akta_docs.models import (
    AktaConfig,
    ChangelogCfg,
    LinterCfg,
    PathsCfg,
    ProjectCfg,
    RuleOptions,
    ScaffoldCfg,
)

SECTION_DIRS = (
    "00-meta",
    "01-adrs",
    "02-business-core",
    "03-business-logic",
    "04-research",
    "05-features",
    "06-technical",
    "07-guides",
    "08-references",
    "09-explanations",
    "10-changelog",
    "99-archive",
)


def slugify(name: str) -> str:
    s = re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")
    return s or "project"


def default_akta_config(project_name: str) -> AktaConfig:
    slug = slugify(project_name)
    return AktaConfig(
        schema_version="1.0.0",
        project=ProjectCfg(name=project_name, slug=slug),
        paths=PathsCfg(docs_root="./akta-docs"),
        linter=LinterCfg(
            default_severity="error",
            rules={
                "META-QUICK": RuleOptions(severity="off"),
                "STRUCT-008": RuleOptions(severity="off"),
                "CONTENT-001": RuleOptions(severity="off"),
            },
        ),
        scaffold=ScaffoldCfg(),
        changelog=ChangelogCfg(
            default_target="akta-docs/10-changelog/CHANGELOG.md",
        ),
    )


class ScaffoldReport:
    def __init__(
        self,
        root: str,
        docs_root: str,
        created_paths: list[str],
        config_path: str,
    ) -> None:
        self.root = root
        self.docs_root = docs_root
        self.created_paths = created_paths
        self.config_path = config_path


def run_scaffold(
    root: str,
    project_name: str,
    *,
    force: bool = False,
    dry_run: bool = False,
    create_readme_in_each_folder: bool = True,
    gitkeep: bool = False,
) -> ScaffoldReport:
    root_p = Path(root).resolve()
    config_path = root_p / ".akta-config.yaml"
    cfg = default_akta_config(project_name)
    cfg.scaffold = ScaffoldCfg(
        create_readme_in_each_folder=create_readme_in_each_folder,
        gitkeep=gitkeep,
    )
    docs_root = root_p / "akta-docs"
    created: list[str] = []

    if not dry_run and config_path.exists() and not force:
        raise FileExistsError(f"{config_path} already exists; use --force to overwrite.")

    slug = cfg.project.slug or slugify(project_name)
    today = date.today().isoformat()

    for dir_name in SECTION_DIRS:
        full = docs_root / dir_name
        if not dry_run:
            full.mkdir(parents=True, exist_ok=True)
        created.append(str(Path("akta-docs") / dir_name))

        if create_readme_in_each_folder:
            readme = full / "README.md"
            doc_id = f"{slug}.{re.sub(r'[^a-z0-9]+', '-', dir_name)}-readme"
            body = f"""---
doc_id: {doc_id}
doc_type: meta
date: {today}
---

# {dir_name}

Placeholder content for this section. Replace with architecture and business documentation aligned with OPENAKTA GEO standards.
"""
            if not dry_run:
                readme.write_text(body, encoding="utf-8")
            created.append(str(Path("akta-docs") / dir_name / "README.md"))

        if gitkeep:
            gk = full / ".gitkeep"
            if not dry_run:
                gk.write_text("", encoding="utf-8")

    changelog_path = docs_root / "10-changelog" / "CHANGELOG.md"
    initial = f"""---
doc_id: {slug}.changelog
doc_type: changelog
date: {today}
---

# Changelog

<!-- akta-changelog-append -->
"""
    if not dry_run:
        changelog_path.write_text(initial, encoding="utf-8")
    created.append("akta-docs/10-changelog/CHANGELOG.md")

    yaml_text = yaml.safe_dump(
        cfg.model_dump(mode="json", exclude_none=True),
        sort_keys=False,
        allow_unicode=True,
    )
    if not dry_run:
        config_path.write_text(yaml_text, encoding="utf-8")

    return ScaffoldReport(
        root=str(root_p),
        docs_root=str(docs_root),
        created_paths=created,
        config_path=str(config_path),
    )
