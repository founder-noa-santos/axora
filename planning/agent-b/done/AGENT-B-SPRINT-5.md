# Agent B — Sprint 5: TOON Serialization

**Sprint:** 5 of Phase 2  
**File:** `crates/axora-cache/src/toon.rs`  
**Estimated Time:** 8 hours  

---

## 🎯 Tarefa

Implementar TOON (Token-Optimized Object Notation) — formato serializado otimizado para LLMs.

### O Que É TOON?

**JSON (verboso):**
```json
[{"user_id": 12345, "username": "john_doe", "email": "john@example.com"}]
// ~50 tokens
```

**TOON (compacto):**
```
Schema: {0:user_id, 1:username, 2:email}
0:12345 1:john_doe 2:john@example.com
// ~15 tokens (70% reduction)
```

### Funcionalidades Requeridas

1. **Schema Definition**
   - Field names → field IDs (integers)
   - Reutilizar schema entre serializações

2. **TOON Encoder**
   - JSON → TOON
   - Tipos: string, number, boolean, null, array, object

3. **TOON Decoder**
   - TOON → JSON
   - Preservar tipos originais

4. **Token Estimation**
   - Calcular tokens saved vs JSON

---

## 📋 Critérios de Done

- [ ] Struct `ToonSerializer` implementada
- [ ] Método `encode(json: &str, schema: &Schema) -> String`
- [ ] Método `decode(toon: &str, schema: &Schema) -> String`
- [ ] 10+ testes unitários passando
- [ ] Benchmarks de economia de tokens
- [ ] Documentação em todos os públicos

---

## 📁 File Boundaries

**Editar APENAS:**
- `crates/axora-cache/src/toon.rs` (CRIAR)
- `crates/axora-cache/src/lib.rs` (adicionar módulo)

**NÃO editar:**
- Nenhum outro arquivo

---

## 🧪 10 Testes Requeridos

```rust
#[test]
fn test_simple_object_encode() { }

#[test]
fn test_simple_object_decode() { }

#[test]
fn test_nested_object_encode() { }

#[test]
fn test_array_encode() { }

#[test]
fn test_schema_reuse() { }

#[test]
fn test_roundtrip() { }

#[test]
fn test_token_savings() { }

#[test]
fn test_type_preservation() { }

#[test]
fn test_special_characters() { }

#[test]
fn test_large_payload() { }
```

---

## 📐 API Design

```rust
pub struct Schema {
    fields: HashMap<String, u8>,
}

pub struct ToonSerializer {
    schema: Schema,
}

pub struct ToonStats {
    pub original_tokens: usize,
    pub toon_tokens: usize,
    pub savings_percentage: f32,
}

impl Schema {
    pub fn new() -> Self;
    pub fn add_field(&mut self, name: &str) -> u8;
    pub fn from_json_sample(json: &str) -> Result<Self>;
}

impl ToonSerializer {
    pub fn new(schema: Schema) -> Self;
    pub fn encode(&self, json: &str) -> Result<String>;
    pub fn decode(&self, toon: &str) -> Result<String>;
    pub fn estimate_savings(&self, json: &str) -> ToonStats;
}
```

---

## 🚀 Passos

1. `cd /Users/noasantos/Downloads/axora`
2. Criar `crates/axora-cache/src/toon.rs`
3. Implementar Schema e ToonSerializer
4. Escrever 10 testes (TDD)
5. Implementar encode/decode
6. `cargo test -p axora-cache`
7. Atualizar `src/lib.rs`

---

## 📊 Success Metrics

- ✅ 10+ testes passando
- ✅ 50-60% token savings vs JSON
- ✅ Roundtrip JSON → TOON → JSON funciona
- ✅ Schema reutilizável

---

**Comece AGORA. Foque em testes e qualidade.**
