CREATE TABLE IF NOT EXISTS runtime_seed_versions (
    seed_name TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    applied_at TEXT NOT NULL
);
