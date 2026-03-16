---
rule_id: "DATA-002"
title: "PII Protection"
category: "Compliance"
severity: "Critical"
applies_to:
  - "src/models/user.rs"
  - "src/database/encryption.rs"
  - "src/api/handlers.rs"
related_rules:
  - "DATA-001"
  - "SEC-001"
version: "1.0.0"
created_at: "2026-03-16"
---

# PII Protection

## Rule Definition

All Personally Identifiable Information (PII) must be encrypted at rest and in transit, with access logging and minimization.

## PII Categories

| Category | Examples | Protection Level |
|----------|----------|-----------------|
| **Sensitive PII** | SSN, Passport, Financial | Encrypt + Access Control + Audit |
| **Personal PII** | Name, Email, Phone | Encrypt + Access Control |
| **Technical PII** | IP Address, Device ID | Encrypt + Minimize |
| **Public PII** | Username, Public Profile | Minimal Protection |

## Validation Criteria

1. **Encryption at Rest** — All PII encrypted in database (AES-256)
2. **Encryption in Transit** — All PII transmitted over TLS 1.3
3. **Access Logging** — All PII access logged with user/timestamp
4. **Data Minimization** — Only collect necessary PII
5. **Purpose Limitation** — Use PII only for stated purpose

## Implementation Notes

- Use `age` or `libsodium` for field-level encryption
- Implement encryption key rotation (annual)
- Mask PII in logs (e.g., `j***@example.com`)
- Implement PII access audit endpoint
- Provide data export for users (GDPR Article 15)

## Encryption Example

```rust
use age::{Encryptor, Decryptor};

pub struct EncryptedField {
    ciphertext: Vec<u8>,
    nonce: [u8; 12],
}

impl EncryptedField {
    pub fn encrypt(plaintext: &str, key: &age::x25519::Recipient) -> Result<Self> {
        let mut encrypted = Vec::new();
        let mut encryptor = Encryptor::with_recipient(key, &mut encrypted)?;
        encryptor.write_all(plaintext.as_bytes())?;
        encryptor.finish()?;
        
        Ok(Self { ciphertext: encrypted, nonce: [0; 12] })
    }
    
    pub fn decrypt(&self, key: &age::x25519::Identity) -> Result<String> {
        let mut decryptor = Decryptor::with_recipient(key, &self.ciphertext)?;
        let mut decrypted = Vec::new();
        decryptor.read_to_end(&mut decrypted)?;
        Ok(String::from_utf8(decrypted)?)
    }
}
```

## Error Handling

```rust
pub enum PIIError {
    EncryptionFailed,
    DecryptionFailed,
    UnauthorizedAccess,
    AccessNotLogged,
    DataMinimizationViolation,
}
```

## Test Coverage

- `tests/database/encryption_test.rs` — Field encryption
- `tests/api/pii_test.rs` — PII handling in API
- `tests/compliance/gdpr_test.rs` — GDPR compliance

## Related Rules

- **DATA-001** (Data Retention) — PII retention periods
- **SEC-001** (Input Validation) — PII input validation

## Compliance

- GDPR — Personal data protection
- CCPA — California consumer privacy
- HIPAA — Health information (if applicable)

---

**Last Updated:** 2026-03-16
**Status:** Active
