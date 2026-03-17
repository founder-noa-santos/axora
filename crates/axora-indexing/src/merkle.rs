//! Merkle tree for efficient change detection

use crate::error::IndexingError;
use crate::Result;
use blake3::{Hash, Hasher};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Hash node in Merkle tree
#[derive(Debug, Clone)]
pub struct HashNode {
    /// Node hash
    pub hash: Hash,
    /// Children paths (for internal nodes)
    pub children: Vec<PathBuf>,
    /// Content hash (for leaf nodes - files)
    pub content_hash: Option<Hash>,
    /// File path (for leaf nodes)
    pub file_path: Option<PathBuf>,
}

/// Merkle tree for efficient change detection
pub struct MerkleTree {
    /// Root hash
    pub root_hash: Hash,
    /// All nodes indexed by path
    pub nodes: HashMap<PathBuf, HashNode>,
    /// Root path of the indexed directory
    pub root_path: PathBuf,
}

impl MerkleTree {
    /// Build Merkle tree from filesystem
    pub fn build(root_path: &Path) -> Result<Self> {
        info!("Building Merkle tree for: {:?}", root_path);

        if !root_path.exists() {
            return Err(IndexingError::MerkleTree("Root path does not exist".to_string()).into());
        }

        let mut nodes: HashMap<PathBuf, HashNode> = HashMap::new();
        let mut file_hashes: Vec<(PathBuf, Hash)> = Vec::new();

        // Walk the directory tree
        Self::walk_directory(root_path, root_path, &mut file_hashes, &mut nodes)?;

        // Build tree from file hashes
        let root_hash = Self::build_tree(&file_hashes, root_path, &mut nodes)?;

        info!(
            "Merkle tree built: {} files, {} nodes",
            file_hashes.len(),
            nodes.len()
        );

        Ok(Self {
            root_hash,
            nodes,
            root_path: root_path.to_path_buf(),
        })
    }

    /// Recursively walk directory and hash files
    fn walk_directory(
        root: &Path,
        current: &Path,
        file_hashes: &mut Vec<(PathBuf, Hash)>,
        nodes: &mut HashMap<PathBuf, HashNode>,
    ) -> Result<()> {
        // Skip common directories that shouldn't be indexed
        if let Some(name) = current.file_name().and_then(|n| n.to_str()) {
            if Self::should_skip_directory(name) {
                debug!("Skipping directory: {:?}", current);
                return Ok(());
            }
        }

        let entries = fs::read_dir(current)
            .map_err(|e| IndexingError::MerkleTree(format!("Failed to read directory: {}", e)))?;

        let mut child_paths = Vec::new();

        for entry in entries {
            let entry = entry
                .map_err(|e| IndexingError::MerkleTree(format!("Failed to read entry: {}", e)))?;

            let path = entry.path();

            if path.is_dir() {
                child_paths.push(path.clone());
                Self::walk_directory(root, &path, file_hashes, nodes)?;
            } else if path.is_file() {
                // Hash file content
                let hash = Self::hash_file(&path)?;
                let relative_path = path.strip_prefix(root).unwrap_or(&path).to_path_buf();

                file_hashes.push((relative_path.clone(), hash));

                // Create leaf node
                let node = HashNode {
                    hash,
                    children: vec![],
                    content_hash: Some(hash),
                    file_path: Some(relative_path.clone()),
                };
                nodes.insert(relative_path, node);
            }
        }

        // Create internal node for this directory
        if !child_paths.is_empty() {
            let dir_path = current.strip_prefix(root).unwrap_or(current).to_path_buf();
            let dir_hash = Self::hash_children(&child_paths, nodes);

            let node = HashNode {
                hash: dir_hash,
                children: child_paths.clone(),
                content_hash: None,
                file_path: None,
            };
            nodes.insert(dir_path, node);
        }

        Ok(())
    }

    /// Build tree structure from file hashes
    fn build_tree(
        file_hashes: &[(PathBuf, Hash)],
        _root_path: &Path,
        nodes: &mut HashMap<PathBuf, HashNode>,
    ) -> Result<Hash> {
        if file_hashes.is_empty() {
            // Empty directory
            let empty_hash = Self::hash_content(b"");
            return Ok(empty_hash);
        }

        // Group files by directory
        let mut dir_files: HashMap<PathBuf, Vec<(PathBuf, Hash)>> = HashMap::new();

        for (path, hash) in file_hashes {
            let parent = path.parent().unwrap_or(Path::new("")).to_path_buf();
            dir_files
                .entry(parent)
                .or_default()
                .push((path.clone(), *hash));
        }

        // Compute directory hashes bottom-up
        let mut dirs: Vec<_> = dir_files.keys().cloned().collect();
        dirs.sort_by(|a, b| b.cmp(a)); // Sort by depth (deepest first)

        // First pass: create directory nodes
        for dir in &dirs {
            let files = &dir_files[dir];

            // Hash of this directory is hash of all children
            let child_hashes: Vec<Hash> = files.iter().map(|(_, h)| *h).collect();
            let dir_hash = Self::hash_hashes(&child_hashes);

            let child_paths: Vec<PathBuf> = files.iter().map(|(p, _)| p.clone()).collect();
            let node = HashNode {
                hash: dir_hash,
                children: child_paths,
                content_hash: None,
                file_path: None,
            };
            nodes.insert(dir.clone(), node);
        }

        // Root hash - use hash of all file hashes for simplicity
        let all_hashes: Vec<Hash> = file_hashes.iter().map(|(_, h)| *h).collect();
        let root_hash = Self::hash_hashes(&all_hashes);

        Ok(root_hash)
    }

    /// Hash a file's content
    fn hash_file(path: &Path) -> Result<Hash> {
        let file = File::open(path)
            .map_err(|e| IndexingError::MerkleTree(format!("Failed to open file: {}", e)))?;

        let mut reader = BufReader::new(file);
        let mut hasher = Hasher::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = reader
                .read(&mut buffer)
                .map_err(|e| IndexingError::MerkleTree(format!("Failed to read file: {}", e)))?;

            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(hasher.finalize())
    }

    /// Hash content directly
    fn hash_content(content: &[u8]) -> Hash {
        blake3::hash(content)
    }

    /// Hash multiple child hashes together
    fn hash_hashes(hashes: &[Hash]) -> Hash {
        let mut hasher = Hasher::new();
        for hash in hashes {
            hasher.update(hash.as_bytes());
        }
        hasher.finalize()
    }

    /// Hash children nodes
    fn hash_children(children: &[PathBuf], nodes: &HashMap<PathBuf, HashNode>) -> Hash {
        let hashes: Vec<Hash> = children
            .iter()
            .filter_map(|path| nodes.get(path).map(|n| n.hash))
            .collect();
        Self::hash_hashes(&hashes)
    }

    /// Check if directory should be skipped
    fn should_skip_directory(name: &str) -> bool {
        matches!(
            name,
            "target"
                | "node_modules"
                | ".git"
                | "dist"
                | "build"
                | "__pycache__"
                | ".venv"
                | "vendor"
        )
    }

    /// Find changed files compared to old tree
    pub fn find_changed(&self, old_tree: &MerkleTree) -> Vec<PathBuf> {
        debug!("Finding changed files");

        let mut changed = Vec::new();

        // If root hash is the same, nothing changed
        if self.root_hash == old_tree.root_hash {
            debug!("Root hashes match, no changes");
            return changed;
        }

        // Find differing nodes
        for (path, node) in &self.nodes {
            if let Some(old_node) = old_tree.nodes.get(path) {
                if node.hash != old_node.hash {
                    // Node changed
                    if node.content_hash.is_some() {
                        // File changed
                        if let Some(file_path) = &node.file_path {
                            changed.push(file_path.clone());
                        }
                    }
                    // For directories, we'll check children recursively
                }
            } else {
                // New file/directory
                if let Some(file_path) = &node.file_path {
                    debug!("New file: {:?}", file_path);
                    changed.push(file_path.clone());
                }
            }
        }

        // Check for deleted files
        for (path, node) in &old_tree.nodes {
            if !self.nodes.contains_key(path) {
                if let Some(file_path) = &node.file_path {
                    debug!("Deleted file: {:?}", file_path);
                    // Mark as changed (will be processed as deletion)
                    changed.push(file_path.clone());
                }
            }
        }

        info!("Found {} changed files", changed.len());
        changed
    }

    /// Update tree with new content
    pub fn update(&mut self, file_path: &Path, new_content: &[u8]) -> Result<()> {
        debug!("Updating tree for file: {:?}", file_path);

        // Hash new content
        let new_hash = Self::hash_content(new_content);

        // Get relative path
        let relative_path = file_path
            .strip_prefix(&self.root_path)
            .unwrap_or(file_path)
            .to_path_buf();

        // Update or create leaf node
        let node = HashNode {
            hash: new_hash,
            children: vec![],
            content_hash: Some(new_hash),
            file_path: Some(relative_path.clone()),
        };

        self.nodes.insert(relative_path.clone(), node);

        // Recompute root hash
        // In a full implementation, we'd update all parent hashes
        // For now, rebuild the tree
        let file_hashes: Vec<(PathBuf, Hash)> = self
            .nodes
            .iter()
            .filter_map(|(path, node)| node.content_hash.map(|h| (path.clone(), h)))
            .collect();

        self.root_hash = Self::build_tree(&file_hashes, &self.root_path, &mut self.nodes)?;

        Ok(())
    }

    /// Get the hash for a specific file
    pub fn get_file_hash(&self, path: &Path) -> Option<Hash> {
        self.nodes.get(path).and_then(|n| n.content_hash)
    }

    /// Get number of files in tree
    pub fn file_count(&self) -> usize {
        self.nodes
            .values()
            .filter(|n| n.content_hash.is_some())
            .count()
    }

    /// Get number of nodes (files + directories)
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_merkle_tree_creation() {
        let temp_dir = TempDir::new().unwrap();

        // Create some test files
        fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "content2").unwrap();

        let tree = MerkleTree::build(temp_dir.path()).unwrap();

        assert_eq!(tree.file_count(), 2);
        assert!(tree.node_count() >= 2); // Files + possibly directory nodes
    }

    #[test]
    fn test_merkle_tree_change_detection() {
        let temp_dir = TempDir::new().unwrap();

        // Create initial file
        fs::write(temp_dir.path().join("file.txt"), "original").unwrap();
        let tree1 = MerkleTree::build(temp_dir.path()).unwrap();

        // Modify file
        fs::write(temp_dir.path().join("file.txt"), "modified").unwrap();
        let tree2 = MerkleTree::build(temp_dir.path()).unwrap();

        let changed = tree2.find_changed(&tree1);

        assert_eq!(changed.len(), 1);
        assert!(changed.iter().any(|p| p.file_name().unwrap() == "file.txt"));
    }

    #[test]
    fn test_merkle_tree_new_file() {
        let temp_dir = TempDir::new().unwrap();

        // Create initial tree (empty)
        let tree1 = MerkleTree::build(temp_dir.path()).unwrap();

        // Add new file
        fs::write(temp_dir.path().join("new.txt"), "new content").unwrap();
        let tree2 = MerkleTree::build(temp_dir.path()).unwrap();

        let changed = tree2.find_changed(&tree1);

        assert_eq!(changed.len(), 1);
        assert!(changed.iter().any(|p| p.file_name().unwrap() == "new.txt"));
    }

    #[test]
    fn test_skip_directories() {
        assert!(MerkleTree::should_skip_directory("target"));
        assert!(MerkleTree::should_skip_directory("node_modules"));
        assert!(MerkleTree::should_skip_directory(".git"));
        assert!(!MerkleTree::should_skip_directory("src"));
        assert!(!MerkleTree::should_skip_directory("my_project"));
    }

    #[test]
    fn test_file_hash_consistency() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "test content").unwrap();

        let hash1 = MerkleTree::hash_file(&file_path).unwrap();
        let hash2 = MerkleTree::hash_file(&file_path).unwrap();

        assert_eq!(hash1, hash2, "Same content should produce same hash");
    }
}
