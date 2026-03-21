# OPENAKTA Feature Backlog — User-Agent Interaction

**Date:** 2026-03-16  
**Priority:** 🔴 HIGH (Fundamental for UX)

---

## 🚨 Critical Limitation Identified

**Current Problem:** When subagents are running, **user cannot converse with the system**.

**Impact:**
- ❌ User is locked out during execution
- ❌ No collaborative decision-making
- ❌ No learning opportunity for user
- ❌ Decisions made in isolation (not captured)

**This is a fundamental UX flaw that must be fixed.**

---

## ✨ Feature 1: Always-On User-Agent Channel

### Problem Statement

**Current flow:**
```
User → Dispatch Subagents → [BLACKOUT] → Results
              ↑
         User locked out
```

**Desired flow:**
```
User ←→ Active Agent (continuous channel)
   ↓
   ↓ (while other agents work)
   ↓
User ←→ Decision Capture ←→ Shared Memory
```

### Requirements

1. **Always Available:** User can message the system at ANY time, even during subagent execution
2. **Decision Capture:** When user makes a decision with an agent, it's automatically captured
3. **Relevance Detection:** System learns to detect when a decision should be saved
4. **Shared Memory:** Decisions persist across sessions, available to all agents
5. **Multi-Level Memory:**
   - **Short-term:** Current session context
   - **Long-term:** Persistent decisions/ADRs
   - **Shared:** Cross-agent knowledge
   - **Domain:** Domain-specific (auth, payments, etc.)

### Proposed Architecture

```rust
pub struct UserAgentChannel {
    user_session: UserSession,
    active_agent: Option<AgentId>,
    decision_detector: DecisionDetector,
    memory: SharedMemory,
}

pub struct Decision {
    id: String,
    title: String,
    context: String,
    options_considered: Vec<Option>,
    selected_option: Option,
    rationale: String,
    timestamp: u64,
    relevance_score: f32,
}

pub struct SharedMemory {
    short_term: SessionContext,      // Current session
    long_term: DecisionLog,          // Persistent ADRs
    shared: Blackboard,              // Cross-agent knowledge
    domain: DomainKnowledge,         // Domain-specific
}

impl UserAgentChannel {
    pub async fn handle_message(&mut self, msg: UserMessage) -> Result<AgentResponse> {
        // Check if this is a decision point
        if self.decision_detector.is_decision_point(&msg)? {
            // Capture decision
            let decision = self.capture_decision(msg).await?;
            
            // Evaluate relevance
            if decision.relevance_score > RELEVANCE_THRESHOLD {
                // Save to long-term memory
                self.memory.long_term.add(decision).await?;
                
                // Notify relevant agents
                self.notify_agents(&decision).await?;
            }
        }
        
        // Route to active agent (or create new one)
        let agent = self.get_or_create_agent(msg.domain)?;
        
        // Get response (with memory context)
        let response = agent.respond(msg, &self.memory).await?;
        
        // Update short-term memory
        self.memory.short_term.add(msg, response.clone()).await?;
        
        Ok(response)
    }
    
    async fn capture_decision(&mut self, msg: UserMessage) -> Result<Decision> {
        // Use LLM to extract decision from conversation
        let decision = self.llm.extract_decision(msg).await?;
        
        // Calculate relevance score
        decision.relevance_score = self.calculate_relevance(&decision)?;
        
        Ok(decision)
    }
    
    fn calculate_relevance(&self, decision: &Decision) -> Result<f32> {
        // Factors:
        // - Architectural impact (high = more relevant)
        // - Reversibility (hard to reverse = more relevant)
        // - Cost impact (high cost = more relevant)
        // - Cross-domain effect (affects multiple domains = more relevant)
        
        let score = self.llm.score_relevance(decision)?;
        Ok(score)
    }
}
```

### User Experience

**Example Conversation:**
```
User: "Should we use JWT or sessions for auth?"

Agent: "Good question! Let me outline the options:

**Option A: JWT**
✅ Stateless, scalable
✅ Good for microservices
❌ Token size overhead
❌ Hard to revoke

**Option B: Sessions**
✅ Easy to revoke
✅ Smaller payload
❌ Server-side state
❌ Sticky sessions needed

**Recommendation:** JWT for your use case (microservices, scalable).

Want me to capture this decision?"

User: "Yes, save it."

[System: Decision captured → ADR-043: Use JWT for session management]
[System: Saved to long-term memory (relevance: 0.85)]
[System: Notified Auth Domain Team]
```

### Implementation Plan

**Phase 1: Basic Channel** (8 hours)
- [ ] Always-on message handler
- [ ] Session context management
- [ ] Basic conversation memory

**Phase 2: Decision Capture** (16 hours)
- [ ] LLM-based decision extraction
- [ ] Relevance scoring
- [ ] ADR auto-generation

**Phase 3: Multi-Level Memory** (24 hours)
- [ ] Short-term (session)
- [ ] Long-term (decisions)
- [ ] Shared (blackboard)
- [ ] Domain (specialized)

**Phase 4: Integration** (16 hours)
- [ ] Notify agents of decisions
- [ ] Decision retrieval during tasks
- [ ] UI for browsing decisions

---

## ✨ Feature 2: Interactive Decision Tools (Clickable Options)

### Problem Statement

**Current limitation:** Agents present options as **text**, user must type response.

**Desired:** Agents present **interactive tools** with clickable options.

### Requirements

1. **Rich Responses:** Agent can send buttons, dropdowns, multi-select
2. **Context-Aware:** Options based on current task/context
3. **Progressive Disclosure:** Start simple, show more on demand
4. **Keyboard + Mouse:** Support both power users (keyboard) and casual users (mouse)

### Proposed Architecture

```rust
pub struct InteractiveTool {
    tool_type: ToolType,
    options: Vec<ToolOption>,
    multi_select: bool,
    allow_custom: bool,
}

pub enum ToolType {
    SingleChoice,    // Radio buttons
    MultiChoice,     // Checkboxes
    Dropdown,        // Select dropdown
    ButtonGroup,     // Action buttons
    Slider,          // Range input
    Toggle,          // On/off switch
}

pub struct ToolOption {
    id: String,
    label: String,
    description: Option<String>,
    icon: Option<String>,
    default: bool,
}

pub struct AgentResponse {
    text: String,
    tool: Option<InteractiveTool>,
    context: ResponseContext,
}

impl AgentResponse {
    pub fn with_choice(text: &str, options: Vec<&str>) -> Self {
        Self {
            text: text.to_string(),
            tool: Some(InteractiveTool {
                tool_type: ToolType::SingleChoice,
                options: options.iter().map(|o| ToolOption {
                    id: o.to_lowercase().replace(' ', "_"),
                    label: o.to_string(),
                    description: None,
                    icon: None,
                    default: false,
                }).collect(),
                multi_select: false,
                allow_custom: false,
            }),
            context: ResponseContext::default(),
        }
    }
}
```

### User Experience

**Example 1: Architecture Decision**
```
Agent: "Which architecture pattern for the domain teams?"

[  DDD (Domain-Driven)   ]  [  Clean Architecture  ]
[  Hexagonal (Ports/Adapters)  ]  [  Event-Driven  ]

User: [clicks "DDD (Domain-Driven)"]

Agent: "Great choice! DDD aligns well with your team structure.

Should I create an ADR for this decision?"

[  Yes, create ADR  ]  [  No, just proceed  ]
```

**Example 2: Task Decomposition**
```
Agent: "I've decomposed 'Implement auth' into 8 tasks.

**Parallel Group 1** (can run concurrently):
☑️ Design database schema (Architect, ~2h)
☑️ Research auth best practices (Researcher, ~1h)
☐ Set up project structure (Coder, ~1h)

**Parallel Group 2** (depends on Group 1):
☐ Implement user model (Coder, ~3h)
☐ Implement JWT utilities (Coder, ~2h)
☐ Write auth tests (Tester, ~2h)

Which tasks should I start now?"

[  Start Group 1  ]  [  Customize...  ]  [  Cancel  ]
```

**Example 3: Code Review**
```
Agent: "Found 3 issues in auth.rs:

**Issue 1:** JWT secret hardcoded (High severity)
```rust
let secret = "my-super-secret-key";  // ❌
```

**Issue 2:** No token expiration (Medium severity)

**Issue 3:** Missing input validation (High severity)

How should I proceed?"

[  Fix All  ]  [  Fix High Severity Only  ]  [  Show Me Each  ]  [  Ignore  ]
```

### Implementation Plan

**Phase 1: Basic Tools** (12 hours)
- [ ] SingleChoice (radio buttons)
- [ ] MultiChoice (checkboxes)
- [ ] ButtonGroup (action buttons)
- [ ] CLI rendering (for terminal UI)

**Phase 2: Advanced Tools** (16 hours)
- [ ] Dropdown (for many options)
- [ ] Slider (for ranges)
- [ ] Toggle (for on/off)
- [ ] Desktop UI rendering (Tauri)

**Phase 3: Integration** (16 hours)
- [ ] Agent can create tools dynamically
- [ ] User selection captured as decision
- [ ] Tools saved to memory
- [ ] Analytics (which options users pick)

---

## 📊 Priority & Timeline

| Feature | Priority | Effort | When |
|---------|----------|--------|------|
| Always-On Channel | 🔴 HIGH | 64 hours | Phase 3 |
| Interactive Tools | 🟠 MEDIUM | 44 hours | Phase 3 |

**Combined:** ~100 hours (2.5 weeks for single developer)

**Recommendation:** Implement **in parallel** with other Phase 3 work (different agents can work on these).

---

## 🔗 Related Research

- [R-07: Memory & State](./prompts/07-memory-state-management.md) — Multi-level memory
- [R-09: Documentation Management](./prompts/09-documentation-management.md) — Decision capture
- [DOCUMENTATION-RESEARCH.md](./DOCUMENTATION-RESEARCH.md) — Knowledge accumulation
- [HEARTBEAT-REANALYSIS.md](./HEARTBEAT-REANALYSIS.md) — Agent lifecycle

---

## ✅ Action Items

1. **Add to Phase 3 backlog** (both features)
2. **Create research prompt** for decision capture patterns
3. **Design UI mockups** for interactive tools
4. **Prioritize vs other Phase 3 work**

---

**These features are CRITICAL for OPENAKTA's differentiation.** No other agent framework has:
- ✅ Always-on user channel during execution
- ✅ Automatic decision capture with relevance detection
- ✅ Multi-level memory (session, long-term, shared, domain)
- ✅ Interactive clickable tools for decisions

**This is a key differentiator. Prioritize accordingly.**
