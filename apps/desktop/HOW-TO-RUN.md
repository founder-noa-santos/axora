# 🚀 AXORA Desktop - Como Rodar

**Status:** ✅ **PRONTO PARA USAR**

---

## ✅ O Que Está Pronto

### Frontend (Desktop App)
- ✅ **Chat Interface** — Interface de chat com assistant-ui
- ✅ **Settings Panel** — Configuração de modelos (Ollama, OpenAI, Anthropic)
- ✅ **Progress Panel** — Monitoramento de progresso em tempo real
- ✅ **Tailwind CSS v4** — Tema personalizado (cores da marca)
- ✅ **Vite 8** — Build otimizado (288ms!)
- ✅ **shadcn/ui** — 15 componentes de UI

### Backend (Phase 3 - Rust)
- ✅ **Coordinator Core** — Orquestração de agentes
- ✅ **Task Decomposition** — Decomposição de tarefas
- ✅ **Worker Pool** — Gerenciamento de workers
- ✅ **Blackboard v2** — Contexto versionado
- ✅ **Task Queue** — Fila com prioridade + DAG
- ✅ **Result Merging** — Merge com detecção de conflitos
- ✅ **Progress Monitoring** — ETA, blockers, status reports

---

## 🎯 Como Abrir a Aplicação

### Opção 1: Development Mode (Recomendado)

```bash
cd /Users/noasantos/Fluri/axora/apps/desktop
pnpm dev
```

**Acesso:** http://localhost:5173

**Vantagens:**
- Hot reload (atualiza automaticamente)
- Source maps para debug
- Logs no terminal

---

### Opção 2: Production Build

```bash
cd /Users/noasantos/Fluri/axora/apps/desktop
pnpm build
pnpm preview
```

**Acesso:** http://localhost:4173

**Vantagens:**
- Build otimizado
- Mais próximo do release final

---

### Opção 3: Tauri Desktop App (Native)

```bash
cd /Users/noasantos/Fluri/axora/apps/desktop
pnpm tauri dev
```

**Requer:**
- Rust instalado
- Xcode Command Line Tools (macOS)

---

## 📊 Funcionalidades Disponíveis

### 1. Chat Panel
- ✅ Chat com IA (interface pronta)
- ✅ Markdown rendering
- ✅ Code syntax highlighting
- ✅ Streaming de respostas
- ✅ Copy message
- ✅ Welcome screen

**Como usar:**
1. Abra o app
2. Clique em "Chat"
3. Digite sua mensagem
4. Envie

**Configurar API:**
1. Vá em "Settings"
2. Escolha provider (Ollama, OpenAI, etc.)
3. Configure URL e API key
4. Salve

---

### 2. Settings Panel
- ✅ Model configuration
- ✅ Token limits
- ✅ Worker pool settings
- ✅ Theme preferences (dark/light)

**Como usar:**
1. Clique em "Settings"
2. Configure seu provider
3. Ajuste limites
4. Clique em "Save"

---

### 3. Progress Panel
- ✅ Progress bars
- ✅ ETA display
- ✅ Worker status
- ✅ Blocker alerts

**Como usar:**
- Visível durante execução de missões
- Atualiza em tempo real

---

## 🔧 Configurações de API

### OpenAI
```
Provider: OpenAI
Model: gpt-4, gpt-3.5-turbo
API Key: sk-...
Base URL: https://api.openai.com/v1
```

### Ollama (Local)
```
Provider: Ollama
Model: qwen2.5-coder:7b, llama2, etc.
Base URL: http://localhost:11434
API Key: (deixe vazio)
```

### Chinese Providers (OpenAI-compatible)
```
Provider: OpenAI
Model: deepseek-chat, qwen-turbo, etc.
API Key: sk-...
Base URL: https://api.deepseek.com
```

---

## 📁 Estrutura do Projeto

```
apps/desktop/
├── src/
│   ├── components/
│   │   ├── chat/        ← Chat components (Thread, Composer, etc.)
│   │   └── ui/          ← shadcn/ui components
│   ├── panels/
│   │   ├── ChatPanel.tsx
│   │   ├── SettingsPanel.tsx
│   │   └── ProgressPanel.tsx
│   ├── store/
│   │   ├── settings-store.ts
│   │   └── chat-store.ts
│   ├── hooks/
│   │   └── useOpenAICompatibleRuntime.ts
│   └── App.tsx
├── package.json
└── vite.config.ts
```

---

## 🧪 Testes

```bash
# Unit tests
pnpm test

# E2E tests (Playwright)
pnpm test:e2e

# Type check
pnpm typecheck

# Lint
pnpm lint
```

---

## 📊 Build Stats

| Metric | Value |
|--------|-------|
| **Build Time** | 288ms |
| **Bundle Size** | 2.69 MB (641 KB gzipped) |
| **CSS Size** | 42.83 KB (7.60 KB gzipped) |
| **Vite Version** | 8.0.0 |
| **Tailwind Version** | 4.2.1 |

---

## 🐛 Troubleshooting

### App não abre
```bash
# Limpe node_modules e reinstale
rm -rf node_modules
pnpm install
```

### Build falha
```bash
# Verifique TypeScript
pnpm typecheck

# Verifique erros de lint
pnpm lint
```

### Chat não funciona
1. Verifique se configurou API em Settings
2. Teste com Ollama local (sem API key)
3. Verifique console do browser (F12)

---

## 🚀 Next Steps

### Phase 5 (Em Planejamento)
- [ ] Integração completa com backend Rust
- [ ] Mission submission real
- [ ] Worker execution
- [ ] Result display

---

## 📚 Documentação

- **Status:** `planning/STATUS-DASHBOARD.md`
- **Current Tasks:** `planning/agent-*/current_task.md`
- **Architecture:** `AGENTS.md`

---

**Aplicação está PRONTA para desenvolvimento!** 🚀

**Para produção:** Aguardando integração completa com backend Phase 3.
