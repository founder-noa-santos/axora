from __future__ import annotations

from typing import Literal

Severity = Literal["error", "warn", "info"]

RuleId = Literal[
    "META-001",
    "META-002",
    "META-003",
    "META-004",
    "META-QUICK",
    "STRUCT-008",
    "CONTENT-001",
]

ChangeType = Literal[
    "added",
    "changed",
    "fixed",
    "deprecated",
    "removed",
    "security",
]

TemplateKind = Literal[
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
]

KINDS: tuple[str, ...] = (
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
