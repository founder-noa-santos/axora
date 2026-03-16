//! Embedding configuration

use serde::{Deserialize, Serialize};

/// Embedding model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Model path or identifier (e.g., "jina-code-v2")
    pub model_name: String,

    /// Embedding dimensions (768 for Jina Code, can truncate via Matryoshka)
    pub dimensions: usize,

    /// Maximum sequence length
    pub max_length: usize,

    /// Batch size for bulk embedding
    pub batch_size: usize,

    /// Device to run inference on ("cpu", "cuda", "metal")
    pub device: String,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model_name: "jina-code-v2".to_string(),
            dimensions: 768,
            max_length: 8192,
            batch_size: 32,
            device: "cpu".to_string(),
        }
    }
}

impl EmbeddingConfig {
    /// Create config for Jina Code Embeddings v2
    pub fn jina_code_v2() -> Self {
        Self {
            model_name: "jina-code-v2".to_string(),
            dimensions: 768,
            max_length: 8192,
            batch_size: 32,
            device: "cpu".to_string(),
        }
    }

    /// Create config with custom dimensions (Matryoshka truncation)
    pub fn with_dimensions(mut self, dimensions: usize) -> Self {
        self.dimensions = dimensions;
        self
    }

    /// Set device for inference
    pub fn with_device(mut self, device: &str) -> Self {
        self.device = device.to_string();
        self
    }
}
