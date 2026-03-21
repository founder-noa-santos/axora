# Agent B — Sprint 8: Context Distribution System

**Sprint:** 8 of Phase 2  
**File:** `crates/openakta-cache/src/context.rs` (NOVO)  
**Estimated Time:** 8 horas  

---

## 🎯 Tarefa

Implementar sistema de **distribuição inteligente de contexto** para agents — cada agent recebe apenas o contexto necessário para sua tarefa.

### Por Que Isso É Importante?

**Problema atual:**
- ❌ Agents recebem contexto completo da missão
- ❌ Contexto enche rápido (token waste)
- ❌ Agents se perdem em informação irrelevante
- ❌ Custo desnecessário

**Solução OPENAKTA:**
- ✅ Contexto **mínimo necessário** por task
- ✅ Contexto **compartilhado** apenas quando necessário
- ✅ Contexto **sob demanda** (pull-based)
- ✅ **50% menos tokens** por agent

---

## 📋 Funcionalidades Requeridas

### 1. Context Manager

```rust
pub struct ContextManager {
    shared_context: SharedContext,      // contexto global
    task_contexts: HashMap<TaskId, TaskContext>, // contexto por task
}

pub struct TaskContext {
    required_docs: Vec<DocId>,
    required_code: Vec<FileId>,
    related_tasks: Vec<TaskId>,
    agent_state: AgentState,
}
```

### 2. Context Allocation

```rust
impl ContextManager {
    pub fn allocate(&mut self, task: &Task, agent: &Agent) -> TaskContext;
    // Aloca contexto MÍNIMO necessário para esta task
}
```

### 3. Context Retrieval (Pull-Based)

```rust
impl TaskContext {
    pub fn get_doc(&self, doc_id: &DocId) -> Option<&Document>;
    pub fn get_code(&self, file_id: &FileId) -> Option<&str>;
    pub fn get_related_task(&self, task_id: &TaskId) -> Option<&TaskResult>;
}
```

### 4. Context Merging (Para Tasks Dependentes)

```rust
impl TaskContext {
    pub fn merge(&mut self, other: &TaskContext);
    // Merge de contextos quando task depende de outra
}
```

---

## 📁 File Boundaries

**Criar:**
- `crates/openakta-cache/src/context.rs` (NOVO)
- Atualizar `crates/openakta-cache/src/lib.rs` (exports)

**NÃO editar:**
- `crates/openakta-agents/` (Agent C está aqui)
- `crates/openakta-docs/` (Agent A está aqui)

---

## 🧪 10 Testes Requeridos

```rust
#[test]
fn test_context_allocation_minimal() { }

#[test]
fn test_context_allocation_with_dependencies() { }

#[test]
fn test_context_pull_based_retrieval() { }

#[test]
fn test_context_merge() { }

#[test]
fn test_context_token_savings() { }

#[test]
fn test_context_shared_vs_private() { }

#[test]
fn test_context_update_on_code_change() { }

#[test]
fn test_context_cleanup() { }

#[test]
fn test_context_with_living_docs() { }

#[test]
fn test_full_workflow() { }
```

---

## 📐 API Design

### `src/context.rs`

```rust
pub struct ContextManager {
    shared_context: SharedContext,
    task_contexts: HashMap<String, TaskContext>,
    doc_index: DocIndex, // do Sprint 6 (Agent A)
    code_index: CodebaseIndex,
}

pub struct SharedContext {
    global_docs: Vec<DocId>,
    global_code: Vec<FileId>,
    decisions: Vec<Adn>, // do Sprint 6 (Agent A)
}

pub struct TaskContext {
    task_id: String,
    agent_id: String,
    required_docs: Vec<DocId>,
    required_code: Vec<FileId>,
    related_tasks: Vec<String>,
    created_at: u64,
    last_accessed: u64,
}

impl ContextManager {
    pub fn new() -> Self;
    
    pub fn allocate(&mut self, task: &Task, agent: &Agent) -> TaskContext;
    
    pub fn get_shared(&self) -> &SharedContext;
    
    pub fn get_task_context(&self, task_id: &str) -> Option<&TaskContext>;
    
    pub fn cleanup(&mut self, max_age_hours: u64);
}

impl TaskContext {
    pub fn merge(&mut self, other: &TaskContext);
    
    pub fn token_count(&self) -> usize;
    
    pub fn is_stale(&self, max_age_hours: u64) -> bool;
}
```

---

## 🚀 Passos

1. **Criar `src/context.rs`:**
   - `ContextManager` struct
   - `TaskContext` struct
   - `SharedContext` struct
   - Métodos de allocate, merge, cleanup

2. **Escrever 10 testes** (TDD)

3. **Rodar testes:**
   ```bash
   cargo test -p openakta-cache
   ```

4. **Atualizar `lib.rs`:**
   ```rust
   pub mod context;
   pub use context::{ContextManager, TaskContext, SharedContext};
   ```

---

## 📊 Success Metrics

- ✅ 10+ testes passando
- ✅ Contexto alocado é **50% menor** que contexto completo
- ✅ Pull-based retrieval funciona
- ✅ Merge de contextos dependentes funciona
- ✅ Cleanup de contextos stale funciona
- ✅ Zero conflitos com outros agents

---

## 💡 Exemplo de Uso (Futuro)

```rust
// Mission decomposed (Sprint 7, Agent C)
let decomposed = decomposer.decompose(mission)?;

// Context manager
let mut ctx_manager = ContextManager::new();

// For each parallel group
for group in decomposed.parallel_groups {
    for task_id in group {
        let task = decomposed.tasks.get(task_id).unwrap();
        let agent = select_agent(task);
        
        // Allocate MINIMAL context for this task
        let ctx = ctx_manager.allocate(task, &agent);
        
        // Agent works with minimal context
        // (50% less tokens than full context)
        let result = agent.execute(task, &ctx).await?;
    }
}
```

---

## 🔗 Contexto

**Sprint Anterior:** Sprint 5 (TOON Serialization) — ✅ APROVADO  
**Próximo Sprint:** Sprint 9 (Integration) ou Sprint 10 (Benchmarking)

**Integração Futura:**
- Vai usar TOON (Sprint 5) para serializar contexto
- Vai usar Doc Index (Sprint 6, Agent A) para docs
- Vai usar Mission Decomposition (Sprint 7, Agent C) para tasks

---

**Comece AGORA. Foque em alocação mínima e pull-based retrieval.**

**Este sprint é CRÍTICO para reduzir custos de tokens por agent.**
