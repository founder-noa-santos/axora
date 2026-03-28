//! Mission Operating Layer (MOL) feature flags for the local runtime.

use serde::{Deserialize, Serialize};

fn default_verification_automation_enabled() -> bool {
    true
}

fn default_mol_raw_execution_allowed() -> bool {
    true
}

fn default_closure_allow_open_findings() -> bool {
    true
}

/// Mission Operating Layer (MOL) feature flags.
///
/// These flags are local workflow policy. They are not cloud API client concerns.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct MolFeatureFlags {
    #[serde(default)]
    pub strict_legacy_fence: bool,
    #[serde(default = "default_mol_raw_execution_allowed")]
    pub raw_execution_allowed: bool,
    #[serde(default = "default_verification_automation_enabled")]
    pub verification_automation_enabled: bool,
    #[serde(default = "default_closure_allow_open_findings")]
    pub closure_allow_open_findings: bool,
}

impl Default for MolFeatureFlags {
    fn default() -> Self {
        Self {
            strict_legacy_fence: false,
            raw_execution_allowed: true,
            verification_automation_enabled: true,
            closure_allow_open_findings: true,
        }
    }
}

impl MolFeatureFlags {
    /// Baseline defaults merged with `MOL_*` environment variables.
    pub fn from_env_with_defaults() -> Self {
        let mut f = Self::default();
        f.apply_env_overrides();
        f
    }

    /// Apply `MOL_STRICT_LEGACY_FENCE`, `MOL_RAW_EXECUTION_ALLOWED`,
    /// `MOL_VERIFICATION_AUTOMATION`, and `MOL_CLOSURE_ALLOW_OPEN_FINDINGS` if set.
    pub fn apply_env_overrides(&mut self) {
        if let Some(v) = parse_bool_str_from_env("MOL_STRICT_LEGACY_FENCE") {
            self.strict_legacy_fence = v;
        }
        if let Some(v) = parse_bool_str_from_env("MOL_RAW_EXECUTION_ALLOWED") {
            self.raw_execution_allowed = v;
        }
        if let Some(v) = parse_bool_str_from_env("MOL_VERIFICATION_AUTOMATION") {
            self.verification_automation_enabled = v;
        }
        if let Some(v) = parse_bool_str_from_env("MOL_CLOSURE_ALLOW_OPEN_FINDINGS") {
            self.closure_allow_open_findings = v;
        }
    }
}

pub(crate) fn parse_bool_str(raw: &str) -> Option<bool> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    match s.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn parse_bool_str_from_env(key: &str) -> Option<bool> {
    let Ok(raw) = std::env::var(key) else {
        return None;
    };
    match parse_bool_str(&raw) {
        Some(b) => Some(b),
        None => {
            tracing::warn!(
                env_key = key,
                value = %raw,
                "invalid boolean for MOL env flag; keeping previous value"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bool_str_accepts_common_forms() {
        assert_eq!(parse_bool_str("true"), Some(true));
        assert_eq!(parse_bool_str("FALSE"), Some(false));
        assert_eq!(parse_bool_str("1"), Some(true));
        assert_eq!(parse_bool_str("off"), Some(false));
        assert_eq!(parse_bool_str("YES"), Some(true));
        assert_eq!(parse_bool_str("maybe"), None);
        assert_eq!(parse_bool_str(""), None);
    }
}
