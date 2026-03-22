import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { lintFiles } from "../src/linter/engine.js";
import { countWords } from "../src/linter/wordCount.js";
import type { AktaConfig } from "../src/config/schema.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

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

describe("countWords", () => {
  it("counts whitespace-separated tokens", () => {
    expect(countWords("a b c")).toBe(3);
    expect(countWords("")).toBe(0);
  });
});

describe("lintFiles", () => {
  it("reports META-001 when frontmatter is missing", async () => {
    const file = path.join(__dirname, "fixtures", "no-frontmatter.md");
    const res = await lintFiles([file], strictConfig(), __dirname);
    const ids = res.diagnostics.map((d) => d.rule_id);
    expect(ids).toContain("META-001");
  });

  it("passes compliant fixture with all rules", async () => {
    const file = path.join(__dirname, "fixtures", "compliant.md");
    const res = await lintFiles([file], strictConfig(), __dirname);
    const errors = res.diagnostics.filter((d) => d.severity === "error");
    expect(errors).toEqual([]);
  });

  it("flags STRUCT-008 when a section is too short", async () => {
    const file = path.join(__dirname, "fixtures", "short-section.md");
    const res = await lintFiles([file], strictConfig(), __dirname);
    expect(res.diagnostics.some((d) => d.rule_id === "STRUCT-008")).toBe(true);
  });
});
