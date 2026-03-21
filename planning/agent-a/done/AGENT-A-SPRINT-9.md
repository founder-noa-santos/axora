# Agent A — Sprint 9: Phase 2 Integration & Benchmarking

**Sprint:** 9 of Phase 2  
**File:** `crates/openakta-cache/benches/` (NOVO) + Integration tests  
**Estimated Time:** 8 horas  

---

## 🎯 Tarefa

Fazer **integração e benchmarking** de TODOS os sprints da Phase 2 para validar que funcionam juntos e atingem as metas de performance.

### Por Que Isso É Importante?

**Problema:**
- ❌ Cada sprint funciona isoladamente
- ❌ Não sabemos se funcionam juntos
- ❌ Sem benchmarks reais de token savings
- ❌ Sem validação end-to-end

**Solução:**
- ✅ Integration tests (todos os sprints juntos)
- ✅ Benchmarks de token savings
- ✅ Validação de metas (90% reduction)
- ✅ Performance profiling

---

## 📋 Sprints para Integrar

| Sprint | Feature | Status |
|--------|---------|--------|
| **Sprint 1** | Prefix Caching | ✅ Complete |
| **Sprint 2** | Diff-Based Communication | ✅ Complete |
| **Sprint 3** | Code Minification | ✅ Complete |
| **Sprint 5** | TOON Serialization | ✅ Complete |
| **Sprint 6** | Documentation Management | ✅ Complete (YOU!) |
| **Sprint 7** | Task Decomposition | 🔄 Agent C |
| **Sprint 8** | Context Distribution | 🔄 Agent B |

---

## 🧪 Integration Tests Required

### 1. Full Token Optimization Pipeline

```rust
#[test]
fn test_full_token_optimization_pipeline() {
    // 1. Start with verbose JSON + full code
    let json_data = r#"{"users": [{"id": 1, "name": "john"}]}"#;
    let code = r#"fn authenticate(user: &str, pass: &str) -> bool { ... }"#;
    
    // 2. Apply TOON serialization
    let toon = toon_serializer.encode(json_data).unwrap();
    
    // 3. Apply code minification
    let minified = minifier.minify(code, "rust").unwrap();
    
    // 4. Apply diff-based communication (simulate change)
    let diff = UnifiedDiff::generate(code, &minified.content, "old.rs", "new.rs");
    
    // 5. Calculate total savings
    let original_tokens = estimate_tokens(&json_data) + estimate_tokens(&code);
    let optimized_tokens = estimate_tokens(&toon) + estimate_tokens(&diff.to_string());
    let savings = (original_tokens - optimized_tokens) as f32 / original_tokens as f32;
    
    // ASSERT: >= 90% savings
    assert!(savings >= 0.90, "Expected 90% savings, got {:.1}%", savings * 100.0);
}
```

### 2. Documentation + Living Docs Integration

```rust
#[test]
fn test_living_docs_with_code_change() {
    // 1. Create living docs
    let mut living_docs = LivingDocs::new();
    
    // 2. Register code file
    living_docs.register_file("auth.rs", code);
    
    // 3. Create associated documentation
    living_docs.add_document(doc);
    
    // 4. Simulate code change
    let new_code = modified_code;
    living_docs.on_code_change("auth.rs", code, &new_code);
    
    // 5. Check if docs were flagged for update
    let updates = living_docs.get_pending_updates();
    
    // ASSERT: At least 1 doc flagged
    assert!(updates.len() >= 1);
}
```

### 3. Context Distribution + Task Decomposition

```rust
#[test]
fn test_context_distribution_with_decomposition() {
    // 1. Decompose mission
    let decomposed = decomposer.decompose("Implement auth system")?;
    
    // 2. Allocate contexts for each task
    for task_id in decomposed.parallel_groups[0] {
        let task = &decomposed.tasks[task_id];
        let ctx = context_manager.allocate(task, &agent);
        
        // ASSERT: Context is minimal (< 50% of full context)
        assert!(ctx.token_count() < full_context.token_count() * 0.5);
    }
}
```

### 4. End-to-End Workflow

```rust
#[test]
fn test_full_phase2_workflow() {
    // Simulate full Phase 2 workflow:
    // 1. User gives mission
    // 2. Decompose into tasks
    // 3. Allocate contexts
    // 4. Execute concurrently
    // 5. Update docs
    // 6. Measure token savings
    
    // ASSERT: All steps work together
    // ASSERT: Token savings >= 90%
    // ASSERT: Execution time < sequential / 3
}
```

---

## 📊 Benchmarks Required

### 1. Token Savings Benchmark

```rust
// benches/token_savings.rs

use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_json_vs_toon(c: &mut Criterion) {
    let json = load_sample_json();
    let schema = Schema::from_json_sample(&json);
    let serializer = ToonSerializer::new(schema);
    
    c.bench_function("json_vs_toon_savings", |b| {
        b.iter(|| {
            let toon = serializer.encode(&json).unwrap();
            let savings = calculate_savings(&json, &toon);
            assert!(savings > 0.50); // >50% savings
        })
    });
}

fn benchmark_code_minification(c: &mut Criterion) {
    let code = load_sample_code();
    let minifier = CodeMinifier::new();
    
    c.bench_function("code_minification_savings", |b| {
        b.iter(|| {
            let minified = minifier.minify(&code, "rust").unwrap();
            let savings = minified.savings_percentage();
            assert!(savings > 0.20); // >20% savings
        })
    });
}

fn benchmark_diff_communication(c: &mut Criterion) {
    let old_code = load_sample_code();
    let new_code = load_modified_code();
    
    c.bench_function("diff_savings", |b| {
        b.iter(|| {
            let diff = UnifiedDiff::generate(&old_code, &new_code, "old.rs", "new.rs");
            let savings = calculate_diff_savings(&old_code, &diff);
            assert!(savings > 0.80); // >80% savings for small changes
        })
    });
}

criterion_group!(benches, 
    benchmark_json_vs_toon, 
    benchmark_code_minification, 
    benchmark_diff_communication
);
criterion_main!(benches);
```

### 2. Performance Benchmark

```rust
// benches/performance.rs

use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;

fn benchmark_prefix_cache(c: &mut Criterion) {
    let mut cache = PrefixCache::new(100);
    cache.add("system", "You are a helpful assistant", 10);
    
    c.bench_function("prefix_cache_hit", |b| {
        b.iter(|| {
            let key = cache.compute_cache_key("You are a helpful assistant");
            cache.get(&key).unwrap();
        })
    });
}

fn benchmark_living_docs(c: &mut Criterion) {
    let mut docs = LivingDocs::new();
    docs.register_file("test.rs", "fn test() {}");
    
    c.bench_function("living_docs_code_change", |b| {
        b.iter(|| {
            docs.on_code_change("test.rs", "fn test() {}", "fn test() { /* new */ }");
        })
    });
}

fn benchmark_context_allocation(c: &mut Criterion) {
    let mut ctx_manager = ContextManager::new();
    let task = Task::new("Implement login");
    let agent = Agent::new("coder-1");
    
    c.bench_function("context_allocation", |b| {
        b.iter(|| {
            ctx_manager.allocate(&task, &agent);
        })
    });
}
```

---

## 📁 File Boundaries

**Criar:**
- `crates/openakta-cache/tests/integration.rs` (NOVO)
- `crates/openakta-cache/benches/token_savings.rs` (NOVO)
- `crates/openakta-cache/benches/performance.rs` (NOVO)
- `crates/openakta-agents/tests/integration.rs` (NOVO)

**Editar:**
- `crates/openakta-cache/Cargo.toml` (add benchmark deps)
- `crates/openakta-agents/Cargo.toml` (add benchmark deps)

**NÃO editar:**
- Implementações existentes (só testes/benchmarks)

---

## 🚀 Passos

1. **Criar integration tests:**
   - `tests/integration.rs` em openakta-cache
   - `tests/integration.rs` em openakta-agents
   - 4+ integration tests (veja exemplos acima)

2. **Criar benchmarks:**
   - `benches/token_savings.rs`
   - `benches/performance.rs`
   - Adicionar criterion ao Cargo.toml

3. **Rodar integration tests:**
   ```bash
   cargo test --test integration --workspace
   ```

4. **Rodar benchmarks:**
   ```bash
   cargo bench --workspace
   ```

5. **Validar metas:**
   - Token savings >= 90%
   - Performance dentro do esperado
   - Zero regressões

---

## 📊 Success Metrics

- ✅ 4+ integration tests passando
- ✅ 3+ benchmarks rodando
- ✅ Token savings >= 90% (validado)
- ✅ Performance dentro do esperado
- ✅ Zero regressões em outros crates
- ✅ Relatório de benchmarks gerado

---

## 📈 Expected Results

### Token Savings (Meta: 90%)

| Optimization | Individual | Combined |
|--------------|------------|----------|
| Prefix Caching | 50-90% | - |
| Diff-Based | 89-98% | - |
| Code Minification | 24-42% | - |
| TOON Serialization | 50-60% | - |
| **Combined** | - | **>= 90%** ✅ |

### Performance (Meta: 3-5x speedup)

| Metric | Sequential | Concurrent | Target |
|--------|------------|------------|--------|
| Execution time | 100% | 20-33% | 3-5x faster ✅ |
| Context per agent | 100% | 50% | 50% reduction ✅ |
| Token cost | 100% | 10% | 90% reduction ✅ |

---

## 🔗 Contexto

**Sprint Anterior:** Sprint 6 (Documentation) — ✅ **APROVADO**  
**Próximo Sprint:** Phase 3 (Desktop App) ou Phase 4 (Self-Orchestration)

**Integração:**
- ✅ Usa TODOS os sprints anteriores
- ✅ Valida Phase 2 completa
- ✅ Prepara para Phase 3

---

## ⚠️ Importante: Este Sprint é o "Final Boss"

**Por que:**
1. **Valida TODO o trabalho da Phase 2**
2. **Prova que 90% savings é atingível**
3. **Prepara base para Phase 3 (Desktop)**
4. **Gera dados reais para marketing**

**Não é só mais um sprint — é a VALIDAÇÃO FINAL da Phase 2.**

---

**Comece AGORA. Foque em integração real e benchmarks precisos.**

**Use dados reais (código do OPENAKTA, JSONs reais, etc.).**
