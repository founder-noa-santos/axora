import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { writeTemplate } from "../src/templates.js";
import { lintFiles } from "../src/linter/engine.js";
import type { AktaConfig } from "../src/config/schema.js";

function strictConfig(): AktaConfig {
  return {
    schema_version: "1.0.0",
    project: { name: "Test", slug: "test" },
    paths: {
      docs_root: "./akta-docs",
      include_globs: ["**/*.md"],
      exclude_globs: [],
    },
    linter: { default_severity: "error", rules: {} },
    scaffold: { create_readme_in_each_folder: true, gitkeep: false },
    changelog: {
      entry_template: "compact",
      summary_max_length: 200,
    },
  };
}

describe("writeTemplate", () => {
  it("generates ADR markdown that passes strict lint", async () => {
    const dir = await fs.mkdtemp(path.join(os.tmpdir(), "akta-docs-"));
    const out = path.join(dir, "adr.md");
    await writeTemplate("adr", out, {
      title: "Use Postgres",
      slug: "postgres",
      doc_id: "test.postgres.adr",
      date: "2025-03-21",
    });
    const res = await lintFiles([out], strictConfig(), dir);
    const errors = res.diagnostics.filter((d) => d.severity === "error");
    expect(errors).toEqual([]);
  });
});
