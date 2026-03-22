from __future__ import annotations

import json
import tempfile
from pathlib import Path

from pydantic import ValidationError

from akta_docs.models import ChangelogEntry

ANCHOR = "<!-- akta-changelog-append -->"


class ChangelogError(Exception):
    pass


def format_entry(entry: ChangelogEntry, template: str) -> str:
    line = f"- **{entry.change_type}** ({entry.timestamp}) {entry.summary}"
    if template == "detailed" and entry.details:
        indented = "\n  ".join(entry.details.split("\n"))
        return f"{line}\n  {indented}"
    return line


def append_changelog_entry(
    target_path: str | Path,
    payload: dict | str | bytes,
    *,
    dry_run: bool = False,
    template: str = "compact",
) -> tuple[str, int, bool]:
    if isinstance(payload, (str, bytes)):
        try:
            data = json.loads(payload)
        except json.JSONDecodeError as e:
            raise ChangelogError(f"Invalid JSON: {e}") from e
    else:
        data = payload
    try:
        entry = ChangelogEntry.model_validate(data)
    except ValidationError as e:
        raise ChangelogError(f"Invalid changelog payload: {e}") from e

    block = f"\n{format_entry(entry, template)}\n"
    path = Path(target_path)

    if path.exists():
        existing = path.read_text(encoding="utf-8")
    else:
        existing = None

    if existing is None:
        created = True
        out = f"""---
doc_id: {entry.doc_id}
doc_type: changelog
date: {entry.timestamp[:10]}
---

# Changelog

{ANCHOR}
{block}"""
    elif ANCHOR in existing:
        created = False
        out = existing.replace(ANCHOR, f"{ANCHOR}{block}")
    else:
        created = False
        sep = "\n" if existing.endswith("\n") else "\n\n"
        out = f"{existing}{sep}{block.strip()}\n"

    nbytes = len(out.encode("utf-8"))
    if dry_run:
        return str(path), nbytes, created

    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        mode="w",
        encoding="utf-8",
        delete=False,
        dir=path.parent,
        prefix=".akta-changelog-",
        suffix=".md",
    ) as f:
        f.write(out)
        tmp = Path(f.name)
    tmp.replace(path)
    return str(path), nbytes, created
