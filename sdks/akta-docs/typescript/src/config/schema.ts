import { z } from "zod";

export const DOC_TYPES = [
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
] as const;

export const SEVERITY = z.enum(["error", "warn", "off"]);

const ruleOptions = z
  .object({
    severity: SEVERITY.optional(),
    min_words: z.number().int().positive().optional(),
    max_words: z.number().int().positive().optional(),
    min_question_ratio: z.number().min(0).max(1).optional(),
    heading_levels: z.array(z.union([z.literal(2), z.literal(3)])).optional(),
  })
  .strict();

export const AktaConfigSchema = z
  .object({
    schema_version: z.string().min(1),
    project: z.object({
      name: z.string().min(1),
      slug: z.string().min(1).optional(),
    }),
    paths: z.object({
      docs_root: z.string().min(1),
      include_globs: z.array(z.string()).default(["**/*.md"]),
      exclude_globs: z
        .array(z.string())
        .default(["**/node_modules/**", "**/.git/**", "**/99-archive/**"]),
    }),
    linter: z.object({
      default_severity: z.enum(["error", "warn", "off"]).default("error"),
      rules: z.record(z.string(), ruleOptions).default({}),
    }),
    scaffold: z
      .object({
        create_readme_in_each_folder: z.boolean().default(true),
        gitkeep: z.boolean().default(false),
      })
      .default({ create_readme_in_each_folder: true, gitkeep: false }),
    changelog: z
      .object({
        default_target: z.string().optional(),
        entry_template: z.enum(["compact", "detailed"]).default("compact"),
        summary_max_length: z.number().int().positive().default(200),
      })
      .default({ entry_template: "compact", summary_max_length: 200 }),
  })
  .strict();

export type AktaConfig = z.infer<typeof AktaConfigSchema>;
export type RuleOptions = z.infer<typeof ruleOptions>;

export const ChangelogEntrySchema = z
  .object({
    schema_version: z.string().min(1),
    doc_id: z.string().min(1),
    timestamp: z.string().min(1),
    change_type: z.enum([
      "added",
      "changed",
      "fixed",
      "deprecated",
      "removed",
      "security",
    ]),
    summary: z.string().min(1),
    details: z.string().optional(),
    scope: z.string().optional(),
    refs: z.array(z.string()).optional(),
  })
  .strict();
