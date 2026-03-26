use async_trait::async_trait;
use std::sync::Arc;

use crate::error::{ApiError, Result};

#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn bearer_token(&self) -> Result<Option<String>>;

    async fn on_unauthenticated(&self) -> Result<()>;
}

#[derive(Debug)]
pub struct StaticTokenAuthProvider {
    token: String,
}

impl StaticTokenAuthProvider {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }
}

#[async_trait]
impl AuthProvider for StaticTokenAuthProvider {
    async fn bearer_token(&self) -> Result<Option<String>> {
        Ok(Some(self.token.clone()))
    }

    async fn on_unauthenticated(&self) -> Result<()> {
        Err(ApiError::AuthRequired(
            "stored bearer token was rejected; run `openakta login`".to_string(),
        ))
    }
}

#[derive(Debug, Default)]
pub struct EnvAuthProvider;

#[async_trait]
impl AuthProvider for EnvAuthProvider {
    async fn bearer_token(&self) -> Result<Option<String>> {
        Ok(std::env::var("OPENAKTA_JWT").ok())
    }

    async fn on_unauthenticated(&self) -> Result<()> {
        Err(ApiError::AuthRequired(
            "environment bearer token was rejected; run `openakta login`".to_string(),
        ))
    }
}

pub fn static_provider(token: Option<String>) -> Option<Arc<dyn AuthProvider>> {
    token.map(|value| Arc::new(StaticTokenAuthProvider::new(value)) as Arc<dyn AuthProvider>)
}
