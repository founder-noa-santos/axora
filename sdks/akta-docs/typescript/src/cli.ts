#!/usr/bin/env node
import fs from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { Command } from "commander";
import fg from "fast-glob";

function getRootOptions(cmd: Command): { config?: string; format?: string } {
  let c: Command | null = cmd;
  while (c?.parent) {
    c = c.parent;
  }
  return (c ?? cmd).opts() as { config?: string; format?: string };
}
import { loadConfig, resolveConfigPath, ConfigError } from "./config/load.js";
import { expandLintPaths, lintFiles } from "./linter/engine.js";
import type { Diagnostic } from "./types.js";
import { appendChangelogEntry, ChangelogError } from "./changelog.js";
import { runScaffold } from "./scaffolder.js";
import { writeTemplate } from "./templates.js";
import type { TemplateKind } from "./types.js";

function formatDefault(d: Diagnostic, cwd: string): string {
  const rel = path.relative(cwd, d.file);
  const loc = `${d.line}:${d.column}`;
  const sev = d.severity;
  return `${rel}:${loc} ${sev} ${d.rule_id} ${d.message}`;
}

function formatJson(diagnostics: Diagnostic[]): string {
  return JSON.stringify(diagnostics, null, 2);
}

async function expandLintInputs(paths: string[]): Promise<string[]> {
  const out: string[] = [];
  for (const p of paths) {
    const abs = path.resolve(p);
    const st = await fs.stat(abs);
    if (st.isDirectory()) {
      const sub = await fg("**/*.md", {
        cwd: abs,
        absolute: true,
        onlyFiles: true,
        ignore: ["**/node_modules/**"],
      });
      out.push(...sub);
    } else {
      out.push(abs);
    }
  }
  return [...new Set(out)].sort();
}

const KINDS: TemplateKind[] = [
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
];

async function main() {
  const program = new Command();
  program
    .name("akta-docs")
    .description("OPENAKTA documentation linter and scaffolding (GEO / AI context)")
    .option("-c, --config <path>", "Path to .akta-config.yaml")
    .option("--format <fmt>", "Output format: default|json", "default")
    .option("--no-color", "Disable ANSI colors (reserved)");

  program
    .command("init")
    .description("Scaffold akta-docs layout and .akta-config.yaml")
    .option("--root <path>", "Repository root", ".")
    .option("--force", "Overwrite existing config", false)
    .option("--dry-run", "Print actions without writing", false)
    .option("--project-name <name>", "Project display name")
    .option("--skip-readme", "Do not write README.md in each folder", false)
    .option("--gitkeep", "Write .gitkeep in each folder", false)
    .action(async (opts) => {
      const root = path.resolve(opts.root);
      const name =
        opts.projectName ?? path.basename(root) ?? "openakta-project";
      const report = await runScaffold({
        root,
        projectName: name,
        force: opts.force,
        dryRun: opts.dryRun,
        createReadmeInEachFolder: !opts.skipReadme,
        gitkeep: opts.gitkeep,
      });
      console.log(`Created docs tree under ${report.docs_root}`);
      console.log(`Wrote ${report.config_path}`);
    });

  program
    .command("lint")
    .description("Lint markdown files")
    .argument("[paths...]", "Files or directories (default: docs_root from config)")
    .option(
      "--max-warnings <n>",
      "Fail if warning count exceeds this (-1 = unlimited)",
      "-1",
    )
    .option("--rule <id>", "Only run these rules (repeatable)", (v, p: string[]) => {
      p.push(v);
      return p;
    }, [] as string[])
    .action(async (paths: string[], opts, command) => {
      const cwd = process.cwd();
      const globals = getRootOptions(command);
      const configPath = resolveConfigPath(cwd, globals.config);
      const config = await loadConfig(configPath);
      const maxWarningsRaw = Number.parseInt(String(opts.maxWarnings), 10);
      const maxWarnings = Number.isFinite(maxWarningsRaw)
        ? maxWarningsRaw
        : -1;
      const ruleFilter: string[] | undefined =
        opts.rule?.length > 0 ? opts.rule : undefined;

      let files: string[];
      if (paths.length > 0) {
        files = await expandLintInputs(
          paths.map((p) => path.resolve(cwd, p)),
        );
      } else {
        files = await expandLintPaths(config, cwd);
      }

      const result = await lintFiles(files, config, cwd);
      let { diagnostics } = result;
      if (ruleFilter) {
        diagnostics = diagnostics.filter((d) => ruleFilter.includes(d.rule_id));
      }

      const fmt = globals.format === "json" ? "json" : "default";
      if (fmt === "json") {
        console.log(formatJson(diagnostics));
      } else {
        for (const d of diagnostics) {
          console.log(formatDefault(d, cwd));
        }
      }

      const errors = diagnostics.filter((d) => d.severity === "error").length;
      const warnings = diagnostics.filter((d) => d.severity === "warn").length;
      const warnCap = maxWarnings < 0 ? Number.POSITIVE_INFINITY : maxWarnings;
      if (warnings > warnCap) {
        process.exitCode = 1;
      }
      if (errors > 0) {
        process.exitCode = 1;
      }
    });

  const ch = program
    .command("changelog")
    .description("Changelog helpers");

  ch.command("append")
    .description("Append a changelog entry (JSON payload)")
    .requiredOption("--file <path>", "Target markdown file")
    .option("--payload <path>", "JSON file (default: stdin)")
    .option("--dry-run", "Print size only", false)
    .action(async (opts, command) => {
      const cwd = process.cwd();
      const globals = getRootOptions(command);
      const target = path.resolve(cwd, opts.file);
      let raw: string;
      if (opts.payload) {
        raw = await fs.readFile(path.resolve(cwd, opts.payload), "utf8");
      } else {
        raw = await readStdin();
      }
      const data = JSON.parse(raw) as unknown;
      const configPath = resolveConfigPath(cwd, globals.config);
      let template: "compact" | "detailed" = "compact";
      try {
        const cfg = await loadConfig(configPath);
        template = cfg.changelog.entry_template;
      } catch {
        /* use default */
      }
      const report = await appendChangelogEntry(target, data, {
        dryRun: opts.dryRun,
        template,
      });
      console.log(
        `${opts.dryRun ? "[dry-run] " : ""}Wrote ${report.bytes_written} bytes to ${report.target}${report.created ? " (created)" : ""}`,
      );
    });

  program
    .command("create")
    .description("Create a linter-friendly markdown template")
    .argument("<kind>", `Template kind: ${KINDS.join(", ")}`)
    .argument("<output_path>", "Output .md path")
    .requiredOption("--title <t>", "Document title")
    .requiredOption("--slug <s>", "Short slug for copy")
    .option("--doc-id <id>", "Explicit doc_id (default: slug + date)")
    .action(async (kind: string, outputPath: string, opts) => {
      if (!KINDS.includes(kind as TemplateKind)) {
        console.error(`Invalid kind. Expected one of: ${KINDS.join(", ")}`);
        process.exitCode = 2;
        return;
      }
      const cwd = process.cwd();
      const date = new Date().toISOString().slice(0, 10);
      const docId =
        opts.docId ?? `${opts.slug.replace(/[^a-z0-9]+/g, "-")}.${date}`;
      await writeTemplate(kind as TemplateKind, path.resolve(cwd, outputPath), {
        title: opts.title,
        slug: opts.slug,
        doc_id: docId,
        date,
      });
      console.log(`Wrote ${outputPath}`);
    });

  await program.parseAsync(process.argv);
}

function readStdin(): Promise<string> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = [];
    process.stdin.on("data", (c) => chunks.push(c as Buffer));
    process.stdin.on("end", () => resolve(Buffer.concat(chunks).toString("utf8")));
    process.stdin.on("error", reject);
  });
}

main().catch((e) => {
  if (e instanceof ConfigError || e instanceof ChangelogError) {
    console.error(e.message);
    process.exitCode = 2;
    return;
  }
  console.error(e);
  process.exitCode = 2;
});
