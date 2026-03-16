# Agent A — Sprint 3: Code Minification

**Sprint:** 3 of Phase 2  
**File:** `crates/axora-cache/src/minifier.rs`  
**Estimated Time:** 8 hours  

---

## 🎯 Tarefa

Implementar code minification para redução de tokens em código enviado para LLMs.

### Funcionalidades Requeridas

1. **Whitespace Removal** (24-42% savings)
   - Remover espaços em branco desnecessários
   - Manter indentação mínima (opcional)

2. **Identifier Compression**
   - Mapear identificadores longos → curtos
   - Manter bidirectional mapping (compress/decompress)
   - Exemplo: `calculateMonthlyRevenueMetrics` → `a1`

3. **Comment Stripping** (10-20% savings)
   - Remover comentários: `//`, `/* */`, `#`, `"""`
   - Manter docstrings? (configurável)

4. **Language Support**
   - Rust (.rs), TypeScript (.ts, .tsx, .js, .jsx)
   - Python (.py), Go (.go)

---

## 📋 Critérios de Done

- [ ] Struct `CodeMinifier` implementada
- [ ] Método `minify(code: &str, language: &str) -> MinifiedCode`
- [ ] Método `decompress(minified: &MinifiedCode) -> String`
- [ ] 10+ testes unitários passando
- [ ] Benchmarks de economia de tokens
- [ ] Documentação em todos os públicos

---

## 📁 File Boundaries

**Editar APENAS:**
- `crates/axora-cache/src/minifier.rs` (CRIAR)
- `crates/axora-cache/src/lib.rs` (adicionar módulo)

**NÃO editar:**
- Nenhum outro arquivo

---

## 🧪 10 Testes Requeridos

```rust
#[test]
fn test_whitespace_removal() { }

#[test]
fn test_identifier_compression() { }

#[test]
fn test_comment_stripping_rust() { }

#[test]
fn test_comment_stripping_typescript() { }

#[test]
fn test_comment_stripping_python() { }

#[test]
fn test_roundtrip_decompress() { }

#[test]
fn test_token_savings() { }

#[test]
fn test_language_detection() { }

#[test]
fn test_preserve_strings() { }

#[test]
fn test_preserve_keywords() { }
```

---

## 📐 API Design

```rust
pub struct CodeMinifier { /* config */ }

pub struct MinifiedCode {
    pub content: String,
    pub identifier_map: HashMap<String, String>,
    pub original_length: usize,
    pub minified_length: usize,
    pub savings_percentage: f32,
}

impl CodeMinifier {
    pub fn new() -> Self;
    pub fn minify(&self, code: &str, language: &str) -> Result<MinifiedCode>;
    pub fn decompress(&self, minified: &MinifiedCode) -> Result<String>;
    pub fn estimate_tokens(code: &str) -> usize;
}
```

---

## 🚀 Passos

1. `cd /Users/noasantos/Downloads/axora`
2. Criar `crates/axora-cache/src/minifier.rs`
3. Implementar structs
4. Escrever 10 testes (TDD)
5. Implementar funcionalidades
6. `cargo test -p axora-cache`
7. Atualizar `src/lib.rs`

---

## 📊 Success Metrics

- ✅ 10+ testes passando
- ✅ 20-40% economia em código real
- ✅ Roundtrip funciona
- ✅ 4+ linguagens suportadas

---

**Comece AGORA. Foque em testes e qualidade.**
