//! Shared embedding runtime registry.

use crate::config::{EmbeddingDomain, EmbeddingProfile};
use crate::error::EmbeddingError;
use crate::Result;
use candle_core::Device;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

/// Stable cache key for a loaded embedding runtime.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelCacheKey {
    /// Retrieval domain.
    pub domain: EmbeddingDomain,
    /// Local model root.
    pub model_root: String,
    /// Tokenizer path.
    pub tokenizer_path: String,
    /// Device selection.
    pub device: String,
    /// Output dimensionality.
    pub dimensions: usize,
}

impl ModelCacheKey {
    /// Build from a profile.
    pub fn from_profile(profile: &EmbeddingProfile) -> Self {
        Self {
            domain: profile.domain,
            model_root: profile.model_root.display().to_string(),
            tokenizer_path: profile.tokenizer_path.display().to_string(),
            device: profile.device.clone(),
            dimensions: profile.dimensions,
        }
    }
}

/// Cached local embedding runtime.
#[derive(Debug)]
pub struct CachedEmbeddingRuntime {
    profile: EmbeddingProfile,
    device: Device,
    inference_lock: Mutex<()>,
}

impl CachedEmbeddingRuntime {
    /// Load a new runtime for the given profile.
    pub fn load(profile: EmbeddingProfile) -> Result<Self> {
        let device = match profile.device.as_str() {
            "cuda" => Device::new_cuda(0).map_err(|err| {
                EmbeddingError::ModelLoad(format!("failed to load CUDA device: {err}"))
            })?,
            "metal" => Device::new_metal(0).map_err(|err| {
                EmbeddingError::ModelLoad(format!("failed to load Metal device: {err}"))
            })?,
            _ => Device::Cpu,
        };

        Ok(Self {
            profile,
            device,
            inference_lock: Mutex::new(()),
        })
    }

    /// Access the loaded profile.
    pub fn profile(&self) -> &EmbeddingProfile {
        &self.profile
    }

    /// Access the selected device.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Execute a non-`Sync` inference block while keeping the runtime cached.
    pub fn with_inference_lock<T>(&self, f: impl FnOnce() -> Result<T>) -> Result<T> {
        let _guard = self.inference_lock.lock();
        f()
    }
}

static MODEL_CACHE: Lazy<Mutex<HashMap<ModelCacheKey, Arc<CachedEmbeddingRuntime>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Fetch or initialize a cached runtime for the given profile.
pub fn get_or_load_runtime(profile: &EmbeddingProfile) -> Result<Arc<CachedEmbeddingRuntime>> {
    let key = ModelCacheKey::from_profile(profile);
    let mut cache = MODEL_CACHE.lock();
    if let Some(runtime) = cache.get(&key) {
        return Ok(runtime.clone());
    }

    let runtime = Arc::new(CachedEmbeddingRuntime::load(profile.clone())?);
    cache.insert(key, runtime.clone());
    Ok(runtime)
}

/// Count cached runtimes. Intended for tests.
pub fn cache_size() -> usize {
    MODEL_CACHE.lock().len()
}
