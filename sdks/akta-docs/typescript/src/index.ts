export type {
  AppendReport,
  ChangelogEntry,
  ChangeType,
  Diagnostic,
  LintResult,
  LintSummary,
  RuleId,
  ScaffoldReport,
  Severity,
  TemplateKind,
} from "./types.js";
export { loadConfig, resolveConfigPath, ConfigError } from "./config/load.js";
export { AktaConfigSchema, ChangelogEntrySchema, DOC_TYPES } from "./config/schema.js";
export type { AktaConfig } from "./config/schema.js";
export { expandLintPaths, lintFiles } from "./linter/engine.js";
export { countWords } from "./linter/wordCount.js";
export { appendChangelogEntry, ChangelogError } from "./changelog.js";
export { runScaffold, defaultAktaConfig } from "./scaffolder.js";
export type { ScaffoldOptions } from "./scaffolder.js";
export { writeTemplate } from "./templates.js";
export type { TemplateVars } from "./templates.js";
