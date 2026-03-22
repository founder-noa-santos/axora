from __future__ import annotations

import glob
import re
from datetime import date, datetime
from pathlib import Path
from typing import Any

import frontmatter
import pathspec
from markdown_it import MarkdownIt
from markdown_it.token import Token

from akta_docs.models import AktaConfig, Diagnostic, LintResult, LintSummary
from akta_docs.rule_severity import rule_number_option, rule_severity
from akta_docs.word_count import count_words

DOC_ID_PATTERN = re.compile(r"^[a-z0-9][a-z0-9._-]*$")
ISO_DATE = re.compile(r"^\d{4}-\d{2}-\d{2}(?:T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?)?$")
QUESTION_START = re.compile(r"^(how|what|when|where|why|who|which|can|should|does|is|are)\b", re.I)

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


def has_frontmatter_block(text: str) -> bool:
    return text.lstrip().startswith("---")


def body_start_line(file_content: str, body: str) -> int:
    idx = file_content.find(body)
    if idx < 0:
        return 1
    return file_content[:idx].count("\n") + 1


def inline_text(t: Token) -> str:
    if not t.children:
        return (t.content or "").strip()
    parts: list[str] = []
    for c in t.children:
        if c.type == "text":
            parts.append(c.content or "")
        elif c.type == "code_inline":
            parts.append(c.content or "")
        elif c.type == "softbreak" or c.type == "hardbreak":
            parts.append(" ")
    return "".join(parts).strip()


def parse_body(body: str) -> list[Token]:
    md = MarkdownIt("commonmark")
    return md.parse(body, {})


def collect_heading_nodes(tokens: list[Token]) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    i = 0
    while i < len(tokens):
        t = tokens[i]
        if t.type == "heading_open" and t.map:
            level = int(t.tag[1])
            hmap = t.map
            text = ""
            j = i + 1
            while j < len(tokens) and tokens[j].type != "heading_close":
                if tokens[j].type == "inline":
                    text = inline_text(tokens[j])
                j += 1
            out.append({"depth": level, "map": hmap, "text": text})
            i = j + 1
            continue
        i += 1
    return out


def find_quick_fence(tokens: list[Token]) -> Token | None:
    i = 0
    while i < len(tokens):
        t = tokens[i]
        if t.type == "heading_open" and t.tag == "h1":
            i += 1
            while i < len(tokens) and tokens[i].type != "heading_close":
                i += 1
            i += 1
            while i < len(tokens):
                t2 = tokens[i]
                if t2.type == "heading_open":
                    return None
                if t2.type == "fence":
                    info = (t2.info or "").strip().split()
                    if info and info[0] == "quick":
                        return t2
                    return None
                if t2.type == "paragraph_open":
                    i += 1
                    while i < len(tokens) and tokens[i].type != "paragraph_close":
                        i += 1
                    if i < len(tokens):
                        i += 1
                    continue
                if t2.type == "html_block":
                    i += 1
                    continue
                return None
            return None
        i += 1
    return None


def map_severity(eff: str) -> str | None:
    if eff == "off":
        return None
    return eff


def sort_diags(diags: list[Diagnostic]) -> list[Diagnostic]:
    return sorted(
        diags,
        key=lambda d: (d.file, d.line, d.column, d.rule_id),
    )


def push_diag(
    diags: list[Diagnostic],
    file: str,
    rule_id: str,
    severity: str | None,
    line: int,
    column: int,
    message: str,
    end_line: int | None = None,
    end_column: int | None = None,
) -> None:
    if severity is None:
        return
    diags.append(
        Diagnostic(
            file=file,
            line=line,
            column=column,
            rule_id=rule_id,
            severity=severity,  # type: ignore[arg-type]
            message=message,
            end_line=end_line,
            end_column=end_column,
        )
    )


def lint_meta(
    file: str,
    file_content: str,
    data: dict[str, Any],
    config: AktaConfig,
    diags: list[Diagnostic],
) -> None:
    r1 = rule_severity(config, "META-001")
    if not has_frontmatter_block(file_content):
        push_diag(
            diags,
            file,
            "META-001",
            map_severity(r1),
            1,
            1,
            "Missing YAML frontmatter (expected leading --- block).",
        )
        return

    r2 = rule_severity(config, "META-002")
    doc_id = data.get("doc_id")
    if not isinstance(doc_id, str) or not doc_id.strip():
        push_diag(
            diags,
            file,
            "META-002",
            map_severity(r2),
            2,
            1,
            "Frontmatter must include non-empty string `doc_id`.",
        )
    elif not DOC_ID_PATTERN.match(doc_id.strip()):
        push_diag(
            diags,
            file,
            "META-002",
            map_severity(r2),
            2,
            1,
            f'doc_id must match pattern {DOC_ID_PATTERN.pattern}: got "{doc_id}".',
        )

    r3 = rule_severity(config, "META-003")
    doc_type = data.get("doc_type")
    if not isinstance(doc_type, str) or not doc_type.strip():
        push_diag(
            diags,
            file,
            "META-003",
            map_severity(r3),
            2,
            1,
            f"Frontmatter must include string `doc_type` (one of: {', '.join(DOC_TYPES)}).",
        )
    elif doc_type.strip() not in DOC_TYPES:
        push_diag(
            diags,
            file,
            "META-003",
            map_severity(r3),
            2,
            1,
            f'Invalid doc_type "{doc_type}".',
        )

    r4 = rule_severity(config, "META-004")
    date_val = data.get("date")
    date_str: str | None = None
    if isinstance(date_val, datetime):
        date_str = date_val.date().isoformat()
    elif isinstance(date_val, date):
        date_str = date_val.isoformat()
    elif isinstance(date_val, str) and date_val.strip():
        date_str = date_val.strip()
    if not date_str:
        push_diag(
            diags,
            file,
            "META-004",
            map_severity(r4),
            2,
            1,
            "Frontmatter must include ISO8601 `date` (YYYY-MM-DD or full instant).",
        )
    elif not ISO_DATE.match(date_str):
        push_diag(
            diags,
            file,
            "META-004",
            map_severity(r4),
            2,
            1,
            f'date must be ISO8601: got "{date_str}".',
        )


def is_question_heading(text: str) -> bool:
    s = text.strip()
    if s.endswith("?"):
        return True
    return bool(QUESTION_START.match(s))


def lint_meta_quick(
    file: str,
    data: dict[str, Any],
    tokens: list[Token],
    config: AktaConfig,
    diags: list[Diagnostic],
    line_offset: int,
) -> None:
    sev = rule_severity(config, "META-QUICK")
    eff = map_severity(sev)
    if eff is None:
        return
    if data.get("doc_type") == "changelog":
        return
    min_w = int(rule_number_option(config, "META-QUICK", "min_words", 40))
    max_w = int(rule_number_option(config, "META-QUICK", "max_words", 80))
    quick = find_quick_fence(tokens)
    if quick is None or not quick.map:
        push_diag(
            diags,
            file,
            "META-QUICK",
            eff,
            1 + line_offset,
            1,
            "After the first H1, expected a fenced code block with language tag `quick` (```quick).",
        )
        return
    text = (quick.content or "").strip()
    words = count_words(text)
    line = quick.map[0] + 1 + line_offset
    col = 1
    if words < min_w or words > max_w:
        end_ln = quick.map[1] + line_offset
        push_diag(
            diags,
            file,
            "META-QUICK",
            eff,
            line,
            col,
            f"Quick Answer block must be {min_w}–{max_w} words; found {words}.",
            end_line=end_ln,
            end_column=None,
        )


def lint_struct008(
    file: str,
    body: str,
    headings: list[dict[str, Any]],
    data: dict[str, Any],
    config: AktaConfig,
    diags: list[Diagnostic],
    line_offset: int,
) -> None:
    sev = rule_severity(config, "STRUCT-008")
    eff = map_severity(sev)
    if eff is None:
        return
    if data.get("doc_type") == "changelog":
        return
    min_w = int(rule_number_option(config, "STRUCT-008", "min_words", 150))
    max_w = int(rule_number_option(config, "STRUCT-008", "max_words", 300))
    hs = [h for h in headings if h["depth"] in (2, 3)]
    hs.sort(key=lambda h: h["map"][0])
    body_lines = body.split("\n")
    for h in hs:
        hmap = h["map"]
        # Match TS: contentStartLine = heading.position.end.line + 1 (1-based)
        content_start_line = hmap[1] + 1
        depth = h["depth"]
        nxt = next(
            (x for x in hs if x is not h and x["map"][0] > hmap[1] and x["depth"] <= depth),
            None,
        )
        # Exclusive 0-based slice end (same as TS bodyLines.slice(a, contentEndLine))
        content_end_exclusive = nxt["map"][0] if nxt else len(body_lines)
        if content_start_line > content_end_exclusive:
            push_diag(
                diags,
                file,
                "STRUCT-008",
                eff,
                hmap[0] + 1 + line_offset,
                1,
                "Section has no body; cannot satisfy STRUCT-008 word range.",
                end_line=hmap[1] + line_offset,
            )
            continue
        slice_ = "\n".join(body_lines[content_start_line - 1 : content_end_exclusive])
        wc = count_words(slice_)
        line = hmap[0] + 1 + line_offset
        if wc < min_w or wc > max_w:
            push_diag(
                diags,
                file,
                "STRUCT-008",
                eff,
                line,
                1,
                f"H{depth} section must be {min_w}–{max_w} words; found {wc}.",
                end_line=hmap[1] + line_offset,
            )


def lint_content001(
    file: str,
    headings: list[dict[str, Any]],
    config: AktaConfig,
    diags: list[Diagnostic],
    line_offset: int,
) -> None:
    sev = rule_severity(config, "CONTENT-001")
    eff = map_severity(sev)
    if eff is None:
        return
    ratio = rule_number_option(config, "CONTENT-001", "min_question_ratio", 0.7)
    h23 = [h for h in headings if h["depth"] in (2, 3)]
    if not h23:
        return
    questions = sum(1 for h in h23 if is_question_heading(h["text"]))
    actual = questions / len(h23)
    if actual + 1e-9 < ratio:
        push_diag(
            diags,
            file,
            "CONTENT-001",
            eff,
            1 + line_offset,
            1,
            f"At least {ratio * 100:.0f}% of H2/H3 headings should be questions; "
            f"got {actual * 100:.1f}% ({questions}/{len(h23)}).",
        )


def lint_files(paths: list[str], config: AktaConfig, cwd: str) -> LintResult:
    diags: list[Diagnostic] = []
    cwd_p = Path(cwd).resolve()
    for p in paths:
        abs_p = Path(p).resolve()
        try:
            rel = str(abs_p.relative_to(cwd_p))
        except ValueError:
            rel = abs_p.name
        try:
            raw = abs_p.read_text(encoding="utf-8")
        except OSError:
            continue
        post = frontmatter.loads(raw)
        data = dict(post.metadata) if post.metadata else {}
        line_offset = body_start_line(raw, post.content) - 1
        lint_meta(rel, raw, data, config, diags)
        if not has_frontmatter_block(raw):
            continue
        body = post.content
        try:
            tokens = parse_body(body)
        except Exception:
            push_diag(
                diags,
                rel,
                "META-001",
                map_severity(rule_severity(config, "META-001")),
                1,
                1,
                "Markdown body failed to parse.",
            )
            continue
        headings = collect_heading_nodes(tokens)
        lint_meta_quick(rel, data, tokens, config, diags, line_offset)
        lint_struct008(rel, body, headings, data, config, diags, line_offset)
        lint_content001(rel, headings, config, diags, line_offset)

    diags = sort_diags(diags)
    err = sum(1 for d in diags if d.severity == "error")
    warn = sum(1 for d in diags if d.severity == "warn")
    return LintResult(
        diagnostics=diags,
        summary=LintSummary(error_count=err, warn_count=warn),
    )


def expand_lint_paths(config: AktaConfig, cwd: str) -> list[str]:
    root = Path(cwd).resolve() / config.paths.docs_root
    root = root.resolve()
    out: list[str] = []
    for g in config.paths.include_globs:
        pattern = str(root / g)
        out.extend(glob.glob(pattern, recursive=True))
    out = sorted(set(out))
    spec = pathspec.PathSpec.from_lines("gitwildmatch", config.paths.exclude_globs)
    filtered: list[str] = []
    for f in out:
        rel = str(Path(f).relative_to(root))
        if spec.match_file(rel):
            continue
        filtered.append(f)
    return sorted(filtered)
