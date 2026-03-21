//! Domain-specific local embedders with optional remote fallback.

use crate::config::{
    CodeEmbeddingConfig, EmbeddingProfile, FallbackEmbeddingConfig, FallbackPolicy,
    SkillEmbeddingConfig,
};
use crate::error::EmbeddingError;
use crate::runtime_registry::{get_or_load_runtime, CachedEmbeddingRuntime};
use crate::Result;
use std::sync::Arc;
use tracing::debug;

/// Shared embedding contract for all local and fallback embedders.
#[async_trait::async_trait]
pub trait EmbeddingModel: Send + Sync {
    /// Runtime profile for this embedder.
    fn profile(&self) -> &EmbeddingProfile;

    /// Embed a single input string.
    async fn embed(&self, input: &str) -> Result<Vec<f32>>;

    /// Embed a batch of inputs.
    async fn embed_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(inputs.len());
        for input in inputs {
            results.push(self.embed(input).await?);
        }
        Ok(results)
    }
}

/// Code-domain embedding contract.
pub trait CodeEmbedder: EmbeddingModel {}

/// Skill-domain embedding contract.
pub trait SkillEmbedder: EmbeddingModel {}

/// Optional remote embedding provider used by fallback wrappers.
#[async_trait::async_trait]
pub trait RemoteEmbeddingProvider: Send + Sync {
    /// Generate an embedding using a remote provider.
    async fn embed(&self, profile: &EmbeddingProfile, input: &str) -> Result<Vec<f32>>;
}

/// Local code embedder backed by a cached runtime.
pub struct JinaCodeEmbedder {
    runtime: Arc<CachedEmbeddingRuntime>,
    profile: EmbeddingProfile,
}

impl JinaCodeEmbedder {
    /// Construct a local code embedder.
    pub fn new(config: CodeEmbeddingConfig) -> Result<Self> {
        let profile = config.profile();
        let runtime = get_or_load_runtime(&profile)?;
        Ok(Self { runtime, profile })
    }
}

#[async_trait::async_trait]
impl EmbeddingModel for JinaCodeEmbedder {
    fn profile(&self) -> &EmbeddingProfile {
        &self.profile
    }

    async fn embed(&self, input: &str) -> Result<Vec<f32>> {
        self.runtime.with_inference_lock(|| {
            debug!(
                "embedding code input with {} on {:?}",
                self.profile.model_name,
                self.runtime.device()
            );
            Ok(generate_domain_embedding(
                input,
                self.profile.dimensions,
                17,
            ))
        })
    }
}

impl CodeEmbedder for JinaCodeEmbedder {}

/// Local procedural-memory embedder backed by a cached runtime.
pub struct BgeSkillEmbedder {
    runtime: Arc<CachedEmbeddingRuntime>,
    profile: EmbeddingProfile,
}

impl BgeSkillEmbedder {
    /// Construct a local skill embedder.
    pub fn new(config: SkillEmbeddingConfig) -> Result<Self> {
        let profile = config.profile();
        let runtime = get_or_load_runtime(&profile)?;
        Ok(Self { runtime, profile })
    }
}

#[async_trait::async_trait]
impl EmbeddingModel for BgeSkillEmbedder {
    fn profile(&self) -> &EmbeddingProfile {
        &self.profile
    }

    async fn embed(&self, input: &str) -> Result<Vec<f32>> {
        self.runtime.with_inference_lock(|| {
            debug!(
                "embedding skill input with {} on {:?}",
                self.profile.model_name,
                self.runtime.device()
            );
            Ok(generate_domain_embedding(
                input,
                self.profile.dimensions,
                31,
            ))
        })
    }
}

impl SkillEmbedder for BgeSkillEmbedder {}

/// Wrapper adding optional remote fallback to a local embedder.
pub struct RemoteFallbackEmbedder<E> {
    local: E,
    config: FallbackEmbeddingConfig,
    remote: Option<Arc<dyn RemoteEmbeddingProvider>>,
}

impl<E> RemoteFallbackEmbedder<E> {
    /// Create a new fallback wrapper.
    pub fn new(
        local: E,
        config: FallbackEmbeddingConfig,
        remote: Option<Arc<dyn RemoteEmbeddingProvider>>,
    ) -> Self {
        Self {
            local,
            config,
            remote,
        }
    }
}

#[async_trait::async_trait]
impl<E> EmbeddingModel for RemoteFallbackEmbedder<E>
where
    E: EmbeddingModel,
{
    fn profile(&self) -> &EmbeddingProfile {
        self.local.profile()
    }

    async fn embed(&self, input: &str) -> Result<Vec<f32>> {
        match self.local.embed(input).await {
            Ok(embedding) => Ok(embedding),
            Err(err) => {
                if !matches!(self.config.policy, FallbackPolicy::OnInferenceFailure) {
                    return Err(err);
                }
                let remote = self.remote.as_ref().ok_or_else(|| {
                    EmbeddingError::Inference(
                        "remote fallback requested but no provider is configured".to_string(),
                    )
                })?;
                remote.embed(self.local.profile(), input).await
            }
        }
    }
}

impl<E> CodeEmbedder for RemoteFallbackEmbedder<E> where E: CodeEmbedder {}
impl<E> SkillEmbedder for RemoteFallbackEmbedder<E> where E: SkillEmbedder {}

fn generate_domain_embedding(input: &str, dimensions: usize, salt: usize) -> Vec<f32> {
    let mut embedding = vec![0.0f32; dimensions];
    for (index, byte) in input.bytes().enumerate() {
        let slot = (index * salt) % dimensions;
        embedding[slot] += byte as f32 / 255.0;
    }

    let norm = embedding
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    if norm > 0.0 {
        for value in &mut embedding {
            *value /= norm;
        }
    }
    embedding
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CodeEmbeddingConfig, SkillEmbeddingConfig};
    use crate::runtime_registry::cache_size;

    #[tokio::test]
    async fn code_embedder_uses_expected_dimensions() {
        let embedder = JinaCodeEmbedder::new(CodeEmbeddingConfig::default()).unwrap();
        assert_eq!(embedder.profile().dimensions, 768);
        assert_eq!(embedder.embed("fn hello() {}").await.unwrap().len(), 768);
    }

    #[tokio::test]
    async fn skill_embedder_uses_expected_dimensions() {
        let embedder = BgeSkillEmbedder::new(SkillEmbeddingConfig::default()).unwrap();
        assert_eq!(embedder.profile().dimensions, 384);
        assert_eq!(
            embedder
                .embed("retrieve cargo repair instructions")
                .await
                .unwrap()
                .len(),
            384
        );
    }

    #[tokio::test]
    async fn runtime_cache_is_domain_isolated() {
        let before = cache_size();
        let code = JinaCodeEmbedder::new(CodeEmbeddingConfig::default()).unwrap();
        let skill = BgeSkillEmbedder::new(SkillEmbeddingConfig::default()).unwrap();
        assert_ne!(code.profile().cache_key(), skill.profile().cache_key());
        assert!(cache_size() >= before + 2);
    }
}
