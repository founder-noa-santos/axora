from __future__ import annotations

from pathlib import Path

from akta_docs.types import KINDS


def words(count: int) -> str:
    return " ".join(f"w{i + 1}" for i in range(count))


QUICK_ADR = (
    "This ADR records one architecture decision including context, the decision itself, "
    "and consequences for teams. It explains why we chose this path and which trade-offs "
    "we accept for maintenance, operations, and future migrations. Readers should leave "
    "with a clear yes or no on scope."
)

QUICK_BR = (
    "This document defines one business rule covering actors, scope, and enforcement. "
    "It states the invariant in plain language and points to validation or audit hooks. "
    "The goal is to align product, legal, and engineering without ambiguous edge cases."
)


def quick_generic(slug: str) -> str:
    return (
        f"This page documents {slug} for the repository. "
        "It orients readers before deeper sections and keeps token use predictable for retrieval. "
        "Skim the quick answer first, then jump to the question headings that match your task."
    )


def section_block() -> str:
    return words(200)


def write_template(
    kind: str,
    output_path: str | Path,
    *,
    title: str,
    slug: str,
    doc_id: str,
    date: str,
) -> None:
    if kind not in KINDS:
        raise ValueError(f"Invalid kind {kind!r}; expected one of {KINDS}")
    out = Path(output_path)
    out.parent.mkdir(parents=True, exist_ok=True)
    if kind == "adr":
        body = f"""---
doc_id: {doc_id}
doc_type: adr
date: {date}
---

# {title}

```quick
{QUICK_ADR}
```

## Why does this decision matter for the product?

{section_block()}

## What constraints shaped the available options?

{section_block()}

## Which option did we select and why?

{section_block()}
"""
    elif kind == "business_rule":
        body = f"""---
doc_id: {doc_id}
doc_type: business_rule
date: {date}
---

# {title}

```quick
{QUICK_BR}
```

## Who must follow this rule and when?

{section_block()}

## What is the exact rule or invariant?

{section_block()}

## How do we validate or audit compliance?

{section_block()}
"""
    else:
        body = f"""---
doc_id: {doc_id}
doc_type: {kind}
date: {date}
---

# {title}

```quick
{quick_generic(slug)}
```

## What problem does this page solve?

{section_block()}

## What are the key facts or steps?

{section_block()}

## Where should readers go next?

{section_block()}
"""
    out.write_text(body, encoding="utf-8")
