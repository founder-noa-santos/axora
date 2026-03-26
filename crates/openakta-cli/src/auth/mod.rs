pub mod credentials;
pub mod login;
pub mod refresh;
pub mod storage;

use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::sync::{Mutex, RwLock};

use openakta_api_client::{ApiError, AuthProvider};

use crate::auth::credentials::{Credentials, WhoAmI};
use crate::auth::login::{login, AuthConfig, LoginOptions};
use crate::auth::refresh::{force_refresh, refresh_if_needed};
use crate::auth::storage::CredentialRepository;

#[derive(Clone)]
pub struct AuthManager {
    inner: Arc<AuthManagerInner>,
}

struct AuthManagerInner {
    config: Option<AuthConfig>,
    store: CredentialRepository,
    cache: RwLock<Option<Credentials>>,
    refresh_lock: Mutex<()>,
}

#[derive(Clone, Debug)]
pub struct AuthStatus {
    pub authenticated: bool,
    pub user_id: Option<String>,
    pub org_id: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl AuthManager {
    pub fn from_env() -> Result<Self> {
        let config = AuthConfig::from_env().ok();
        let store = CredentialRepository::new()?;
        Ok(Self {
            inner: Arc::new(AuthManagerInner {
                config,
                store,
                cache: RwLock::new(None),
                refresh_lock: Mutex::new(()),
            }),
        })
    }

    pub async fn login(&self, options: LoginOptions) -> Result<WhoAmI> {
        let config = self.oauth_config()?;
        let result = login(config, options).await?;
        self.set_credentials(result.credentials.clone()).await?;
        result.credentials.whoami()
    }

    pub async fn logout(&self) -> Result<()> {
        self.inner.store.clear()?;
        std::env::remove_var("OPENAKTA_JWT");
        *self.inner.cache.write().await = None;
        Ok(())
    }

    pub async fn status(&self) -> Result<AuthStatus> {
        let credentials = self.maybe_credentials().await?;
        Ok(match credentials {
            Some(credentials) => AuthStatus {
                authenticated: true,
                user_id: Some(credentials.user_id),
                org_id: credentials.org_id,
                expires_at: Some(credentials.expires_at),
            },
            None => AuthStatus {
                authenticated: false,
                user_id: None,
                org_id: None,
                expires_at: None,
            },
        })
    }

    pub async fn whoami(&self) -> Result<WhoAmI> {
        let credentials = self
            .ensure_credentials()
            .await
            .context("not logged in; run `openakta login` first")?
            .context("not logged in; run `openakta login` first")?;
        credentials.whoami()
    }

    pub async fn refresh(&self) -> Result<WhoAmI> {
        let credentials = self
            .ensure_credentials()
            .await
            .context("not logged in; run `openakta login` first")?
            .context("not logged in; run `openakta login` first")?;
        let config = self.oauth_config()?;
        let refreshed = force_refresh(config, &credentials).await?;
        self.set_credentials(refreshed.clone()).await?;
        refreshed.whoami()
    }

    pub async fn auth_provider(&self) -> Arc<dyn AuthProvider> {
        Arc::new(self.clone())
    }

    async fn maybe_credentials(&self) -> Result<Option<Credentials>> {
        if let Some(credentials) = self.inner.cache.read().await.clone() {
            return Ok(Some(credentials));
        }

        let loaded = self.inner.store.load()?;
        if let Some(credentials) = &loaded {
            std::env::set_var("OPENAKTA_JWT", &credentials.access_token);
        }
        *self.inner.cache.write().await = loaded.clone();
        Ok(loaded)
    }

    async fn ensure_credentials(&self) -> Result<Option<Credentials>> {
        let Some(credentials) = self.maybe_credentials().await? else {
            return Ok(None);
        };

        if credentials.needs_refresh(300, 60) {
            let config = self.oauth_config()?;
            if let Some(refreshed) = refresh_if_needed(config, &credentials).await? {
                self.set_credentials(refreshed.clone()).await?;
                return Ok(Some(refreshed));
            }
        }

        if credentials.is_expired(60) {
            let config = self.oauth_config()?;
            let refreshed = force_refresh(config, &credentials).await?;
            self.set_credentials(refreshed.clone()).await?;
            return Ok(Some(refreshed));
        }

        Ok(Some(credentials))
    }

    fn oauth_config(&self) -> Result<&AuthConfig> {
        self.inner.config.as_ref().context(
            "OPENAKTA_CLERK_ISSUER and OPENAKTA_CLERK_CLIENT_ID must be set for this operation",
        )
    }

    async fn set_credentials(&self, credentials: Credentials) -> Result<()> {
        self.inner.store.save(&credentials)?;
        std::env::set_var("OPENAKTA_JWT", &credentials.access_token);
        *self.inner.cache.write().await = Some(credentials);
        Ok(())
    }

    async fn refresh_after_unauthenticated(&self) -> Result<()> {
        let _guard = self.inner.refresh_lock.lock().await;
        let credentials = self
            .maybe_credentials()
            .await?
            .context("not logged in; run `openakta login` first")?;
        let config = self.oauth_config()?;
        let refreshed = force_refresh(config, &credentials).await?;
        self.set_credentials(refreshed).await
    }
}

#[async_trait]
impl AuthProvider for AuthManager {
    async fn bearer_token(&self) -> openakta_api_client::Result<Option<String>> {
        self.ensure_credentials()
            .await
            .map(|value| value.map(|credentials| credentials.access_token))
            .map_err(|error| ApiError::AuthRequired(error.to_string()))
    }

    async fn on_unauthenticated(&self) -> openakta_api_client::Result<()> {
        match self.refresh_after_unauthenticated().await {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = self.logout().await;
                Err(ApiError::RefreshFailed(error.to_string()))
            }
        }
    }
}
