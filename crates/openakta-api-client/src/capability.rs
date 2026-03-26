//! Capability negotiation for intelligent routing
//!
//! This module provides capability negotiation protocol for determining
//! which execution path (local vs hosted) should be used based on:
//! - Required capabilities (vision, function calling, etc.)
//! - Provider health status
//! - Cost and latency considerations
//! - Subscription tier

use serde::{Deserialize, Serialize};

/// Capabilities that can be required for routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Basic text generation (always available)
    TextGeneration,
    /// Vision/image understanding
    Vision,
    /// Function/tool calling
    FunctionCalling,
    /// JSON mode/structured output
    JsonMode,
    /// Prompt caching
    PromptCaching,
    /// Extended thinking/reasoning
    ExtendedThinking,
    /// Code understanding
    CodeUnderstanding,
    /// Embedding generation
    Embedding,
    /// Reranking
    Reranking,
}

impl Capability {
    /// Check if this capability is available locally
    pub fn is_local_capable(&self) -> bool {
        match self {
            // Local-first capabilities
            Capability::TextGeneration => true,    // Ollama
            Capability::CodeUnderstanding => true, // Local models
            Capability::Embedding => true,         // Candle
            Capability::Reranking => true,         // Cross-encoder

            // Hosted-only capabilities
            Capability::Vision => false, // Requires cloud models
            Capability::FunctionCalling => false, // Limited local support
            Capability::JsonMode => true, // Can be done locally
            Capability::PromptCaching => false, // Provider-specific
            Capability::ExtendedThinking => false, // Cloud models only
        }
    }

    /// Check if this capability requires hosted execution
    pub fn requires_hosted(&self) -> bool {
        !self.is_local_capable()
    }

    /// Get all capabilities required for a given task
    pub fn for_task(task: &str) -> Vec<Capability> {
        match task {
            "chat" => vec![Capability::TextGeneration],
            "code_completion" => vec![Capability::TextGeneration, Capability::CodeUnderstanding],
            "vision" => vec![Capability::TextGeneration, Capability::Vision],
            "function_calling" => vec![Capability::TextGeneration, Capability::FunctionCalling],
            "embedding" => vec![Capability::Embedding],
            "reranking" => vec![Capability::Reranking],
            _ => vec![Capability::TextGeneration],
        }
    }
}

/// Provider capability information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    /// Provider name
    pub provider: String,
    /// Model name
    pub model: String,
    /// Available capabilities
    pub capabilities: Vec<Capability>,
    /// Context window size
    pub context_window: u32,
    /// Maximum output tokens
    pub max_output_tokens: u32,
    /// Supports streaming
    pub supports_streaming: bool,
    /// Cost per 1K input tokens (USD)
    pub cost_per_1k_input: f64,
    /// Cost per 1K output tokens (USD)
    pub cost_per_1k_output: f64,
    /// Average latency (ms)
    pub avg_latency_ms: u32,
    /// P99 latency (ms)
    pub p99_latency_ms: u32,
}

impl ProviderCapabilities {
    /// Check if provider has all required capabilities
    pub fn has_capabilities(&self, required: &[Capability]) -> bool {
        required.iter().all(|cap| self.capabilities.contains(cap))
    }

    /// Check if provider can handle the request
    pub fn can_handle(
        &self,
        required: &[Capability],
        max_cost: Option<f64>,
        max_latency: Option<u32>,
    ) -> bool {
        // Check capabilities
        if !self.has_capabilities(required) {
            return false;
        }

        // Check cost constraint
        if let Some(max) = max_cost {
            let estimated_cost = self.cost_per_1k_input + self.cost_per_1k_output;
            if estimated_cost > max {
                return false;
            }
        }

        // Check latency constraint
        if let Some(max) = max_latency {
            if self.p99_latency_ms > max {
                return false;
            }
        }

        true
    }
}

/// Capability negotiation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiationResult {
    /// Selected provider
    pub provider: String,
    /// Selected model
    pub model: String,
    /// Execution strategy
    pub execution_strategy: crate::execution_strategy::ExecutionStrategy,
    /// Reason for selection
    pub reason: String,
    /// Estimated cost (USD)
    pub estimated_cost: f64,
    /// Estimated latency (ms)
    pub estimated_latency_ms: u32,
}

/// Capability negotiator for intelligent routing
pub struct CapabilityNegotiator {
    /// Available local capabilities
    local_capabilities: Vec<Capability>,
    /// Available hosted providers
    hosted_providers: Vec<ProviderCapabilities>,
}

impl CapabilityNegotiator {
    /// Create a new negotiator
    pub fn new() -> Self {
        Self {
            local_capabilities: vec![
                Capability::TextGeneration,
                Capability::CodeUnderstanding,
                Capability::Embedding,
                Capability::Reranking,
                Capability::JsonMode,
            ],
            hosted_providers: Vec::new(),
        }
    }

    /// Add a hosted provider
    pub fn add_provider(&mut self, provider: ProviderCapabilities) {
        self.hosted_providers.push(provider);
    }

    /// Negotiate the best execution path for required capabilities
    pub fn negotiate(
        &self,
        required: &[Capability],
        constraints: NegotiationConstraints,
    ) -> NegotiationResult {
        // Check if all required capabilities are available locally
        let all_local = required
            .iter()
            .all(|cap| self.local_capabilities.contains(cap));

        if all_local {
            // Prefer local for speed and offline capability
            return NegotiationResult {
                provider: "local".to_string(),
                model: "ollama/codellama".to_string(),
                execution_strategy: crate::execution_strategy::ExecutionStrategy::LocalOnly,
                reason: "All capabilities available locally".to_string(),
                estimated_cost: 0.0,
                estimated_latency_ms: 100, // Local inference
            };
        }

        // Find best hosted provider
        let best_hosted = self.find_best_hosted_provider(required, &constraints);

        match best_hosted {
            Some(provider) => {
                // Check if we should use local with fallback or hosted
                let has_partial_local = required
                    .iter()
                    .any(|cap| self.local_capabilities.contains(cap));

                if has_partial_local && constraints.allow_fallback {
                    NegotiationResult {
                        provider: "hybrid".to_string(),
                        model: format!("{}/{}", provider.provider, provider.model),
                        execution_strategy:
                            crate::execution_strategy::ExecutionStrategy::LocalWithFallback,
                        reason: "Partial local capabilities, using hybrid approach".to_string(),
                        estimated_cost: provider.cost_per_1k_input + provider.cost_per_1k_output,
                        estimated_latency_ms: provider.avg_latency_ms,
                    }
                } else {
                    NegotiationResult {
                        provider: provider.provider.clone(),
                        model: provider.model.clone(),
                        execution_strategy:
                            crate::execution_strategy::ExecutionStrategy::HostedOnly,
                        reason: "Required capabilities only available in hosted".to_string(),
                        estimated_cost: provider.cost_per_1k_input + provider.cost_per_1k_output,
                        estimated_latency_ms: provider.avg_latency_ms,
                    }
                }
            }
            None => {
                // No suitable provider found, fall back to local with degraded capabilities
                NegotiationResult {
                    provider: "local".to_string(),
                    model: "ollama/codellama".to_string(),
                    execution_strategy: crate::execution_strategy::ExecutionStrategy::LocalOnly,
                    reason: "No suitable hosted provider, using local with degraded capabilities"
                        .to_string(),
                    estimated_cost: 0.0,
                    estimated_latency_ms: 200,
                }
            }
        }
    }

    /// Find the best hosted provider for required capabilities
    fn find_best_hosted_provider(
        &self,
        required: &[Capability],
        constraints: &NegotiationConstraints,
    ) -> Option<&ProviderCapabilities> {
        let mut best: Option<&ProviderCapabilities> = None;
        let mut best_score = f64::NEG_INFINITY;

        for provider in &self.hosted_providers {
            // Skip if provider doesn't have required capabilities
            if !provider.has_capabilities(required) {
                continue;
            }

            // Calculate score (higher is better)
            let mut score = 0.0;

            // Cost score (lower cost = higher score)
            let total_cost = provider.cost_per_1k_input + provider.cost_per_1k_output;
            score -= total_cost * 10.0;

            // Latency score (lower latency = higher score)
            score -= provider.p99_latency_ms as f64 * 0.01;

            // Capability match score (more capabilities = higher score)
            score += provider.capabilities.len() as f64 * 0.5;

            // Apply constraints
            if let Some(max_cost) = constraints.max_cost {
                if total_cost > max_cost {
                    continue;
                }
            }

            if let Some(max_latency) = constraints.max_latency_ms {
                if provider.p99_latency_ms > max_latency {
                    continue;
                }
            }

            if score > best_score {
                best_score = score;
                best = Some(provider);
            }
        }

        best
    }
}

impl Default for CapabilityNegotiator {
    fn default() -> Self {
        Self::new()
    }
}

/// Constraints for capability negotiation
#[derive(Debug, Clone, Default)]
pub struct NegotiationConstraints {
    /// Maximum acceptable cost (USD)
    pub max_cost: Option<f64>,
    /// Maximum acceptable latency (ms)
    pub max_latency_ms: Option<u32>,
    /// Allow fallback to degraded capabilities
    pub allow_fallback: bool,
    /// Prefer cheaper options
    pub cost_sensitive: bool,
    /// Prefer lower latency
    pub latency_sensitive: bool,
}

impl NegotiationConstraints {
    /// Create new constraints with cost limit
    pub fn with_max_cost(mut self, max_cost: f64) -> Self {
        self.max_cost = Some(max_cost);
        self
    }

    /// Create new constraints with latency limit
    pub fn with_max_latency(mut self, max_latency_ms: u32) -> Self {
        self.max_latency_ms = Some(max_latency_ms);
        self
    }

    /// Create new constraints allowing fallback
    pub fn with_fallback(mut self, allow: bool) -> Self {
        self.allow_fallback = allow;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_capabilities() {
        assert!(Capability::TextGeneration.is_local_capable());
        assert!(Capability::Embedding.is_local_capable());
        assert!(!Capability::Vision.is_local_capable());
        assert!(!Capability::FunctionCalling.is_local_capable());
    }

    #[test]
    fn test_provider_capabilities() {
        let provider = ProviderCapabilities {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            capabilities: vec![
                Capability::TextGeneration,
                Capability::Vision,
                Capability::FunctionCalling,
            ],
            context_window: 128000,
            max_output_tokens: 4096,
            supports_streaming: true,
            cost_per_1k_input: 0.01,
            cost_per_1k_output: 0.03,
            avg_latency_ms: 500,
            p99_latency_ms: 1000,
        };

        assert!(provider.has_capabilities(&[Capability::TextGeneration]));
        assert!(provider.has_capabilities(&[Capability::TextGeneration, Capability::Vision]));
        assert!(!provider.has_capabilities(&[Capability::Embedding]));
    }

    #[test]
    fn test_negotiation_local() {
        let negotiator = CapabilityNegotiator::new();
        let required = vec![Capability::TextGeneration, Capability::CodeUnderstanding];

        let result = negotiator.negotiate(&required, NegotiationConstraints::default());

        assert_eq!(
            result.execution_strategy,
            crate::execution_strategy::ExecutionStrategy::LocalOnly
        );
        assert_eq!(result.estimated_cost, 0.0);
    }
}
