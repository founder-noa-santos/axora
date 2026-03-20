//! Embedding configuration for dual retrieval domains.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Embedding domain managed by the runtime.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingDomain {
    /// Source code and AST retrieval.
    Code,
    /// Procedural `SKILL.md` retrieval.
    Skill,
}

/// Immutable model profile surfaced by embedders.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbeddingProfile {
    /// Retrieval domain.
    pub domain: EmbeddingDomain,
    /// Human-readable model identifier.
    pub model_name: String,
    /// Local model root or logical identifier.
    pub model_root: PathBuf,
    /// Optional tokenizer path.
    pub tokenizer_path: PathBuf,
    /// Output vector dimensionality.
    pub dimensions: usize,
    /// Maximum input length.
    pub max_length: usize,
    /// Batch size for inference.
    pub batch_size: usize,
    /// Device selection.
    pub device: String,
}

impl EmbeddingProfile {
    /// Create a stable cache key for the runtime registry.
    pub fn cache_key(&self) -> String {
        format!(
            "{:?}:{}:{}:{}:{}",
            self.domain,
            self.model_root.display(),
            self.tokenizer_path.display(),
            self.device,
            self.dimensions
        )
    }
}

/// Remote embedding fallback policy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FallbackPolicy {
    /// Never use a remote provider.
    Never,
    /// Use remote embeddings only when local model load fails.
    OnModelLoadFailure,
    /// Use remote embeddings only when local inference fails.
    OnInferenceFailure,
}

/// Optional remote fallback configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FallbackEmbeddingConfig {
    /// Fallback policy.
    pub policy: FallbackPolicy,
    /// Optional provider name.
    pub provider: Option<String>,
    /// Optional remote model identifier.
    pub model_name: Option<String>,
}

impl Default for FallbackEmbeddingConfig {
    fn default() -> Self {
        Self {
            policy: FallbackPolicy::Never,
            provider: None,
            model_name: None,
        }
    }
}

/// Code retrieval embedding configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodeEmbeddingConfig {
    /// Local model name.
    pub model_name: String,
    /// Root directory containing local artifacts.
    pub model_root: PathBuf,
    /// Tokenizer path.
    pub tokenizer_path: PathBuf,
    /// Embedding dimensions.
    pub dimensions: usize,
    /// Maximum sequence length.
    pub max_length: usize,
    /// Batch size.
    pub batch_size: usize,
    /// Device selection.
    pub device: String,
}

impl Default for CodeEmbeddingConfig {
    fn default() -> Self {
        let model_root = std::env::var("AXORA_CODE_EMBED_MODEL_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(".axora/models/jina-code-embeddings-v2"));
        Self {
            model_name: "jina-code-embeddings-v2".to_string(),
            tokenizer_path: model_root.join("tokenizer.json"),
            model_root,
            dimensions: 768,
            max_length: 8192,
            batch_size: 16,
            device: std::env::var("AXORA_CODE_EMBED_DEVICE").unwrap_or_else(|_| "cpu".to_string()),
        }
    }
}

impl CodeEmbeddingConfig {
    /// Convert to a shared profile.
    pub fn profile(&self) -> EmbeddingProfile {
        EmbeddingProfile {
            domain: EmbeddingDomain::Code,
            model_name: self.model_name.clone(),
            model_root: self.model_root.clone(),
            tokenizer_path: self.tokenizer_path.clone(),
            dimensions: self.dimensions,
            max_length: self.max_length,
            batch_size: self.batch_size,
            device: self.device.clone(),
        }
    }
}

/// Skill retrieval embedding configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillEmbeddingConfig {
    /// Local model name.
    pub model_name: String,
    /// Root directory containing local artifacts.
    pub model_root: PathBuf,
    /// Tokenizer path.
    pub tokenizer_path: PathBuf,
    /// Embedding dimensions.
    pub dimensions: usize,
    /// Maximum sequence length.
    pub max_length: usize,
    /// Batch size.
    pub batch_size: usize,
    /// Device selection.
    pub device: String,
}

impl Default for SkillEmbeddingConfig {
    fn default() -> Self {
        let model_root = std::env::var("AXORA_SKILL_EMBED_MODEL_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(".axora/models/bge-small-en-v1.5"));
        Self {
            model_name: "bge-small-en-v1.5".to_string(),
            tokenizer_path: model_root.join("tokenizer.json"),
            model_root,
            dimensions: 384,
            max_length: 512,
            batch_size: 32,
            device: std::env::var("AXORA_SKILL_EMBED_DEVICE").unwrap_or_else(|_| "cpu".to_string()),
        }
    }
}

impl SkillEmbeddingConfig {
    /// Convert to a shared profile.
    pub fn profile(&self) -> EmbeddingProfile {
        EmbeddingProfile {
            domain: EmbeddingDomain::Skill,
            model_name: self.model_name.clone(),
            model_root: self.model_root.clone(),
            tokenizer_path: self.tokenizer_path.clone(),
            dimensions: self.dimensions,
            max_length: self.max_length,
            batch_size: self.batch_size,
            device: self.device.clone(),
        }
    }
}

/// Full dual-embedding runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DualEmbeddingConfig {
    /// Code retrieval config.
    pub code: CodeEmbeddingConfig,
    /// Skill retrieval config.
    pub skill: SkillEmbeddingConfig,
    /// Optional remote fallback.
    pub fallback: FallbackEmbeddingConfig,
}
