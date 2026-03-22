from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Annotated, Any, Literal, Optional

import typer

from akta_docs.changelog import ChangelogError, append_changelog_entry
from akta_docs.config import ConfigError, load_config, resolve_config_path
from akta_docs.engine import expand_lint_paths, lint_files
from akta_docs.models import Diagnostic
from akta_docs.scaffolder import run_scaffold
from akta_docs.templates import write_template
from akta_docs.types import KINDS

app = typer.Typer(
    name="akta-docs",
    help="OPENAKTA documentation linter and scaffolding (GEO / AI context)",
    no_args_is_help=True,
)


def _root_obj(ctx: typer.Context) -> dict[str, Any]:
    p = ctx
    while p.parent is not None:
        p = p.parent
    o = getattr(p, "obj", None)
    return o if isinstance(o, dict) else {}


def _format_default(d: Diagnostic, cwd: Path) -> str:
    try:
        rel = str(Path(d.file).relative_to(cwd))
    except ValueError:
        rel = d.file
    return f"{rel}:{d.line}:{d.column} {d.severity} {d.rule_id} {d.message}"


def _expand_lint_inputs(paths: list[Path]) -> list[Path]:
    out: list[Path] = []
    for p in paths:
        p = p.resolve()
        if p.is_dir():
            out.extend(sorted(p.rglob("*.md")))
        else:
            out.append(p)
    # de-dupe, drop node_modules
    seen: set[Path] = set()
    res: list[Path] = []
    for f in out:
        if "node_modules" in f.parts:
            continue
        if f not in seen:
            seen.add(f)
            res.append(f)
    return sorted(res)


@app.callback()
def _global(
    ctx: typer.Context,
    config: Annotated[
        Optional[Path],
        typer.Option("--config", "-c", help="Path to .akta-config.yaml"),
    ] = None,
    output_format: Annotated[
        Literal["default", "json"],
        typer.Option("--format", help="Lint output format"),
    ] = "default",
) -> None:
    ctx.ensure_object(dict)
    ctx.obj["config"] = config
    ctx.obj["format"] = output_format


@app.command("init")
def cmd_init(
    ctx: typer.Context,
    root: Annotated[Path, typer.Option("--root", help="Repository root")] = Path("."),
    force: Annotated[bool, typer.Option("--force")] = False,
    dry_run: Annotated[bool, typer.Option("--dry-run")] = False,
    project_name: Annotated[Optional[str], typer.Option("--project-name")] = None,
    skip_readme: Annotated[bool, typer.Option("--skip-readme")] = False,
    gitkeep: Annotated[bool, typer.Option("--gitkeep")] = False,
) -> None:
    r = root.resolve()
    name = project_name or r.name or "openakta-project"
    report = run_scaffold(
        str(r),
        name,
        force=force,
        dry_run=dry_run,
        create_readme_in_each_folder=not skip_readme,
        gitkeep=gitkeep,
    )
    typer.echo(f"Created docs tree under {report.docs_root}")
    typer.echo(f"Wrote {report.config_path}")


@app.command("lint")
def cmd_lint(
    ctx: typer.Context,
    paths: list[Path] = typer.Argument(default_factory=list),
    max_warnings: Annotated[
        int,
        typer.Option("--max-warnings", help="-1 = unlimited"),
    ] = -1,
    rule: Annotated[Optional[list[str]], typer.Option("--rule")] = None,
) -> None:
    cwd = Path.cwd()
    g = _root_obj(ctx)
    cfg_path = resolve_config_path(cwd, g.get("config"))
    try:
        cfg = load_config(cfg_path)
    except ConfigError as e:
        typer.echo(str(e), err=True)
        raise typer.Exit(2) from e

    if paths:
        files = _expand_lint_inputs(paths)
    else:
        files = [Path(p) for p in expand_lint_paths(cfg, str(cwd))]

    result = lint_files([str(f) for f in files], cfg, str(cwd))
    diags = result.diagnostics
    if rule:
        diags = [d for d in diags if d.rule_id in rule]

    fmt = g.get("format", "default")
    if fmt == "json":
        typer.echo(json.dumps([d.model_dump() for d in diags], indent=2))
    else:
        for d in diags:
            typer.echo(_format_default(d, cwd))

    errors = sum(1 for d in diags if d.severity == "error")
    warnings = sum(1 for d in diags if d.severity == "warn")
    cap = float("inf") if max_warnings < 0 else max_warnings
    code = 0
    if warnings > cap or errors > 0:
        code = 1
    raise typer.Exit(code)


changelog_app = typer.Typer(help="Changelog helpers")
app.add_typer(changelog_app, name="changelog")


@changelog_app.command("append")
def cmd_changelog_append(
    ctx: typer.Context,
    file: Annotated[Path, typer.Option("--file", help="Target markdown", exists=False)],
    payload: Annotated[Optional[Path], typer.Option("--payload", help="JSON file")] = None,
    dry_run: Annotated[bool, typer.Option("--dry-run")] = False,
) -> None:
    cwd = Path.cwd()
    g = _root_obj(ctx)
    cfg_path = resolve_config_path(cwd, g.get("config"))
    template = "compact"
    try:
        cfg = load_config(cfg_path)
        template = cfg.changelog.entry_template
    except ConfigError:
        pass

    if payload:
        raw = payload.read_text(encoding="utf-8")
    else:
        raw = sys.stdin.read()

    try:
        data: Any = json.loads(raw)
    except json.JSONDecodeError as e:
        typer.echo(f"Invalid JSON: {e}", err=True)
        raise typer.Exit(2) from e

    try:
        target, nbytes, created = append_changelog_entry(
            file.resolve(),
            data,
            dry_run=dry_run,
            template=template,
        )
    except ChangelogError as e:
        typer.echo(str(e), err=True)
        raise typer.Exit(2) from e

    prefix = "[dry-run] " if dry_run else ""
    typer.echo(f"{prefix}Wrote {nbytes} bytes to {target}" + (" (created)" if created else ""))


@app.command("create")
def cmd_create(
    kind: Annotated[str, typer.Argument()],
    output_path: Annotated[Path, typer.Argument()],
    title: Annotated[str, typer.Option("--title")],
    slug: Annotated[str, typer.Option("--slug")],
    doc_id: Annotated[Optional[str], typer.Option("--doc-id")] = None,
) -> None:
    if kind not in KINDS:
        typer.echo(f"Invalid kind. Expected one of: {', '.join(KINDS)}", err=True)
        raise typer.Exit(2)
    from datetime import date

    today = date.today().isoformat()
    did = doc_id or f"{re_slug(slug)}.{today}"
    cwd = Path.cwd()
    write_template(
        kind,
        cwd / output_path,
        title=title,
        slug=slug,
        doc_id=did,
        date=today,
    )
    typer.echo(f"Wrote {output_path}")


def re_slug(slug: str) -> str:
    import re as _re

    return _re.sub(r"[^a-z0-9]+", "-", slug.lower()).strip("-") or "doc"


def main() -> None:
    app()


if __name__ == "__main__":
    main()
