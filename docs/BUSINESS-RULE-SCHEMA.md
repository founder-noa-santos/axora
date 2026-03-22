# Business Rule Documentation Schema

**Date:** 2026-03-16
**Status:** ADOPTED
**Purpose:** Define format for business rule documentation with machine validation

---

## 📋 Format

Business rules are documented in **Markdown files with YAML frontmatter**:

- **Human-readable:** Markdown content
- **Machine-parseable:** YAML frontmatter
- **Validated:** JSON Schema enforcement
- **Traceable:** Bidirectional links to code

### Example File

```markdown
---
rule_id: "AUTH-001"
title: "User Authentication Protocol"
category: "Security"
severity: "Critical"
applies_to:
  - "src/auth/login.rs"
  - "src/middleware/auth.rs"
related_rules:
  - "AUTH-002"
---

# User Authentication Protocol

## Rule Definition
All users must successfully authenticate via a cryptographically verified JWT token before accessing routes nested under the `/api/secure/` namespace.

## Validation Criteria
- Token must not be expired (check `exp` claim)
- Token signature must match the active RS256 public key
- Token must contain valid `user_id` claim

## Implementation Notes
- Use `jsonwebtoken` crate for JWT validation
- Public keys are rotated every 24 hours

## Test Coverage
- `tests/auth/login_test.rs` — Valid login flow
- `tests/auth/jwt_test.rs` — Token validation
```

---

## 🏷️ YAML Frontmatter Fields

### rule_id (required)

**Unique identifier for the business rule.**

| Property | Value |
|----------|-------|
| **Type** | String |
| **Pattern** | `^[A-Z]{3,4}-\d{3}$` |
| **Example** | `AUTH-001`, `SEC-042`, `PAY-100` |

**Categories:**
- `AUTH` — Authentication & Authorization
- `SEC` — Security
- `PAY` — Payments & Billing
- `DATA` — Data Management
- `API` — API Design
- `PERF` — Performance
- `REL` — Reliability

---

### title (required)

**Human-readable title for the rule.**

| Property | Value |
|----------|-------|
| **Type** | String |
| **Min Length** | 1 character |
| **Max Length** | 200 characters |
| **Example** | `User Authentication Protocol` |

---

### category (required)

**Classification of the rule type.**

| Property | Value |
|----------|-------|
| **Type** | String |
| **Enum** | `Security`, `Compliance`, `Business`, `Performance`, `Reliability` |

**Category Definitions:**

| Category | Description | Examples |
|----------|-------------|----------|
| **Security** | Security-related rules | Auth, encryption, access control |
| **Compliance** | Regulatory requirements | GDPR, PCI-DSS, SOC2 |
| **Business** | Business logic rules | Pricing, refunds, quotas |
| **Performance** | Performance requirements | Response time, throughput |
| **Reliability** | Reliability requirements | Uptime, error handling |

---

### severity (required)

**Impact level if rule is violated.**

| Property | Value |
|----------|-------|
| **Type** | String |
| **Enum** | `Critical`, `High`, `Medium`, `Low` |

**Severity Definitions:**

| Severity | Description | Response Time |
|----------|-------------|---------------|
| **Critical** | System security/stability at risk | Immediate |
| **High** | Significant business impact | < 24 hours |
| **Medium** | Moderate impact | < 1 week |
| **Low** | Minor impact | Next sprint |

---

### applies_to (required)

**List of source files where this rule is implemented.**

| Property | Value |
|----------|-------|
| **Type** | Array of strings |
| **Pattern** | `^src/.*\.(rs|ts|js|py|go)$` |
| **Example** | `["src/auth/login.rs", "src/middleware/auth.rs"]` |

**Purpose:** Enables bidirectional traceability between rules and code.

---

### related_rules (optional)

**List of related business rule IDs.**

| Property | Value |
|----------|-------|
| **Type** | Array of strings |
| **Pattern** | `^[A-Z]{3,4}-\d{3}$` |
| **Example** | `["AUTH-002", "SEC-001"]` |

**Purpose:** Creates a graph of related rules for impact analysis.

---

### version (optional)

**Rule version for tracking changes.**

| Property | Value |
|----------|-------|
| **Type** | String |
| **Pattern** | Semantic versioning |
| **Default** | `1.0.0` |
| **Example** | `1.0.0`, `2.1.0` |

---

### created_at (optional)

**Rule creation date.**

| Property | Value |
|----------|-------|
| **Type** | String (ISO 8601) |
| **Example** | `2026-03-16` |
| **Default** | Current date |

---

### updated_at (optional)

**Last update date.**

| Property | Value |
|----------|-------|
| **Type** | String (ISO 8601) |
| **Example** | `2026-03-16` |
| **Default** | Current date |

---

## 📐 JSON Schema

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://openakta.dev/schemas/business-rule.json",
  "title": "Business Rule",
  "description": "Schema for OPENAKTA business rule documentation",
  "type": "object",
  "properties": {
    "rule_id": {
      "type": "string",
      "description": "Unique identifier for the business rule",
      "pattern": "^[A-Z]{3,4}-\\d{3}$",
      "examples": ["AUTH-001", "SEC-042", "PAY-100"]
    },
    "title": {
      "type": "string",
      "description": "Human-readable title",
      "minLength": 1,
      "maxLength": 200,
      "examples": ["User Authentication Protocol"]
    },
    "category": {
      "type": "string",
      "description": "Rule classification",
      "enum": ["Security", "Compliance", "Business", "Performance", "Reliability"],
      "examples": ["Security"]
    },
    "severity": {
      "type": "string",
      "description": "Impact level if violated",
      "enum": ["Critical", "High", "Medium", "Low"],
      "examples": ["Critical"]
    },
    "applies_to": {
      "type": "array",
      "description": "Source files where rule is implemented",
      "items": {
        "type": "string",
        "pattern": "^src/.*\\.(rs|ts|js|py|go)$"
      },
      "minItems": 1,
      "examples": [["src/auth/login.rs", "src/middleware/auth.rs"]]
    },
    "related_rules": {
      "type": "array",
      "description": "Related business rule IDs",
      "items": {
        "type": "string",
        "pattern": "^[A-Z]{3,4}-\\d{3}$"
      },
      "examples": [["AUTH-002", "SEC-001"]]
    },
    "version": {
      "type": "string",
      "description": "Rule version",
      "pattern": "^\\d+\\.\\d+\\.\\d+$",
      "default": "1.0.0",
      "examples": ["1.0.0", "2.1.0"]
    },
    "created_at": {
      "type": "string",
      "description": "Creation date",
      "format": "date",
      "examples": ["2026-03-16"]
    },
    "updated_at": {
      "type": "string",
      "description": "Last update date",
      "format": "date",
      "examples": ["2026-03-16"]
    }
  },
  "required": ["rule_id", "title", "category", "severity", "applies_to"],
  "additionalProperties": false
}
```

---

## ✅ Validation

### Using JSON Schema CLI

```bash
# Install jsonschema CLI
pip install jsonschema

# Validate single file
jsonschema --instance docs/business_rules/AUTH-001.md docs/BUSINESS-RULE-SCHEMA.json

# Validate all files
for file in docs/business_rules/*.md; do
    jsonschema --instance "$file" docs/BUSINESS-RULE-SCHEMA.json
done
```

### Using remark-lint-frontmatter-schema

```bash
# Install remark
npm install -g remark-cli remark-lint remark-lint-frontmatter-schema

# Create .remarkrc.js
cat > .remarkrc.js << 'EOF'
module.exports = {
  plugins: [
    ['lint', {
      'frontmatter-schema': {
        schema: 'docs/BUSINESS-RULE-SCHEMA.json',
        frontmatter: 'yaml'
      }
    }]
  ]
};
EOF

# Validate all business rules
remark docs/business_rules/*.md
```

### Using Custom Script

```bash
# Run validation script
./scripts/validate-business-rules.sh
```

---

## 🔗 Bidirectional Traceability

### YAML → Code (applies_to)

The `applies_to` field in YAML frontmatter links to source files:

```yaml
---
rule_id: "AUTH-001"
applies_to:
  - "src/auth/login.rs"
  - "src/middleware/auth.rs"
---
```

### Code → YAML (@req annotations)

Source files reference rules using `@req` annotations:

```rust
/// @req AUTH-001 — User Authentication Protocol
/// @req AUTH-002 — Password Requirements
pub fn authenticate_user(username: &str, password: &str) -> Result<Token> {
    // Implementation
}
```

### Validation Script

```bash
# Check bidirectional links
./scripts/check-traceability.sh
```

**Expected output:**
```
✓ AUTH-001 → src/auth/login.rs (found @req AUTH-001)
✓ AUTH-001 → src/middleware/auth.rs (found @req AUTH-001)
✓ All rules have bidirectional links
```

---

## 📁 File Structure

```
docs/
├── BUSINESS-RULE-SCHEMA.md       # This file
├── BUSINESS-RULE-SCHEMA.json     # JSON Schema (generated)
└── business_rules/
    ├── AUTH-001.md               # Authentication rules
    ├── AUTH-002.md
    ├── AUTH-003.md
    ├── SEC-001.md                # Security rules
    ├── SEC-002.md
    ├── PAY-001.md                # Payment rules
    ├── PAY-002.md
    ├── DATA-001.md               # Data rules
    ├── DATA-002.md
    └── API-001.md                # API rules

scripts/
└── validate-business-rules.sh    # Validation script
```

---

## 📊 Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Schema Compliance | 100% | % of rules passing JSON Schema validation |
| Bidirectional Links | 100% | % of rules with valid `applies_to` + `@req` |
| File Existence | 100% | % of `applies_to` files that exist |
| Rule Coverage | >80% | % of critical code paths with rules |

---

## 🔗 Related Documents

- [`DOCUMENTATION-FORMAT.md`](./DOCUMENTATION-FORMAT.md) — Documentation standards
- [Architecture communication](./architecture-communication.md) — traceability and messaging (reference)

---

**This schema enables MACHINE-VALIDATED business rule documentation with bidirectional traceability.**
