//! Mission Operating Layer (MOL) feature flags shared by daemon (`CoreConfig`) and API (`AppState`).
//!
//! Environment: `MOL_STRICT_LEGACY_FENCE`, `MOL_RAW_EXECUTION_ALLOWED`, `MOL_VERIFICATION_AUTOMATION`,
//! `MOL_CLOSURE_ALLOW_OPEN_FINDINGS`.
//! TOML: `[mol]` under daemon `openakta.toml` (see `openakta_core::CoreConfig`).

use serde::{Deserialize, Serialize};

fn default_verification_automation_enabled() -> bool {
    true
}

/// Mission Operating Layer (MOL) feature flags.
///
/// Loaded from `[mol]` in `openakta.toml` / `CoreConfig` TOML, then overridden by environment:
/// `MOL_STRICT_LEGACY_FENCE`, `MOL_RAW_EXECUTION_ALLOWED`, `MOL_VERIFICATION_AUTOMATION`,
/// `MOL_CLOSURE_ALLOW_OPEN_FINDINGS` (see [`MolFeatureFlags::apply_env_overrides`]).
///
/// Defaults are **permissive** for brownfield compatibility until hard gates (AB/ABC) are enforced
/// project-wide (`strict_legacy_fence = false`, `raw_execution_allowed = true`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct MolFeatureFlags {
    /// When `true`, legacy work-item APIs must not bypass MOL invariants for prepared stories (AB1).
    #[serde(default)]
    pub strict_legacy_fence: bool,
    /// When `false`, raw execution paths for MOL-prepared stories are blocked (ABC3).
    #[serde(default = "default_mol_raw_execution_allowed")]
    pub raw_execution_allowed: bool,
    /// When `true`, the daemon advances `pending` verification runs to `passed` with minimal automated findings (AB10).
    #[serde(default = "default_verification_automation_enabled")]
    pub verification_automation_enabled: bool,
    /// When `true`, `transition_story_preparation` may set `closed` even if verification findings are still `open` (ABC2).
    /// Set to `false` to enforce resolution of open findings and (when the profile requires verification) completed runs.
    #[serde(default = "default_closure_allow_open_findings")]
    pub closure_allow_open_findings: bool,
}

fn default_mol_raw_execution_allowed() -> bool {
    true
}

fn default_closure_allow_open_findings() -> bool {
    true
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
    /// Baseline defaults merged with `MOL_*` environment variables (for processes without TOML, e.g. API).
    pub fn from_env_with_defaults() -> Self {
        let mut f = Self::default();
        f.apply_env_overrides();
        f
    }

    /// Apply `MOL_STRICT_LEGACY_FENCE`, `MOL_RAW_EXECUTION_ALLOWED`, `MOL_VERIFICATION_AUTOMATION`,
    /// and `MOL_CLOSURE_ALLOW_OPEN_FINDINGS` if set.
    /// Accepts (case-insensitive): `1`/`0`, `true`/`false`, `yes`/`no`, `on`/`off`.
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
