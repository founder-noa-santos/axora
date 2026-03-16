# R-02: Inter-Agent Communication Protocols

## Research Prompt

Copy and paste the following into Claude/GPT-4/Perplexity with web search enabled:

---

```
# Deep Research: Inter-Agent Communication for Multi-Agent AI Systems

## Context
I'm building AXORA, a multi-agent AI coding system where multiple LLM agents collaborate on software development tasks. Agents need to communicate efficiently, share context, and coordinate work. We need SCIENTIFIC-LEVEL understanding of communication protocols. This research must be production-grade.

## Core Research Questions

### 1. Communication Protocol Fundamentals

a) **Message Patterns**
   - Request/Response vs Publish/Subscribe vs Message Queue
   - Synchronous vs Asynchronous communication
   - Unicast vs Multicast vs Broadcast
   - When is each pattern appropriate for agent systems?

b) **Protocol Options**
   Deep-dive into each with technical specifics:
   
   1. **gRPC/HTTP2**
      - Performance characteristics (latency, throughput)
      - Streaming capabilities
      - Code generation benefits
      - Rust ecosystem support (tonic)
      
   2. **WebSocket**
      - Real-time bidirectional communication
      - Overhead vs gRPC
      - Browser compatibility (for future web UI)
      
   3. **Message Queues**
      - NATS (and NATS JetStream)
      - Redis Pub/Sub
      - RabbitMQ
      - Apache Kafka (overkill?)
      - ZeroMQ
      
   4. **Custom Protocols**
      - When does it make sense?
      - What do existing multi-agent systems use?

c) **Serialization Formats**
   Compare for agent messages:
   - Protocol Buffers (our current choice)
   - Apache Avro
   - MessagePack
   - CBOR
   - JSON (baseline)
   - Binary formats for token efficiency
   
   For each: size, speed, schema evolution, Rust support

### 2. Multi-Agent System Research (Academic)

a) **Foundational Papers**
   Research and summarize:
   
   1. **Contract Net Protocol** (Smith 1980)
      - What is it?
      - Is it still relevant?
      - Modern implementations?
      
   2. **FIPA ACL** (Foundation for Intelligent Physical Agents)
      - Agent Communication Language specification
      - Performatives (inform, request, propose, etc.)
      - Why did it not become mainstream?
      
   3. **KQML** (Knowledge Query and Manipulation Language)
      - Historical significance
      - Why did it fail?
      
   4. **Modern Multi-Agent Communication (2020-2026)**
      - What do recent papers use?
      - Search: "multi-agent system communication protocol"
      - Search: "LLM agent communication"

b) **Coordination Mechanisms**
   - Blackboard systems (classic AI pattern)
   - Tuple spaces (Linda coordination language)
   - Market-based approaches (auctions for task assignment)
   - Swarm intelligence patterns
   - Which are relevant for LLM agents?

### 3. LLM-Specific Communication

This is NEW - traditional MAS didn't have LLMs:

a) **Natural Language vs Structured**
   - Should agents communicate in natural language or structured formats?
   - Tradeoffs:
     - NL: Flexible, human-readable, token-heavy, ambiguous
     - Structured: Efficient, unambiguous, rigid, requires schema
    
   - Hybrid approaches?
   
b) **Token Efficiency**
   - If agents use NL, how do we minimize tokens?
   - Abbreviation protocols?
   - Semantic compression?
   - Reference by ID vs full content?
   
c) **Context Sharing**
   - How does Agent A share relevant context with Agent B?
   - Full conversation history? (expensive)
   - Summarized context? (lossy)
   - Retrieval-based context sharing?
   - Shared memory/blackboard?

d) **Semantic Communication**
   - Research: "Semantic communication for AI agents"
   - Can agents exchange embeddings instead of text?
   - When is this appropriate?
   - Token/cost savings?

### 4. Production Multi-Agent Systems

Research these specific implementations:

a) **AutoGen (Microsoft)**
   - How do agents communicate?
   - What transport do they use?
   - Message format?
   - Group chat patterns?

b) **LangGraph / LangChain**
   - Their approach to agent communication
   - State machine vs message passing

c) **CrewAI**
   - Agent communication patterns
   - Process orchestration

d) **OpenAI Swarm** (if public)
   - Handoff protocols
   - Context transfer

e) **Aider** (multi-file editing)
   - How do they handle context?
   - Not multi-agent but relevant

f) **Enterprise Systems**
   - Any public info from companies doing multi-agent AI?
   - Cognition Labs (Devin)?
   - Magic?
   - Other AI coding startups?

### 5. Message Design

a) **Message Types**
   What types of messages do agents need to send?
   
   1. **Task-related**
      - Task assignment
      - Status updates
      - Completion notifications
      - Help requests
      
   2. **Information Sharing**
      - Code snippets
      - Documentation references
      - Error messages
      - Test results
      
   3. **Coordination**
      - Lock acquisition (for shared resources)
      - Conflict detection
      - Consensus requests
      
   4. **Meta-communication**
      - Capability advertisements
      - Availability status
      - Heartbeat/health

b) **Message Schema Design**
   For each message type, design optimal schema:
   - Required fields
   - Optional fields
   - Compression opportunities
   - Versioning strategy

c) **Conversation Management**
   - How do we track conversation threads?
   - Message correlation (request → response)
   - Timeout handling
   - Retry logic

### 6. Scalability Considerations

a) **Performance at Scale**
   - What happens at 10 agents? 100 agents? 1000 agents?
   - Message volume projections
   - Bottleneck analysis
   
b) **Network Topology**
   - Star topology (central coordinator)
   - Mesh topology (peer-to-peer)
   - Hierarchical topology (teams of agents)
   - Which is best for our use case?

c) **Fault Tolerance**
   - What happens when an agent fails mid-task?
   - Message durability requirements
   - At-least-once vs exactly-once delivery
   - Dead letter queues

### 7. Security Considerations

a) **Authentication**
   - How do agents authenticate each other?
   - Preventing rogue agents?
   
b) **Authorization**
   - What actions is each agent allowed?
   - Capability-based security?
   
c) **Message Integrity**
   - Preventing message tampering
   - Signing requirements

## Required Output Format

### Section 1: Executive Summary
- Recommended communication architecture
- Key tradeoffs identified
- Confidence level in recommendations

### Section 2: Protocol Analysis
- Detailed comparison of protocol options
- Performance benchmarks where available
- Rust ecosystem maturity for each option

### Section 3: Message Design
- Proposed message schema for agent communication
- Token efficiency analysis
- Example messages

### Section 4: Academic Foundations
- Summary of relevant MAS research
- Which classic patterns are still relevant
- What's new with LLM agents

### Section 5: Competitive Analysis
- What AutoGen, CrewAI, etc. do
- Where we can differentiate
- What to copy vs what to innovate

### Section 6: Implementation Plan
- Specific libraries to use (Rust)
- Architecture diagram
- Phased implementation approach

## Sources Required

Must include:
- At least 5 academic papers on multi-agent communication
- At least 3 production multi-agent system analyses
- At least 2 protocol specification documents
- Performance benchmarks where available

## Quality Bar

This research determines how our agents talk to each other - a fundamental architectural decision. It must be:
- Technically rigorous
- Evidence-based
- Practical for implementation
- Forward-looking (LLM agents are new)
```

---

## Follow-up Prompts

### Follow-up 1: Token-Efficient Communication
```
Deep-dive into token-efficient communication protocols for LLM agents:

1. **Compression Strategies**
   Research:
   - Semantic compression for natural language
   - Abbreviation protocols (like brevity codes)
   - Reference-by-ID patterns
   - Delta compression (only send changes)
   
2. **Quantitative Analysis**
   For a typical agent conversation:
   - How many tokens with naive NL communication?
   - How many with optimized protocol?
   - Cost implications at scale?
   
3. **Tradeoffs**
   - Does compression hurt agent understanding?
   - Any research on this?
   
4. **Recommendation**
   Design a token-efficient message format for our agents.
```

### Follow-up 2: Context Sharing
```
Deep-dive into how agents should share context:

1. **Approaches to Compare**
   a) Full context transfer (send everything)
   b) Summarized context (LLM summarizes)
   c) Retrieval-based (agent retrieves what it needs)
   d) Shared memory (blackboard/database)
   e) Hybrid approaches
   
2. **For each, analyze:**
   - Token cost
   - Latency
   - Information loss
   - Implementation complexity
   - Scalability
   
3. **Research**
   - Any papers on multi-agent context sharing?
   - How do human teams share context? (analogy)
   
4. **Recommendation**
   What approach for our coding agent system?
```

### Follow-up 3: NATS for Agent Communication
```
Evaluate NATS as a communication layer for our agents:

1. **NATS Core Features**
   - Pub/sub performance
   - Request/reply pattern
   - Queue groups
   - Rust client (nats.rs) quality
   
2. **NATS JetStream** (streaming/persistence)
   - Do we need message persistence?
   - Performance impact?
   - Complexity tradeoff?
   
3. **Comparison**
   - NATS vs Redis Pub/Sub
   - NATS vs gRPC direct
   - NATS vs WebSocket
   
4. **Deployment**
   - Embedded NATS server (nats-server as library)?
   - External NATS deployment?
   - Operational complexity?
   
5. **Recommendation**
   Should we use NATS? Why or why not?
```

---

## Findings Template

Save research findings in `research/findings/inter-agent-communication/`:

```markdown
# R-02 Findings: Inter-Agent Communication

**Research Date:** YYYY-MM-DD  
**Researcher:** [AI Model Used]  
**Sources:** [List of papers, articles, etc.]

## Key Findings

### Finding 1: [Title]
**Description:** ...
**Source:** [Link]
**Confidence:** High/Medium/Low
**Implication for AXORA:** ...

## Recommended Protocol Stack

| Layer | Technology | Rationale |
|-------|------------|-----------|
| Transport | ... | ... |
| Serialization | ... | ... |
| Pattern | ... | ... |

## Message Schema

```protobuf
// Proposed message format
message AgentMessage {
  string id = 1;
  MessageType type = 2;
  string sender_id = 3;
  string recipient_id = 4;
  bytes content = 5;  // Compressed?
  int64 timestamp = 6;
}
```

## Token Efficiency Analysis

| Approach | Tokens/Message | Savings |
|----------|---------------|---------|
| Naive NL | ~500 | baseline |
| Optimized | ~150 | 70% |

## Open Questions

- [ ] Question 1
- [ ] Question 2

## Next Steps

1. [Action item]
2. [Action item]
```

---

## Related Research

- [R-03: Token Efficiency](./03-token-efficiency-compression.md) - Overlaps with message compression
- [R-06: Agent Architecture](./06-agent-architecture-orchestration.md) - How communication fits into architecture
- [R-07: Memory & State](./07-memory-state-management.md) - Shared context mechanisms
