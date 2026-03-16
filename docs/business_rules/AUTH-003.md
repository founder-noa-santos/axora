---
rule_id: "AUTH-003"
title: "Session Management"
category: "Security"
severity: "Critical"
applies_to:
  - "src/auth/session.rs"
  - "src/middleware/session.rs"
related_rules:
  - "AUTH-001"
  - "AUTH-002"
version: "1.0.0"
created_at: "2026-03-16"
---

# Session Management

## Rule Definition

All user sessions must be properly managed with secure creation, validation, and termination to prevent session hijacking and fixation attacks.

## Validation Criteria

1. **Session ID Generation** — Session IDs must be cryptographically random (256-bit)
2. **Session Expiration** — Sessions must expire after 24 hours of inactivity
3. **Secure Storage** — Session tokens must be stored in HttpOnly, Secure cookies
4. **Session Invalidation** — Sessions must be invalidated on logout and password change

## Implementation Notes

- Use `uuid` crate with v4 for session ID generation
- Store sessions in Redis with TTL for automatic expiration
- Implement sliding window expiration (reset on activity)
- Regenerate session ID after authentication

## Session Lifecycle

```
1. Create → User authenticates successfully
2. Validate → Check session on each request
3. Refresh → Extend TTL on activity
4. Invalidate → Logout, password change, or expiration
```

## Error Handling

```rust
pub enum SessionError {
    InvalidSessionId,
    ExpiredSession,
    SessionMismatch, // IP/User-Agent mismatch
    SessionRevoked,
}
```

## Test Coverage

- `tests/auth/session_test.rs` — Session lifecycle
- `tests/middleware/session_test.rs` — Session validation
- `tests/auth/logout_test.rs` — Session invalidation

## Related Rules

- **AUTH-001** (User Authentication Protocol) — Defines authentication
- **AUTH-002** (Password Requirements) — Password changes invalidate sessions

## Compliance

- OWASP ASVS V3.2 — Session Management
- CWE-384 — Session Fixation

---

**Last Updated:** 2026-03-16
**Status:** Active
