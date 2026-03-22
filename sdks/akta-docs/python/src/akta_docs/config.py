from __future__ import annotations

from pathlib import Path
from typing import Any

import yaml
from pydantic import ValidationError

from akta_docs.models import AktaConfig, parse_akta_config


class ConfigError(Exception):
    pass


def load_config(config_path: str | Path) -> AktaConfig:
    p = Path(config_path)
    try:
        raw = p.read_text(encoding="utf-8")
    except OSError as e:
        raise ConfigError(f"Cannot read config: {p}") from e
    try:
        data: Any = yaml.safe_load(raw)
    except yaml.YAMLError as e:
        raise ConfigError(f"Invalid YAML in {p}") from e
    if not isinstance(data, dict):
        raise ConfigError("Invalid .akta-config.yaml: expected mapping at root")
    try:
        return parse_akta_config(data)
    except ValidationError as e:
        raise ConfigError(f"Invalid .akta-config.yaml: {e}") from e


def resolve_config_path(root: str | Path, explicit: str | Path | None = None) -> Path:
    r = Path(root).resolve()
    if explicit is not None:
        return Path(explicit).resolve()
    return r / ".akta-config.yaml"
