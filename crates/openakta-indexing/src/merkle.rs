//! Persisted Merkle-style index for file and block level change detection.

use crate::chunker::{BlockId, Chunker};
use crate::error::IndexingError;
use crate::Result;
use blake3::hash;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// File-level hash entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileHashEntry {
    /// Relative file path.
    pub file_path: PathBuf,
    /// BLAKE3 hash of the file content.
    pub hash: String,
}

/// Block-level hash entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockHashEntry {
    /// Stable block identifier.
    pub block_id: BlockId,
    /// Relative file path.
    pub file_path: PathBuf,
    /// Line range.
    pub line_range: (usize, usize),
    /// Language tag.
    pub language: String,
    /// Symbol path when known.
    pub symbol_path: Option<String>,
    /// BLAKE3 hash of the chunk content.
    pub hash: String,
}

/// Incremental indexing delta.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IndexDelta {
    /// Upsert a changed or new block.
    UpsertBlock(BlockHashEntry),
    /// Delete a removed block.
    DeleteBlock {
        /// Stable block identifier.
        block_id: BlockId,
        /// Relative file path.
        file_path: PathBuf,
    },
    /// File did not require any block updates.
    Noop {
        /// Relative file path.
        file_path: PathBuf,
    },
}

/// Persisted Merkle-style index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTree {
    /// Root of the indexed workspace.
    pub root_path: PathBuf,
    /// File-level hashes keyed by relative path.
    pub file_hashes: HashMap<PathBuf, FileHashEntry>,
    /// Block-level hashes keyed by stable block id.
    pub block_hashes: HashMap<BlockId, BlockHashEntry>,
}

impl MerkleTree {
    /// Build the index from the filesystem.
    pub fn build(root_path: &Path) -> Result<Self> {
        info!("Building persisted Merkle index for {:?}", root_path);
        if !root_path.exists() {
            return Err(IndexingError::MerkleTree("Root path does not exist".to_string()).into());
        }

        let mut chunker = Chunker::new()?;
        let mut file_hashes = HashMap::new();
        let mut block_hashes = HashMap::new();

        for path in walk_files(root_path)? {
            if should_skip_file(&path) {
                debug!("Skipping file {}", path.display());
                continue;
            }
            let relative_path = path.strip_prefix(root_path).unwrap_or(&path).to_path_buf();
            let content = match fs::read_to_string(&path) {
                Ok(content) => content,
                Err(err) => {
                    warn!(
                        "Skipping unreadable/non-UTF8 file {}: {}",
                        path.display(),
                        err
                    );
                    continue;
                }
            };
            let file_hash = hash(content.as_bytes()).to_hex().to_string();
            file_hashes.insert(
                relative_path.clone(),
                FileHashEntry {
                    file_path: relative_path.clone(),
                    hash: file_hash,
                },
            );

            let language = Chunker::detect_language(&path).unwrap_or_else(|| "unknown".to_string());
            let chunks = chunker.extract_chunks(&content, &relative_path, &language)?;
            for chunk in chunks {
                block_hashes.insert(
                    chunk.id.clone(),
                    BlockHashEntry {
                        block_id: chunk.id,
                        file_path: relative_path.clone(),
                        line_range: chunk.line_range,
                        language: chunk.language,
                        symbol_path: chunk.metadata.symbol_path,
                        hash: chunk.metadata.content_hash,
                    },
                );
            }
        }

        Ok(Self {
            root_path: root_path.to_path_buf(),
            file_hashes,
            block_hashes,
        })
    }

    /// Save the current index state to disk.
    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        let serialized = serde_json::to_vec_pretty(self)
            .map_err(|e| IndexingError::MerkleTree(format!("Failed to serialize state: {}", e)))?;
        fs::write(path, serialized).map_err(|e| {
            IndexingError::MerkleTree(format!("Failed to persist {}: {}", path.display(), e))
        })?;
        Ok(())
    }

    /// Load a previously persisted state.
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let bytes = fs::read(path).map_err(|e| {
            IndexingError::MerkleTree(format!("Failed to load {}: {}", path.display(), e))
        })?;
        serde_json::from_slice(&bytes)
            .map_err(|e| IndexingError::MerkleTree(format!("Failed to parse state: {}", e)).into())
    }

    /// Compute block-level deltas against a previous state.
    pub fn diff(&self, old_tree: &MerkleTree) -> Vec<IndexDelta> {
        let mut deltas = Vec::new();
        let file_paths: HashSet<PathBuf> = self
            .file_hashes
            .keys()
            .chain(old_tree.file_hashes.keys())
            .cloned()
            .collect();

        for file_path in file_paths {
            let new_hash = self
                .file_hashes
                .get(&file_path)
                .map(|entry| entry.hash.as_str());
            let old_hash = old_tree
                .file_hashes
                .get(&file_path)
                .map(|entry| entry.hash.as_str());

            if new_hash == old_hash {
                deltas.push(IndexDelta::Noop { file_path });
                continue;
            }

            let old_blocks = old_tree.blocks_for_file(&file_path);
            let new_blocks = self.blocks_for_file(&file_path);

            for (block_id, old_entry) in &old_blocks {
                match new_blocks.get(block_id) {
                    None => deltas.push(IndexDelta::DeleteBlock {
                        block_id: block_id.clone(),
                        file_path: old_entry.file_path.clone(),
                    }),
                    Some(new_entry) if new_entry.hash != old_entry.hash => {
                        deltas.push(IndexDelta::UpsertBlock(new_entry.clone()))
                    }
                    Some(_) => {}
                }
            }

            for (block_id, new_entry) in &new_blocks {
                if !old_blocks.contains_key(block_id) {
                    deltas.push(IndexDelta::UpsertBlock(new_entry.clone()));
                }
            }
        }

        deltas
    }

    /// Find changed files compared to a previous state.
    pub fn find_changed(&self, old_tree: &MerkleTree) -> Vec<PathBuf> {
        let mut changed = HashSet::new();
        for delta in self.diff(old_tree) {
            match delta {
                IndexDelta::UpsertBlock(entry) => {
                    changed.insert(entry.file_path);
                }
                IndexDelta::DeleteBlock { file_path, .. } => {
                    changed.insert(file_path);
                }
                IndexDelta::Noop { .. } => {}
            }
        }
        changed.into_iter().collect()
    }

    /// Update a single file in memory with new content.
    pub fn update(&mut self, file_path: &Path, new_content: &[u8]) -> Result<()> {
        let relative_path = file_path
            .strip_prefix(&self.root_path)
            .unwrap_or(file_path)
            .to_path_buf();
        let content = String::from_utf8(new_content.to_vec()).map_err(|e| {
            IndexingError::MerkleTree(format!(
                "File {} is not valid UTF-8: {}",
                file_path.display(),
                e
            ))
        })?;
        let mut chunker = Chunker::new()?;
        let language =
            Chunker::detect_language(&relative_path).unwrap_or_else(|| "unknown".to_string());

        self.file_hashes.insert(
            relative_path.clone(),
            FileHashEntry {
                file_path: relative_path.clone(),
                hash: hash(content.as_bytes()).to_hex().to_string(),
            },
        );

        self.block_hashes
            .retain(|_, entry| entry.file_path != relative_path);
        for chunk in chunker.extract_chunks(&content, &relative_path, &language)? {
            self.block_hashes.insert(
                chunk.id.clone(),
                BlockHashEntry {
                    block_id: chunk.id,
                    file_path: relative_path.clone(),
                    line_range: chunk.line_range,
                    language: chunk.language,
                    symbol_path: chunk.metadata.symbol_path,
                    hash: chunk.metadata.content_hash,
                },
            );
        }

        Ok(())
    }

    /// Get the hash for a specific file.
    pub fn get_file_hash(&self, path: &Path) -> Option<String> {
        self.file_hashes.get(path).map(|entry| entry.hash.clone())
    }

    /// Get number of indexed files.
    pub fn file_count(&self) -> usize {
        self.file_hashes.len()
    }

    /// Get total number of entries.
    pub fn node_count(&self) -> usize {
        self.file_hashes.len() + self.block_hashes.len()
    }

    fn blocks_for_file(&self, file_path: &Path) -> HashMap<BlockId, BlockHashEntry> {
        self.block_hashes
            .iter()
            .filter(|(_, entry)| entry.file_path == file_path)
            .map(|(block_id, entry)| (block_id.clone(), entry.clone()))
            .collect()
    }
}

fn walk_files(root_path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    walk_dir(root_path, root_path, &mut files)?;
    Ok(files)
}

fn walk_dir(root_path: &Path, current: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if let Some(name) = current.file_name().and_then(|value| value.to_str()) {
        if should_skip_directory(name) {
            debug!("Skipping directory {}", current.display());
            return Ok(());
        }
    }

    let entries = fs::read_dir(current).map_err(|e| {
        IndexingError::MerkleTree(format!("Failed to read {}: {}", current.display(), e))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            IndexingError::MerkleTree(format!(
                "Failed to read entry in {}: {}",
                current.display(),
                e
            ))
        })?;
        let path = entry.path();
        if path.is_dir() {
            walk_dir(root_path, &path, files)?;
        } else if path.is_file() && path.starts_with(root_path) {
            files.push(path);
        }
    }

    Ok(())
}

fn should_skip_directory(name: &str) -> bool {
    matches!(
        name,
        "target" | "node_modules" | ".git" | "dist" | "build" | "__pycache__" | ".venv" | "vendor"
    )
}

fn should_skip_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(|name| matches!(name, ".DS_Store"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_merkle_tree_creation() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "content2").unwrap();

        let tree = MerkleTree::build(temp_dir.path()).unwrap();

        assert_eq!(tree.file_count(), 2);
        assert!(tree.node_count() >= 2);
    }

    #[test]
    fn test_merkle_tree_change_detection() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file.rs"), "fn alpha() {}\n").unwrap();
        let tree1 = MerkleTree::build(temp_dir.path()).unwrap();

        fs::write(temp_dir.path().join("file.rs"), "fn beta() {}\n").unwrap();
        let tree2 = MerkleTree::build(temp_dir.path()).unwrap();

        let changed = tree2.find_changed(&tree1);
        assert_eq!(changed, vec![PathBuf::from("file.rs")]);
    }

    #[test]
    fn test_merkle_tree_produces_block_deltas() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.rs");
        fs::write(&file_path, "fn alpha() {}\nfn beta() {}\n").unwrap();
        let tree1 = MerkleTree::build(temp_dir.path()).unwrap();

        fs::write(&file_path, "fn alpha() {}\nfn gamma() {}\n").unwrap();
        let tree2 = MerkleTree::build(temp_dir.path()).unwrap();

        let deltas = tree2.diff(&tree1);
        assert!(deltas
            .iter()
            .any(|delta| matches!(delta, IndexDelta::UpsertBlock(_))));
        assert!(deltas
            .iter()
            .any(|delta| matches!(delta, IndexDelta::DeleteBlock { .. })));
    }

    #[test]
    fn test_merkle_tree_state_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file.rs"), "fn alpha() {}\n").unwrap();
        let tree = MerkleTree::build(temp_dir.path()).unwrap();
        let state_path = temp_dir.path().join("merkle.json");

        tree.save_to_path(&state_path).unwrap();
        let loaded = MerkleTree::load_from_path(&state_path).unwrap();

        assert_eq!(loaded.file_count(), tree.file_count());
        assert_eq!(loaded.block_hashes.len(), tree.block_hashes.len());
    }

    #[test]
    fn test_merkle_tree_update_refreshes_blocks() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.rs");
        fs::write(&file_path, "fn alpha() {}\n").unwrap();
        let mut tree = MerkleTree::build(temp_dir.path()).unwrap();
        let original_blocks = tree.block_hashes.len();

        tree.update(&file_path, b"fn beta() {}\n").unwrap();
        assert_eq!(tree.file_count(), 1);
        assert_eq!(tree.block_hashes.len(), original_blocks);
    }
}
