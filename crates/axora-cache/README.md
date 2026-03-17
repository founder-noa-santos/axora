# AXORA Cache

**Multi-tier caching and token optimization for AXORA**

[![Crates.io](https://img.shields.io/crates/v/axora-cache.svg)](https://crates.io/crates/axora-cache)
[![Documentation](https://docs.rs/axora-cache/badge.svg)](https://docs.rs/axora-cache)
[![Tests](https://github.com/axora/axora/workflows/CI/badge.svg)](https://github.com/axora/axora/actions)

## Overview

AXORA Cache is a high-performance caching and token optimization library designed for LLM-based development agents. It provides multiple strategies to reduce token consumption by **90%+** while maintaining sub-20ms latency.

## Key Features

### 🎯 Prefix Caching
Cache static prompts and system messages for 50-90% token savings on repeated interactions.

```rust
use axora_cache::{PrefixCache, CachedPromptBuilder};

let mut cache = PrefixCache::new(100);
cache.add("system", "You are a helpful assistant", 10);

let mut builder = CachedPromptBuilder::new(&cache);
builder.add_cached("system");
builder.add_dynamic(user_input);
let prompt = builder.build(); // Reuses cached prefix
```

**Savings:** 50-90% | **Latency:** <1ms

---

### 📝 Diff-Based Communication
Send code diffs instead of full files for 89-98% token reduction.

```rust
use axora_cache::{calculate_token_savings, UnifiedDiff};

let old_code = "fn hello() { println!(\"Hi\"); }";
let new_code = "fn hello() { println!(\"Hello, World!\"); }";

let diff = UnifiedDiff::generate(old_code, new_code, "rust");
let savings = calculate_token_savings(old_code, new_code, &diff);

println!("Token savings: {:.1}%", savings.savings_percentage);
```

**Savings:** 89-98% | **Latency:** <10ms

---

### 🗜️ Code Minification
Remove whitespace, comments, and compress identifiers for 24-42% savings.

```rust
use axora_cache::{CodeMinifier, MinifierConfig};

let minifier = CodeMinifier::new();
let config = MinifierConfig::default().with_language("rust");

let code = r#"
    pub fn add(a: i32, b: i32) -> i32 {
        // Add two numbers
        a + b
    }
"#;

let minified = minifier.minify(code, "rust")?;
// Result: "pub fn add(a:i32,b:i32)->i32{a+b}"
```

**Savings:** 24-42% | **Latency:** <5ms

**Supported Languages:** Rust, TypeScript, JavaScript, Python, Go

---

### 📦 TOON Serialization
Token-Optimized Object Notation — JSON alternative using schema-based field IDs.

```rust
use axora_cache::{Schema, ToonSerializer};

let json = r#"{"user_id": 12345, "username": "john_doe", "email": "john@example.com"}"#;

// Create schema from JSON sample
let schema = Schema::from_json_sample(json)?;
let serializer = ToonSerializer::new(schema);

// Encode to TOON (50-60% smaller)
let toon = serializer.encode(json)?;
// Schema: {0:user_id,1:username,2:email}
// 0:12345
// 1:"john_doe"
// 2:"john@example.com"

// Decode back to JSON
let decoded = serializer.decode(&toon)?;
```

**Savings:** 50-60% | **Latency:** <2ms

---

### 🧠 Context Distribution
Intelligent context allocation — each agent receives only the context necessary for its task.

```rust
use axora_cache::{ContextManager, Task, Agent, Document};

let mut manager = ContextManager::new();

// Store documents and code
manager.store_document(Document::new("spec", "API specification...", "spec"));
manager.store_code(CodeFile::new("main", "src/main.rs", "fn main() {}"));

// Allocate minimal context for a task
let task = Task::new("task-1", vec!["spec"], vec!["main"], vec![]);
let agent = Agent::new("developer");
let ctx = manager.allocate(&task, &agent);

// Pull-based retrieval
if let Some(doc) = ctx.get_doc("spec") {
    // Use document
}

// Calculate token savings (50%+ vs full context)
let savings = manager.calculate_savings(full_context_tokens);
```

**Savings:** 50%+ | **Latency:** <5ms

---

### 🗜️ Context Compacting
Roll long-running conversation and execution history into a smaller, high-signal prompt.

```rust
use axora_cache::{CompactorConfig, Context, ContextCompactor, ContextEntry, ItemKind};

let mut context = Context::new();
context.add_entry(
    ContextEntry::new("t-1", "user", "Initial requirements and constraints")
        .with_kind(ItemKind::Document),
);
context.add_entry(
    ContextEntry::new(
        "t-2",
        "assistant",
        "Critical architecture decision: required database migration before release.",
    )
    .with_kind(ItemKind::Decision)
    .with_priority(1.0),
);

let compactor = ContextCompactor::new(CompactorConfig::default());
let compacted = compactor.compact(&context)?;

println!("Saved {:.1}%", compacted.compression_ratio * 100.0);
println!("{}", compacted.content);
```

**Savings:** 60-80% | **Latency:** <5ms

---

## Performance Benchmarks

| Feature | Token Savings | Latency | Memory |
|---------|---------------|---------|--------|
| Prefix Caching | 50-90% | <1ms | ~100KB |
| Diff-Based | 89-98% | <10ms | ~50KB |
| Minification | 24-42% | <5ms | ~10KB |
| TOON | 50-60% | <2ms | ~5KB |
| Context Distribution | 50%+ | <5ms | ~200KB |
| Context Compacting | 60-80% | <5ms | ~50KB |
| **Combined** | **≥90%** | **<20ms** | **~400KB** |

### Combined Token Reduction

When all features are used together:

```
Original:     100,000 tokens
After Prefix:  50,000 tokens (50% savings)
After Diff:    10,000 tokens (80% additional)
After Minify:   6,000 tokens (40% additional)
After TOON:     3,000 tokens (50% additional)
After Context:  1,500 tokens (50% additional)
─────────────────────────────────────────────
Total Savings: 98.5% reduction
```

---

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
axora-cache = "0.1"
```

Basic example combining multiple features:

```rust
use axora_cache::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup cache
    let mut prefix_cache = PrefixCache::new(100);
    prefix_cache.add("system", "You are a Rust expert", 10);
    
    // Setup minifier
    let minifier = CodeMinifier::new();
    
    // Setup TOON serializer
    let json = r#"{"task": "implement", "priority": 1}"#;
    let schema = Schema::from_json_sample(json)?;
    let toon_serializer = ToonSerializer::new(schema);
    
    // Setup context manager
    let mut ctx_manager = ContextManager::new();
    ctx_manager.store_document(Document::new("reqs", "Requirements...", "spec"));
    
    // Use together for maximum savings
    let task = Task::new("task-1", vec!["reqs"], vec![], vec![]);
    let agent = Agent::new("developer");
    let ctx = ctx_manager.allocate(&task, &agent);
    
    Ok(())
}
```

---

## Documentation

Detailed guides for each feature:

- **[TOON Serialization](docs/TOON.md)** — Schema-based JSON alternative
- **[Code Minification](docs/MINIFIER.md)** — Whitespace removal and compression
- **[Context Distribution](docs/CONTEXT.md)** — Minimal context allocation
- **Context Compacting** — Rolling summary + hierarchical pruning for long histories

---

## Testing

Run all tests:

```bash
cargo test -p axora-cache
```

Run benchmarks:

```bash
cargo bench -p axora-cache
```

Run specific feature tests:

```bash
cargo test -p axora-cache toon
cargo test -p axora-cache context
cargo test -p axora-cache minifier
cargo test -p axora-cache context_compactor
```

---

## Architecture

```
axora-cache/
├── src/
│   ├── l1_cache.rs      # In-memory L1 cache (DashMap)
│   ├── l2_cache.rs      # RocksDB L2 cache
│   ├── l3_cache.rs      # Qdrant vector L3 cache
│   ├── prefix_cache.rs  # Prompt prefix caching
│   ├── diff.rs          # Diff-based communication
│   ├── minifier.rs      # Code minification
│   ├── toon.rs          # TOON serialization
│   ├── context.rs       # Context distribution
│   └── compactor.rs     # Context compaction
├── docs/
│   ├── TOON.md
│   ├── MINIFIER.md
│   └── CONTEXT.md
├── benches/
│   ├── token_savings.rs
│   └── performance.rs
└── tests/
    └── integration.rs
```

---

## Integration Guide

### For Agent A (Documentation)

```rust
use axora_cache::{PrefixCache, CachedPromptBuilder};

// Cache system prompts and common patterns
let mut cache = PrefixCache::new(100);
cache.add("rust-expert", "You are a Rust expert...", 10);

// Build prompts with cached prefixes
let mut builder = CachedPromptBuilder::new(&cache);
builder.add_cached("rust-expert");
builder.add_dynamic(&user_question);
let prompt = builder.build();
```

### For Agent C (Mission Decomposition)

```rust
use axora_cache::{ContextManager, Task, Agent};

let mut ctx_manager = ContextManager::new();

// For each decomposed task
for task in decomposed_tasks {
    let agent = select_agent(&task);
    let ctx = ctx_manager.allocate(&task, &agent);
    
    // Agent works with minimal context
    execute_task(&task, &agent, &ctx).await?;
}

// Calculate overall savings
let savings = ctx_manager.calculate_savings(full_context_tokens);
println!("Token savings: {:.1}%", savings.savings_percentage);
```

---

## License

MIT License — see [LICENSE](../../LICENSE) for details.

## Contributing

Contributions are welcome! Please read our [Contributing Guide](../../CONTRIBUTING.md) first.
