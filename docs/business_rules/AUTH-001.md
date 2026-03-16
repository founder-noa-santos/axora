---
rule_id: "AUTH-001"
title: "User Authentication Protocol"
category: "Security"
severity: "Critical"
applies_to:
  - "src/auth/login.rs"
  - "src/auth/jwt.rs"
  - "src/middleware/auth.rs"
related_rules:
  - "AUTH-002"
  - "AUTH-003"
version: "1.0.0"
created_at: "2026-03-16"
---

# User Authentication Protocol

## Rule Definition

All users must successfully authenticate via a cryptographically verified JWT token before accessing routes nested under the `/api/secure/` namespace.

## Validation Criteria

1. **Token Expiration** — Token must not be expired (check `exp` claim)
2. **Signature Verification** — Token signature must match the active RS256 public key
3. **Valid Claims** — Token must contain valid `user_id` and `role` claims
4. **Issuer Verification** — Token issuer must match configured identity provider

## Implementation Notes

- Use `jsonwebtoken` crate for JWT validation
- Public keys are rotated every 24 hours via JWKS endpoint
- Expired tokens should return `401 Unauthorized` with `token_expired` error code
- Invalid signatures should return `401 Unauthorized` with `invalid_signature` error code
- Missing tokens should return `401 Unauthorized` with `missing_token` error code

## Error Handling

```rust
pub enum AuthError {
    MissingToken,
    InvalidSignature,
    TokenExpired,
    InvalidIssuer,
    MissingClaims,
}
```

## Test Coverage

- `tests/auth/login_test.rs` — Valid login flow
- `tests/auth/jwt_test.rs` — Token validation
- `tests/middleware/auth_test.rs` — Middleware enforcement
- `tests/auth/error_test.rs` — Error handling

## Related Rules

- **AUTH-002** (Password Requirements) — Defines password complexity
- **AUTH-003** (Session Management) — Defines session lifecycle

## Compliance

- OWASP ASVS V2.1 — Authentication
- NIST SP 800-63B — Digital Identity Guidelines

---

**Last Updated:** 2026-03-16
**Status:** Active
