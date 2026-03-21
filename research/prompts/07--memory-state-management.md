# R-07: Memory & State Management

## Research Prompt

Copy and paste the following into Claude/GPT-4/Perplexity with web search enabled:

---

```
# Deep Research: Memory & State Management for Multi-Agent AI Systems

## Context
I'm building OPENAKTA, a multi-agent AI coding system. Agents need memory - both short-term (conversation context) and long-term (learned knowledge, past experiences). We need SCIENTIFIC-LEVEL understanding of memory architectures for LLM agents. This research must be production-grade.

## Core Research Questions

### 1. Memory Taxonomy

a) **Types of Memory**
   Research and define for LLM agents:

   1. **Sensory Memory** (if applicable)
      - Immediate perception
      - Very short duration
      - What would this be for agents?

   2. **Short-Term Memory (STM) / Working Memory**
      - Current conversation context
      - Active task information
      - Limited capacity (context window)
      - How to manage?

   3. **Long-Term Memory (LTM)**
      - **Episodic:** Past experiences, conversations
      - **Semantic:** Facts, knowledge, skills
      - **Procedural:** How to do things
      - Storage and retrieval mechanisms?

b) **Human Memory Analogies**
   - What can we learn from cognitive science?
   - Atkinson-Shiffrin model
   - Working memory models
   - Applicability to AI?

c) **Memory vs Context**
   - Distinction between:
     - Context window (immediate)
     - Memory (stored, retrieved)
   - When to use each?

### 2. Short-Term Memory Management

a) **Context Window Optimization**
   - How to maximize useful context?
   - Priority-based retention
   - Sliding window approaches
   - "Lost in the Middle" mitigation

b) **Conversation Summarization**
   - Summarize old conversation turns
   - Keep recent turns verbatim
   - Summary quality vs token cost
   - Research on summarization strategies

c) **Attention Mechanisms**
   - What deserves attention in context?
   - Salience detection
   - Importance scoring
   - Dynamic context pruning

### 3. Long-Term Memory Architectures

a) **Vector Database Memory**
   - Store experiences as embeddings
   - Retrieve by similarity
   - Implementation patterns
   - Pros/cons

b) **Episodic Memory Systems**
   - Store complete experiences
   - Indexed by:
     - Time
     - Topic
     - Participants
     - Outcome
   - Retrieval strategies

c) **Semantic Memory Systems**
   - Store facts and knowledge
   - Knowledge graph approach?
   - Triple store?
   - Update mechanisms (how to handle conflicting info?)

d) **Procedural Memory**
   - Store "how-to" knowledge
   - Skill representations
   - Fine-tuning vs explicit storage?
   - Research on this?

### 4. Memory Operations

a) **Encoding (Writing)**
   - What to remember?
   - When to encode?
   - Format for storage?
   - Consolidation process?

b) **Retrieval (Reading)**
   - Trigger-based retrieval
   - Query formulation
   - Relevance scoring
   - Re-ranking results

c) **Forgetting**
   - What to forget?
   - When to forget?
   - Forgetting algorithms:
     - Time-based decay
     - Interference-based
     - Importance-based retention
   - Why forgetting is important

d) **Consolidation**
   - Transfer from STM to LTM
   - Sleep-like processes?
   - Batch vs continuous?

### 5. Memory in Production Systems

a) **AutoGen Memory**
   - What memory capabilities?
   - Implementation details?

b) **LangChain Memory**
   - ConversationBufferMemory
   - ConversationSummaryMemory
   - VectorStoreRetrieverMemory
   - How do they work?

c) **LlamaIndex Memory**
   - Their approach to memory
   - Comparison to LangChain

d) **Enterprise Systems**
   - How do commercial AI assistants handle memory?
   - Any public info?

### 6. Multi-Agent Memory Sharing

a) **Private vs Shared Memory**
   - What should be private to each agent?
   - What should be shared?
   - Shared memory architecture?

b) **Memory Consistency**
   - How to keep shared memory consistent?
   - Update propagation?
   - Conflict resolution?

c) **Memory Access Control**
   - Who can read what?
   - Who can write what?
   - Privacy considerations?

d) **Collective Memory**
   - Team knowledge
   - Shared experiences
   - How to represent?

### 7. Code-Specific Memory

For our coding agent system:

a) **Codebase Knowledge**
   - Store understanding of codebase structure
   - Past changes and their impacts
   - Common patterns in the code

b) **User Preferences**
   - Coding style preferences
   - Common patterns user likes
   - Pet peeves to avoid

c) **Project Context**
   - Project goals
   - Architecture decisions
   - Technical debt notes

d) **Session State**
   - Current task progress
   - Open issues
   - Pending changes

### 8. Implementation Considerations

a) **Storage Backend**
   - SQLite (what we're using)
   - Vector DB (for embeddings)
   - Hybrid approach?

b) **Memory APIs**
   - How agents access memory
   - Query language?
   - Abstraction level?

c) **Performance**
   - Memory operation latency
   - Batch operations?
   - Caching strategies?

d) **Persistence**
   - Across sessions?
   - User control over memory?
   - Export/import?

## Required Output Format

### Section 1: Memory Architecture
- Recommended memory types
- Diagram of architecture
- Data flow for memory operations

### Section 2: Implementation Details
- Storage schema
- Retrieval algorithms
- Forgetting mechanisms

### Section 3: Multi-Agent Sharing
- Private vs shared memory design
- Consistency approach
- Access control

### Section 4: Code-Specific Features
- What to remember about codebases
- User preference storage
- Session management

### Section 5: Implementation Plan
- Rust-specific considerations
- Phased implementation
- Testing approach

## Sources Required

Must include:
- At least 5 papers on AI memory systems
- At least 3 framework documentation (LangChain, etc.)
- At least 2 cognitive science references (for memory theory)

## Quality Bar

This research determines how agents remember and learn. It must be:
- Grounded in memory research (AI and cognitive science)
- Practical for implementation
- Specific to multi-agent LLM systems
- Actionable recommendations
```

---

## Follow-up Prompts

### Follow-up 1: Memory Schema Design
```
Design the database schema for agent memory:

1. **Tables Needed**
   - episodic_memory
   - semantic_memory
   - procedural_memory
   - conversation_history
   - user_preferences
   - ...

2. **For Each Table**
   - Columns
   - Indexes
   - Relationships

3. **SQLite Schema**
   Actual SQL CREATE statements

4. **Rust Structs**
   Corresponding Rust types
```

### Follow-up 2: Retrieval Algorithm
```
Design memory retrieval algorithm:

1. **Query Processing**
   - How to formulate queries?
   - Embedding the query?

2. **Scoring**
   - Relevance scoring
   - Recency bonus
   - Importance weighting

3. **Re-ranking**
   - Initial retrieval
   - Re-ranking strategy

4. **Code Example**
   Rust implementation
```

### Follow-up 3: Forgetting Strategy
```
Design forgetting mechanism:

1. **Why Forget?**
   - Token budget
   - Relevance decay
   - Privacy

2. **Forgetting Algorithm**
   - Time decay function
   - Importance thresholds
   - Trigger-based forgetting

3. **Implementation**
   - Background job?
   - On-demand?
   - User-initiated?

4. **Code Example**
   Rust implementation
```

---

## Findings Template

Save research findings in `research/findings/memory-state/`:

```markdown
# R-07 Findings: Memory & State Management

**Research Date:** YYYY-MM-DD  
**Researcher:** [AI Model Used]  
**Sources:** [List of papers, articles, etc.]

## Memory Architecture

```
[Diagram of memory architecture]
```

**Components:**
1. Short-term: ...
2. Episodic: ...
3. Semantic: ...
4. Procedural: ...

## Database Schema

```sql
CREATE TABLE episodic_memory (
    id TEXT PRIMARY KEY,
    agent_id TEXT,
    content TEXT,
    embedding BLOB,
    importance REAL,
    created_at TEXT,
    ...
);
```

## Retrieval Algorithm

```rust
fn retrieve_memory(query: &str, agent_id: &str) -> Vec<Memory> {
    // ...
}
```

## Forgetting Strategy

**Algorithm:** [description]
**Decay Function:** [formula]
**Thresholds:** [values]

## Open Questions

- [ ] Question 1
- [ ] Question 2

## Next Steps

1. [Action item]
2. [Action item]
```

---

## Related Research

- [R-01: Context Management](./01-context-management-rag.md) - Context vs memory
- [R-04: Local Indexing](./04-local-indexing-embedding.md) - Vector storage
- [R-06: Agent Architecture](./06-agent-architecture-orchestration.md) - Shared state
