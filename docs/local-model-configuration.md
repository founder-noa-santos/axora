# Local Model Configuration

OPENAKTA supports an optional local fast path through Ollama. You can run cloud-only, local-only, or heterogeneous cloud+local routing from the same workspace.

## Recommended models

- `qwen2.5-coder:7b`
- `qwen2.5-coder:14b`
- `llama3.2:3b`

`qwen2.5-coder:7b` is the recommended default for low-complexity editing tasks.

## Start Ollama

Install Ollama and pull a model:

```bash
ollama pull qwen2.5-coder:7b
ollama serve
```

By default OPENAKTA expects Ollama at `http://127.0.0.1:11434`.

## Configure `openakta.toml`

Cloud and local lanes are both optional. If only one lane is configured, OPENAKTA stays in single-model mode. If both lanes are configured, DAAO routing can be enabled.

```toml
fallback_policy = "explicit"
routing_enabled = true
local_validation_retry_budget = 1

[cloud_model]
provider = "anthropic"
model = "claude-sonnet-4-5"

[local_model]
provider = "ollama"
base_url = "http://127.0.0.1:11434"
default_model = "qwen2.5-coder:7b"
enabled_for = ["syntax_fix", "docstring", "autocomplete", "small_edit"]
```

For local-only mode, omit the `[cloud_model]` section.

## Config precedence

Runtime config is applied in this order:

1. CLI flags
2. Environment variables
3. `openakta.toml`
4. Defaults

Relevant overrides for the heterogeneous runtime are:

- CLI: `--provider`, `--model`, `--local-model`, `--ollama-url`, `--fallback-policy`
- Environment: `OPENAKTA_CLOUD_MODEL`, `OPENAKTA_LOCAL_MODEL`, `OPENAKTA_OLLAMA_URL`, `OPENAKTA_FALLBACK_POLICY`, `OPENAKTA_ROUTING_ENABLED`, `OPENAKTA_LOCAL_RETRY_BUDGET`

## CLI overrides

The CLI can override config file values:

```bash
openakta do "fix the syntax error in src/lib.rs" \
  --local-model qwen2.5-coder:7b \
  --ollama-url http://127.0.0.1:11434 \
  --fallback-policy explicit
```

## Fallback policy

- `never`: fail immediately when the cloud lane is unavailable.
- `explicit`: return a structured failure that advertises a local retry path, but do not downgrade automatically.
- `automatic`: downgrade to the configured local lane when the cloud lane is unreachable.

`explicit` is the default because OPENAKTA should not silently change trust, cost, or quality characteristics unless you asked for it.

## Local validation retry budget

`local_validation_retry_budget` controls how many local diff-validation failures OPENAKTA tolerates before escalating a code task to the cloud arbiter. The default is `1`.
