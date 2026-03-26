# CLI `do` smoke test and minimal environment

This document describes the **minimum configuration** required for `openakta do` to succeed, and how to diagnose failures.

## Expected behavior on a clean machine

**Without any provider configuration**, `openakta do` is **expected to fail** with a clear, actionable error message explaining that no providers are configured.

**With a valid provider**, `openakta do` should succeed and print the mission result.

## Minimal configuration for success

### Option 1: Local provider (Ollama)

1. Install [Ollama](https://ollama.ai)
2. Pull a model:
   ```bash
   ollama pull qwen2.5-coder:7b
   ```
3. Ensure Ollama is running (default: `http://localhost:11434`)
4. Create or update `openakta.toml` in your workspace root:
   ```toml
   [providers]
   default_local_instance = "local-ollama"

   [providers.instances.local-ollama]
   is_local = true
   base_url = "http://localhost:11434"
   default_model = "qwen2.5-coder:7b"
   ```

### Option 2: Cloud provider (OpenAI, etc.)

1. Obtain API key from provider
2. Set environment variable (example for OpenAI):
   ```bash
   export OPENAI_API_KEY="sk-..."
   ```
3. Create or update `openakta.toml`:
   ```toml
   [providers]
   default_cloud_instance = "openai"

   [providers.instances.openai]
   is_local = false
   provider_kind = "OpenAI"
   default_model = "gpt-4o-mini"
   # API key loaded from OPENAI_API_KEY env var
   ```

## Diagnosis: what failed and why

### Exit code classification

The CLI distinguishes two failure modes:

1. **`Err(...)`** - Bootstrap/orchestration failure (printed by `anyhow`)
2. **`Ok(success=false)`** - Mission completed with task failures

Both now print **actionable stderr**.

### Log levels

Use these log levels for debugging:

```bash
# Minimal
RUST_LOG=openakta=info

# Detailed
RUST_LOG=openakta=debug,openakta_agents=debug

# Full traces (very noisy)
RUST_LOG=trace
```

### Common failure patterns

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| "No provider instances configured" | Empty `openakta.toml` or no instances section | Add provider config as above |
| "Connection refused" to localhost:11434 | Ollama not running | Start Ollama: `ollama serve` |
| "401 Unauthorized" | Missing/invalid API key | Check env var and `openakta.toml` |
| "task timed out" | Model too slow or network issue | Increase `task_timeout` in config or check network |
| Empty stderr with exit 1 | Bug class A (should be fixed) | If still occurs, file bug with `RUST_LOG=debug` logs |

## Reproduction script

```bash
#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."  # aktacode root

export RUST_LOG=openakta=debug,openakta_agents=debug,openakta_core=debug
export RUST_BACKTRACE=1

echo "=== Running smoke test ==="
cargo run --bin openakta -- do "Apenas diga: Olá" 2>&1 | tee /tmp/openakta-do-smoke.log
EXIT_CODE=${PIPESTATUS[0]}

echo ""
echo "=== Exit code: $EXIT_CODE ==="

if [ $EXIT_CODE -eq 0 ]; then
    echo "SUCCESS: Mission completed"
else
    echo "FAILURE: Mission failed (see logs above)"
fi

exit $EXIT_CODE
```

Save as `scripts/smoke-test-do.sh` and run with `bash scripts/smoke-test-do.sh`.

## Validation checklist

After fixes, verify:

- [ ] `exit 0` with valid provider config and mission success
- [ ] `exit 1` with **non-empty stderr** when mission fails
- [ ] `exit 1` with **actionable message** when no providers configured
- [ ] INFO logs show: `mission started`, `mission decomposed into N tasks`, `mission completed`
- [ ] Failed tasks append error text to stderr output (not just silent failure)

## Related

- [`COMO_RODAR.md`](../COMO_RODAR.md) — General usage guide
- [CLI observability plan](../../.cursor/plans/cli_do_observability_6ed646ed.plan.md) — Implementation details
