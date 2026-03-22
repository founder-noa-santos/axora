import fs from "node:fs/promises";
import path from "node:path";
import YAML from "yaml";
import type { AktaConfig } from "./config/schema.js";
import type { ScaffoldReport } from "./types.js";

const SECTION_DIRS = [
  "00-meta",
  "01-adrs",
  "02-business-core",
  "03-business-logic",
  "04-research",
  "05-features",
  "06-technical",
  "07-guides",
  "08-references",
  "09-explanations",
  "10-changelog",
  "99-archive",
] as const;

function slugify(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "") || "project";
}

export function defaultAktaConfig(projectName: string): AktaConfig {
  const slug = slugify(projectName);
  return {
    schema_version: "1.0.0",
    project: { name: projectName, slug },
    paths: {
      docs_root: "./akta-docs",
      include_globs: ["**/*.md"],
      exclude_globs: ["**/node_modules/**", "**/.git/**", "**/99-archive/**"],
    },
    linter: {
      default_severity: "error",
      rules: {
        "META-QUICK": { severity: "off" },
        "STRUCT-008": { severity: "off" },
        "CONTENT-001": { severity: "off" },
      },
    },
    scaffold: { create_readme_in_each_folder: true, gitkeep: false },
    changelog: {
      default_target: "akta-docs/10-changelog/CHANGELOG.md",
      entry_template: "compact",
      summary_max_length: 200,
    },
  };
}

export interface ScaffoldOptions {
  root: string;
  projectName: string;
  force: boolean;
  dryRun: boolean;
  createReadmeInEachFolder: boolean;
  gitkeep: boolean;
}

export async function runScaffold(
  options: ScaffoldOptions,
): Promise<ScaffoldReport> {
  const root = path.resolve(options.root);
  const configPath = path.join(root, ".akta-config.yaml");
  const cfg = defaultAktaConfig(options.projectName);
  cfg.scaffold = {
    create_readme_in_each_folder: options.createReadmeInEachFolder,
    gitkeep: options.gitkeep,
  };

  const docsRoot = path.join(root, "akta-docs");
  const created: string[] = [];

  if (!options.dryRun) {
    try {
      await fs.access(configPath);
      if (!options.force) {
        throw new Error(
          `${configPath} already exists; use --force to overwrite.`,
        );
      }
    } catch (e) {
      if ((e as NodeJS.ErrnoException).code !== "ENOENT") throw e;
    }
  }

  for (const dir of SECTION_DIRS) {
    const full = path.join(docsRoot, dir);
    if (!options.dryRun) {
      await fs.mkdir(full, { recursive: true });
    }
    created.push(path.join("akta-docs", dir));

    if (options.createReadmeInEachFolder) {
      const readme = path.join(full, "README.md");
      const today = new Date().toISOString().slice(0, 10);
      const docId = `${cfg.project.slug}.${dir.replace(/[^a-z0-9]+/g, "-")}-readme`;
      const body = `---
doc_id: ${docId}
doc_type: meta
date: ${today}
---

# ${dir}

Placeholder content for this section. Replace with architecture and business documentation aligned with OPENAKTA GEO standards.
`;
      if (!options.dryRun) {
        await fs.writeFile(readme, body, "utf8");
      }
      created.push(path.join("akta-docs", dir, "README.md"));
    }
    if (options.gitkeep) {
      const gk = path.join(full, ".gitkeep");
      if (!options.dryRun) {
        await fs.writeFile(gk, "", "utf8");
      }
    }
  }

  const changelogPath = path.join(docsRoot, "10-changelog", "CHANGELOG.md");
  const initialChangelog = `---
doc_id: ${cfg.project.slug}.changelog
doc_type: changelog
date: ${new Date().toISOString().slice(0, 10)}
---

# Changelog

<!-- akta-changelog-append -->
`;
  if (!options.dryRun) {
    await fs.writeFile(changelogPath, initialChangelog, "utf8");
  }
  created.push("akta-docs/10-changelog/CHANGELOG.md");

  const yamlText = YAML.stringify(cfg, { lineWidth: 0 });
  if (!options.dryRun) {
    await fs.writeFile(configPath, yamlText, "utf8");
  }

  return {
    root,
    docs_root: docsRoot,
    created_paths: created,
    config_path: configPath,
  };
}
