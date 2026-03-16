//! Embedding engine using Candle

use crate::config::EmbeddingConfig;
use crate::error::EmbeddingError;
use crate::Result;
use candle_core::{Device, Tensor};
use tracing::{info, debug};

/// Embedding engine for code
pub struct EmbeddingEngine {
    config: EmbeddingConfig,
    device: Device,
}

impl EmbeddingEngine {
    /// Create new embedding engine
    pub fn new(config: EmbeddingConfig) -> Result<Self> {
        let device = match config.device.as_str() {
            "cuda" => Device::new_cuda(0).map_err(|e| {
                EmbeddingError::ModelLoad(format!("Failed to load CUDA device: {}", e))
            })?,
            "metal" => Device::new_metal(0).map_err(|e| {
                EmbeddingError::ModelLoad(format!("Failed to load Metal device: {}", e))
            })?,
            _ => Device::Cpu,
        };

        info!("Initialized embedding engine on {:?}", device);
        debug!("Model: {}, Dimensions: {}", config.model_name, config.dimensions);

        Ok(Self {
            config,
            device,
        })
    }

    /// Generate embedding for single code snippet
    ///
    /// Target: <25ms for 512 tokens
    pub async fn embed(&self, code: &str) -> Result<Vec<f32>> {
        debug!("Embedding {} characters", code.len());

        // Generate pseudo-embedding using hash-based approach
        // This is a placeholder - in production, load real Jina Code model
        let embedding = self.generate_pseudo_embedding(code)?;

        Ok(embedding)
    }

    /// Generate embeddings for batch of code snippets
    ///
    /// Target: >100 chunks/sec
    pub async fn embed_batch(&self, codes: &[&str]) -> Result<Vec<Vec<f32>>> {
        debug!("Batch embedding {} snippets", codes.len());

        let mut embeddings = Vec::with_capacity(codes.len());

        for code in codes {
            let embedding = self.embed(code).await?;
            embeddings.push(embedding);
        }

        Ok(embeddings)
    }

    /// Get embedding dimensions
    pub fn dimensions(&self) -> usize {
        self.config.dimensions
    }

    /// Generate pseudo-embedding using character hash
    /// This produces consistent, normalized vectors for testing
    fn generate_pseudo_embedding(&self, code: &str) -> Result<Vec<f32>> {
        let dim = self.config.dimensions;
        let mut embedding = vec![0.0f32; dim];

        // Simple hash-based embedding (placeholder for real model)
        for (i, byte) in code.bytes().enumerate() {
            let idx = i % dim;
            embedding[idx] += (byte as f32) / 256.0;
        }

        // Normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 1e-10 {
            for x in &mut embedding {
                *x /= norm;
            }
        }

        Ok(embedding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embedder_creation() {
        let config = EmbeddingConfig::default();
        let engine = EmbeddingEngine::new(config).unwrap();
        assert_eq!(engine.dimensions(), 768);
    }

    #[tokio::test]
    async fn test_single_embed() {
        let config = EmbeddingConfig::default();
        let engine = EmbeddingEngine::new(config).unwrap();

        let code = "fn hello() { println!(\"world\"); }";
        let embedding = engine.embed(code).await.unwrap();

        assert_eq!(embedding.len(), 768);
        
        // Check normalization (should be close to 1.0)
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01, "Embedding should be normalized");
    }

    #[tokio::test]
    async fn test_batch_embed() {
        let config = EmbeddingConfig::default();
        let engine = EmbeddingEngine::new(config).unwrap();

        let codes = vec![
            "fn foo() {}",
            "fn bar() {}",
            "fn baz() {}",
        ];
        let embeddings = engine.embed_batch(&codes).await.unwrap();

        assert_eq!(embeddings.len(), 3);
        for embedding in embeddings {
            assert_eq!(embedding.len(), 768);
        }
    }

    #[tokio::test]
    async fn test_embedding_consistency() {
        let config = EmbeddingConfig::default();
        let engine = EmbeddingEngine::new(config).unwrap();

        let code = "fn test() { }";
        let emb1 = engine.embed(code).await.unwrap();
        let emb2 = engine.embed(code).await.unwrap();

        // Same input should produce same embedding
        assert_eq!(emb1, emb2);
    }

    #[tokio::test]
    async fn test_embedding_different() {
        let config = EmbeddingConfig::default();
        let engine = EmbeddingEngine::new(config).unwrap();

        let code1 = "fn foo() {}";
        let code2 = "fn bar() {}";
        
        let emb1 = engine.embed(code1).await.unwrap();
        let emb2 = engine.embed(code2).await.unwrap();

        // Different inputs should produce different embeddings
        assert_ne!(emb1, emb2);
    }
}
