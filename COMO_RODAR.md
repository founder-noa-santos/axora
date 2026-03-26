# OPENAKTA - Como Rodar

## 🚀 Quick Start

### Rodar a CLI

```bash
# Opção 1: Usando o script de conveniência
./run-cli.sh do "sua missão aqui"
./run-cli.sh doc

# Opção 2: Usando cargo diretamente
cargo run --bin openakta -- do "sua missão aqui"
cargo run --bin openakta -- doc

# Opção 3: Especificando o pacote
cargo run -p openakta-cli -- do "sua missão aqui"
```

### Rodar o Daemon

```bash
# Opção 1: Usando o script de conveniência
./run-daemon.sh

# Opção 2: Usando cargo diretamente
cargo run --bin openakta-daemon

# Opção 3: Especificando o pacote
cargo run -p openakta-daemon
```

## 📦 Comandos da CLI

### `openakta do` - Executar uma missão

```bash
# Rodar uma missão
./run-cli.sh do "analyze the codebase structure"

# Com ajuda
./run-cli.sh do --help
```

### `openakta doc` - Inicializar documentação

```bash
# Inicializar documentação otimizada para AI
./run-cli.sh doc
```

### Ajuda

```bash
# Ver ajuda geral
./run-cli.sh --help

# Ver ajuda de um comando
./run-cli.sh do --help
```

## 🔧 Instalação Global (Opcional)

Se quiser usar a CLI sem `cargo run`:

```bash
# Instalar como comando global
cargo install --path crates/openakta-cli

# Usar diretamente
openakta do "sua missão"
openakta doc
```

## 📁 Estrutura do Projeto

```
aktacode/
├── crates/
│   ├── openakta-cli/       # CLI executável
│   ├── openakta-daemon/    # Daemon executável
│   ├── openakta-core/      # Core logic
│   ├── openakta-memory/    # Memory system
│   ├── openakta-storage/   # SQLite storage
│   └── ...
├── run-cli.sh              # Script para rodar CLI
├── run-daemon.sh           # Script para rodar daemon
└── README.md               # Este arquivo
```

## ⚠️ Notas Importantes

### sqlite-vec

O sqlite-vec é **vinculado estaticamente** no build - não requer instalação manual.

A inicialização ocorre automaticamente no startup da aplicação, antes de qualquer uso do SQLite.

**Importante:** Não é necessário instalar manualmente o sqlite-vec. Se você encontrar erros relacionados ao sqlite-vec, isso indica um bug no produto, não falta de instalação.

### Diretórios de Dados

Os dados são armazenados em:
- `./.openakta/` - Banco de dados e arquivos de runtime
- `./.openakta/semantic-memory.db` - Memória semântica
- `./.openakta/openakta.episodic.db` - Memória episódica

## 🐛 Troubleshooting

### Erro relacionado ao sqlite-vec

Se você encontrar erros como "sqlite-vec initialization failed":

1. Isso é um **bug no produto**, não falta de instalação
2. Reporte o erro com o log completo
3. Não tente instalar sqlite-vec manualmente - a aplicação já inclui tudo necessário

```bash
# Se precisar debugar, rebuild limpo:
cargo clean
cargo build
```

**Nota para desenvolvedores:** O sqlite-vec usa static linking via `sqlite3_auto_extension()`.
Não há necessidade de copiar .dylib/.so arquivos.


### Erro: "could not determine which binary to run"

Use `--bin` para especificar:
```bash
cargo run --bin openakta -- do "missão"
cargo run --bin openakta-daemon
```

### Limpar e reconstruir

```bash
cargo clean
cargo build
```
