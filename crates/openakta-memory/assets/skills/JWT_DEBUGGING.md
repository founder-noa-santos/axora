---
skill_id: "JWT_DEBUGGING"
name: "Debug JWT Authentication Failures"
triggers:
  - "jwt auth"
  - "token validation failed"
  - "authentication failure"
domain: "security"
created_at: 0
updated_at: 0
success_count: 0
failure_count: 0
utility_score: 0.75
---

# Debug JWT Authentication Failures

### Step 1: Validate token source and expected issuer
**Validation:** Confirm the request carries the correct token for the environment.

### Step 2: Check claims, clock skew, and signing key
**Validation:** Compare exp, nbf, aud, iss, and key identifiers.

### Step 3: Reproduce with the narrowest protected route
**Validation:** Confirm the failure disappears after the root cause fix.
