import fs from "node:fs/promises";
import path from "node:path";
import matter from "gray-matter";
import { unified } from "unified";
import remarkParse from "remark-parse";
import type { Code, Heading, Root } from "mdast";
import { visit } from "unist-util-visit";
import { toString } from "mdast-util-to-string";
import fg from "fast-glob";
import type { AktaConfig } from "../config/schema.js";
import { DOC_TYPES } from "../config/schema.js";
import type { Diagnostic, LintResult, RuleId, Severity } from "../types.js";
import { countWords } from "./wordCount.js";
import { ruleNumberOption, ruleSeverity } from "./ruleSeverity.js";

const DOC_ID_PATTERN = /^[a-z0-9][a-z0-9._-]*$/;
const ISO_DATE =
  /^\d{4}-\d{2}-\d{2}(?:T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?)?$/;

const QUESTION_START =
  /^(how|what|when|where|why|who|which|can|should|does|is|are)\b/i;

function isQuestionHeading(text: string): boolean {
  const s = text.trim();
  if (s.endsWith("?")) return true;
  return QUESTION_START.test(s);
}

function mapSeverity(eff: "error" | "warn" | "off"): Severity | null {
  if (eff === "off") return null;
  return eff;
}

function sortDiagnostics(a: Diagnostic, b: Diagnostic): number {
  if (a.file !== b.file) return a.file.localeCompare(b.file);
  if (a.line !== b.line) return a.line - b.line;
  if (a.column !== b.column) return a.column - b.column;
  return a.rule_id.localeCompare(b.rule_id);
}

function bodyStartLine(fileContent: string, body: string): number {
  const idx = fileContent.indexOf(body);
  if (idx < 0) return 1;
  return fileContent.slice(0, idx).split(/\r?\n/).length;
}

function parseMarkdownBody(body: string): Root {
  const processor = unified().use(remarkParse);
  return processor.parse(body) as Root;
}

function pushDiag(
  list: Diagnostic[],
  file: string,
  ruleId: RuleId,
  severity: Severity | null,
  line: number,
  column: number,
  message: string,
  endLine?: number,
  endColumn?: number,
) {
  if (!severity) return;
  list.push({
    file,
    line,
    column,
    end_line: endLine,
    end_column: endColumn,
    rule_id: ruleId,
    severity,
    message,
  });
}

function lintMeta(
  file: string,
  fileContent: string,
  fm: matter.GrayMatterFile<string>,
  config: AktaConfig,
  diags: Diagnostic[],
  _lineOffset: number,
) {
  const r1 = ruleSeverity(config, "META-001");
  if (!matter.test(fileContent)) {
    pushDiag(
      diags,
      file,
      "META-001",
      mapSeverity(r1),
      1,
      1,
      "Missing YAML frontmatter (expected leading --- block).",
    );
    return;
  }

  const data = fm.data as Record<string, unknown>;

  const r2 = ruleSeverity(config, "META-002");
  const docId = data.doc_id;
  if (typeof docId !== "string" || !docId.trim()) {
    pushDiag(
      diags,
      file,
      "META-002",
      mapSeverity(r2),
      2,
      1,
      "Frontmatter must include non-empty string `doc_id`.",
    );
  } else if (!DOC_ID_PATTERN.test(docId.trim())) {
    pushDiag(
      diags,
      file,
      "META-002",
      mapSeverity(r2),
      2,
      1,
      `doc_id must match pattern ${DOC_ID_PATTERN}: got "${docId}".`,
    );
  }

  const r3 = ruleSeverity(config, "META-003");
  const docType = data.doc_type;
  if (typeof docType !== "string" || !docType.trim()) {
    pushDiag(
      diags,
      file,
      "META-003",
      mapSeverity(r3),
      2,
      1,
      `Frontmatter must include string \`doc_type\` (one of: ${DOC_TYPES.join(", ")}).`,
    );
  } else if (!DOC_TYPES.includes(docType as (typeof DOC_TYPES)[number])) {
    pushDiag(
      diags,
      file,
      "META-003",
      mapSeverity(r3),
      2,
      1,
      `Invalid doc_type "${docType}".`,
    );
  }

  const r4 = ruleSeverity(config, "META-004");
  const dateVal = data.date;
  let dateStr: string | undefined;
  if (dateVal instanceof Date && !Number.isNaN(dateVal.getTime())) {
    dateStr = dateVal.toISOString().slice(0, 10);
  } else if (typeof dateVal === "string" && dateVal.trim()) {
    dateStr = dateVal.trim();
  }
  if (!dateStr) {
    pushDiag(
      diags,
      file,
      "META-004",
      mapSeverity(r4),
      2,
      1,
      "Frontmatter must include ISO8601 `date` (YYYY-MM-DD or full instant).",
    );
  } else if (!ISO_DATE.test(dateStr)) {
    pushDiag(
      diags,
      file,
      "META-004",
      mapSeverity(r4),
      2,
      1,
      `date must be ISO8601: got "${dateStr}".`,
    );
  }
}

function findFirstH1Index(children: Root["children"]): number {
  return children.findIndex(
    (n) => n.type === "heading" && (n as Heading).depth === 1,
  );
}

function isIgnorableBeforeQuick(n: Root["children"][number]): boolean {
  if (n.type === "paragraph") {
    return toString(n).trim() === "";
  }
  if (n.type === "html" && typeof (n as { value?: string }).value === "string") {
    return (n as { value: string }).value.trim() === "";
  }
  return false;
}

function findQuickCodeAfterH1(tree: Root): Code | undefined {
  const { children } = tree;
  const h1 = findFirstH1Index(children);
  if (h1 < 0) return undefined;
  for (let i = h1 + 1; i < children.length; i++) {
    const n = children[i];
    if (n.type === "heading") return undefined;
    if (n.type === "code" && (n as Code).lang === "quick") {
      return n as Code;
    }
    if (isIgnorableBeforeQuick(n)) continue;
    return undefined;
  }
  return undefined;
}

function collectHeadings(tree: Root): Heading[] {
  const out: Heading[] = [];
  visit(tree, "heading", (node: Heading) => {
    out.push(node);
  });
  return out;
}

function lintMetaQuick(
  file: string,
  fm: matter.GrayMatterFile<string>,
  tree: Root,
  config: AktaConfig,
  diags: Diagnostic[],
  lineOffset: number,
) {
  const sev = ruleSeverity(config, "META-QUICK");
  const eff = mapSeverity(sev);
  if (!eff) return;

  const data = fm.data as { doc_type?: string };
  if (data.doc_type === "changelog") return;

  const minW = Math.floor(
    ruleNumberOption(config, "META-QUICK", "min_words", 40),
  );
  const maxW = Math.floor(
    ruleNumberOption(config, "META-QUICK", "max_words", 80),
  );

  const quick = findQuickCodeAfterH1(tree);
  if (!quick || !quick.position) {
    pushDiag(
      diags,
      file,
      "META-QUICK",
      eff,
      1 + lineOffset,
      1,
      "After the first H1, expected a fenced code block with language tag `quick` (```quick).",
    );
    return;
  }

  const text = String(quick.value ?? "");
  const words = countWords(text);
  const line = quick.position.start.line + lineOffset;
  const col = quick.position.start.column ?? 1;
  if (words < minW || words > maxW) {
    pushDiag(
      diags,
      file,
      "META-QUICK",
      eff,
      line,
      col,
      `Quick Answer block must be ${minW}–${maxW} words; found ${words}.`,
      quick.position.end.line + lineOffset,
      quick.position.end.column,
    );
  }
}

function lintStruct008(
  file: string,
  fm: matter.GrayMatterFile<string>,
  tree: Root,
  config: AktaConfig,
  diags: Diagnostic[],
  lineOffset: number,
) {
  const sev = ruleSeverity(config, "STRUCT-008");
  const eff = mapSeverity(sev);
  if (!eff) return;

  const data = fm.data as { doc_type?: string };
  if (data.doc_type === "changelog") return;

  const minW = Math.floor(
    ruleNumberOption(config, "STRUCT-008", "min_words", 150),
  );
  const maxW = Math.floor(
    ruleNumberOption(config, "STRUCT-008", "max_words", 300),
  );

  const headings = collectHeadings(tree)
    .filter((h) => h.depth === 2 || h.depth === 3)
    .sort(
      (a, b) =>
        (a.position?.start.line ?? 0) - (b.position?.start.line ?? 0),
    );
  const bodyLines = fm.content.split(/\r?\n/);

  for (const h of headings) {
    if (!h.position) continue;
    const contentStartLine = h.position.end.line + 1;
    const next = headings.find(
      (x) =>
        x !== h &&
        x.position &&
        h.position &&
        x.position.start.line > h.position.end.line &&
        x.depth <= h.depth,
    );
    const contentEndLine = next
      ? next.position!.start.line - 1
      : bodyLines.length;

    if (contentStartLine > contentEndLine) {
      pushDiag(
        diags,
        file,
        "STRUCT-008",
        eff,
        h.position.start.line + lineOffset,
        h.position.start.column ?? 1,
        "Section has no body; cannot satisfy STRUCT-008 word range.",
        h.position.end.line + lineOffset,
        h.position.end.column,
      );
      continue;
    }

    const slice = bodyLines
      .slice(contentStartLine - 1, contentEndLine)
      .join("\n");
    const words = countWords(slice);
    const line = h.position.start.line + lineOffset;
    const col = h.position.start.column ?? 1;
    if (words < minW || words > maxW) {
      pushDiag(
        diags,
        file,
        "STRUCT-008",
        eff,
        line,
        col,
        `H${h.depth} section must be ${minW}–${maxW} words; found ${words}.`,
        h.position.end.line + lineOffset,
        h.position.end.column,
      );
    }
  }
}

function lintContent001(
  file: string,
  tree: Root,
  config: AktaConfig,
  diags: Diagnostic[],
  lineOffset: number,
) {
  const sev = ruleSeverity(config, "CONTENT-001");
  const eff = mapSeverity(sev);
  if (!eff) return;

  const ratio = ruleNumberOption(config, "CONTENT-001", "min_question_ratio", 0.7);

  const headings = collectHeadings(tree).filter(
    (h) => h.depth === 2 || h.depth === 3,
  );
  if (headings.length === 0) return;

  let questions = 0;
  for (const h of headings) {
    const t = toString(h);
    if (isQuestionHeading(t)) questions += 1;
  }

  const actual = questions / headings.length;
  if (actual + 1e-9 < ratio) {
    pushDiag(
      diags,
      file,
      "CONTENT-001",
      eff,
      1 + lineOffset,
      1,
      `At least ${(ratio * 100).toFixed(0)}% of H2/H3 headings should be questions; got ${(actual * 100).toFixed(1)}% (${questions}/${headings.length}).`,
    );
  }
}

export async function lintFiles(
  paths: string[],
  config: AktaConfig,
  cwd: string,
): Promise<LintResult> {
  const diags: Diagnostic[] = [];

  for (const p of paths) {
    const abs = path.resolve(p);
    const rel = path.relative(cwd, abs) || path.basename(abs);
    let raw: string;
    try {
      raw = await fs.readFile(abs, "utf8");
    } catch {
      continue;
    }

    const fm = matter(raw);
    const lineOffset = bodyStartLine(raw, fm.content) - 1;
    lintMeta(rel, raw, fm, config, diags, 0);
    if (!matter.test(raw)) {
      continue;
    }

    let tree: Root;
    try {
      tree = parseMarkdownBody(fm.content);
    } catch {
      pushDiag(
        diags,
        rel,
        "META-001",
        mapSeverity(ruleSeverity(config, "META-001")),
        1,
        1,
        "Markdown body failed to parse.",
      );
      continue;
    }

    lintMetaQuick(rel, fm, tree, config, diags, lineOffset);
    lintStruct008(rel, fm, tree, config, diags, lineOffset);
    lintContent001(rel, tree, config, diags, lineOffset);
  }

  diags.sort(sortDiagnostics);
  const error_count = diags.filter((d) => d.severity === "error").length;
  const warn_count = diags.filter((d) => d.severity === "warn").length;
  return { diagnostics: diags, summary: { error_count, warn_count } };
}

export async function expandLintPaths(
  config: AktaConfig,
  cwd: string,
): Promise<string[]> {
  const root = path.resolve(cwd, config.paths.docs_root);
  const entries = await fg(config.paths.include_globs, {
    cwd: root,
    absolute: true,
    onlyFiles: true,
    ignore: config.paths.exclude_globs,
  });
  return entries.sort();
}
