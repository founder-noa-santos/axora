use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::timeout;
use url::Url;

use crate::auth::credentials::Credentials;

const DEFAULT_SCOPES: &str = "profile email offline_access";
const DEFAULT_REDIRECT_URI: &str = "http://127.0.0.1:8976/callback";
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone, Debug)]
pub struct AuthConfig {
    pub issuer: String,
    pub client_id: String,
    pub scopes: String,
    pub redirect_uri: String,
}

#[derive(Clone, Debug)]
pub struct LoginOptions {
    pub no_browser: bool,
}

#[derive(Clone, Debug)]
pub struct LoginResult {
    pub credentials: Credentials,
}

#[derive(Clone, Debug, Deserialize)]
struct AuthorizationServerMetadata {
    authorization_endpoint: String,
    token_endpoint: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
}

#[derive(Debug)]
struct LoopbackCallback {
    code: String,
    state: String,
}

#[derive(Debug)]
struct PkceState {
    code_verifier: String,
    code_challenge: String,
    state: String,
}

impl AuthConfig {
    pub fn from_env() -> Result<Self> {
        let issuer = std::env::var("OPENAKTA_CLERK_ISSUER")
            .context("OPENAKTA_CLERK_ISSUER must be set for CLI login")?;
        let client_id = std::env::var("OPENAKTA_CLERK_CLIENT_ID")
            .context("OPENAKTA_CLERK_CLIENT_ID must be set for CLI login")?;
        let scopes =
            std::env::var("OPENAKTA_CLERK_SCOPES").unwrap_or_else(|_| DEFAULT_SCOPES.to_string());
        let redirect_uri = std::env::var("OPENAKTA_CLERK_REDIRECT_URI")
            .unwrap_or_else(|_| DEFAULT_REDIRECT_URI.to_string());
        Ok(Self {
            issuer,
            client_id,
            scopes,
            redirect_uri,
        })
    }

    async fn discover(&self, http: &Client) -> Result<AuthorizationServerMetadata> {
        let base = self.issuer.trim_end_matches('/');
        let url = if base.ends_with("/.well-known/oauth-authorization-server") {
            base.to_string()
        } else {
            format!("{base}/.well-known/oauth-authorization-server")
        };

        http.get(url)
            .send()
            .await
            .context("failed to fetch Clerk OAuth metadata")?
            .error_for_status()
            .context("Clerk OAuth metadata request failed")?
            .json()
            .await
            .context("failed to parse Clerk OAuth metadata")
    }
}

pub async fn login(config: &AuthConfig, options: LoginOptions) -> Result<LoginResult> {
    let http = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("failed to build HTTP client")?;
    let metadata = config.discover(&http).await?;
    let listener = bind_loopback_listener(&config.redirect_uri).await?;
    let pkce = PkceState::new();
    let authorize_url = build_authorize_url(config, &metadata, &config.redirect_uri, &pkce)?;

    if options.no_browser {
        println!("Open this URL in your browser:");
        println!("{authorize_url}");
    } else {
        println!("Opening browser for Clerk login...");
        println!("If nothing opens, use this URL manually:");
        println!("{authorize_url}");
        if let Err(error) = webbrowser::open(authorize_url.as_str()) {
            tracing::warn!(error = %error, "failed to open system browser automatically");
        }
    }
    println!("Waiting for OAuth callback on {}...", config.redirect_uri);

    let callback = timeout(CALLBACK_TIMEOUT, wait_for_callback(listener))
        .await
        .context("timed out waiting for login callback")??;

    if callback.state != pkce.state {
        bail!("received invalid OAuth state");
    }

    let token_response = exchange_code_for_tokens(
        &http,
        &metadata,
        &config.client_id,
        &config.redirect_uri,
        &pkce.code_verifier,
        &callback.code,
    )
    .await?;

    let credentials =
        Credentials::from_tokens(token_response.access_token, token_response.refresh_token)?;
    Ok(LoginResult { credentials })
}

async fn bind_loopback_listener(redirect_uri: &str) -> Result<TcpListener> {
    let url = Url::parse(redirect_uri).context("invalid OPENAKTA_CLERK_REDIRECT_URI")?;
    if url.scheme() != "http" {
        bail!("OPENAKTA_CLERK_REDIRECT_URI must use http for loopback callbacks");
    }

    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("OPENAKTA_CLERK_REDIRECT_URI is missing a host"))?;
    if host != "127.0.0.1" && host != "localhost" {
        bail!("OPENAKTA_CLERK_REDIRECT_URI host must be 127.0.0.1 or localhost");
    }

    let port = url
        .port_or_known_default()
        .ok_or_else(|| anyhow!("OPENAKTA_CLERK_REDIRECT_URI is missing a port"))?;
    let bind_addr = format!("{host}:{port}");
    TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind OAuth callback listener at {bind_addr}"))
}

pub async fn refresh_access_token(config: &AuthConfig, refresh_token: &str) -> Result<Credentials> {
    let http = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("failed to build HTTP client")?;
    let metadata = config.discover(&http).await?;

    let response = http
        .post(&metadata.token_endpoint)
        .form(&HashMap::from([
            ("grant_type", "refresh_token"),
            ("client_id", config.client_id.as_str()),
            ("refresh_token", refresh_token),
        ]))
        .send()
        .await
        .context("failed to refresh Clerk access token")?;
    let response = response
        .error_for_status()
        .context("Clerk token refresh failed")?;
    let token_response: TokenResponse = response
        .json()
        .await
        .context("failed to parse Clerk refresh response")?;
    let fallback_refresh = token_response
        .refresh_token
        .or_else(|| Some(refresh_token.to_string()));
    Credentials::from_tokens(token_response.access_token, fallback_refresh)
}

fn build_authorize_url(
    config: &AuthConfig,
    metadata: &AuthorizationServerMetadata,
    redirect_uri: &str,
    pkce: &PkceState,
) -> Result<Url> {
    let mut url = Url::parse(&metadata.authorization_endpoint)
        .context("invalid Clerk authorization endpoint")?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &config.client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", &config.scopes)
        .append_pair("state", &pkce.state)
        .append_pair("code_challenge", &pkce.code_challenge)
        .append_pair("code_challenge_method", "S256");
    Ok(url)
}

async fn exchange_code_for_tokens(
    http: &Client,
    metadata: &AuthorizationServerMetadata,
    client_id: &str,
    redirect_uri: &str,
    code_verifier: &str,
    code: &str,
) -> Result<TokenResponse> {
    let response = http
        .post(&metadata.token_endpoint)
        .form(&HashMap::from([
            ("grant_type", "authorization_code"),
            ("client_id", client_id),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", code_verifier),
        ]))
        .send()
        .await
        .context("failed to exchange authorization code for tokens")?;
    let response = response
        .error_for_status()
        .context("Clerk token exchange failed")?;
    response
        .json()
        .await
        .context("failed to parse Clerk token exchange response")
}

async fn wait_for_callback(listener: TcpListener) -> Result<LoopbackCallback> {
    let (mut socket, _) = listener
        .accept()
        .await
        .context("failed to accept login callback")?;
    let mut buffer = vec![0u8; 4096];
    let bytes = socket
        .read(&mut buffer)
        .await
        .context("failed to read login callback")?;
    let request = String::from_utf8_lossy(&buffer[..bytes]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| anyhow!("invalid callback request"))?;
    let url =
        Url::parse(&format!("http://localhost{path}")).context("failed to parse callback URL")?;
    let params: HashMap<_, _> = url.query_pairs().into_owned().collect();
    let body = "Openakta login completed. You can close this tab.";
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/plain; charset=utf-8\r\ncontent-length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    socket
        .write_all(response.as_bytes())
        .await
        .context("failed to write callback response")?;

    if let Some(error) = params.get("error") {
        bail!("Clerk login failed: {error}");
    }

    Ok(LoopbackCallback {
        code: params
            .get("code")
            .cloned()
            .ok_or_else(|| anyhow!("missing authorization code in callback"))?,
        state: params
            .get("state")
            .cloned()
            .ok_or_else(|| anyhow!("missing state in callback"))?,
    })
}

impl PkceState {
    fn new() -> Self {
        let code_verifier = random_urlsafe(64);
        let code_challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()));
        let state = random_urlsafe(32);
        Self {
            code_verifier,
            code_challenge,
            state,
        }
    }
}

fn random_urlsafe(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
