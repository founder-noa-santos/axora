package dev.openakta.aktadocs;

public final class RuleSeverity {
    private RuleSeverity() {}

    public static String effective(AktaConfig cfg, String ruleId) {
        RuleOptions ro = cfg.linter.rules.get(ruleId);
        if (ro != null && ro.severity != null) {
            return ro.severity;
        }
        return cfg.linter.defaultSeverity;
    }

    public static double numberOption(AktaConfig cfg, String ruleId, String key, double fallback) {
        RuleOptions ro = cfg.linter.rules.get(ruleId);
        if (ro == null) return fallback;
        return switch (key) {
            case "min_words" -> ro.minWords != null ? ro.minWords : fallback;
            case "max_words" -> ro.maxWords != null ? ro.maxWords : fallback;
            case "min_question_ratio" ->
                    ro.minQuestionRatio != null ? ro.minQuestionRatio : fallback;
            default -> fallback;
        };
    }

    public static String mapSeverity(String eff) {
        if ("off".equals(eff)) return null;
        return eff;
    }
}
