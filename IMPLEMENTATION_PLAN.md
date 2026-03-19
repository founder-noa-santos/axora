# IMPLEMENTATION_PLAN.md

## Summary

AXORA’s codebase is ahead of the prompt in several areas, but much of that progress is library-level or doc-level rather than runtime-integrated. The main pattern is: docs say “implemented,” Rust code often says “designed/scaffolded,” and the live coordinator still runs synthetic paths. Implement in this order to minimize rework: **Theme 3 (live providers) → Theme 2 (MCP boundary) → Theme 5 (true dual-thread runtime) → Theme 1 (MetaGlyph/C2C wire prep) → Theme 4 (memory integration) → Theme 6 (LivingDocs daemonization).**

## Important Interface Changes

- Extend the collective protobufs with a **canonical compressed context envelope**.
- Add a new **MCP gRPC surface** in `axora-proto`.
- Introduce a provider runtime abstraction in `axora-agents`.
- Introduce daemon-managed services for memory, docs, MCP, and ReAct runtime.

## Themes

1. Advanced Semantic & Latent Communication
2. MCP for Secure Tool Sandboxing
3. Live Cloud LLM Execution
4. Tripartite Memory & FadeMem
5. Dual-Thread ReAct Loops
6. LivingDocs (Active Governance & Auto-Healing)

## Assumptions and Defaults

- Use the R-18 hybrid CLI-first, gRPC-backed MCP architecture.
- Keep cloud LLMs for reasoning and local-first storage/indexing for memory and embeddings.
- Treat latent/C2C as experimental and non-canonical until benchmarked.
- Treat conflicting architecture docs as aspirational when code disagrees.
