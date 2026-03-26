use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Credentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub user_id: String,
    pub org_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AccessClaims {
    pub sub: String,
    pub exp: i64,
    #[serde(default)]
    pub org_id: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Clone, Debug)]
pub struct WhoAmI {
    pub user_id: String,
    pub org_id: Option<String>,
    pub email: Option<String>,
    pub expires_at: DateTime<Utc>,
}

impl Credentials {
    pub fn from_tokens(access_token: String, refresh_token: Option<String>) -> Result<Self> {
        let claims = decode_access_claims(&access_token)?;
        let expires_at = Utc
            .timestamp_opt(claims.exp, 0)
            .single()
            .ok_or_else(|| anyhow!("invalid access token expiration"))?;
        Ok(Self {
            access_token,
            refresh_token,
            expires_at,
            user_id: claims.sub,
            org_id: claims.org_id,
        })
    }

    pub fn is_expired(&self, skew_seconds: i64) -> bool {
        self.expires_at <= Utc::now() + chrono::Duration::seconds(skew_seconds)
    }

    pub fn needs_refresh(&self, refresh_window_seconds: i64, skew_seconds: i64) -> bool {
        self.expires_at
            <= Utc::now()
                + chrono::Duration::seconds(refresh_window_seconds)
                + chrono::Duration::seconds(skew_seconds)
    }

    pub fn whoami(&self) -> Result<WhoAmI> {
        let claims = decode_access_claims(&self.access_token)?;
        Ok(WhoAmI {
            user_id: self.user_id.clone(),
            org_id: self.org_id.clone(),
            email: claims.email,
            expires_at: self.expires_at,
        })
    }
}

pub fn decode_access_claims(token: &str) -> Result<AccessClaims> {
    let payload = decode_jwt_payload(token)?;
    serde_json::from_slice(&payload).context("failed to decode access token claims")
}

fn decode_jwt_payload(token: &str) -> Result<Vec<u8>> {
    let payload = token
        .split('.')
        .nth(1)
        .ok_or_else(|| anyhow!("access token is not a JWT"))?;
    URL_SAFE_NO_PAD
        .decode(payload)
        .context("failed to base64url decode JWT payload")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_jwt(sub: &str, org_id: Option<&str>, exp: i64) -> String {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","typ":"JWT"}"#);
        let payload_json = match org_id {
            Some(org) => format!(
                r#"{{"sub":"{sub}","org_id":"{org}","exp":{exp},"email":"person@example.com"}}"#
            ),
            None => format!(r#"{{"sub":"{sub}","exp":{exp},"email":"person@example.com"}}"#),
        };
        let payload = URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
        format!("{header}.{payload}.signature")
    }

    #[test]
    fn credentials_prefer_org_id_for_tenant() {
        let jwt = build_test_jwt("user_123", Some("org_456"), Utc::now().timestamp() + 3600);
        let creds = Credentials::from_tokens(jwt, Some("refresh".to_string())).unwrap();
        assert_eq!(creds.org_id.as_deref(), Some("org_456"));
    }

    #[test]
    fn credentials_detect_refresh_window() {
        let jwt = build_test_jwt("user_123", None, Utc::now().timestamp() + 120);
        let creds = Credentials::from_tokens(jwt, Some("refresh".to_string())).unwrap();
        assert!(creds.needs_refresh(300, 60));
    }
}
