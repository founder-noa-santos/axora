use anyhow::{Context, Result};

use crate::auth::credentials::Credentials;
use crate::auth::login::{refresh_access_token, AuthConfig};

pub const REFRESH_WINDOW_SECONDS: i64 = 300;
pub const CLOCK_SKEW_SECONDS: i64 = 60;

pub async fn refresh_if_needed(
    config: &AuthConfig,
    credentials: &Credentials,
) -> Result<Option<Credentials>> {
    if !credentials.needs_refresh(REFRESH_WINDOW_SECONDS, CLOCK_SKEW_SECONDS) {
        return Ok(None);
    }

    let refresh_token = credentials
        .refresh_token
        .as_deref()
        .context("session has expired and no refresh token is available")?;
    let refreshed = refresh_access_token(config, refresh_token).await?;
    Ok(Some(refreshed))
}

pub async fn force_refresh(config: &AuthConfig, credentials: &Credentials) -> Result<Credentials> {
    let refresh_token = credentials
        .refresh_token
        .as_deref()
        .context("no refresh token is available")?;
    refresh_access_token(config, refresh_token).await
}
