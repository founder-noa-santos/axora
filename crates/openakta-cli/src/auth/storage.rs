use std::fs::OpenOptions;
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{anyhow, bail, Context, Result};
use argon2::Argon2;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Utc;
use directories::BaseDirs;
use keyring::Entry;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::auth::credentials::Credentials;

const KEYRING_SERVICE: &str = "openakta-cli";
const KEYRING_ACCOUNT: &str = "default";
const PASSPHRASE_ENV: &str = "OPENAKTA_AUTH_PASSPHRASE";

#[derive(Debug)]
pub struct CredentialRepository {
    credentials_path: PathBuf,
    backup_path: PathBuf,
    keyring_entry: Entry,
    passphrase_override: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct EncryptedCredentialsFile {
    version: u8,
    salt_b64: String,
    nonce_b64: String,
    ciphertext_b64: String,
    created_at: chrono::DateTime<Utc>,
}

impl CredentialRepository {
    pub fn new() -> Result<Self> {
        let credentials_path = default_credentials_path()?;
        Self::for_path(credentials_path)
    }

    pub fn for_path(credentials_path: PathBuf) -> Result<Self> {
        Ok(Self {
            backup_path: credentials_path.with_extension("bak"),
            keyring_entry: Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
                .context("failed to initialize keyring entry")?,
            credentials_path,
            passphrase_override: None,
        })
    }

    #[cfg(test)]
    pub fn for_path_with_passphrase(
        credentials_path: PathBuf,
        passphrase: impl Into<String>,
    ) -> Result<Self> {
        let mut repo = Self::for_path(credentials_path)?;
        repo.passphrase_override = Some(passphrase.into());
        Ok(repo)
    }

    pub fn load(&self) -> Result<Option<Credentials>> {
        match self.load_from_keyring() {
            Ok(Some(value)) => return Ok(Some(value)),
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(error = %error, "keyring unavailable, falling back to encrypted file")
            }
        }

        self.load_from_file()
    }

    pub fn save(&self, credentials: &Credentials) -> Result<()> {
        match self.save_to_keyring(credentials) {
            Ok(()) => {
                let _ = std::fs::remove_file(&self.credentials_path);
                let _ = std::fs::remove_file(&self.backup_path);
                return Ok(());
            }
            Err(error) => {
                tracing::warn!(error = %error, "failed to save credentials to keyring, falling back to encrypted file")
            }
        }

        self.save_to_file(credentials)
    }

    pub fn clear(&self) -> Result<()> {
        match self.keyring_entry.delete_credential() {
            Ok(()) => {}
            Err(error) => {
                let message = error.to_string();
                if !message.contains("No matching entry found") {
                    tracing::warn!(error = %message, "failed to delete keyring credential");
                }
            }
        }

        remove_file_if_exists(&self.credentials_path)?;
        remove_file_if_exists(&self.backup_path)?;
        Ok(())
    }

    fn load_from_keyring(&self) -> Result<Option<Credentials>> {
        match self.keyring_entry.get_password() {
            Ok(serialized) => serde_json::from_str(&serialized)
                .context("failed to deserialize keyring credentials")
                .map(Some),
            Err(error) => {
                let message = error.to_string();
                if message.contains("No matching entry found")
                    || message.contains("Platform secure storage failure: no entry found")
                {
                    Ok(None)
                } else {
                    Err(anyhow!(message))
                }
            }
        }
    }

    fn save_to_keyring(&self, credentials: &Credentials) -> Result<()> {
        let serialized =
            serde_json::to_string(credentials).context("failed to serialize credentials")?;
        self.keyring_entry
            .set_password(&serialized)
            .context("failed to persist credentials to keyring")
    }

    fn load_from_file(&self) -> Result<Option<Credentials>> {
        if !self.credentials_path.exists() {
            return Ok(None);
        }

        match self.load_from_file_path(&self.credentials_path) {
            Ok(credentials) => Ok(Some(credentials)),
            Err(error) => {
                tracing::warn!(error = %error, "credential file is corrupted, attempting recovery");
                self.quarantine_corrupt_file(&self.credentials_path)?;
                if self.backup_path.exists() {
                    match self.load_from_file_path(&self.backup_path) {
                        Ok(credentials) => {
                            self.save_to_file(&credentials)?;
                            Ok(Some(credentials))
                        }
                        Err(backup_error) => {
                            self.quarantine_corrupt_file(&self.backup_path)?;
                            Err(backup_error)
                        }
                    }
                } else {
                    Ok(None)
                }
            }
        }
    }

    fn load_from_file_path(&self, path: &Path) -> Result<Credentials> {
        let mut contents = String::new();
        std::fs::File::open(path)
            .with_context(|| format!("failed to open {}", path.display()))?
            .read_to_string(&mut contents)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let encrypted: EncryptedCredentialsFile =
            serde_json::from_str(&contents).context("failed to parse encrypted credential file")?;
        self.decrypt_file(encrypted)
    }

    fn save_to_file(&self, credentials: &Credentials) -> Result<()> {
        let encrypted = self.encrypt_credentials(credentials)?;
        let serialized = serde_json::to_vec_pretty(&encrypted)
            .context("failed to serialize encrypted credentials")?;

        if let Some(parent) = self.credentials_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        if self.credentials_path.exists() {
            std::fs::copy(&self.credentials_path, &self.backup_path).with_context(|| {
                format!("failed to back up {}", self.credentials_path.display())
            })?;
        }

        let tmp_path = self.credentials_path.with_extension("tmp");
        let mut options = OpenOptions::new();
        options.create(true).truncate(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&tmp_path)
            .with_context(|| format!("failed to open {}", tmp_path.display()))?;
        file.write_all(&serialized)
            .with_context(|| format!("failed to write {}", tmp_path.display()))?;
        file.sync_all()
            .with_context(|| format!("failed to sync {}", tmp_path.display()))?;
        std::fs::rename(&tmp_path, &self.credentials_path)
            .with_context(|| format!("failed to replace {}", self.credentials_path.display()))?;
        Ok(())
    }

    fn encrypt_credentials(&self, credentials: &Credentials) -> Result<EncryptedCredentialsFile> {
        let passphrase = self.passphrase()?;
        let serialized =
            serde_json::to_vec(credentials).context("failed to serialize credentials")?;
        let (salt, key) = derive_key(passphrase.as_bytes())?;
        let cipher = Aes256Gcm::new_from_slice(&key).context("failed to initialize cipher")?;
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), serialized.as_ref())
            .map_err(|error| anyhow!("failed to encrypt credentials: {error}"))?;

        Ok(EncryptedCredentialsFile {
            version: 1,
            salt_b64: STANDARD.encode(salt),
            nonce_b64: STANDARD.encode(nonce),
            ciphertext_b64: STANDARD.encode(ciphertext),
            created_at: Utc::now(),
        })
    }

    fn decrypt_file(&self, encrypted: EncryptedCredentialsFile) -> Result<Credentials> {
        if encrypted.version != 1 {
            bail!("unsupported credential file version {}", encrypted.version);
        }

        let passphrase = self.passphrase()?;
        let salt = STANDARD
            .decode(encrypted.salt_b64)
            .context("failed to decode file salt")?;
        let nonce = STANDARD
            .decode(encrypted.nonce_b64)
            .context("failed to decode file nonce")?;
        let ciphertext = STANDARD
            .decode(encrypted.ciphertext_b64)
            .context("failed to decode ciphertext")?;
        let key = derive_key_with_salt(passphrase.as_bytes(), &salt)?;
        let cipher = Aes256Gcm::new_from_slice(&key).context("failed to initialize cipher")?;
        let plaintext = cipher
            .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
            .map_err(|error| anyhow!("failed to decrypt credential file: {error}"))?;
        serde_json::from_slice(&plaintext).context("failed to deserialize decrypted credentials")
    }

    fn quarantine_corrupt_file(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }
        let quarantined =
            path.with_extension(format!("corrupt.{}", Utc::now().format("%Y%m%d%H%M%S")));
        std::fs::rename(path, quarantined).context("failed to quarantine corrupted credential file")
    }

    fn passphrase(&self) -> Result<Zeroizing<String>> {
        if let Some(passphrase) = &self.passphrase_override {
            return Ok(Zeroizing::new(passphrase.clone()));
        }

        if let Ok(passphrase) = std::env::var(PASSPHRASE_ENV) {
            if !passphrase.is_empty() {
                return Ok(Zeroizing::new(passphrase));
            }
        }

        if std::io::stdin().is_terminal() {
            let prompt = format!(
                "Keyring unavailable. Enter passphrase for {}: ",
                self.credentials_path.display()
            );
            let passphrase =
                rpassword::prompt_password(prompt).context("failed to read auth passphrase")?;
            if passphrase.is_empty() {
                bail!("empty passphrase is not allowed");
            }
            return Ok(Zeroizing::new(passphrase));
        }

        bail!(
            "keyring is unavailable and {} is not set; cannot access encrypted credentials",
            PASSPHRASE_ENV
        )
    }
}

fn derive_key(passphrase: &[u8]) -> Result<([u8; 16], [u8; 32])> {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    let key = derive_key_with_salt(passphrase, &salt)?;
    Ok((salt, key))
}

fn derive_key_with_salt(passphrase: &[u8], salt: &[u8]) -> Result<[u8; 32]> {
    let mut output = [0u8; 32];
    Argon2::default()
        .hash_password_into(passphrase, salt, &mut output)
        .map_err(|error| anyhow!("failed to derive encryption key: {error}"))?;
    Ok(output)
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("failed to delete {}", path.display())),
    }
}

fn default_credentials_path() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| anyhow!("APPDATA is not set"))?;
        return Ok(appdata.join(".openakta").join("credentials"));
    }

    #[cfg(not(target_os = "windows"))]
    {
        let base_dirs =
            BaseDirs::new().ok_or_else(|| anyhow!("home directory is not available"))?;
        Ok(base_dirs.home_dir().join(".openakta").join("credentials"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_credentials() -> Credentials {
        Credentials {
            access_token: "access".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: Utc::now(),
            user_id: "user_123".to_string(),
            org_id: Some("org_456".to_string()),
        }
    }

    #[test]
    fn encrypted_file_roundtrip() {
        let temp = tempfile::tempdir().unwrap();
        let repo = CredentialRepository::for_path_with_passphrase(
            temp.path().join("credentials"),
            "correct horse battery staple",
        )
        .unwrap();
        let creds = test_credentials();
        repo.save_to_file(&creds).unwrap();
        let loaded = repo.load_from_file().unwrap().unwrap();
        assert_eq!(loaded.user_id, creds.user_id);
        assert_eq!(loaded.org_id, creds.org_id);
    }

    #[test]
    fn corrupted_file_is_quarantined() {
        let temp = tempfile::tempdir().unwrap();
        let repo = CredentialRepository::for_path_with_passphrase(
            temp.path().join("credentials"),
            "correct horse battery staple",
        )
        .unwrap();
        std::fs::write(temp.path().join("credentials"), b"not-json").unwrap();
        let loaded = repo.load_from_file().unwrap();
        assert!(loaded.is_none());
    }
}
