# SQLite Database Corruption Recovery

This document describes how to recover from SQLite database corruption errors in the OPENAKTA runtime.

## Symptoms

When running `cargo run -p openakta-cli -- do "..."`, you may encounter:

```
openakta_storage::db: "Running database migrations" and "Database initialized at: <workspace>/.openakta/openakta.db"
Then: Error: database error: database disk image is malformed
```

## Root Cause

SQLite databases can become corrupted due to:
- Disk full errors during writes (errno=28)
- Interrupted write operations
- File system corruption
- WAL checkpoint failures

## Affected Database Files

The OPENAKTA runtime maintains multiple SQLite databases under `.openakta/`:

1. **Main database**: `.openakta/openakta.db` (+ `-wal`, `-shm`)
2. **Episodic store**: `.openakta/openakta.episodic.db` (+ `-wal`, `-shm`)
3. **Skill catalog**: `.openakta/skill-index/skill-catalog.db` (+ `-wal`, `-shm`)
4. **Semantic store**: `.openakta/semantic-memory.db` (+ `-wal`, `-shm`)
5. **Vector store** (if SqliteVec enabled): `.openakta/vectors.db` (+ `-wal`, `-shm`)

## Recovery Steps

### Quick Recovery (Recommended)

Remove all corrupted SQLite artifacts:

```bash
# From your workspace root
rm -rf .openakta/openakta.db .openakta/openakta.db-wal .openakta/openakta.db-shm
rm -rf .openakta/openakta.episodic.db .openakta/openakta.episodic.db-wal .openakta/openakta.episodic.db-shm
rm -rf .openakta/skill-index/skill-catalog.db .openakta/skill-index/skill-catalog.db-wal .openakta/skill-index/skill-catalog.db-shm
rm -rf .openakta/semantic-memory.db .openakta/semantic-memory.db-wal .openakta/semantic-memory.db-shm
rm -rf .openakta/vectors.db .openakta/vectors.db-wal .openakta/vectors.db-shm  # If SqliteVec enabled

# Or remove the entire .openakta directory (will be recreated on next run)
rm -rf .openakta/
```

Then re-run your command:

```bash
cargo run -p openakta-cli -- do "..."
```

### Verify Disk Space

Before recovery, ensure you have sufficient disk space:

```bash
df -h .
```

The `.openakta/` directory typically requires 50-200MB depending on skill corpus size and episodic history.

## Improved Error Messages

After the fix, error messages will include the database path:

```
Error: database error at /path/to/workspace/.openakta/openakta.episodic.db: database disk image is malformed
```

This helps identify which specific database file is corrupted.

## Prevention

1. **Monitor disk space**: Ensure adequate free space before running missions
2. **Graceful shutdown**: Allow the runtime to complete WAL checkpoints operations
3. **Regular backups**: Consider backing up `.openakta/` for long-running projects

## Technical Details

### Error Propagation

The runtime now wraps `rusqlite::Error` with path context in:
- `openakta-storage/src/lib.rs`: `StorageError::Database { path, source }`
- `openakta-memory/src/episodic_store.rs`: `EpisodicError::Database { path, source }`
- `openakta-memory/src/semantic_store.rs`: `SemanticError::Storage { path, source }`
- `openakta-memory/src/procedural_store.rs`: `ProceduralError::Database { path, source }`

### Bootstrap Order

Databases are initialized in this order:
1. Main database (`.openakta/openakta.db`) - in `bootstrap.rs`
2. Episodic store (`.openakta/openakta.episodic.db`) - in `runtime_services.rs`
3. Skill catalog (`.openakta/skill-index/skill-catalog.db`) - in `runtime_services.rs`
4. Dual vector store (Qdrant or SqliteVec) - in `runtime_services.rs`
5. Semantic store (`.openakta/semantic-memory.db`) - in `runtime_services.rs`

The error message will indicate which database failed during initialization.
