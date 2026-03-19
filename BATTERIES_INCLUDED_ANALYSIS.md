# BATTERIES_INCLUDED_ANALYSIS.md

## The Positives (What we got right)

- `CoordinatorV2` is already the real execution runtime and now has a built-in Base Squad bootstrap instead of only anonymous worker slots.
- MCP was already native Rust, which made the batteries-included pivot feasible without external Node.js or Python dependencies.
- Tripartite memory, pruning, consolidation, and LivingDocs were already local-first and fast to initialize.
- The provider layer was already separated enough to support a single mission-first runtime bootstrap.

## The Gaps (The brutal truth)

- The previous visible backend entrypoint was still `axora-daemon`, which forced an operator mental model.
- The default squad was capacity-based instead of role-based.
- Native MCP tooling was too narrow for a serious out-of-the-box coding workflow.
- Procedural memory created empty directories on first run instead of a useful standard skill library.
- Core docs still described daemon-first activation and manual runtime bring-up.

## Actionable Implementation Plan

### Implemented now

1. Added a real user-facing CLI in `crates/axora-cli` with `axora do "<mission>"`.
2. Added `RuntimeBootstrap` in `crates/axora-core` to:
   - infer the workspace
   - create `.axora/` runtime paths
   - initialize SQLite
   - seed default procedural skills
   - start native MCP
   - start memory/doc background services
   - execute the mission through `CoordinatorV2`
3. Replaced generic worker bootstrap with a built-in Base Squad manifest:
   - Architect
   - Coder
   - Tester
   - Executor
   - Reviewer
4. Refactored `axora-mcp-server` around an embedded native tool registry and added core tools:
   - `read_file`
   - `generate_diff`
   - `apply_patch`
   - `ast_chunk`
   - `symbol_lookup`
   - `run_command`
   - `graph_context`
5. Added `SkillSeeder` plus a storage checkpoint table for idempotent first-run seeding.
6. Shipped an initial built-in skill library covering:
   - Rust test writing
   - cargo repair
   - diff review
   - merge conflict resolution
   - safe patch application
   - repository exploration
   - JWT debugging
7. Rewrote key onboarding and product docs around the new mission-first flow.

### Resulting default user journey

1. Export a provider API key.
2. Run `cargo run -p axora-cli -- do "add JWT auth"`.
3. AXORA bootstraps its own local runtime and executes the mission.

### Remaining work after this pass

- Route more of the internal execution path through the expanded MCP registry instead of direct helper calls.
- Add a daemon-managed mode owned by the CLI for long-lived sessions and desktop integration.
- Expand the standard skill library and attach richer upgrade semantics beyond seed version `v1`.
