-- Memory-domain schema for tripartite memory and lifecycle management

CREATE TABLE IF NOT EXISTS episodic_events (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    turn_number INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    content TEXT NOT NULL,
    success INTEGER,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_episodic_events_session_turn
    ON episodic_events(session_id, turn_number);
CREATE INDEX IF NOT EXISTS idx_episodic_events_created_at
    ON episodic_events(created_at);

CREATE TABLE IF NOT EXISTS semantic_memory_registry (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    doc_type TEXT NOT NULL,
    embedding_ref TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_semantic_memory_registry_source
    ON semantic_memory_registry(source);

CREATE TABLE IF NOT EXISTS procedural_skills (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'staged',
    trigger_summary TEXT NOT NULL DEFAULT '',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_procedural_skills_status
    ON procedural_skills(status);

CREATE TABLE IF NOT EXISTS memory_utility_scores (
    skill_id TEXT PRIMARY KEY,
    utility_score REAL NOT NULL DEFAULT 0.5,
    retrieval_count INTEGER NOT NULL DEFAULT 0,
    success_count INTEGER NOT NULL DEFAULT 0,
    failure_count INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS pruning_checkpoints (
    worker_name TEXT PRIMARY KEY,
    last_run_at TEXT NOT NULL,
    last_cursor TEXT,
    pruned_count INTEGER NOT NULL DEFAULT 0,
    metadata TEXT NOT NULL DEFAULT '{}'
);
