# Business Logic Template

Describe rules, state transitions, domain entities, and invariants.

## Machine-Verifiable Expectations

```akta-expect
code_path: src/domain/rules/example.ts
symbol: resolvePlan
kind: function
signature: export function resolvePlan(accountId: string): Promise<string>
rule_ids:
  - BR-001
```
