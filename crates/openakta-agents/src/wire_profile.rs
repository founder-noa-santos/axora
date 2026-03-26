//! Wire protocol profiles for provider transport.
//!
//! This module separates wire protocol selection (how to build HTTP requests)
//! from telemetry identification (how to label metrics/logs).
//!
//! ## Design
//! - `WireProfile` drives request building and transport selection
//! - `ProviderKind` (in `provider.rs`) is used only for telemetry/metrics
//!
//! ## Anthropic Removal Note
//!
//! Anthropic support has been intentionally removed from aktacode.
//! Only OpenAI-compatible wire profiles remain.
//! Future provider integrations (including Anthropic) must be implemented behind openakta-api.
//!
//! ## Example
//! ```
//! use openakta_agents::wire_profile::WireProfile;
//!
//! let profile = WireProfile::OpenAiChatCompletions;
//! let telemetry = profile.telemetry_kind(); // e.g. OpenAi for metrics
//! ```

use serde::{Deserialize, Serialize};

/// Wire protocol profile - drives request building and transport selection.
///
/// ## Design Principle
/// - All providers use OpenAI Chat Completions format
/// - Anthropic was removed (may re-enter via openakta-api)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum WireProfile {
    /// OpenAI Chat Completions API - for OpenAI AND all compatible providers.
    #[default]
    OpenAiChatCompletions,
}

impl WireProfile {
    /// Derive the telemetry kind from this wire profile.
    ///
    /// All providers map to ProviderKind::OpenAi.
    pub fn telemetry_kind(&self) -> crate::provider::ProviderKind {
        crate::provider::ProviderKind::OpenAi
    }

    /// Get the content-type header for this wire profile.
    pub fn content_type(&self) -> &'static str {
        "application/json" // Both protocols use the same content-type
    }

    /// Check if this wire profile supports prompt caching.
    ///
    /// Note: Prompt caching was an Anthropic-specific feature.
    /// OpenAI-compatible providers may have their own caching mechanisms.
    pub fn supports_caching(&self) -> bool {
        false
    }
}

impl std::fmt::Display for WireProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WireProfile::OpenAiChatCompletions => write!(f, "open_ai_chat_completions"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProviderKind;

    #[test]
    fn openai_profile_maps_to_openai_telemetry() {
        assert_eq!(
            WireProfile::OpenAiChatCompletions.telemetry_kind(),
            ProviderKind::OpenAi
        );
    }

    #[test]
    fn openai_profile_does_not_support_caching() {
        assert!(!WireProfile::OpenAiChatCompletions.supports_caching());
    }

    #[test]
    fn display_format() {
        assert_eq!(
            format!("{}", WireProfile::OpenAiChatCompletions),
            "open_ai_chat_completions"
        );
    }
}
