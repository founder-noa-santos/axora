# Preflight / Doctor command evaluation

## Context

This evaluation is required by the [CLI do observability plan](../../.cursor/plans/cli_do_observability_6ed646ed.plan.md) to determine whether OPENAKTA should add a `doctor` subcommand or inline preflight checks.

## Problem statement

Users currently run `openakta do` and may fail due to:
- Missing provider configuration
- Unreachable local provider (Ollama not running)
- Missing API keys
- Invalid model names

These failures now produce **actionable error messages** (bug class A fix), but users might benefit from **earlier validation**.

## Options evaluated

### Option 1: Inline preflight before `do`

**Description:** Run quick checks at the start of `openakta do` before mission execution.

**Checks:**
- Provider instances configured
- Local providers reachable (TCP connect)
- API keys present (env var check)
- Model names valid (optional API call)

**Pros:**
- Fast feedback (fail before orchestration starts)
- No new CLI surface to discover
- Applies to all `do` invocations automatically

**Cons:**
- Slows every `do` run (even when config is correct)
- Adds latency to happy path
- May produce false positives (ephemeral network issues)

**Implementation effort:** Medium (add `preflight_check()` in `RuntimeBootstrap::run_mission`)

### Option 2: `openakta doctor` subcommand

**Description:** Dedicated diagnostic command users run when troubleshooting.

**Example:**
```bash
openakta doctor
# or
openakta doctor --workspace /path/to/project
```

**Checks:** All of Option 1, plus:
- Database migrations status
- Model registry validation
- Skill corpus sync status
- Optional network latency tests

**Pros:**
- Clear separation of concerns
- No impact on `do` performance
- Can be extended with more diagnostics over time
- Users can run proactively before important missions

**Cons:**
- Discoverability (users may not know it exists)
- Extra step (users must remember to run it)
- May duplicate some validation already in `do`

**Implementation effort:** Medium-High (new CLI subcommand, diagnostic module)

### Option 3: `OPENAKTA_DOCTOR=1` environment flag

**Description:** Optional preflight when env var is set.

**Pros:**
- No new CLI surface
- Opt-in (no performance impact by default)
- Can be documented for troubleshooting

**Cons:**
- Hidden feature (users won't discover naturally)
- Still adds friction when enabled

**Implementation effort:** Low (guard existing preflight with env check)

## Recommendation

**Implement Option 2 (`openakta doctor`) with lightweight inline validation.**

### Rationale

1. **Bug class A is fixed:** Mission failures now print actionable errors, reducing the urgency for inline preflight.

2. **User experience:** A dedicated `doctor` command:
   - Can be run proactively before important missions
   - Provides comprehensive diagnostics without slowing the happy path
   - Can grow over time (e.g., `openakta doctor --verbose`)

3. **Performance:** Keeps `openakta do` fast for the common case (config already correct).

4. **Extensibility:** `doctor` can include additional checks beyond provider config (database health, skill sync, etc.).

### Implementation outline

```rust
// New file: crates/openakta-cli/src/doctor.rs
pub async fn run_doctor(workspace_root: &Path) -> anyhow::Result<DoctorReport> {
    // 1. Load and validate openakta.toml
    // 2. Check provider instances
    // 3. Test local provider connectivity
    // 4. Validate API keys present
    // 5. Optional: test model availability
    // 6. Report status
}
```

**Exit codes:**
- `0` — All checks passed
- `1` — One or more critical checks failed (with actionable messages)
- `2` — Warning (non-critical issues)

### When to reconsider inline preflight

Consider adding inline preflight (Option 1) if:
- Users frequently run `do` without running `doctor` first
- Error patterns show users need earlier validation
- Team decides the latency cost is acceptable for better UX

## Decision record

**Date:** 2026-03-23  
**Decision:** Implement `openakta doctor` subcommand  
**Owner:** TBD  
**Status:** Pending implementation

## Next steps

1. Create `crates/openakta-cli/src/doctor.rs` module
2. Add `Doctor` subcommand to CLI
3. Implement provider validation checks
4. Add tests for doctor command
5. Update `COMO_RODAR.md` and `docs/cli-do-smoke-test.md` with doctor usage

## Appendix: Example output

```
$ openakta doctor

Checking OPENAKTA configuration...

✓ Provider configuration: 1 instance configured
✓ Local provider "local-ollama": reachable (http://localhost:11434)
✓ Model "qwen2.5-coder:7b": available
✗ API key "OPENAI_API_KEY": not set (required for cloud fallback)

Status: 1 warning, 0 errors

Recommendations:
- Set OPENAI_API_KEY if you want cloud provider fallback

```
