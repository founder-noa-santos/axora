---
rule_id: "AUTH-002"
title: "Password Requirements"
category: "Security"
severity: "Critical"
applies_to:
  - "src/auth/password.rs"
  - "src/auth/register.rs"
related_rules:
  - "AUTH-001"
  - "SEC-001"
version: "1.0.0"
created_at: "2026-03-16"
---

# Password Requirements

## Rule Definition

All user passwords must meet minimum complexity requirements to prevent brute-force and dictionary attacks.

## Validation Criteria

1. **Minimum Length** — Password must be at least 12 characters
2. **Character Diversity** — Password must contain at least 3 of 4 character types:
   - Uppercase letters (A-Z)
   - Lowercase letters (a-z)
   - Numbers (0-9)
   - Special characters (!@#$%^&*()_+-=[]{}|;:,.<>?)
3. **No Common Passwords** — Password must not appear in common password lists
4. **No Personal Information** — Password must not contain username or email

## Implementation Notes

- Use `zxcvbn` crate for password strength estimation
- Minimum zxcvbn score: 3 (good)
- Check against Have I Been Pwned API for breached passwords
- Password validation occurs client-side AND server-side

## Error Messages

```rust
pub enum PasswordError {
    TooShort { min_length: usize, actual: usize },
    InsufficientComplexity { required: usize, actual: usize },
    CommonPassword,
    ContainsPersonalInfo,
    BreachedPassword,
}
```

## Test Coverage

- `tests/auth/password_test.rs` — Password validation
- `tests/auth/register_test.rs` — Registration flow
- `tests/auth/breach_test.rs` — Breached password check

## Related Rules

- **AUTH-001** (User Authentication Protocol) — Defines authentication flow
- **SEC-001** (Input Validation) — Defines input sanitization

## Compliance

- NIST SP 800-63B — Memorized Secret Verifiers
- OWASP ASVS V3.1 — Password Security

---

**Last Updated:** 2026-03-16
**Status:** Active
