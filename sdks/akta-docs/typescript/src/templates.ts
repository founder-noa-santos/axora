import fs from "node:fs/promises";
import path from "node:path";
import type { TemplateKind } from "./types.js";

export interface TemplateVars {
  title: string;
  slug: string;
  doc_id: string;
  date: string;
}

/** Deterministic filler: exactly `count` words. */
function words(count: number): string {
  return Array.from({ length: count }, (_, i) => `w${i + 1}`).join(" ");
}

const QUICK_ADR =
  "This ADR records one architecture decision including context, the decision itself, and consequences for teams. " +
  "It explains why we chose this path and which trade-offs we accept for maintenance, operations, and future migrations. " +
  "Readers should leave with a clear yes or no on scope.";

const QUICK_BR =
  "This document defines one business rule covering actors, scope, and enforcement. " +
  "It states the invariant in plain language and points to validation or audit hooks. " +
  "The goal is to align product, legal, and engineering without ambiguous edge cases.";

const QUICK_GENERIC = (slug: string) =>
  `This page documents ${slug} for the repository. ` +
  `It orients readers before deeper sections and keeps token use predictable for retrieval. ` +
  `Skim the quick answer first, then jump to the question headings that match your task.`;

function sectionBlock(): string {
  return words(200);
}

const ADR = (v: TemplateVars) => `---
doc_id: ${v.doc_id}
doc_type: adr
date: ${v.date}
---

# ${v.title}

\`\`\`quick
${QUICK_ADR}
\`\`\`

## Why does this decision matter for the product?

${sectionBlock()}

## What constraints shaped the available options?

${sectionBlock()}

## Which option did we select and why?

${sectionBlock()}
`;

const BUSINESS_RULE = (v: TemplateVars) => `---
doc_id: ${v.doc_id}
doc_type: business_rule
date: ${v.date}
---

# ${v.title}

\`\`\`quick
${QUICK_BR}
\`\`\`

## Who must follow this rule and when?

${sectionBlock()}

## What is the exact rule or invariant?

${sectionBlock()}

## How do we validate or audit compliance?

${sectionBlock()}
`;

const GENERIC = (v: TemplateVars, kind: TemplateKind) => `---
doc_id: ${v.doc_id}
doc_type: ${kind}
date: ${v.date}
---

# ${v.title}

\`\`\`quick
${QUICK_GENERIC(v.slug)}
\`\`\`

## What problem does this page solve?

${sectionBlock()}

## What are the key facts or steps?

${sectionBlock()}

## Where should readers go next?

${sectionBlock()}
`;

export async function writeTemplate(
  kind: TemplateKind,
  outputPath: string,
  vars: TemplateVars,
): Promise<void> {
  let body: string;
  if (kind === "adr") body = ADR(vars);
  else if (kind === "business_rule") body = BUSINESS_RULE(vars);
  else body = GENERIC(vars, kind);

  await fs.mkdir(path.dirname(outputPath), { recursive: true });
  await fs.writeFile(outputPath, body, "utf8");
}
