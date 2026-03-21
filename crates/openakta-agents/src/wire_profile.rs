//! Wire protocol profiles for provider transport.
//!
//! This module separates wire protocol selection (how to build HTTP requests)
//! from telemetry identification (how to label metrics/logs).
//!
//! ## Design
//! - `WireProfile` drives request building and transport selection
//! - `ProviderKind` (in `provider.rs`) is used only for telemetry/metrics
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
/// - `AnthropicMessagesV1`: ONLY for Anthropic/Claude native API
/// - `OpenAiChatCompletions`: For OpenAI AND all OpenAI-compatible providers
///   (DeepSeek, Qwen, Moonshot, Gemini, Mistral, Ollama, etc.)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum WireProfile {
    /// Anthropic Messages API v1 - ONLY for Anthropic/Claude.
    AnthropicMessagesV1,
    /// OpenAI Chat Completions API - for OpenAI AND all compatible providers.
    #[default]
    OpenAiChatCompletions,
}

impl WireProfile {
    /// Derive the telemetry kind from this wire profile.
    ///
    /// - AnthropicMessagesV1 → ProviderKind::Anthropic
    /// - OpenAiChatCompletions → ProviderKind::OpenAi (for OpenAI AND all compatible providers)
    pub fn telemetry_kind(&self) -> crate::provider::ProviderKind {
        match self {
            WireProfile::AnthropicMessagesV1 => crate::provider::ProviderKind::Anthropic,
            WireProfile::OpenAiChatCompletions => crate::provider::ProviderKind::OpenAi,
        }
    }

    /// Get the content-type header for this wire profile.
    pub fn content_type(&self) -> &'static str {
        "application/json" // Both protocols use the same content-type
    }

    /// Check if this wire profile supports prompt caching.
    ///
    /// Only Anthropic's native API supports prompt caching.
    pub fn supports_caching(&self) -> bool {
        matches!(self, WireProfile::AnthropicMessagesV1)
    }
}

impl std::fmt::Display for WireProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WireProfile::AnthropicMessagesV1 => write!(f, "anthropic_messages_v1"),
            WireProfile::OpenAiChatCompletions => write!(f, "open_ai_chat_completions"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProviderKind;

    #[test]
    fn anthropic_profile_maps_to_anthropic_telemetry() {
        assert_eq!(
            WireProfile::AnthropicMessagesV1.telemetry_kind(),
            ProviderKind::Anthropic
        );
    }

    #[test]
    fn openai_profile_maps_to_openai_telemetry() {
        assert_eq!(
            WireProfile::OpenAiChatCompletions.telemetry_kind(),
            ProviderKind::OpenAi
        );
    }

    #[test]
    fn anthropic_supports_caching() {
        assert!(WireProfile::AnthropicMessagesV1.supports_caching());
    }

    #[test]
    fn openai_profile_does_not_support_caching() {
        assert!(!WireProfile::OpenAiChatCompletions.supports_caching());
    }

    #[test]
    fn display_format() {
        assert_eq!(
            format!("{}", WireProfile::AnthropicMessagesV1),
            "anthropic_messages_v1"
        );
        assert_eq!(
            format!("{}", WireProfile::OpenAiChatCompletions),
            "open_ai_chat_completions"
        );
    }
}
