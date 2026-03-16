---
rule_id: "DATA-001"
title: "Data Retention"
category: "Compliance"
severity: "High"
applies_to:
  - "src/database/cleanup.rs"
  - "src/models/user.rs"
  - "src/models/payment.rs"
related_rules:
  - "DATA-002"
  - "PAY-002"
version: "1.0.0"
created_at: "2026-03-16"
---

# Data Retention

## Rule Definition

All data must be retained for specified periods based on data type and regulatory requirements, then securely deleted.

## Retention Periods

| Data Type | Retention Period | Legal Basis |
|-----------|-----------------|-------------|
| User Accounts | 7 years after deletion | Tax/Financial records |
| Payment Records | 7 years after transaction | PCI-DSS, Tax law |
| Session Logs | 90 days | Security monitoring |
| API Logs | 30 days | Debugging, Security |
| Audit Logs | 7 years | Compliance |
| Deleted User Data | 30 days (grace period) | User recovery |

## Implementation Notes

- Implement automated cleanup jobs (daily)
- Use soft deletes with `deleted_at` timestamp
- Hard delete after retention period expires
- Log all deletions for audit trail
- Provide user data export before deletion (GDPR)

## Cleanup Job Example

```rust
pub async fn cleanup_expired_data(pool: &PgPool) -> Result<CleanupStats> {
    let mut stats = CleanupStats::default();
    
    // Soft-delete expired sessions
    stats.sessions = sqlx::query(
        "DELETE FROM sessions WHERE expires_at < NOW() - INTERVAL '90 days'"
    )
    .execute(pool)
    .await?
    .rows_affected();
    
    // Hard-delete expired user data
    stats.users = sqlx::query(
        "DELETE FROM users WHERE deleted_at < NOW() - INTERVAL '30 days'"
    )
    .execute(pool)
    .await?
    .rows_affected();
    
    Ok(stats)
}
```

## Error Handling

```rust
pub enum CleanupError {
    DatabaseError(sqlx::Error),
    LockTimeout,
    PartialFailure { deleted: usize, failed: usize },
}
```

## Test Coverage

- `tests/database/cleanup_test.rs` — Cleanup jobs
- `tests/database/retention_test.rs` — Retention periods
- `tests/database/gdpr_test.rs` — GDPR compliance

## Related Rules

- **DATA-002** (PII Protection) — Personal data handling
- **PAY-002** (Refund Policy) — Financial record retention

## Compliance

- GDPR — Data minimization, right to erasure
- PCI-DSS — Cardholder data retention
- SOC2 — Data lifecycle management

---

**Last Updated:** 2026-03-16
**Status:** Active
