import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { ChangelogEntrySchema } from "./config/schema.js";
import type { AppendReport, ChangelogEntry } from "./types.js";

const ANCHOR = "<!-- akta-changelog-append -->";

export class ChangelogError extends Error {
  constructor(
    message: string,
    public readonly cause?: unknown,
  ) {
    super(message);
    this.name = "ChangelogError";
  }
}

function formatEntry(entry: ChangelogEntry, template: "compact" | "detailed"): string {
  const ts = entry.timestamp;
  const line = `- **${entry.change_type}** (${ts}) ${entry.summary}`;
  if (template === "detailed" && entry.details) {
    return `${line}\n  ${entry.details.split("\n").join("\n  ")}`;
  }
  return line;
}

export async function appendChangelogEntry(
  targetPath: string,
  rawPayload: unknown,
  options: { dryRun?: boolean; template?: "compact" | "detailed" } = {},
): Promise<AppendReport> {
  const parsed = ChangelogEntrySchema.safeParse(rawPayload);
  if (!parsed.success) {
    throw new ChangelogError(
      `Invalid changelog payload: ${parsed.error.flatten().formErrors.join("; ")}`,
      parsed.error,
    );
  }
  const entry = parsed.data;
  const template = options.template ?? "compact";
  const block = `\n${formatEntry(entry, template)}\n`;

  let existing: string | undefined;
  try {
    existing = await fs.readFile(targetPath, "utf8");
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code !== "ENOENT") throw e;
  }

  let out: string;
  let created = false;
  if (existing === undefined) {
    created = true;
    out = `---
doc_id: ${entry.doc_id}
doc_type: changelog
date: ${entry.timestamp.slice(0, 10)}
---

# Changelog

${ANCHOR}
${block}`;
  } else if (existing.includes(ANCHOR)) {
    out = existing.replace(ANCHOR, `${ANCHOR}${block}`);
  } else {
    const sep = existing.endsWith("\n") ? "\n" : "\n\n";
    out = `${existing}${sep}${block.trimStart()}\n`;
  }

  if (options.dryRun) {
    return {
      target: targetPath,
      bytes_written: Buffer.byteLength(out, "utf8"),
      created,
    };
  }

  const dir = path.dirname(targetPath);
  await fs.mkdir(dir, { recursive: true });
  const tmp = path.join(
    os.tmpdir(),
    `akta-changelog-${Date.now()}-${Math.random().toString(36).slice(2)}.md`,
  );
  await fs.writeFile(tmp, out, "utf8");
  await fs.rename(tmp, targetPath);

  return {
    target: targetPath,
    bytes_written: Buffer.byteLength(out, "utf8"),
    created,
  };
}
