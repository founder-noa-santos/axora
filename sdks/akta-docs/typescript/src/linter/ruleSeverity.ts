import type { AktaConfig } from "../config/schema.js";
import type { RuleId } from "../types.js";

export type EffectiveSeverity = "error" | "warn" | "off";

export function ruleSeverity(
  config: AktaConfig,
  ruleId: RuleId,
): EffectiveSeverity {
  const override = config.linter.rules[ruleId]?.severity;
  if (override) return override;
  return config.linter.default_severity;
}

export function ruleNumberOption(
  config: AktaConfig,
  ruleId: RuleId,
  key: "min_words" | "max_words" | "min_question_ratio",
  fallback: number,
): number {
  const v = config.linter.rules[ruleId]?.[key];
  return typeof v === "number" ? v : fallback;
}
