-- Task queue table
-- Implements atomic checkout semantics for task assignment

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    description TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    assignee_id TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    checked_out_at TEXT,
    completed_at TEXT,
    result TEXT
);

-- Index for efficient checkout queries
-- Orders by priority (DESC) then created_at (ASC) for FIFO within priority
CREATE INDEX IF NOT EXISTS idx_tasks_status_priority 
ON tasks(status, priority DESC, created_at ASC);

-- Index for assignee lookup
CREATE INDEX IF NOT EXISTS idx_tasks_assignee 
ON tasks(assignee_id);

-- Index for timeout detection
CREATE INDEX IF NOT EXISTS idx_tasks_checked_out 
ON tasks(checked_out_at) WHERE status = 'in_progress';
