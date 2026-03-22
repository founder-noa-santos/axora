//! Local research embeddings: `all-MiniLM-L6-v2` (384-d) via Candle [`BertModel`].
//!
//! Loads weights from a Hugging Face–style directory (`config.json`, `tokenizer.json`,
//! `model.safetensors`). No cloud APIs.

use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::Context;
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use sha2::{Digest, Sha256};
use tokenizers::{EncodeInput, Tokenizer};
use tracing::debug;

use crate::error::EmbeddingError;
use crate::Result;

/// Output dimension for `sentence-transformers/all-MiniLM-L6-v2`.
pub const RESEARCH_EMBED_DIM: usize = 384;

/// Byte length of a stored embedding BLOB (`f32` LE × 384).
pub const RESEARCH_EMBED_BYTES: usize = RESEARCH_EMBED_DIM * 4;

/// Hard cap on UTF-8 characters passed to the tokenizer to avoid pathological allocations (no cloud).
pub const MAX_EMBED_TEXT_CHARS: usize = 100_000;

/// Pluggable local embedding backend for research memory (Plan 9).
pub trait EmbeddingProvider: Send + Sync {
    /// Vector dimensionality (384 for MiniLM-L6-v2).
    fn dimensions(&self) -> usize {
        RESEARCH_EMBED_DIM
    }

    /// Canonical text for hashing and embedding (must match insert and query pipelines).
    fn canonicalize(&self, title: &str, url: &str, snippet: &str) -> String;

    /// Embed a single string into an L2-normalized vector.
    fn embed_text(&self, text: &str) -> anyhow::Result<Vec<f32>>;
}

/// Deterministic local embedder for tests: SHA-256–seeded 384-d vector, L2-normalized. **No disk, no network.**
#[derive(Debug, Clone, Default)]
pub struct DeterministicTestEmbeddingProvider {
    /// Passed through [`EmbeddingProvider::canonicalize`].
    pub max_canonical_chars: usize,
}

impl DeterministicTestEmbeddingProvider {
    /// New provider with default canonical length cap.
    pub fn new() -> Self {
        Self {
            max_canonical_chars: 4096,
        }
    }
}

impl EmbeddingProvider for DeterministicTestEmbeddingProvider {
    fn canonicalize(&self, title: &str, url: &str, snippet: &str) -> String {
        let raw = format!("{title}\n{url}\n{snippet}");
        truncate_chars(&raw, self.max_canonical_chars)
    }

    fn embed_text(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let text = truncate_chars(text, MAX_EMBED_TEXT_CHARS);
        let digest = Sha256::digest(text.as_bytes());
        let mut v = vec![0f32; RESEARCH_EMBED_DIM];
        for i in 0..RESEARCH_EMBED_DIM {
            let b = digest[i % digest.len()];
            v[i] = b as f32 / 255.0;
        }
        let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if n > 0.0 {
            for x in &mut v {
                *x /= n;
            }
        }
        debug_assert_eq!(v.len(), RESEARCH_EMBED_DIM);
        Ok(v)
    }
}

/// Configuration for loading [`ResearchMinilmEmbedder`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchMinilmConfig {
    /// Directory containing `config.json`, `tokenizer.json`, and `model.safetensors`.
    pub model_root: PathBuf,
    pub tokenizer_filename: String,
    pub config_filename: String,
    pub weights_filename: String,
    /// Max tokenizer length (MiniLM supports 512).
    pub max_length: usize,
    /// Max UTF-8 characters for [`EmbeddingProvider::canonicalize`] output.
    pub max_canonical_chars: usize,
    /// `cpu`, `cuda`, or `metal`.
    pub device: String,
}

impl Default for ResearchMinilmConfig {
    fn default() -> Self {
        let model_root = std::env::var("OPENAKTA_RESEARCH_EMBED_MODEL_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(".openakta/models/all-MiniLM-L6-v2"));
        Self {
            model_root,
            tokenizer_filename: "tokenizer.json".to_string(),
            config_filename: "config.json".to_string(),
            weights_filename: "model.safetensors".to_string(),
            max_length: 256,
            max_canonical_chars: 4096,
            device: std::env::var("OPENAKTA_RESEARCH_EMBED_DEVICE")
                .unwrap_or_else(|_| "cpu".to_string()),
        }
    }
}

/// Candle-backed MiniLM sentence embedder (mean pooling + L2 normalize).
pub struct ResearchMinilmEmbedder {
    tokenizer: Tokenizer,
    model: Mutex<BertModel>,
    device: Device,
    max_length: usize,
    max_canonical_chars: usize,
}

impl ResearchMinilmEmbedder {
    /// Load model and tokenizer from disk.
    pub fn new(config: ResearchMinilmConfig) -> Result<Self> {
        let tokenizer_path = config.model_root.join(&config.tokenizer_filename);
        let weights_path = config.model_root.join(&config.weights_filename);
        let model_config_path = config.model_root.join(&config.config_filename);

        let tokenizer = Tokenizer::from_file(&tokenizer_path).map_err(|e| {
            EmbeddingError::ModelLoad(format!("tokenizer {}: {e}", tokenizer_path.display()))
        })?;

        let bert_cfg: BertConfig = std::fs::read_to_string(&model_config_path)
            .map_err(|e| EmbeddingError::ModelLoad(e.to_string()))
            .and_then(|s| {
                serde_json::from_str(&s).map_err(|e| EmbeddingError::ModelLoad(e.to_string()))
            })?;

        if bert_cfg.hidden_size != RESEARCH_EMBED_DIM {
            return Err(EmbeddingError::DimensionMismatch {
                expected: RESEARCH_EMBED_DIM,
                actual: bert_cfg.hidden_size,
            }
            .into());
        }

        let device = load_device(&config.device)?;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[&weights_path], DType::F32, &device)
        }
        .map_err(|e| EmbeddingError::ModelLoad(e.to_string()))?;

        let model = BertModel::load(vb, &bert_cfg)
            .map_err(|e| EmbeddingError::ModelLoad(format!("BertModel::load: {e}")))?;

        Ok(Self {
            tokenizer,
            model: Mutex::new(model),
            device,
            max_length: config.max_length,
            max_canonical_chars: config.max_canonical_chars,
        })
    }

    fn embed_text_inner(&self, text: &str) -> Result<Vec<f32>> {
        let text = truncate_chars(text, MAX_EMBED_TEXT_CHARS);
        let mut encoding = self
            .tokenizer
            .encode(EncodeInput::Single(text.into()), true)
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?;
        encoding.truncate(
            self.max_length,
            0,
            tokenizers::TruncationDirection::Right,
        );

        let ids = encoding.get_ids().to_vec();
        let type_ids = encoding.get_type_ids().to_vec();
        let attn = encoding.get_attention_mask().to_vec();
        let seq_len = ids.len().max(1);

        let input_ids = Tensor::from_vec(ids, (1, seq_len), &self.device)
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?;
        let token_type_ids = Tensor::from_vec(type_ids, (1, seq_len), &self.device)
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?;
        let attention_mask = Tensor::from_vec(attn, (1, seq_len), &self.device)
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

        let hidden = {
            let model = self.model.lock().map_err(|_| {
                EmbeddingError::Inference("MiniLM model mutex poisoned".to_string())
            })?;
            model.forward(&input_ids, &token_type_ids, Some(&attention_mask))
        }
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

        let pooled = mean_pool(&hidden, &attention_mask)?;
        let normalized = l2_normalize_batch(&pooled)?;
        let vec = normalized
            .flatten_all()
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?
            .to_vec1::<f32>()
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

        if vec.len() != RESEARCH_EMBED_DIM {
            return Err(EmbeddingError::DimensionMismatch {
                expected: RESEARCH_EMBED_DIM,
                actual: vec.len(),
            }
            .into());
        }
        Ok(vec)
    }
}

impl EmbeddingProvider for ResearchMinilmEmbedder {
    fn canonicalize(&self, title: &str, url: &str, snippet: &str) -> String {
        let raw = format!("{title}\n{url}\n{snippet}");
        truncate_chars(&raw, self.max_canonical_chars)
    }

    fn embed_text(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        debug!(chars = text.len(), "research embed");
        self.embed_text_inner(text)
            .map_err(|e| anyhow::Error::from(e))
            .context("MiniLM embed_text")
    }
}

fn load_device(name: &str) -> Result<Device> {
    match name {
        "cuda" => Ok(Device::new_cuda(0).map_err(|e| EmbeddingError::ModelLoad(e.to_string()))?),
        "metal" => Ok(Device::new_metal(0).map_err(|e| EmbeddingError::ModelLoad(e.to_string()))?),
        _ => Ok(Device::Cpu),
    }
}

fn mean_pool(hidden: &Tensor, attention_mask: &Tensor) -> Result<Tensor> {
    let mask = attention_mask
        .to_dtype(DType::F32)
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?;
    let mask_exp = mask.unsqueeze(2).map_err(|e| EmbeddingError::Inference(e.to_string()))?;
    let weighted = hidden
        .broadcast_mul(&mask_exp)
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?;
    let summed = weighted
        .sum(1)
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?;
    let mask_sum = mask.sum(1).map_err(|e| EmbeddingError::Inference(e.to_string()))?;
    let eps = Tensor::ones_like(&mask_sum).map_err(|e| EmbeddingError::Inference(e.to_string()))?
        * 1e-9f64;
    let mask_sum = (mask_sum + eps).map_err(|e| EmbeddingError::Inference(e.to_string()))?;
    let denom = mask_sum.unsqueeze(1).map_err(|e| EmbeddingError::Inference(e.to_string()))?;
    Ok(summed
        .broadcast_div(&denom)
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?)
}

fn l2_normalize_batch(x: &Tensor) -> Result<Tensor> {
    let sq = x.sqr().map_err(|e| EmbeddingError::Inference(e.to_string()))?;
    let norm = sq
        .sum_keepdim(1)
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?
        .sqrt()
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?;
    Ok(x
        .broadcast_div(&norm)
        .map_err(|e| EmbeddingError::Inference(e.to_string()))?)
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    s.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_chars_respects_unicode() {
        let s = "a".repeat(100) + "β" + &"b".repeat(5000);
        let t = truncate_chars(&s, 102);
        assert_eq!(t.chars().count(), 102);
    }

    #[test]
    fn deterministic_provider_empty_string_is_normalized_384() {
        let p = DeterministicTestEmbeddingProvider::new();
        let v = p.embed_text("").expect("embed");
        assert_eq!(v.len(), RESEARCH_EMBED_DIM);
        let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((n - 1.0).abs() < 1e-5);
    }

    #[test]
    fn deterministic_provider_massive_input_truncated_no_panic() {
        let p = DeterministicTestEmbeddingProvider::new();
        let huge = "z".repeat(MAX_EMBED_TEXT_CHARS + 50_000);
        let v = p.embed_text(&huge).expect("embed");
        assert_eq!(v.len(), RESEARCH_EMBED_DIM);
        assert!(v.iter().map(|x| x * x).sum::<f32>().sqrt() > 0.99);
    }

    #[test]
    fn deterministic_provider_no_network_types_in_api() {
        let p = DeterministicTestEmbeddingProvider::new();
        let _ = p.embed_text("local only").unwrap();
    }

    /// Set `OPENAKTA_EMBEDDING_MODEL_TEST=1` and point `OPENAKTA_RESEARCH_EMBED_MODEL_ROOT` at a
    /// directory with `config.json`, `tokenizer.json`, and `model.safetensors` (e.g. HF
    /// `sentence-transformers/all-MiniLM-L6-v2`).
    #[test]
    fn minilm_optional_smoke() {
        if std::env::var("OPENAKTA_EMBEDDING_MODEL_TEST").ok().as_deref() != Some("1") {
            return;
        }
        let cfg = ResearchMinilmConfig::default();
        if !cfg.model_root.join(&cfg.weights_filename).exists() {
            return;
        }
        let embedder = ResearchMinilmEmbedder::new(cfg).expect("load MiniLM");
        let v = embedder.embed_text("hello world").expect("embed");
        assert_eq!(v.len(), RESEARCH_EMBED_DIM);
        let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((n - 1.0).abs() < 1e-3, "expected L2-normalized vector");
    }
}
