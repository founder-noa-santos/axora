namespace OpenAkta.AktaDocs;

public static class RuleSeverity
{
    public static string Effective(AktaConfig cfg, string ruleId)
    {
        if (cfg.Linter.Rules.TryGetValue(ruleId, out var ro) && ro.Severity != null)
            return ro.Severity;
        return cfg.Linter.DefaultSeverity;
    }

    public static double NumberOption(AktaConfig cfg, string ruleId, string key, double fallback)
    {
        if (!cfg.Linter.Rules.TryGetValue(ruleId, out var ro) || ro == null) return fallback;
        return key switch
        {
            "min_words" => ro.MinWords ?? fallback,
            "max_words" => ro.MaxWords ?? fallback,
            "min_question_ratio" => ro.MinQuestionRatio ?? fallback,
            _ => fallback
        };
    }

    public static string? MapSeverity(string eff) => eff == "off" ? null : eff;
}
