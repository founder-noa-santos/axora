# Agent A — Sprint 6: Documentation Management System

**Sprint:** 6 of Phase 2  
**File:** `crates/axora-docs/src/lib.rs` (NEW CRATE)  
**Estimated Time:** 8 hours  

---

## 🎯 Tarefa

Criar sistema de documentação **agent-native** — documentação que agents podem ler, escrever e atualizar automaticamente.

### Por Que Isso É Importante?

**Problema atual:**
- ❌ Documentação fica desatualizada
- ❌ Agents não conseguem atualizar docs
- ❌ Docs vivem em silos (README, docs/, inline comments)
- ❌ Sem feedback loop (código muda → docs não atualizam)

**Solução AXORA:**
- ✅ Docs estruturadas para agents (YAML/JSON schemas)
- ✅ Auto-update quando código muda
- ✅ Living documentation (sempre atualizada)
- ✅ Decision log (ADRs) automático

---

## 📋 Funcionalidades Requeridas

### 1. Doc Format & Schema

```rust
pub struct DocSchema {
    pub module: String,
    pub version: String,
    pub last_updated: u64,
    pub maintainer: String,
    pub sections: Vec<DocSection>,
}

pub enum DocSection {
    ApiContract { endpoints: Vec<Endpoint> },
    Architecture { decisions: Vec<Adn> },
    Patterns { examples: Vec<CodeExample> },
}
```

### 2. Doc Index & Retrieval

```rust
pub struct DocIndex {
    docs: HashMap<DocId, Document>,
    vector_index: VectorStore, // para semantic search
}

impl DocIndex {
    pub fn retrieve(&self, query: &str, context: &AgentContext) -> Vec<Document>;
}
```

### 3. Living Docs (Auto-Update)

```rust
pub struct LivingDocs {
    codebase: CodebaseIndex,
    docs: DocIndex,
}

impl LivingDocs {
    pub fn on_code_change(&mut self, file: &Path, old: &str, new: &str);
    // Detecta mudanças → atualiza docs afetadas
}
```

### 4. Decision Log (ADR System)

```rust
pub struct Adr {
    pub id: String,      // e.g., "AUTH-001"
    pub title: String,
    pub status: AdrStatus,
    pub context: String,
    pub decision: String,
    pub consequences: Vec<String>,
    pub related: Vec<String>, // IDs de ADRs relacionados
}
```

---

## 📁 File Boundaries

**Criar NOVO crate:**
- `crates/axora-docs/Cargo.toml` (criar)
- `crates/axora-docs/src/lib.rs` (criar)
- `crates/axora-docs/src/schema.rs` (criar)
- `crates/axora-docs/src/index.rs` (criar)
- `crates/axora-docs/src/living.rs` (criar)
- `crates/axora-docs/src/adr.rs` (criar)

**NÃO editar:**
- `crates/axora-cache/` (Agent B está trabalhando)
- `crates/axora-agents/` (Agent C está trabalhando)

---

## 🧪 10 Testes Requeridos

```rust
#[test]
fn test_doc_schema_creation() { }

#[test]
fn test_doc_index_add_and_retrieve() { }

#[test]
fn test_doc_semantic_search() { }

#[test]
fn test_living_docs_code_change_detection() { }

#[test]
fn test_living_docs_auto_update() { }

#[test]
fn test_adr_creation() { }

#[test]
fn test_adr_linking() { }

#[test]
fn test_doc_versioning() { }

#[test]
fn test_doc_staleness_detection() { }

#[test]
fn test_full_workflow() { }
```

---

## 📐 API Design

### `src/lib.rs`

```rust
pub mod schema;
pub mod index;
pub mod living;
pub mod adr;

pub use schema::{DocSchema, DocSection, Document};
pub use index::{DocIndex, DocQuery};
pub use living::LivingDocs;
pub use adr::{Adr, AdrStatus};
```

### `src/schema.rs`

```rust
pub struct Document {
    pub id: String,
    pub schema: DocSchema,
    pub content: String,
    pub created_at: u64,
    pub updated_at: u64,
}

pub struct DocSchema {
    pub module: String,
    pub version: String,
    pub last_updated: u64,
    pub maintainer: String,
    pub sections: Vec<DocSection>,
}

pub enum DocSection {
    ApiContract { endpoints: Vec<Endpoint> },
    Architecture { decisions: Vec<String> },
    Patterns { examples: Vec<CodeExample> },
    Tests { test_cases: Vec<TestCase> },
}
```

### `src/index.rs`

```rust
pub struct DocIndex {
    docs: HashMap<String, Document>,
    // Future: vector_index para semantic search
}

impl DocIndex {
    pub fn new() -> Self;
    pub fn add(&mut self, doc: Document);
    pub fn retrieve(&self, query: &str) -> Vec<&Document>;
    pub fn find_stale(&self, max_age_days: u64) -> Vec<&Document>;
}
```

### `src/living.rs`

```rust
pub struct LivingDocs {
    docs: DocIndex,
    codebase_hash: HashMap<PathBuf, String>, // file → hash
}

impl LivingDocs {
    pub fn new() -> Self;
    
    pub fn on_code_change(
        &mut self, 
        file: &Path, 
        old_content: &str, 
        new_content: &str
    ) -> Vec<DocUpdate>;
    
    pub fn flag_for_review(&mut self, doc_id: &str, reason: &str);
}

pub struct DocUpdate {
    pub doc_id: String,
    pub update_type: UpdateType,
    pub suggested_changes: String,
}

pub enum UpdateType {
    AutoUpdate,      // Safe to auto-apply
    FlagForReview,   // Needs human/agent review
    Deprecate,       // Doc is now obsolete
}
```

### `src/adr.rs`

```rust
pub struct Adr {
    pub id: String,
    pub title: String,
    pub status: AdrStatus,
    pub context: String,
    pub decision: String,
    pub consequences: Vec<String>,
    pub related: Vec<String>,
    pub created_at: u64,
}

pub enum AdrStatus {
    Proposed,
    Accepted,
    Deprecated,
    Superseded,
}

pub struct AdrLog {
    adrs: HashMap<String, Adr>,
}

impl AdrLog {
    pub fn new() -> Self;
    pub fn add(&mut self, adr: Adr);
    pub fn link(&mut self, adr_id: &str, related_id: &str);
    pub fn get(&self, adr_id: &str) -> Option<&Adr>;
}
```

---

## 🚀 Passos

1. **Criar estrutura do crate:**
   ```bash
   cd crates
   mkdir axora-docs
   cd axora-docs
   mkdir src
   ```

2. **Criar `Cargo.toml`:**
   ```toml
   [package]
   name = "axora-docs"
   version.workspace = true
   edition.workspace = true
   
   [dependencies]
   axora-indexing.workspace = true
   serde.workspace = true
   serde_json.workspace = true
   tracing.workspace = true
   ```

3. **Implementar módulos:**
   - `src/lib.rs` (exports)
   - `src/schema.rs` (DocSchema, Document)
   - `src/index.rs` (DocIndex, retrieval)
   - `src/living.rs` (LivingDocs, auto-update)
   - `src/adr.rs` (Adr, AdrLog)

4. **Escrever 10 testes** (TDD)

5. **Rodar testes:**
   ```bash
   cargo test -p axora-docs
   ```

6. **Atualizar workspace:**
   - Adicionar `axora-docs` ao `Cargo.toml` root

---

## 📊 Success Metrics

- ✅ 10+ testes passando
- ✅ DocSchema funciona para 3+ tipos de docs
- ✅ DocIndex retrieve funciona
- ✅ LivingDocs detecta mudanças no código
- ✅ ADR system cria e linka decisions
- ✅ Zero conflitos com outros agents (crates isolados)

---

## 🔗 Contexto

**Sprint Anterior:** Sprint 3 (Code Minification) — ✅ COMPLETO  
**Próximo Sprint:** Sprint 6b (Doc RAG) ou Sprint 6c (Integration)

**Integração Futura:**
- Este crate será usado pelo RAG pipeline para retriever docs
- LivingDocs vai integrar com Merkle tree (axora-indexing)
- ADRs vão integrar com audit logging (axora-agents)

---

## 💡 Dicas

1. **Comece pelo schema:** Defina estruturas de dados primeiro
2. **Testes primeiro:** Escreva 1-2 testes antes de implementar
3. **Mantenha simples:** Não adicione vector search ainda (só HashMap)
4. **Pense em agents:** Como um agent vai ler/escrever essas docs?

---

## 🎯 Exemplo de Uso (Futuro)

```rust
// Agent cria documentação
let mut docs = LivingDocs::new();
docs.add_document(Document {
    id: "auth-api".to_string(),
    schema: DocSchema {
        module: "auth".to_string(),
        version: "1.0".to_string(),
        // ...
    },
    // ...
});

// Código muda → docs atualizam automaticamente
docs.on_code_change(
    Path::new("src/auth/login.rs"),
    old_code,
    new_code
);
// Retorna: Vec<DocUpdate> com sugestões de atualização

// Agent cria ADR
let mut adr_log = AdrLog::new();
adr_log.add(Adr {
    id: "AUTH-001".to_string(),
    title: "Use JWT for session management".to_string(),
    status: AdrStatus::Accepted,
    // ...
});
```

---

**Comece AGORA. Foque em criar um crate limpo e bem testado.**

**Este é um crate NOVO — zero risco de conflito com outros agents!**
