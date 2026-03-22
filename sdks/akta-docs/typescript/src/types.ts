/** ESLint-style severity */
export type Severity = "error" | "warn" | "info";

/** Rule identifiers (normative) */
export type RuleId =
  | "META-001"
  | "META-002"
  | "META-003"
  | "META-004"
  | "META-QUICK"
  | "STRUCT-008"
  | "CONTENT-001";

export interface Diagnostic {
  file: string;
  line: number;
  column: number;
  end_line?: number;
  end_column?: number;
  rule_id: RuleId;
  severity: Severity;
  message: string;
  doc_url?: string;
}

export interface LintSummary {
  error_count: number;
  warn_count: number;
}

export interface LintResult {
  diagnostics: Diagnostic[];
  summary: LintSummary;
}

export type ChangeType =
  | "added"
  | "changed"
  | "fixed"
  | "deprecated"
  | "removed"
  | "security";

export interface ChangelogEntry {
  schema_version: string;
  doc_id: string;
  timestamp: string;
  change_type: ChangeType;
  summary: string;
  details?: string;
  scope?: string;
  refs?: string[];
}

export interface AppendReport {
  target: string;
  bytes_written: number;
  created: boolean;
}

export interface ScaffoldReport {
  root: string;
  docs_root: string;
  created_paths: string[];
  config_path: string;
}

export type TemplateKind =
  | "adr"
  | "business_rule"
  | "feature"
  | "guide"
  | "reference"
  | "explanation"
  | "research"
  | "meta"
  | "changelog"
  | "technical"
  | "other";
