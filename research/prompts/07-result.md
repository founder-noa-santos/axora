R-07 Findings: Memory & State ManagementResearch Date: 2026-03-16Researcher: AI Architecture LeadSources: Section 1: Memory ArchitectureThe development of autonomous multi-agent systems necessitates a departure from stateless generative paradigms toward sophisticated, persistent memory architectures. Without structured memory, artificial agents suffer from catastrophic forgetting, context window exhaustion, and the inability to iteratively improve or maintain alignment over long horizons. A scientifically grounded approach to artificial memory requires adapting proven cognitive science models to the computational realities of Large Language Models (LLMs).1.1 Human Memory Analogies and Cognitive FrameworksThe foundational architecture for modern AI memory systems draws heavily from the Atkinson-Shiffrin modal model of human memory, which conceptualizes information processing as a multi-stage pipeline comprising sensory registers, short-term stores, and long-term stores. This taxonomy provides a robust blueprint for engineering agentic memory. In parallel, Baddeley's model of working memory introduces the concept of a central executive managing specialized buffers (such as the episodic buffer), which directly maps to an LLM orchestrator managing discrete context windows and specialized retrieval tools.The Cognitive Architectures for Language Agents (CoALA) framework further formalizes these biological concepts into computational constraints, defining how language agents must explicitly organize internal memory modules and interface with external environments through structured action spaces. Unlike biological systems where memory encoding is continuously biochemical, artificial systems must treat memory as a discrete optimization problem: deciding exactly what to write, what to retrieve, and what to forget within strict computational budgets.1.2 Memory vs. Context DistinctionA critical architectural distinction must be drawn between "context" and "memory." The context window represents the immediate, ephemeral state actively loaded into the LLM's attention mechanism, strictly bounded by the model's maximum token capacity (e.g., 16k to 128k tokens). Memory, conversely, represents the persistent, latent state residing in external storage media.Systems like MemGPT (now Letta) pioneered the "LLM as an Operating System" paradigm, explicitly separating these domains. In this architecture, the context window functions analogously to physical RAM, while external vector and graph databases serve as disk storage. The LLM is granted autonomous agency through function calling to page information between the latent external memory and the active context window, thereby creating the illusion of infinite memory within finite token constraints.1.3 Recommended Memory Types for LLM AgentsA production-grade agent requires a unified taxonomy encompassing four distinct memory modalities, each optimized for different temporal durations and semantic densities.1. Sensory MemoryIn cognitive science, sensory memory captures massive volumes of raw environmental stimuli with extremely rapid decay (milliseconds). For an AI coding agent, sensory memory acts as a high-bandwidth, volatile buffer for immediate computational perception. This includes raw Language Server Protocol (LSP) diagnostic payloads, real-time terminal stdout streams, graphical user interface (GUI) DOM states, and file system watcher events. Because injecting unparsed sensory data directly into an LLM context window causes immediate token exhaustion and severe distraction, artificial sensory memory requires an intermediate attention mechanism. Lightweight heuristic filters, deterministic parsers, or small classifier models serve to sieve through this redundant data, promoting only salient signals (e.g., specific stack traces or structural code modifications) into the agent's working memory.2. Short-Term Memory (STM) / Working MemoryWorking memory constitutes the active reasoning workspace. It maintains the current conversation context, intermediate task objectives, and loaded file buffers necessary for the immediate execution cycle. STM is highly limited in capacity due to the quadratic scaling costs of transformer attention mechanisms. Management of STM involves dynamic sliding windows, where the most recent interaction turns are kept verbatim, while older turns undergo semantic compression to preserve the core conversational intent without the token overhead. It serves as the staging ground where retrieved long-term facts are instantiated to ground the current prompt.3. Long-Term Memory (LTM)LTM provides durable, infinitely scalable storage for information that persists across sessions and agent lifecycles. It is structurally subdivided into three categories based on Tulving's cognitive taxonomy :Episodic Memory: This system records autobiographical, time-stamped experiences. It captures the chronological state of the environment, the agent's specific actions, the underlying reasoning chains, and the terminal outcomes of those actions. Episodic memory is critical for experiential learning. For instance, if an agent previously crashed a production build by misconfiguring a specific database driver, the episodic memory log allows the agent to recall the specific failure and avoid repeating the exact sequence. Frameworks like the Generative Agents simulacra rely heavily on an episodic memory stream to synthesize believable long-term behavioral patterns.Semantic Memory: This constitutes abstracted, de-contextualized factual knowledge. While episodic memory remembers a specific event ("I fixed a memory leak in main.rs by utilizing an Arc<Mutex<T>> at 14:00 yesterday"), semantic memory abstracts this into a universal fact ("The project architecture requires thread-safe shared state using Arc<Mutex>"). Semantic memory houses the overarching understanding of the codebase structure, organizational policies, and learned user preferences. It is typically implemented using Knowledge Graphs or Vector Databases for rapid semantic retrieval.Procedural Memory: Procedural memory encodes executable skills and "how-to" knowledge. In autonomous systems, this involves the storage of verified operational scripts, tool-use patterns, and multi-step workflows. Architectures such as MemP and LEGOMem treat procedural memory as a first-class optimization object. Rather than relying solely on the implicit knowledge stored in the LLM's parametric weights, explicit procedural memory distills successful past trajectories into modular, script-like abstractions. When the agent encounters a similar task, it retrieves the verified script (e.g., a multi-step git rebase and testing pipeline) from its skill library rather than reasoning through the entire sequence from scratch, drastically reducing execution latency and error rates.1.4 Architectural Diagram+-----------------------------------------------------------------------------------+| ENVIRONMENT (IDE / OS / User Inputs) |+-----------------------------------------------------------------------------------+| (Raw Data Streams)v+-----------------------------------------------------------------------------------+| || - Terminal stdout, LSP diagnostics, File System Events, Raw User Keystrokes || -> Salience Filter / Fast Classifier (Prunes >95% of noise) |+-----------------------------------------------------------------------------------+| (Salient Events)v+-----------------------------------------------------------------------------------+| Context Window || - System Instructions / Persona      - Active File Buffers || - Immediate Conversation Buffer      - Current Execution State |+-----------------------------------------------------------------------------------+| (Query) ^ (Retrieve) | (Write) ^ (Consolidate)v | v |+-----------------------------------------------------------------------------------+| (SQLite + Vector Search + Graph Relations) || || +------------------------+   +-----------------------+   +----------------------+ || | Episodic Memory | | Semantic Memory | | Procedural Memory | || | (Time-stamped event | | (Abstracted facts, | | (Verified skills, | || | logs, execution traces)| | Knowledge Graphs) | | SKILL.md scripts) | || +------------------------+   +-----------------------+   +----------------------+ |+-----------------------------------------------------------------------------------+1.5 Data Flow for Memory OperationsThe operational lifecycle of memory follows a continuous loop of ingestion, activation, retrieval, execution, and consolidation. Raw streams enter sensory memory, where deterministic filters block irrelevant telemetry. The filtered payload enters working memory, prompting the agent to evaluate its current objective. The orchestration layer intercepts this state and issues dense vector queries against the LTM databases to pull relevant historical episodes and semantic rules. The LLM generates its output based on this augmented prompt. As the working memory fills, a background process summarizes older conversational turns. During system idle times, an asynchronous consolidation cycle evaluates raw episodic traces, abstracts valuable patterns into semantic facts, distills successful workflows into procedural skills, and applies decay algorithms to prune redundant data.Section 2: Short-Term Memory ManagementOptimizing Short-Term Memory is paramount for maintaining agent coherence while managing API costs and inference latency. The inherent limitations of transformer architectures require strict supervision of the active context window.2.1 Context Window OptimizationModern LLMs, despite boasting massive context windows, suffer from the "Lost in the Middle" phenomenon, where retrieval accuracy degrades significantly for information placed in the center of the prompt. To mitigate this, context must be aggressively optimized. Optimization strategies involve priority-based retention, where the system instructions and the most recent tool outputs are anchored to the absolute beginning and end of the context window. Sliding window approaches are utilized to continuously evict the oldest raw messages from the prompt, replacing them with highly compressed summaries.2.2 Conversation SummarizationMaintaining long-running agent interactions requires transitioning from verbatim storage to semantic summarization. A dual-buffer approach is optimal: the last $N$ turns (e.g., 5-10 messages) are kept verbatim to maintain immediate conversational nuance and syntactic precision. Messages older than this threshold are recursively processed by a lightweight LLM (e.g., gpt-4o-mini) to generate a running summary. This summary is injected into the context window as a single consolidated block. The trade-off involves summary quality versus token cost; while raw messages provide the highest fidelity, they exhaust the context budget rapidly. Abstractive summarization preserves the core intent while discarding conversational pleasantries and redundant tool-call confirmations.2.3 Attention Mechanisms and Salience ScoringNot all context deserves equal attention. Dynamic context pruning relies on importance scoring to determine which elements remain in the active prompt. Token dropping algorithms and cache eviction methods evaluate the semantic weight of stored segments. By analyzing the content, the system assigns a salience score; critical user directives or system-level error codes are assigned high importance and pinned to the working memory, while iterative debugging failures are relegated to episodic storage and removed from the active context once resolved.Section 3: Long-Term Memory ArchitecturesThe persistence layer dictates an agent's ability to learn across sessions. The architecture requires a multi-modal storage approach that handles dense vectors, relational metadata, and graph-based entity linking.3.1 Vector Database MemoryVector databases form the backbone of modern AI memory, allowing agents to store text chunks as high-dimensional embeddings and retrieve them via cosine similarity.Pros: Exceptional for fuzzy matching, semantic search, and handling variations in natural language queries. It allows an agent to find relevant past problems even if the exact variable names have changed.Cons: Standard Retrieval-Augmented Generation (RAG) suffers from a lack of temporal awareness and structural understanding. Flat vector indices struggle with multi-hop reasoning (e.g., tracing a variable through multiple files) and can fall victim to semantic drift over long periods.3.2 Episodic Memory SystemsEpisodic memory stores complete, chronological experiences. To enable precise retrieval, these episodes cannot simply be dumped into a vector store; they must be indexed with rich relational metadata. Each episode is tagged with a timestamp, a categorical topic, the participants involved (e.g., the specific sub-agent and the user), and the terminal outcome (success, failure, error code). Retrieval strategies rely on hybrid search: using SQL to filter for specific date ranges or agent IDs, followed by vector similarity to find the most semantically relevant event within that filtered subset.3.3 Semantic Memory and Knowledge GraphsTo overcome the limitations of flat vector storage, semantic memory is best implemented using a Knowledge Graph approach, often referred to as GraphRAG. This system models memory as a network of nodes (entities, facts) and edges (relationships, dependencies). For an AI system, this involves storing triples (Subject-Predicate-Object). Graph memory provides strong cross-session reasoning and a global, structural view of the data.Update Mechanisms & Conflict Resolution: When conflicting information is encountered (e.g., the user changes their preferred formatting style), updating semantic memory requires a deterministic resolution protocol. A timestamp-based "Last-Write-Wins" policy is often insufficient for complex logic. Instead, systems must employ an LLM-guided evaluation step where an auditing agent compares the new fact against the existing graph, validates the provenance of the new data, and explicitly overwrites or versions the outdated semantic node.3.4 Procedural MemoryProcedural memory represents the codification of executable workflows. Instead of relying on the LLM to figure out how to deploy a microservice every time, the agent stores the validated deployment steps.Skill Representations: Skills are explicitly stored as modular, executable files (e.g., a SKILL.md format or a JSON array of deterministic tool calls) rather than relying on model fine-tuning. Explicit storage allows for immediate updates, easy human auditing, and zero retraining costs.Implementation: Frameworks like Voyager and MemP demonstrate that agents can write their own code, verify it through testing, and save the successful script to a procedural library. Upon encountering a similar task, the agent queries the procedural index, retrieves the script, and executes the predefined steps.Section 4: Memory OperationsThe lifecycle of artificial memory is governed by four primary operations: encoding, retrieval, forgetting, and consolidation.4.1 Encoding (Writing)Encoding involves transforming transient working memory into durable long-term storage.What to remember: Agents must selectively encode final outcomes, extracted architectural rules, user preferences, and distinct execution failures. High-frequency, low-value telemetry must be ignored.When to encode: Encoding triggers automatically at session termination, upon the successful completion of a complex sub-task, or when the context window reaches a specific saturation threshold (e.g., 70% capacity).Format: Data is structured into strict JSON schemas incorporating the raw text, the generated dense vector embedding, and comprehensive metadata (timestamp, source, importance score) prior to database insertion.4.2 Retrieval (Reading)Retrieval must bridge the semantic gap between the current operational state and the historical archive.Query Formulation: The system does not simply embed the user's raw prompt. An intermediate LLM call is often utilized to synthesize a highly optimized search query based on the current conversational context, extracting keywords and technical concepts.Relevance Scoring and Re-ranking: Initial retrieval uses approximate nearest neighbor (ANN) search to pull a broad candidate pool. These candidates are then rigorously re-ranked using a composite scoring function. This function weighs the cosine similarity against an exponential recency bonus (favoring newer memories) and an importance multiplier (favoring memories tagged as highly critical).4.3 ForgettingA system that remembers everything quickly degrades into operational paralysis due to context pollution and vector database bloating. Forgetting is not a flaw, but a critical feature for efficiency.Forgetting Algorithms: The architecture must implement biologically-inspired forgetting, mirroring the Ebbinghaus forgetting curve. The FadeMem architecture provides a blueprint: memory strength undergoes exponential time-based decay, represented by $v(t) = v(0) \cdot \exp(-\lambda \cdot (t - \tau)^\beta)$. The decay rate $\lambda$ adapts based on the memory's assigned importance.Mechanism: Short-term, low-importance episodic logs experience rapid, super-linear decay. High-importance semantic facts experience sub-linear, gradual decay. When a memory's strength value drops below a defined threshold, it is automatically purged or archived to cold storage.4.4 ConsolidationConsolidation is the process of synthesizing raw, noisy episodic data into clean semantic and procedural knowledge.Sleep-like Processes: Drawing inspiration from the "wake-sleep" algorithm in machine learning and human biological rhythms, consolidation is executed as an asynchronous background job during periods of low user interaction.Batch vs. Continuous: Running in batch mode, a dedicated "Reflector" agent analyzes clusters of recent episodic memories. It identifies recurring failures, deduces systemic patterns, and writes new abstract rules into the Semantic Memory while aggressively pruning the now-redundant episodic logs.Section 5: Memory in Production SystemsThe landscape of agentic frameworks provides varied approaches to memory management, ranging from simple prompt buffers to enterprise-grade operating systems.Framework ComparisonsFramework / SystemCore Memory PhilosophySTM ManagementLTM IntegrationEnterprise SuitabilityLangChainOrchestrator-first, modular chains.Highly configurable. Offers ConversationBufferMemory (verbatim) and ConversationSummaryMemory (LLM-compressed).Achieved via VectorStoreRetrieverMemory. Requires manual wiring of databases and embeddings.High for custom builds, requires significant developer effort to manage state.LlamaIndexData-first, retrieval-focused.Basic session tracking, but heavily optimized for ingesting and chunking large contexts.Superior RAG capabilities. Built-in GraphRAG and advanced index structures (Tree, Keyword).Excellent for knowledge-heavy document retrieval; less ideal for complex multi-step reasoning.AutoGenMulti-agent conversational tracking.Agents share a message history buffer. Stateful orchestration with explicit turn-taking.Utilizes an AgentMemory class, but often requires custom extensions for complex cross-session persistence.Strong for multi-agent simulation, but requires custom engineering for long-term semantic storage.MemGPT (Letta)OS-inspired virtual memory management.Pinned "Core Memory" blocks directly managed by the LLM via function calls."Archival" and "Recall" memory layers paged in and out of context on demand.Production-ready. Solves infinite context illusion through strict LLM-driven pagination.ZepTemporal Knowledge Graph engine.Ultra-low latency (<200ms) context injection via APIs.Dynamically synthesizes conversational data into a temporally-aware graph.Enterprise-grade. Outperforms standard RAG by maintaining historical relationships and entity tracking.Mem0Universal, self-improving memory layer.Manages session state natively.Handles user, agent, and session memory via vector stores and graph services.Highly scalable. Features built-in automatic conflict resolution and cost optimization.For the OPENAKTA system, the optimal path combines LangGraph's robust state management for working memory with a custom, Letta-inspired persistence layer using local SQLite vector stores, avoiding the overhead of external SaaS APIs.Section 6: Multi-Agent Memory SharingAs the OPENAKTA ecosystem scales horizontally with specialized agents (e.g., Planning, Coding, Testing), coordinating state becomes the primary architectural bottleneck.6.1 Private vs. Shared Memory DesignA strict partition between private and shared state is required to prevent context bloat and ensure security.Private Memory: Individual agents maintain isolated STM buffers and private episodic scratchpads. The Coding Agent's granular syntax iterations or terminal debugging outputs are stored privately. Broadcasting these low-level traces to the Planning Agent would cause severe attention degradation.Shared Memory: The global state encompasses the Semantic Knowledge Graph of the codebase, the shared Procedural Skill Library, and high-level milestone completion logs. This acts as the single source of truth for team progress.6.2 Memory ConsistencyWhen multiple agents attempt to modify the shared semantic memory concurrently, severe race conditions and logical divergences occur. Sequential, lock-based coordination cripples parallel speedups.CRDT Architecture: OPENAKTA adopts the CodeCRDT paradigm, utilizing Conflict-Free Replicated Data Types to provide Strong Eventual Consistency (SEC). Agents coordinate by observing updates to a shared state graph without explicit message passing. CRDTs guarantee that concurrent text or array modifications mathematically converge without requiring central locking. Vector clocks are employed to track causality and execution ordering.Conflict Resolution: While CRDTs resolve structural merge conflicts automatically, they cannot resolve semantic conflicts (e.g., two agents implementing contradictory architectural patterns). To address this 5-10% semantic failure rate , the Adaptive Memory via Multi-Agent Collaboration (AMA) framework is utilized. A background "Judge" agent audits the shared state for logical consistency. If a conflict is detected, it triggers a "Refresher" agent to execute targeted rollbacks or synthesize a unified solution.6.3 Memory Access ControlEnterprise deployments necessitate strict privacy controls across multi-user, multi-agent systems.Bipartite Access Graphs: Memory access is governed by dynamic bipartite graphs mapping agent-to-resource permissions. During the retrieval phase, queries enforce a hard filter against the agent's cryptographic role. A Frontend UI Agent is structurally prohibited from reading the private episodic traces of the Backend Database Agent, preventing unauthorized exposure of sensitive connection strings or environment variables.6.4 Collective MemoryThe ultimate advantage of a multi-agent system is the emergence of collective intelligence. The shared memory substrate functions as a continuous feedback loop. As individual agents solve novel errors, the asynchronous consolidation "sleep cycle" abstracts these solutions into the global Procedural Skill Library. Consequently, the entire ecosystem becomes progressively more resilient and capable, leveraging the aggregated historical experience of all participating agents.Section 7: Code-Specific MemoryApplying general-purpose text retrieval to software engineering environments results in catastrophic context loss. Code represents highly structured, multi-hop logic that flat vector indices fail to capture.7.1 Codebase Knowledge RepresentationTo accurately represent a repository, OPENAKTA parses the raw source code into an Abstract Syntax Tree (AST).cAST Chunking: The tree-sitter library is used to recursively chunk the code based on syntax boundaries (functions, classes, logic blocks) rather than arbitrary character counts.Knowledge Graph Linking: These syntactic chunks are embedded into the semantic memory database, but crucially, they are linked via graph edges representing control flow, inheritance, and import dependencies. When an agent queries a specific function, GraphRAG techniques retrieve not just the function body, but its entire web of structural dependencies, providing the precise context required for accurate code generation.7.2 User Preferences StorageFriction between developers and AI agents frequently arises from stylistic disagreements. OPENAKTA implements a dedicated user_preferences table. Through observation of manual user overrides (e.g., the user consistently altering the agent's generated snake_case variables to camelCase), the system infers and explicitly stores coding style rules. These preferences are fetched and forcefully appended to the system prompt as immutable constraints for all subsequent code generation tasks in that repository.7.3 Project ContextLong-running projects accumulate complex architectural decisions and technical debt that are rarely documented in the immediate code syntax. The semantic memory layer tracks high-level project goals, framework choices, and systemic bottlenecks. By retrieving this project context prior to planning new features, the agent avoids introducing anti-patterns or violating established structural norms.7.4 Session State ManagementDuring complex, multi-file refactoring, an agent must maintain persistent state across multiple tool calls and potential interruptions. The session state acts as an execution checkpoint, tracking the queue of open compilation errors, pending file modifications, and sub-task progress. Leveraging LangGraph's checkpointer mechanism, the exact graph state is serialized at every step. If the agent crashes or requires human-in-the-loop intervention, execution can be perfectly resumed without losing the operational context.Section 8: Implementation ConsiderationsTo achieve production-grade performance, low latency, and robust privacy, the architectural implementation of the memory substrate requires specific engineering choices tailored to the Rust ecosystem.8.1 Storage BackendOPENAKTA avoids high-latency external SaaS vector databases by utilizing an embedded, hybrid approach. The core storage mechanism is SQLite, augmented by the sqlite-vec C extension. This allows the system to store vast quantities of 1536-dimensional float embeddings alongside standard relational metadata (timestamps, agent IDs, access counts) within a single, highly portable .db file. The local execution ensures sub-millisecond retrieval latencies, eliminates network overhead, and guarantees absolute data privacy.8.2 Memory APIsAgents interface with the memory substrate through an abstraction layer that handles the complexity of dense vector math and SQL queries. The API exposes semantic functions such as memory.search(query, context_filters), memory.encode_episode(trace), and memory.update_preference(key, value). This decoupled design ensures that the cognitive reasoning loops of the LLM agent remain independent from the underlying database mechanics.8.3 Performance and PersistencePerformance: The sqlite-vec extension provides extremely rapid Approximate Nearest Neighbor (ANN) search capabilities, scanning millions of vectors in under 2 milliseconds. Database connection pooling via the sqlx crate in Rust manages concurrent multi-agent read/write operations safely.Persistence: Storing all memory state within a standard SQLite file allows for trivial persistence across sessions. Users maintain total control over their data, with the ability to export, backup, or cryptographically wipe the memory database on command, ensuring complete lifecycle ownership.Database SchemaSQL-- Enable vector search extension
--.load sqlite-vec

-- 1. Episodic Memory (Event Logs & Conversation History)
CREATE TABLE episodic_memory (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL, 
    content TEXT NOT NULL,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    importance_score REAL DEFAULT 1.0,
    access_count INTEGER DEFAULT 0,
    last_accessed DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE VIRTUAL TABLE episodic_embeddings USING vec0(
    id TEXT PRIMARY KEY,
    embedding FLOAT
);

-- 2. Semantic Memory (Facts & Codebase Knowledge)
CREATE TABLE semantic_memory (
    id TEXT PRIMARY KEY,
    entity_id TEXT, 
    topic TEXT NOT NULL,
    content TEXT NOT NULL,
    confidence_score REAL DEFAULT 1.0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_updated DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE VIRTUAL TABLE semantic_embeddings USING vec0(
    id TEXT PRIMARY KEY,
    embedding FLOAT
);

-- 3. Procedural Memory (Skills & Action Patterns)
CREATE TABLE procedural_memory (
    id TEXT PRIMARY KEY,
    skill_name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    execution_steps TEXT NOT NULL, 
    success_rate REAL DEFAULT 1.0,
    invocation_count INTEGER DEFAULT 0
);

CREATE VIRTUAL TABLE procedural_embeddings USING vec0(
    id TEXT PRIMARY KEY,
    embedding FLOAT
);

-- 4. User Preferences
CREATE TABLE user_preferences (
    id TEXT PRIMARY KEY,
    category TEXT NOT NULL, 
    preference TEXT NOT NULL,
    inferred_from_episode_id TEXT,
    FOREIGN KEY(inferred_from_episode_id) REFERENCES episodic_memory(id)
);

-- Indexes for rapid filtering
CREATE INDEX idx_episodic_agent_session ON episodic_memory(agent_id, session_id);
CREATE INDEX idx_semantic_topic ON semantic_memory(topic);
Rustuse chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#
pub struct EpisodicMemory {
    pub id: Uuid,
    pub agent_id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub timestamp: DateTime<Utc>,
    pub importance_score: f64,
    pub access_count: i32,
    pub last_accessed: DateTime<Utc>,
}

#
pub struct SemanticMemory {
    pub id: Uuid,
    pub entity_id: Option<String>,
    pub topic: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub confidence_score: f64,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

#
pub struct ProceduralMemory {
    pub id: Uuid,
    pub skill_name: String,
    pub description: String,
    pub execution_steps: String,
    pub embedding: Vec<f32>,
    pub success_rate: f64,
    pub invocation_count: i32,
}
Retrieval AlgorithmRustuse std::f64::consts::E;

pub fn calculate_retrieval_score(
    similarity: f64,
    importance: f64,
    timestamp: DateTime<Utc>,
    last_accessed: DateTime<Utc>,
    access_count: i32,
) -> f64 {
    let now = Utc::now();
    let hours_since_creation = (now - timestamp).num_hours() as f64;
    let hours_since_access = (now - last_accessed).num_hours() as f64;

    // Temporal recency bonus (logistic decay curve)
    let recency_bonus = 1.0 / (1.0 + (hours_since_creation / 24.0).powf(0.5));
    
    // Access frequency multiplier
    let access_multiplier = 1.0 + (0.1 * (access_count as f64).ln_1p());

    // FadeMem-inspired importance decay penalty
    let lambda = 0.1 / importance.max(0.1); 
    let decay_penalty = E.powf(-lambda * (hours_since_access).powf(0.8));

    // Composite Score Algorithm
    (similarity * 0.5) + (recency_bonus * 0.2) + (decay_penalty * 0.3) * access_multiplier
}

pub fn retrieve_and_rerank_memory(
    query_embedding: Vec<f32>, 
    candidates: Vec<EpisodicMemory>
) -> Vec<EpisodicMemory> {
    let mut scored_candidates: Vec<(f64, EpisodicMemory)> = candidates.into_iter().map(|mem| {
        let sim = compute_cosine_similarity(&query_embedding, &mem.embedding);
        let final_score = calculate_retrieval_score(
            sim,
            mem.importance_score,
            mem.timestamp,
            mem.last_accessed,
            mem.access_count,
        );
        (final_score, mem)
    }).collect();

    // Sort descending by final composite score
    scored_candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    
    // Return top reranked results
    scored_candidates.into_iter().map(|(_, mem)| mem).take(5).collect()
}

// Helper function placeholder
fn compute_cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    // Standard dot product over magnitudes implementation
    0.99 
}
Forgetting StrategyAlgorithm: Biologically-Inspired Adaptive Forgetting (FadeMem Integration)
The system calculates a continuous decay curve for all episodic memory vectors, simulating human neural pruning to prevent database bloat and context irrelevance. During the background "sleep cycle" cron job, the system evaluates the retention value of memories. Low importance memories decay super-linearly and are purged quickly, while high importance semantic facts decay sub-linearly.Decay Function:$$v(t) = v(0) \cdot \exp(-\lambda \cdot (t - \tau)^\beta)$$Where:$v(t)$: Current memory retention strength.$v(0)$: Initial importance score at encoding.$\lambda$: Adaptive decay rate (inversely proportional to initial importance).$t - \tau$: Time elapsed since last access.$\beta$: Shape parameter determining the curve (0.8 for long-term critical storage, 1.2 for volatile short-term traces).Thresholds:Archival Threshold (< 0.2): Memory is removed from the active sqlite-vec index and compressed into cold storage (disk text logs).Deletion Threshold (< 0.05): Memory is permanently dropped.Rustpub fn process_forgetting_cycle(memories: &mut Vec<EpisodicMemory>) {
    let now = Utc::now();
    let archive_threshold = 0.2;
    let delete_threshold = 0.05;

    memories.retain_mut(|mem| {
        let hours_elapsed = (now - mem.last_accessed).num_hours() as f64;
        
        // Lambda adapts based on importance. High importance = low lambda (slow decay).
        let lambda = 0.1 / mem.importance_score.max(0.01);
        
        // Beta determines curve shape. 1.2 = super-linear (rapid) decay for episodic data.
        let beta = 1.2;
        
        // Calculate current retention strength v(t)
        let retention_strength = mem.importance_score * E.powf(-lambda * hours_elapsed.powf(beta));
        
        if retention_strength < delete_threshold {
            // Flag for complete database deletion
            false 
        } else if retention_strength < archive_threshold {
            // Flag to remove from vector index but keep in deep archival storage
            archive_memory(mem);
            true
        } else {
            true // Keep active
        }
    });
}

fn archive_memory(mem: &EpisodicMemory) {
    // Implementation to move to cold storage
}
Open Questions[ ] Given the overhead of computing multi-hop dependencies via GraphRAG, at what specific repository scale (measured in lines of code or AST node count) does the latency of graph traversal outweigh the accuracy benefits compared to traditional hierarchical chunking?[ ] How will the background consolidation "sleep cycle" accurately classify the boundary between a successful procedural trace (to be saved as a skill) and a highly specific, non-generalizable hack without direct human-in-the-loop verification?[ ] During SEC conflict resolution using CRDTs, if the AMA Judge agent forces a rollback of a semantically conflicting code edit, how is the corresponding episodic memory trace retroactively invalidated to prevent the agent from learning from the reverted hallucination?Next StepsImplement the foundational rusqlite database schema, integrating the sqlite-vec C extension, and run a localized benchmark testing k-NN retrieval latency across a simulated dataset of 100,000 episodic vectors.Develop the tree-sitter integration module to ingest a sample Python/Rust repository, testing the recursive AST parsing logic and mapping the output into the Semantic Memory SQL tables.Construct a simplified, two-agent test harness utilizing the yrs (Yjs) Rust crate to simulate concurrent mutations on a shared document, explicitly monitoring for the emergence of semantic conflicts to evaluate the necessary intervention thresholds for the Judge agent