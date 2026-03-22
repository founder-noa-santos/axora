from __future__ import annotations

from typing import Literal

from akta_docs.models import AktaConfig

EffectiveSeverity = Literal["error", "warn", "off"]


def rule_severity(config: AktaConfig, rule_id: str) -> EffectiveSeverity:
    override = None
    if rule_id in config.linter.rules:
        override = config.linter.rules[rule_id].severity
    if override is not None:
        return override
    return config.linter.default_severity


def rule_number_option(
    config: AktaConfig,
    rule_id: str,
    key: str,
    fallback: float,
) -> float:
    if rule_id not in config.linter.rules:
        return fallback
    ro = config.linter.rules[rule_id]
    v = getattr(ro, key, None)
    if isinstance(v, (int, float)):
        return float(v)
    return fallback
