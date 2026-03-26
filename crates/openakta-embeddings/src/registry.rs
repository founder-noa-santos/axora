//! Embedding registry for unified access to all embedding models.
//!
//! This registry provides a single point of access for all embedding needs
//! across the OPENAKTA system, ensuring consistent embedding usage.

use std::sync::Arc;

use crate::embedder::EmbeddingModel;
use crate::Result;

/// Unified embedding registry for all OPENAKTA embedding needs.
///
/// This registry holds references to all embedding models used across
/// different domains (semantic, code, skill) and provides a single
/// point of access for embedding operations.
pub struct EmbeddingRegistry {
    /// Semantic memory embeddings (384-dim for Minilm/BGE).
    pub semantic: Arc<dyn EmbeddingModel>,
    /// Code embeddings (768-dim for JinaCode).
    pub code: Arc<dyn EmbeddingModel>,
    /// Skill embeddings (384-dim for BGE skill).
    pub skill: Arc<dyn EmbeddingModel>,
}

impl EmbeddingRegistry {
    /// Create a new embedding registry with the specified models.
    pub fn new(
        semantic: Arc<dyn EmbeddingModel>,
        code: Arc<dyn EmbeddingModel>,
        skill: Arc<dyn EmbeddingModel>,
    ) -> Self {
        Self {
            semantic,
            code,
            skill,
        }
    }

    /// Embed text using the semantic model.
    pub async fn embed_semantic(&self, input: &str) -> Result<Vec<f32>> {
        self.semantic.embed(input).await
    }

    /// Embed code using the code model.
    pub async fn embed_code(&self, input: &str) -> Result<Vec<f32>> {
        self.code.embed(input).await
    }

    /// Embed skill using the skill model.
    pub async fn embed_skill(&self, input: &str) -> Result<Vec<f32>> {
        self.skill.embed(input).await
    }

    /// Embed a batch using the semantic model.
    pub async fn embed_semantic_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>> {
        self.semantic.embed_batch(inputs).await
    }

    /// Embed a batch using the code model.
    pub async fn embed_code_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>> {
        self.code.embed_batch(inputs).await
    }

    /// Embed a batch using the skill model.
    pub async fn embed_skill_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>> {
        self.skill.embed_batch(inputs).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CodeEmbeddingConfig, SkillEmbeddingConfig};
    use crate::embedder::{BgeSkillEmbedder, EmbeddingModel, JinaCodeEmbedder};

    #[tokio::test]
    async fn test_registry_creation() {
        // Create mock embedders (using actual implementations for integration test)
        let code_config = CodeEmbeddingConfig::default();
        let skill_config = SkillEmbeddingConfig::default();

        let code_embedder = Arc::new(JinaCodeEmbedder::new(code_config).unwrap());
        let skill_embedder = Arc::new(BgeSkillEmbedder::new(skill_config).unwrap());

        // For semantic, we'd use a research embedder in production
        // For this test, we reuse skill as a placeholder
        let semantic_embedder = skill_embedder.clone();

        let registry = EmbeddingRegistry::new(semantic_embedder, code_embedder, skill_embedder);

        assert!(registry.semantic.as_ref().profile().dimensions > 0);
        assert!(registry.code.as_ref().profile().dimensions > 0);
        assert!(registry.skill.as_ref().profile().dimensions > 0);
    }

    #[tokio::test]
    async fn test_registry_embed_all_paths() {
        let code_config = CodeEmbeddingConfig::default();
        let skill_config = SkillEmbeddingConfig::default();

        let code_embedder = Arc::new(JinaCodeEmbedder::new(code_config).unwrap());
        let skill_embedder = Arc::new(BgeSkillEmbedder::new(skill_config).unwrap());
        let semantic_embedder = skill_embedder.clone();

        let registry = EmbeddingRegistry::new(
            semantic_embedder.clone(),
            code_embedder.clone(),
            skill_embedder.clone(),
        );

        // Test all embed paths
        let semantic_input = "test semantic query";
        let code_input = "fn test() {}";
        let skill_input = "SKILL: test skill";

        let semantic_emb = registry.embed_semantic(semantic_input).await.unwrap();
        let code_emb = registry.embed_code(code_input).await.unwrap();
        let skill_emb = registry.embed_skill(skill_input).await.unwrap();

        // Verify dimensions match expected
        assert_eq!(semantic_emb.len(), semantic_embedder.profile().dimensions);
        assert_eq!(code_emb.len(), code_embedder.profile().dimensions);
        assert_eq!(skill_emb.len(), skill_embedder.profile().dimensions);
    }
}
