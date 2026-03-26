//! Local cross-encoder reranking over query-document pairs.

use crate::error::RagError;
use crate::OpenaktaRagError;
use crate::Result;
use candle_core::{DType, Device, IndexOp, Tensor};
use candle_nn::{linear, Linear, Module, VarBuilder};
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokenizers::{EncodeInput, Encoding, Tokenizer};

/// Minimal document representation for reranking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RerankDocument {
    /// Stable identifier.
    pub id: String,
    /// Display title.
    pub title: String,
    /// Compact summary.
    pub summary: String,
    /// Markdown body.
    pub body_markdown: String,
}

/// Runtime configuration for the local cross-encoder.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossEncoderConfig {
    /// Directory containing `config.json`, `tokenizer.json`, and weights.
    pub model_root: PathBuf,
    /// Tokenizer filename within `model_root`.
    pub tokenizer_filename: String,
    /// Model config filename within `model_root`.
    pub config_filename: String,
    /// Safetensors checkpoint filename within `model_root`.
    pub weights_filename: String,
    /// Maximum sequence length for pair tokenization.
    pub max_length: usize,
    /// Inference batch size.
    pub batch_size: usize,
    /// Device selection (`cpu`, `cuda`, `metal`).
    pub device: String,
}

impl Default for CrossEncoderConfig {
    fn default() -> Self {
        // Prefer explicit env; otherwise resolve relative to cwd (CLI may override via
        // `OpenaktaReranker::for_workspace` with the real workspace root).
        let model_root = std::env::var("OPENAKTA_CROSS_ENCODER_MODEL_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(".openakta/models/cross-encoder"));
        Self {
            model_root,
            tokenizer_filename: "tokenizer.json".to_string(),
            config_filename: "config.json".to_string(),
            weights_filename: "model.safetensors".to_string(),
            max_length: 256,
            batch_size: 8,
            device: std::env::var("OPENAKTA_CROSS_ENCODER_DEVICE")
                .unwrap_or_else(|_| "cpu".to_string()),
        }
    }
}

/// Query-document reranker contract.
#[async_trait::async_trait]
pub trait CrossEncoderScorer: Send + Sync {
    /// Score every query-document pair in order.
    async fn score_pairs(&self, query: &str, docs: &[RerankDocument]) -> Result<Vec<f32>>;
}

/// Local Candle cross-encoder backed by a cached BERT classifier.
pub struct CandleCrossEncoder {
    runtime: Arc<CachedCrossEncoder>,
    config: CrossEncoderConfig,
}

impl CandleCrossEncoder {
    /// Create a new reranker with the default on-disk model location.
    pub fn new() -> Result<Self> {
        Self::with_config(CrossEncoderConfig::default())
    }

    /// Create a new reranker with an explicit model config.
    pub fn with_config(config: CrossEncoderConfig) -> Result<Self> {
        let cache_key = config.model_root.to_string_lossy().to_string();
        let runtime = {
            let mut cache = MODEL_CACHE.lock();
            if let Some(runtime) = cache.get(&cache_key) {
                runtime.clone()
            } else {
                let runtime = Arc::new(CachedCrossEncoder::load(&config)?);
                cache.insert(cache_key, runtime.clone());
                runtime
            }
        };
        Ok(Self { runtime, config })
    }
}

#[async_trait::async_trait]
impl CrossEncoderScorer for CandleCrossEncoder {
    async fn score_pairs(&self, query: &str, docs: &[RerankDocument]) -> Result<Vec<f32>> {
        if docs.is_empty() {
            return Ok(Vec::new());
        }

        let mut scores = Vec::with_capacity(docs.len());
        for batch in docs.chunks(self.config.batch_size.max(1)) {
            let encodings = batch
                .iter()
                .map(|doc| {
                    self.runtime
                        .tokenizer
                        .encode(
                            EncodeInput::Dual(
                                query.to_string().into(),
                                compact_document_text(doc).into(),
                            ),
                            true,
                        )
                        .map_err(|err| RagError::Rerank(err.to_string()))
                        .map(|mut encoding| {
                            encoding.truncate(
                                self.config.max_length,
                                0,
                                tokenizers::TruncationDirection::Right,
                            );
                            encoding
                        })
                })
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(OpenaktaRagError::from)?;

            let batch_scores = self.runtime.score_batch(&encodings)?;
            scores.extend(batch_scores);
        }
        Ok(scores)
    }
}

impl Default for CandleCrossEncoder {
    fn default() -> Self {
        Self::new().expect("cross-encoder must initialize")
    }
}

/// Neutral scores when no cross-encoder checkpoint is available (keeps retrieval usable).
#[derive(Debug, Clone, Copy, Default)]
pub struct HeuristicCrossEncoder;

#[async_trait::async_trait]
impl CrossEncoderScorer for HeuristicCrossEncoder {
    async fn score_pairs(&self, _query: &str, docs: &[RerankDocument]) -> Result<Vec<f32>> {
        Ok(vec![1.0f32; docs.len()])
    }
}

/// Local Candle cross-encoder when weights exist; otherwise [`HeuristicCrossEncoder`].
pub enum OpenaktaReranker {
    /// BERT cross-encoder (`tokenizer.json`, `config.json`, `model.safetensors`).
    Candle(CandleCrossEncoder),
    /// Fallback: uniform scores so MemGAS + knapsack still run without model files.
    Heuristic(HeuristicCrossEncoder),
}

impl OpenaktaReranker {
    /// Resolve `OPENAKTA_CROSS_ENCODER_MODEL_ROOT` or `<workspace>/.openakta/models/cross-encoder`.
    ///
    /// If loading fails (missing files), logs a warning and uses heuristic scoring.
    pub fn for_workspace(workspace_root: &Path) -> Self {
        let mut cfg = CrossEncoderConfig::default();
        if std::env::var("OPENAKTA_CROSS_ENCODER_MODEL_ROOT").is_err() {
            cfg.model_root = workspace_root.join(".openakta/models/cross-encoder");
        }
        let model_root = cfg.model_root.clone();
        match CandleCrossEncoder::with_config(cfg) {
            Ok(c) => Self::Candle(c),
            Err(err) => {
                tracing::warn!(
                    target: "openakta_rag",
                    model_root = %model_root.display(),
                    error = %err,
                    "cross-encoder checkpoint not loaded; using neutral rerank scores (install tokenizer.json, config.json, model.safetensors or set OPENAKTA_CROSS_ENCODER_MODEL_ROOT)"
                );
                Self::Heuristic(HeuristicCrossEncoder)
            }
        }
    }
}

#[async_trait::async_trait]
impl CrossEncoderScorer for OpenaktaReranker {
    async fn score_pairs(&self, query: &str, docs: &[RerankDocument]) -> Result<Vec<f32>> {
        match self {
            Self::Candle(c) => c.score_pairs(query, docs).await,
            Self::Heuristic(h) => h.score_pairs(query, docs).await,
        }
    }
}

static MODEL_CACHE: Lazy<Mutex<HashMap<String, Arc<CachedCrossEncoder>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

struct CachedCrossEncoder {
    tokenizer: Tokenizer,
    model: Mutex<BertSequenceClassifier>,
}

impl CachedCrossEncoder {
    fn load(config: &CrossEncoderConfig) -> Result<Self> {
        let tokenizer_path = config.model_root.join(&config.tokenizer_filename);
        let weights_path = config.model_root.join(&config.weights_filename);
        let model_config_path = config.model_root.join(&config.config_filename);

        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|err| RagError::Rerank(err.to_string()))?;
        let model =
            BertSequenceClassifier::load(&model_config_path, &weights_path, &config.device)?;

        Ok(Self {
            tokenizer,
            model: Mutex::new(model),
        })
    }

    fn score_batch(&self, encodings: &[Encoding]) -> Result<Vec<f32>> {
        self.model.lock().score(encodings)
    }
}

struct BertSequenceClassifier {
    model: BertModel,
    classifier: Linear,
    device: Device,
}

impl BertSequenceClassifier {
    fn load(config_path: &Path, weights_path: &Path, device_name: &str) -> Result<Self> {
        let config = std::fs::read_to_string(config_path)
            .map_err(|err| RagError::Rerank(err.to_string()))
            .and_then(|content| {
                serde_json::from_str::<BertConfig>(&content)
                    .map_err(|err| RagError::Rerank(err.to_string()))
            })?;
        let device = load_device(device_name)?;
        let vb =
            unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &device) }
                .map_err(|err| RagError::Rerank(err.to_string()))?;

        let model = BertModel::load(vb.pp("bert"), &config)
            .or_else(|_| BertModel::load(vb.clone(), &config))
            .map_err(|err| RagError::Rerank(err.to_string()))?;
        let classifier = linear(config.hidden_size, 1, vb.pp("classifier"))
            .or_else(|_| linear(config.hidden_size, 1, vb.pp("score")))
            .map_err(|err| RagError::Rerank(err.to_string()))?;

        Ok(Self {
            model,
            classifier,
            device,
        })
    }

    fn score(&self, encodings: &[Encoding]) -> Result<Vec<f32>> {
        let max_len = encodings
            .iter()
            .map(|encoding| encoding.len())
            .max()
            .unwrap_or(1);
        let batch = encodings.len();

        let mut input_ids = Vec::with_capacity(batch * max_len);
        let mut token_type_ids = Vec::with_capacity(batch * max_len);
        let mut attention_mask = Vec::with_capacity(batch * max_len);
        for encoding in encodings {
            extend_with_padding(&mut input_ids, encoding.get_ids(), max_len);
            extend_with_padding(&mut token_type_ids, encoding.get_type_ids(), max_len);
            extend_with_padding(&mut attention_mask, encoding.get_attention_mask(), max_len);
        }

        let input_ids = Tensor::from_vec(input_ids, (batch, max_len), &self.device)
            .map_err(|err| RagError::Rerank(err.to_string()))?;
        let token_type_ids = Tensor::from_vec(token_type_ids, (batch, max_len), &self.device)
            .map_err(|err| RagError::Rerank(err.to_string()))?;
        let attention_mask = Tensor::from_vec(attention_mask, (batch, max_len), &self.device)
            .map_err(|err| RagError::Rerank(err.to_string()))?;

        let hidden_states = self
            .model
            .forward(&input_ids, &token_type_ids, Some(&attention_mask))
            .map_err(|err| RagError::Rerank(err.to_string()))?;
        let cls = hidden_states
            .i((.., 0, ..))
            .map_err(|err| RagError::Rerank(err.to_string()))?;
        let logits = self
            .classifier
            .forward(&cls)
            .map_err(|err| RagError::Rerank(err.to_string()))?;
        let logits = logits
            .reshape((batch,))
            .map_err(|err| RagError::Rerank(err.to_string()))?;

        Ok(logits
            .to_vec1::<f32>()
            .map_err(|err| RagError::Rerank(err.to_string()))?)
    }
}

fn load_device(device_name: &str) -> Result<Device> {
    match device_name {
        "cuda" => Ok(Device::new_cuda(0).map_err(|err| RagError::Rerank(err.to_string()))?),
        "metal" => Ok(Device::new_metal(0).map_err(|err| RagError::Rerank(err.to_string()))?),
        _ => Ok(Device::Cpu),
    }
}

fn compact_document_text(doc: &RerankDocument) -> String {
    let mut body = doc.body_markdown.replace('\n', " ");
    if body.len() > 768 {
        body.truncate(768);
    }
    format!("{} [SEP] {} [SEP] {}", doc.title, doc.summary, body)
}

fn extend_with_padding(target: &mut Vec<u32>, values: &[u32], max_len: usize) {
    target.extend_from_slice(values);
    if values.len() < max_len {
        target.resize(target.len() + (max_len - values.len()), 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_document_text_truncates_body() {
        let doc = RerankDocument {
            id: "skill-1".to_string(),
            title: "Debug auth".to_string(),
            summary: "Inspect token flow".to_string(),
            body_markdown: "a".repeat(1024),
        };

        let compact = compact_document_text(&doc);
        assert!(compact.contains("Debug auth"));
        assert!(compact.len() < 900);
    }

    #[test]
    fn padding_extends_to_batch_width() {
        let mut values = Vec::new();
        extend_with_padding(&mut values, &[1, 2, 3], 5);
        assert_eq!(values, vec![1, 2, 3, 0, 0]);
    }
}
