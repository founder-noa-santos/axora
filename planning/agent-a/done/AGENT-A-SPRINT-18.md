# Agent A — Sprint 18: Business Rule Documentation

**Phase:** 2  
**Sprint:** 18 (Documentation)  
**File:** `docs/business_rules/` + `docs/BUSINESS-RULE-SCHEMA.md`  
**Priority:** MEDIUM (standalone, blocks C-19)  
**Estimated Tokens:** ~70K output  

---

## 🎯 Task

Create **Business Rule Documentation** format (Markdown + YAML frontmatter) and document 10+ example business rules.

### Context

Research validates our approach and provides CRITICAL format details:
- **Markdown + YAML Frontmatter** — Human-readable + machine-parseable
- **JSON Schema Validation** — Enforce structure, prevent drift
- **Bidirectional Links** — YAML `applies_to` + code `@req` annotations

**Your job:** Define format and create example business rules (so Agent C can implement traceability).

---

## 📋 Deliverables

### 1. Create BUSINESS-RULE-SCHEMA.md

**File:** `docs/BUSINESS-RULE-SCHEMA.md`

**Structure:**
```markdown
# Business Rule Documentation Schema

## Format

Business rules are documented in Markdown files with YAML frontmatter:

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
All users must successfully authenticate via a cryptographically verified JWT token before accessing routes nested under the /api/secure/ namespace.

## Validation Criteria
- Token must not be expired
- Token signature must match the active RS256 public key
```

## YAML Frontmatter Fields

### rule_id (required)
- **Type:** String
- **Pattern:** `^[A-Z]{3,4}-\d{3}$`
- **Example:** `AUTH-001`, `SEC-042`, `PAY-100`

### title (required)
- **Type:** String
- **Example:** `User Authentication Protocol`

### category (required)
- **Type:** String
- **Enum:** `Security`, `Compliance`, `Business`, `Performance`, `Reliability`

### severity (required)
- **Type:** String
- **Enum:** `Critical`, `High`, `Medium`, `Low`

### applies_to (required)
- **Type:** Array of strings (file paths)
- **Example:** `["src/auth/login.rs", "src/middleware/auth.rs"]`

### related_rules (optional)
- **Type:** Array of strings (rule IDs)
- **Example:** `["AUTH-002", "SEC-001"]`

## JSON Schema

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "rule_id": {
      "type": "string",
      "pattern": "^[A-Z]{3,4}-\\d{3}$"
    },
    "title": {
      "type": "string",
      "minLength": 1
    },
    "category": {
      "type": "string",
      "enum": ["Security", "Compliance", "Business", "Performance", "Reliability"]
    },
    "severity": {
      "type": "string",
      "enum": ["Critical", "High", "Medium", "Low"]
    },
    "applies_to": {
      "type": "array",
      "items": {
        "type": "string",
        "pattern": "^src/.*\\.rs$"
      }
    },
    "related_rules": {
      "type": "array",
      "items": {
        "type": "string",
        "pattern": "^[A-Z]{3,4}-\\d{3}$"
      }
    }
  },
  "required": ["rule_id", "title", "category", "severity", "applies_to"]
}
```

## Validation

Use `remark-lint-frontmatter-schema` or `jsonschema` to validate:

```bash
# Validate all business rules
npx remark-lint-frontmatter-schema docs/business_rules/*.md

# Or use jsonschema CLI
jsonschema --instance docs/business_rules/AUTH-001.md docs/BUSINESS-RULE-SCHEMA.json
```
```

---

### 2. Create Example Business Rules (10+)

**Directory:** `docs/business_rules/`

**Create Files:**
1. `AUTH-001.md` — User Authentication Protocol
2. `AUTH-002.md` — Password Requirements
3. `AUTH-003.md` — Session Management
4. `SEC-001.md` — Input Validation
5. `SEC-002.md` — SQL Injection Prevention
6. `PAY-001.md` — Payment Processing
7. `PAY-002.md` — Refund Policy
8. `DATA-001.md` — Data Retention
9. `DATA-002.md` — PII Protection
10. `API-001.md` — API Versioning

**Example File (`docs/business_rules/AUTH-001.md`):**
```markdown
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
---

# User Authentication Protocol

## Rule Definition
All users must successfully authenticate via a cryptographically verified JWT token before accessing routes nested under the `/api/secure/` namespace.

## Validation Criteria
1. Token must not be expired (check `exp` claim)
2. Token signature must match the active RS256 public key
3. Token must contain valid `user_id` claim

## Implementation Notes
- Use `jsonwebtoken` crate for JWT validation
- Public keys are rotated every 24 hours
- Expired tokens should return 401 Unauthorized

## Related Rules
- AUTH-002 (Password Requirements)
- AUTH-003 (Session Management)

## Test Coverage
- `tests/auth/login_test.rs` — Valid login flow
- `tests/auth/jwt_test.rs` — Token validation
- `tests/middleware/auth_test.rs` — Middleware enforcement
```

---

### 3. Create Validation Script

**File:** `scripts/validate-business-rules.sh`

**Content:**
```bash
#!/bin/bash

# Validate all business rules against schema

set -e

echo "Validating business rules..."

# Check if all files exist
for rule_file in docs/business_rules/*.md; do
    if [ ! -f "$rule_file" ]; then
        echo "ERROR: Missing business rule file: $rule_file"
        exit 1
    fi
done

# Validate YAML frontmatter (using yq or similar)
for rule_file in docs/business_rules/*.md; do
    echo "Validating $rule_file..."
    
    # Extract YAML frontmatter
    yaml=$(sed -n '/^---$/,/^---$/p' "$rule_file" | sed '1d;$d')
    
    # Validate rule_id pattern
    rule_id=$(echo "$yaml" | grep "^rule_id:" | cut -d'"' -f2)
    if ! [[ "$rule_id" =~ ^[A-Z]{3,4}-[0-9]{3}$ ]]; then
        echo "ERROR: Invalid rule_id in $rule_file: $rule_id"
        exit 1
    fi
    
    # Validate severity enum
    severity=$(echo "$yaml" | grep "^severity:" | cut -d'"' -f2)
    if ! [[ "$severity" =~ ^(Critical|High|Medium|Low)$ ]]; then
        echo "ERROR: Invalid severity in $rule_file: $severity"
        exit 1
    fi
    
    # Validate applies_to (files must exist)
    echo "$yaml" | grep -A 100 "^applies_to:" | grep "^  - " | while read -r line; do
        file_path=$(echo "$line" | sed 's/^  - "//' | sed 's/"$//')
        if [ ! -f "$file_path" ]; then
            echo "ERROR: File not found in $rule_file: $file_path"
            exit 1
        fi
    done
done

echo "All business rules validated successfully!"
```

---

### 4. Update DOCUMENTATION-FORMAT.md

**File:** `docs/DOCUMENTATION-FORMAT.md` (UPDATE)

**Add Section:**
```markdown
## Business Rules

Business rules are documented in Markdown with YAML frontmatter:

- **Location:** `docs/business_rules/`
- **Format:** Markdown + YAML frontmatter
- **Validation:** JSON Schema + custom script
- **Links:** Bidirectional (YAML `applies_to` + code `@req` annotations)

See [`BUSINESS-RULE-SCHEMA.md`](./BUSINESS-RULE-SCHEMA.md) for full spec.
```

---

## 📁 File Boundaries

**Create:**
- `docs/BUSINESS-RULE-SCHEMA.md` (NEW)
- `docs/business_rules/AUTH-001.md` (NEW)
- `docs/business_rules/AUTH-002.md` (NEW)
- `docs/business_rules/AUTH-003.md` (NEW)
- `docs/business_rules/SEC-001.md` (NEW)
- `docs/business_rules/SEC-002.md` (NEW)
- `docs/business_rules/PAY-001.md` (NEW)
- `docs/business_rules/PAY-002.md` (NEW)
- `docs/business_rules/DATA-001.md` (NEW)
- `docs/business_rules/DATA-002.md` (NEW)
- `docs/business_rules/API-001.md` (NEW)
- `scripts/validate-business-rules.sh` (NEW)

**Update:**
- `docs/DOCUMENTATION-FORMAT.md` (add business rules section)

**DO NOT Edit:**
- `crates/` (implementation — Agents B and C's domain)

---

## ✅ Success Criteria

- [ ] `BUSINESS-RULE-SCHEMA.md` created (format spec)
- [ ] 10+ business rule files created
- [ ] All business rules follow schema
- [ ] Validation script works
- [ ] All files pass validation
- [ ] Bidirectional links documented (YAML `applies_to` + code `@req`)

---

## 🔗 References

- [`PHASE-2-INTEGRATION-INFLUENCE-GRAPH.md`](../shared/PHASE-2-INTEGRATION-INFLUENCE-GRAPH.md) — Integration doc
- Research document — Business rule format spec

---

**Start AFTER Sprint 12 (ACONIC Decomposition Docs) is complete.**

**Priority: MEDIUM — blocks Agent C Sprint 19 (Bidirectional Traceability).**

**Dependencies:**
- None (can start independently)

**Blocks:**
- Agent C Sprint 19 (Bidirectional Traceability)
