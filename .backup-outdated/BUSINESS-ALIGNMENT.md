# OPENAKTA Business Alignment Report

**Date:** 2026-03-16  
**Status:** ✅ Complete  
**Participants:** Founder + AI Assistant

---

## Executive Summary

OPENAKTA is a **multi-agent AI coding system** designed for **individual developers** who want a **ready-to-use team of specialized AI agents** that work collaboratively to help them develop software.

**Core Value Proposition:**
> "A primeira equipe de AI que trabalha pra você, do seu jeito."

**Key Differentiators:**
1. **Multi-agent swarm** (not a single assistant, a team)
2. **Specialized agents out-of-the-box** (batteries included)
3. **Configurable & flexible** (BYOK or pay-us, local or cloud)
4. **Token efficiency innovation** (across all layers)

---

## Target Audience

**Primary:** Individual Developers

**Profile:**
- Freelancers, hobbyists, professional developers
- Want AI pair programming
- Value privacy and control
- Price sensitivity: ~$10-20/month range

**Implications:**
- ✅ Price must be accessible
- ✅ Simplicity > enterprise features
- ✅ Local-first is a differentiator (privacy, no cloud dependency)
- ✅ UX must be polished (developers are critical users)

---

## Core Pain Points Addressed

| Priority | Pain Point | Solution |
|----------|------------|----------|
| #1 | Privacy / Code ownership | 100% local-first option, code never leaves machine |
| #2 | Flexibility / Control | BYOK, choose models, configure agents |
| #3 | Setup complexity | Pre-configured agents, plug-and-play |
| #4 | Token waste | Multi-layer optimization |

---

## Product Vision

### User Experience Model: **Hybrid + Collaborative**

```
┌─────────────────────────────────────────────────────────┐
│                    OPENAKTA Experience                      │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  Default Mode: "Invisible Orchestra"                     │
│  - User makes high-level request                         │
│  - Agents self-organize behind the scenes                │
│  - User sees final result                                │
│                                                          │
│  Advanced Mode: "Direct Agent Control"                   │
│  - User can talk to specific agents                      │
│  - "Arquiteto, me ajuda com o design"                    │
│  - Override when needed                                  │
│                                                          │
│  Collaborative Mode: "AI advises, Human decides"         │
│  - Agents consult user on important decisions            │
│  - User approves/rejects suggestions                     │
│  - Human-in-the-loop for critical choices                │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### Agent Interaction Model

**User = Brain + Business**  
**Agents = Implementers**  
**OPENAKTA = Advisor that "pulls your ear"** (quality, security, simplicity)

User converses and specifies → Agents implement → User reviews and iterates

---

## Native Agents (V1 MVP)

**6-8 Specialized Agents** (not editable by users, but users can create custom ones):

| Agent | Responsibility | Key Skills |
|-------|----------------|------------|
| **Arquiteto** | Architecture, folder structure, separation of concerns, design | System design, patterns, best practices |
| **Coder** | Code implementation | All languages, frameworks |
| **Tester** | Unit, e2e, integration tests; writes test requirements | Testing frameworks, QA |
| **Browser Specialist** | Browser automation, e2e testing, screenshots, documentation | Playwright, Puppeteer, visual testing |
| **Debugger** | Debugging (including browser control for reproduction) | Debug tools, browser devtools |
| **Documenter** | Documentation generation | Technical writing, docs as code |
| **Researcher** | Internet research, finds information everywhere | Web search, API docs, Stack Overflow |
| **Security Auditor** | Code security analysis | Security scanning, vulnerability detection |
| **Optimizer** | Code efficiency | Performance profiling, optimization |
| **Simplifier** | Prevents excessive complexity | Code review, refactoring |

### User-Created Agents
- ✅ Users can create custom agents via friendly UI
- ✅ LLM helper assists in agent creation
- ✅ Native agents are not editable (preserves quality)

---

## Technical Architecture

### Model Strategy: **Cloud-First, Hybrid-Intelligent**

```
┌─────────────────────────────────────────┐
│  Agents Especializados (Presidents)     │
│  - Arquiteto, Coder, Reviewer, Tester   │
│  - Cloud (BYOK or paid via OPENAKTA)       │
│  - Best quality for complex tasks       │
└─────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────┐
│  "Estagiários" (Helpers locais)         │
│  - Simple tasks, pre-processing         │
│  - Local models (7B, fast, free)        │
│  - Privacy, no API costs                │
└─────────────────────────────────────────┘
```

**User Options:**
- **BYOK**: Use your own API keys (pay provider directly)
- **Pay OPENAKTA**: Use our keys, pay us (convenience, markup)
- **Local**: Free, unlimited, less capable

### Technology Stack

| Layer | Technology | Rationale |
|-------|------------|-----------|
| Backend | Rust | Performance, safety, local-first |
| Frontend | React + TypeScript | Ecosystem, familiarity |
| Desktop | Tauri v2 | Lightweight, Rust integration |
| IPC | gRPC | Strong typing, streaming |
| Storage | SQLite | Simple, embedded, Rust support |
| Vector DB | TBD (research pending) | For embeddings, RAG |

---

## Token Efficiency Strategy

**Core Innovation:** Multi-layer token optimization

| Layer | Strategy | Expected Savings |
|-------|----------|------------------|
| **Context** | Intelligent RAG, only send relevant code | 5-10x less input tokens |
| **Local Filtering** | Local model (7B) pre-processes, only sends what needs cloud | 3-5x less cloud calls |
| **Agent Communication** | Compressed protocol, not natural language | 10-20x less coordination tokens |
| **Cache + Deduplication** | Aggressive caching of similar responses | 2-3x less repeated calls |

**Combined Impact:** Potentially 50-100x reduction in token costs vs naive implementation

---

## Monetization Model

**Primary:** Subscription + Usage

- **Base subscription** (~$10-20/month): Access to app, use your own API keys (BYOK)
- **Usage-based**: Pay OPENAKTA for API usage (convenience, potential markup)
- **Local**: Free, unlimited (for users who can't/won't pay)

**Philosophy:**
- Don't necessarily profit from API markup
- Revenue from subscription for sustainability
- Token efficiency = lower costs for everyone

---

## Risk Mitigation

### Primary Risk: Multi-Agent Communication Chaos

**Concern:** Agents communicating poorly, contradicting each other, creating chaos

**Mitigation Strategy: Clear Hierarchy**

```
                    ┌─────────────┐
                    │  Arquiteto  │ (senior, coordinates)
                    └──────┬──────┘
                           │
         ┌─────────────────┼─────────────────┐
         │                 │                 │
    ┌────▼────┐      ┌────▼────┐      ┌────▼────┐
    │  Coder  │      │ Tester  │      │Debugger │
    └────┬────┘      └────┬────┘      └────┬────┘
         │                 │                 │
         └─────────────────┼─────────────────┘
                           │
                    ┌──────▼──────┐
                    │  Browser    │ (executes)
                    │ Specialist  │
                    └─────────────┘
```

**Principles:**
- Architect > Coder > Tester (chain of command)
- "Senior" agents coordinate "junior" agents
- Defined escalation paths
- Organization > equality

---

## Development Approach

**Founder Role:** Brain + Business  
**Execution:** AI-driven implementation

**Model:**
- Founder specifies through conversation
- Agents implement code, tests, docs
- Founder reviews and iterates
- Speed: 10-100x faster than manual coding

**Timeline:** 6-12 months for MVP (solo founder, part-time)

---

## MVP Scope (V1)

**Theme:** "Complete experience, but simple"

**Must-Have:**
- ✅ Swarm functioning (agents communicating)
- ✅ BYOK configuration
- ✅ 6-8 native agents
- ✅ Polished UI
- ✅ Local + cloud hybrid
- ✅ Browser automation (Browser Specialist)
- ✅ Repo organization (human-readable, clean)

**Can Wait:**
- ❌ Advanced enterprise features
- ❌ Marketplace for agents
- ❌ Team collaboration features

---

## Quality Pillars

All code produced by OPENAKTA must adhere to:

1. **Security** - No vulnerabilities, safe by default
2. **Efficiency** - Performant, resource-conscious
3. **Simplicity** - Avoid excessive complexity, KISS principle

**Repo Standards:**
- Organized folder structure
- Human-readable code
- Consistent formatting
- Documentation included

---

## Research Dependencies

Before implementation begins, research must be completed in these areas:

| ID | Area | Status | Decision Impact |
|----|------|--------|-----------------|
| R-01 | Context Management & RAG | 🔄 In Progress | ADR-012 |
| R-02 | Inter-Agent Communication | 🔄 In Progress | ADR-002, ADR-009 |
| R-03 | Token Efficiency | 🔄 In Progress | Multiple |
| R-04 | Local Indexing & Embedding | 🔄 In Progress | ADR-004, ADR-006, ADR-007 |
| R-05 | Model Optimization (Local) | 🔄 In Progress | ADR-008 |
| R-06 | Agent Architecture | 🔄 In Progress | ADR-010 |
| R-07 | Memory & State | 🔄 In Progress | ADR-011 |
| R-08 | Evaluation & Benchmarking | 🔄 In Progress | Multiple |

---

## Decision Log

All architectural decisions are recorded in: [`DECISIONS.md`](./DECISIONS.md)

**Pending Decisions** (awaiting research):

| ID | Topic | Target Date |
|----|-------|-------------|
| ADR-006 | Embedding Model | After R-04 |
| ADR-007 | Vector Database | After R-04 |
| ADR-008 | Local LLM Model | After R-05 |
| ADR-009 | Agent Communication Protocol | After R-02 |
| ADR-010 | Agent Architecture | After R-06 |
| ADR-011 | Memory Architecture | After R-07 |
| ADR-012 | Context Management Strategy | After R-01 |

---

## Next Steps

### Immediate (This Week)
1. ⏳ Wait for research results (8 areas)
2. ⏳ Review research findings
3. ⏳ Begin creating ADRs based on research

### Short-Term (Next 2-4 Weeks)
1. Create comprehensive architecture document
2. Define agent communication protocol
3. Design database schema (including vector storage)
4. Create detailed implementation plan

### Medium-Term (1-3 Months)
1. Implement core daemon functionality
2. Implement storage layer with migrations
3. Implement basic agent framework
4. Create first native agents (Coder, Arquiteto, Tester)

---

## Success Criteria

**MVP is successful when:**
- ✅ User can configure BYOK or pay-us
- ✅ User can make a high-level request ("create a CRUD")
- ✅ Agents self-organize and implement
- ✅ Code is secure, efficient, simple
- ✅ Browser Specialist can automate e2e testing
- ✅ Repo is organized and human-readable
- ✅ Token costs are 10x lower than naive implementation

---

## Document Maintenance

**Owner:** Founder  
**Update Frequency:** As decisions are made  
**Location:** `/research/BUSINESS-ALIGNMENT.md`

**Related Documents:**
- [`DECISIONS.md`](./DECISIONS.md) - Architectural decisions
- [`README.md`](./README.md) - Research overview
- `/prompts/` - Research prompts
- `/planning/` - Implementation phases

---

## Appendix: Conversation Summary

**16 questions** were asked to align on:

1. Target audience → Individual developers
2. Primary pain point → Privacy + flexibility
3. Agent interaction model → Hybrid + collaborative
4. Differentiation → Multi-agent + pre-configured
5. Model strategy → Cloud-first, hybrid-intelligent
6. Monetization → Subscription + usage (BYOK)
7. Token efficiency → All layers (core innovation)
8. MVP scope → Complete but simple (6-8 agents)
9. Native agents → 10 specialized agents defined
10. Timeline → 6-12 months (solo founder)
11. Risk mitigation → Hierarchical agent structure
12. Development approach → AI-driven implementation
13. Tech stack → Rust + React
14. Quality pillars → Security, Efficiency, Simplicity
15. Research dependencies → 8 areas identified
16. Success criteria → Defined above

**Conclusion:** Vision is clear, research-driven approach will guide implementation.
