# Agent C — Sprint 7: Task Decomposition & Concurrency

**Sprint:** 7 of Phase 2  
**File:** `crates/axora-agents/src/decomposer.rs` (NOVO)  
**Estimated Time:** 8 horas  

---

## 🎯 Tarefa

Implementar sistema de **decomposição automática de missões** em tarefas concorrentes.

### Por Que Isso É Importante?

**Problema atual:**
- ❌ Usuário dá missão complexa → um único agent trabalha
- ❌ Tudo sequencial, lento
- ❌ Contexto enche rápido
- ❌ Não escala

**Solução AXORA:**
- ✅ Missão → decomposta em tarefas independentes
- ✅ Múltiplos agents trabalham **concorrentemente**
- ✅ Cada agent tem **contexto mínimo** necessário
- ✅ **3-5x mais rápido**

---

## 📋 Funcionalidades Requeridas

### 1. Mission Decomposer

```rust
pub struct MissionDecomposer {
    llm: LLM, // futuro: integrar com axora-agents
    rules: Vec<DecompositionRule>,
}

pub struct DecomposedMission {
    pub tasks: Vec<Task>,
    pub dependencies: Vec<Dependency>,
    pub critical_path: Vec<TaskId>,
    pub parallel_groups: Vec<Vec<TaskId>>, // tarefas que podem rodar concorrentemente
}
```

### 2. Dependency Graph

```rust
pub struct Dependency {
    pub from: TaskId, // task that depends
    pub to: TaskId,   // task that must complete first
    pub dep_type: DependencyType,
}

pub enum DependencyType {
    Hard,    // Must wait (blocking)
    Soft,    // Should wait (can proceed with risk)
    Data,    // Needs output data from other task
}
```

### 3. Parallel Group Executor

```rust
pub struct ConcurrentExecutor {
    agents: Vec<AgentId>,
    tasks: Vec<Task>,
}

impl ConcurrentExecutor {
    pub async fn execute_parallel(&self, group: &[TaskId]) -> Vec<TaskResult>;
}
```

---

## 📁 File Boundaries

**Criar:**
- `crates/axora-agents/src/decomposer.rs` (NOVO)
- `crates/axora-agents/src/executor.rs` (NOVO)
- Atualizar `crates/axora-agents/src/lib.rs` (exports)

**NÃO editar:**
- `crates/axora-cache/` (Agent B está aqui)
- `crates/axora-docs/` (Agent A está aqui)

---

## 🧪 10 Testes Requeridos

```rust
#[test]
fn test_decompose_simple_mission() { }

#[test]
fn test_decompose_complex_mission() { }

#[test]
fn test_identify_dependencies() { }

#[test]
fn test_parallel_groups() { }

#[test]
fn test_critical_path() { }

#[test]
fn test_concurrent_execution() { }

#[test]
fn test_dependency_hard_vs_soft() { }

#[test]
fn test_task_assignment_to_agents() { }

#[test]
fn test_mission_with_cross_domain_tasks() { }

#[test]
fn test_full_workflow() { }
```

---

## 📐 API Design

### `src/decomposer.rs`

```rust
pub struct MissionDecomposer {
    rules: Vec<DecompositionRule>,
}

pub struct DecompositionRule {
    pub pattern: String, // regex ou pattern matching
    pub template: MissionTemplate,
}

pub struct MissionTemplate {
    pub task_templates: Vec<TaskTemplate>,
    pub default_dependencies: Vec<(usize, usize)>, // (from, to)
}

impl MissionDecomposer {
    pub fn new() -> Self;
    
    pub fn decompose(&self, mission: &str) -> Result<DecomposedMission>;
    
    pub fn add_rule(&mut self, rule: DecompositionRule);
}

pub struct DecomposedMission {
    pub tasks: Vec<Task>,
    pub dependencies: Vec<Dependency>,
    pub critical_path: Vec<usize>, // indices das tasks no critical path
    pub parallel_groups: Vec<Vec<usize>>, // grupos de tasks que podem rodar concorrentemente
}
```

### `src/executor.rs`

```rust
pub struct ConcurrentExecutor {
    state_machine: Arc<Mutex<StateMachine>>,
}

impl ConcurrentExecutor {
    pub fn new(state_machine: StateMachine) -> Self;
    
    pub async fn execute_group(&self, task_ids: &[usize], mission: &DecomposedMission) -> Vec<TaskResult>;
    
    pub async fn execute_all(&self, mission: &DecomposedMission) -> MissionResult;
}

pub struct MissionResult {
    pub success: bool,
    pub task_results: HashMap<usize, TaskResult>,
    pub total_time: Duration,
    pub parallelization_factor: f32, // quanto > 1, mais paralelismo
}
```

---

## 🚀 Passos

1. **Criar `src/decomposer.rs`:**
   - `MissionDecomposer` struct
   - `DecomposedMission` struct
   - `Dependency` enum
   - Método `decompose()`

2. **Criar `src/executor.rs`:**
   - `ConcurrentExecutor` struct
   - Método `execute_group()`
   - Método `execute_all()`

3. **Escrever 10 testes** (TDD)

4. **Rodar testes:**
   ```bash
   cargo test -p axora-agents
   ```

5. **Atualizar `lib.rs`:**
   ```rust
   pub mod decomposer;
   pub mod executor;
   ```

---

## 📊 Success Metrics

- ✅ 10+ testes passando
- ✅ Decompõe missões em 3+ parallel groups
- ✅ Identifica dependencies corretamente
- ✅ Critical path calculado
- ✅ Execução concorrente funciona
- ✅ Zero conflitos com outros agents

---

## 💡 Exemplo de Uso (Futuro)

```rust
// Usuário dá missão
let mission = "Implement authentication system with login, signup, and JWT";

// Decompor
let decomposer = MissionDecomposer::new();
let decomposed = decomposer.decompose(mission)?;

// Resultado:
// parallel_groups: [
//   [0, 1, 2],  // Group 1: Design schema, Research best practices, Set up structure
//   [3, 4, 5],  // Group 2: Implement user model, JWT utils, Write tests
//   [6, 7],     // Group 3: Login endpoint, Signup endpoint
// ]

// Executar concorrentemente
let executor = ConcurrentExecutor::new(state_machine);
let result = executor.execute_all(&decomposed).await;

// 3-5x mais rápido que sequencial!
```

---

## 🔗 Contexto

**Sprint Anterior:** Sprint 3b (Heartbeat) — ✅ COMPLETO  
**Próximo Sprint:** Sprint 8 (Context Distribution) ou Sprint 9 (Integration)

**Integração Futura:**
- Vai usar Heartbeat (Sprint 3b) para gerenciar agents
- Vai integrar com DDD Agent Teams (se R-10 validar)
- Vai usar Documentation system (Sprint 6) para contexto

---

**Comece AGORA. Foque em decomposição e execução concorrente.**

**Este sprint é CRÍTICO para o diferencial do AXORA (concorrência real).**
