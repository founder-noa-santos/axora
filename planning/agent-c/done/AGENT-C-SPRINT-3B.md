# Agent C — Sprint 3b: Heartbeat System

**Sprint:** 3b of Phase 2  
**File:** `crates/axora-agents/src/heartbeat.rs`  
**Estimated Time:** 8 hours  

---

## 🎯 Tarefa

Implementar Heartbeat System para gerenciamento de lifecycle de agents.

### Por Que Heartbeat?

**Sem heartbeat:**
- Agents em `Idle` ficam em memória
- Recursos desperdiçados
- Não escala para 20+ agents

**Com heartbeat:**
- Agents persistem estado e "dormem"
- Acordam por timer OU evento
- ~60-80% economia de memória

### Funcionalidades Requeridas

1. **Heartbeat Timer**
   - Agents acordam periodicamente (30s default)
   - Checam se há trabalho
   - Se não há, voltam a dormir

2. **Event-Driven Wake**
   - Agents acordam por eventos (mensagens)
   - Mais eficiente que polling

3. **Hybrid Approach**
   - Timer como fallback
   - Eventos como primary

4. **State Persistence**
   - Salvar estado antes de dormir
   - Carregar estado ao acordar

---

## 📋 Critérios de Done

- [ ] Struct `Heartbeat` implementada
- [ ] Método `schedule_wake(agent_id: &str, interval: Duration)`
- [ ] Método `wake_on_event(agent_id: &str, event: &Event)`
- [ ] Integração com `StateMachine` (já existe)
- [ ] 10+ testes unitários passando
- [ ] Documentação em todos os públicos

---

## 📁 File Boundaries

**Editar APENAS:**
- `crates/axora-agents/src/heartbeat.rs` (CRIAR)
- `crates/axora-agents/src/state_machine.rs` (pequena integração)
- `crates/axora-agents/src/lib.rs` (adicionar módulo)

**NÃO editar:**
- Nenhum outro arquivo

---

## 🧪 10 Testes Requeridos

```rust
#[test]
fn test_heartbeat_creation() { }

#[test]
fn test_schedule_wake() { }

#[test]
fn test_wake_on_event() { }

#[test]
fn test_state_persistence() { }

#[test]
fn test_hybrid_timer_and_event() { }

#[test]
fn test_agent_sleep() { }

#[test]
fn test_memory_savings() { }

#[test]
fn test_concurrent_heartbeats() { }

#[test]
fn test_stuck_agent_recovery() { }

#[test]
fn test_heartbeat_with_state_machine() { }
```

---

## 📐 API Design

```rust
pub struct Heartbeat {
    timer_tx: mpsc::Sender<HeartbeatMessage>,
    event_tx: mpsc::Sender<HeartbeatMessage>,
}

pub enum HeartbeatMessage {
    ScheduleWake { agent_id: String, interval: Duration },
    WakeNow { agent_id: String },
    Event { agent_id: String, event: Event },
}

pub struct HeartbeatConfig {
    pub default_interval: Duration,
    pub max_sleep_time: Duration,
    pub stuck_threshold: u32,
}

impl Heartbeat {
    pub fn new(config: HeartbeatConfig) -> Self;
    pub async fn schedule_wake(&self, agent_id: &str, interval: Duration);
    pub async fn wake_on_event(&self, agent_id: &str, event: &Event);
    pub async fn run(&self, state_machine: &mut StateMachine);
}
```

### Integração com StateMachine

```rust
// Em state_machine.rs
impl StateMachine {
    pub fn transition_to_idle(&mut self, agent_id: &str) -> Result<()> {
        self.persist_agent_state(agent_id)?;
        self.heartbeat.schedule_wake(
            agent_id, 
            Duration::from_secs(30)
        ).await;
        Ok(())
    }
}
```

---

## 🚀 Passos

1. `cd /Users/noasantos/Downloads/axora`
2. Criar `crates/axora-agents/src/heartbeat.rs`
3. Implementar Heartbeat struct e enums
4. Escrever 10 testes (TDD)
5. Implementar timer e event-driven wake
6. Integrar com StateMachine
7. `cargo test -p axora-agents`
8. Atualizar `src/lib.rs`

---

## 📊 Success Metrics

- ✅ 10+ testes passando
- ✅ Agents acordam por timer (30s)
- ✅ Agents acordam por evento
- ✅ Estado persiste entre sleep/wake
- ✅ 60-80% memória economizada

---

**Comece AGORA. Foque em testes e qualidade.**
