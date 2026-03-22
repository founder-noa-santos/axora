-- Plan 9: local research memory (sessions + FTS5 + 384-d f32 embeddings as BLOB)

CREATE TABLE IF NOT EXISTS research_schema_migrations (
  version INTEGER PRIMARY KEY NOT NULL
);

CREATE TABLE IF NOT EXISTS research_sessions (
  id TEXT PRIMARY KEY NOT NULL,
  workspace_root TEXT NOT NULL,
  query_text TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  provider_used TEXT,
  raw_metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS search_results (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL REFERENCES research_sessions(id) ON DELETE CASCADE,
  rank_in_session INTEGER NOT NULL,
  title TEXT NOT NULL,
  url TEXT NOT NULL,
  snippet TEXT NOT NULL,
  embedding BLOB NOT NULL CHECK (length(embedding) = 1536),
  embedded_text_hash TEXT NOT NULL,
  UNIQUE(session_id, url)
);

CREATE INDEX IF NOT EXISTS idx_search_results_session ON search_results(session_id);

CREATE VIRTUAL TABLE IF NOT EXISTS search_results_fts USING fts5(
  title,
  snippet,
  url,
  content='search_results',
  content_rowid='id',
  tokenize='unicode61'
);

-- Keep FTS5 in sync with content table (external content)
CREATE TRIGGER IF NOT EXISTS search_results_ai AFTER INSERT ON search_results BEGIN
  INSERT INTO search_results_fts(rowid, title, snippet, url)
  VALUES (new.id, new.title, new.snippet, new.url);
END;

CREATE TRIGGER IF NOT EXISTS search_results_ad AFTER DELETE ON search_results BEGIN
  INSERT INTO search_results_fts(search_results_fts, rowid, title, snippet, url)
  VALUES('delete', old.id, old.title, old.snippet, old.url);
END;

CREATE TRIGGER IF NOT EXISTS search_results_au AFTER UPDATE ON search_results BEGIN
  INSERT INTO search_results_fts(search_results_fts, rowid, title, snippet, url)
  VALUES('delete', old.id, old.title, old.snippet, old.url);
  INSERT INTO search_results_fts(rowid, title, snippet, url)
  VALUES (new.id, new.title, new.snippet, new.url);
END;

INSERT OR IGNORE INTO research_schema_migrations (version) VALUES (1);
