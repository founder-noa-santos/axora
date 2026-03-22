from __future__ import annotations

from typing import Any, Literal

from pydantic import BaseModel, Field

DOC_TYPES = (
    "adr",
    "business_rule",
    "feature",
    "guide",
    "reference",
    "explanation",
    "research",
    "meta",
    "changelog",
    "technical",
    "other",
)


class RuleOptions(BaseModel):
    model_config = {"extra": "forbid"}

    severity: Literal["error", "warn", "off"] | None = None
    min_words: int | None = Field(default=None, gt=0)
    max_words: int | None = Field(default=None, gt=0)
    min_question_ratio: float | None = Field(default=None, ge=0, le=1)
    heading_levels: list[Literal[2, 3]] | None = None


class ProjectCfg(BaseModel):
    model_config = {"extra": "forbid"}

    name: str = Field(min_length=1)
    slug: str | None = Field(default=None, min_length=1)


class PathsCfg(BaseModel):
    model_config = {"extra": "forbid"}

    docs_root: str = Field(min_length=1)
    include_globs: list[str] = Field(default_factory=lambda: ["**/*.md"])
    exclude_globs: list[str] = Field(
        default_factory=lambda: [
            "**/node_modules/**",
            "**/.git/**",
            "**/99-archive/**",
        ]
    )


class LinterCfg(BaseModel):
    model_config = {"extra": "forbid"}

    default_severity: Literal["error", "warn", "off"] = "error"
    rules: dict[str, RuleOptions] = Field(default_factory=dict)


class ScaffoldCfg(BaseModel):
    model_config = {"extra": "forbid"}

    create_readme_in_each_folder: bool = True
    gitkeep: bool = False


class ChangelogCfg(BaseModel):
    model_config = {"extra": "forbid"}

    default_target: str | None = None
    entry_template: Literal["compact", "detailed"] = "compact"
    summary_max_length: int = Field(default=200, gt=0)


class AktaConfig(BaseModel):
    model_config = {"extra": "forbid"}

    schema_version: str = Field(min_length=1)
    project: ProjectCfg
    paths: PathsCfg
    linter: LinterCfg = Field(default_factory=LinterCfg)
    scaffold: ScaffoldCfg = Field(default_factory=ScaffoldCfg)
    changelog: ChangelogCfg = Field(default_factory=ChangelogCfg)


class ChangelogEntry(BaseModel):
    model_config = {"extra": "forbid"}

    schema_version: str = Field(min_length=1)
    doc_id: str = Field(min_length=1)
    timestamp: str = Field(min_length=1)
    change_type: Literal["added", "changed", "fixed", "deprecated", "removed", "security"]
    summary: str = Field(min_length=1)
    details: str | None = None
    scope: str | None = None
    refs: list[str] | None = None


class Diagnostic(BaseModel):
    file: str
    line: int
    column: int
    rule_id: str
    severity: Literal["error", "warn", "info"]
    message: str
    end_line: int | None = None
    end_column: int | None = None
    doc_url: str | None = None


class LintSummary(BaseModel):
    error_count: int
    warn_count: int


class LintResult(BaseModel):
    diagnostics: list[Diagnostic]
    summary: LintSummary


def parse_akta_config(data: Any) -> AktaConfig:
    return AktaConfig.model_validate(data)
