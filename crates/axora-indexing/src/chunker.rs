//! Code chunking with Tree-sitter

use crate::error::IndexingError;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tree_sitter::{Parser, Query, QueryCursor, Node};

/// Type of code chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkType {
    /// Function or method
    Function,
    /// Class or struct
    Class,
    /// Module or file-level
    Module,
    /// Comment or documentation
    Comment,
    /// Other
    Other,
}

/// Metadata for a code chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Function name (if applicable)
    pub function_name: Option<String>,
    /// Class name (if applicable)
    pub class_name: Option<String>,
    /// Function signature
    pub signature: String,
    /// Docstring or documentation
    pub docstring: Option<String>,
    /// Imported modules
    pub imports: Vec<String>,
    /// Functions called by this chunk
    pub callees: Vec<String>,
    /// Type references
    pub type_references: Vec<String>,
}

/// A chunk of code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    /// Unique identifier
    pub id: String,
    /// File path
    pub file_path: PathBuf,
    /// Line range (start, end)
    pub line_range: (usize, usize),
    /// Content
    pub content: String,
    /// Programming language
    pub language: String,
    /// Chunk type
    pub chunk_type: ChunkType,
    /// Metadata
    pub metadata: ChunkMetadata,
    /// Token count
    pub token_count: usize,
}

/// Code chunker using Tree-sitter
pub struct Chunker {
    parser: Parser,
    language_queries: HashMap<String, Query>,
}

impl Chunker {
    /// Create new chunker
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        
        // Initialize parsers for supported languages
        // Note: In production, you'd load the actual grammar .so/.dll files
        // For now, we create a working chunker that can be extended
        
        let mut language_queries = HashMap::new();
        
        // Rust function query
        let rust_query = Query::new(
            &tree_sitter_rust::LANGUAGE.into(),
            r#"
            (function_item
                name: (identifier) @name
                parameters: (parameters) @params
                body: (block) @body) @function
            "#,
        ).map_err(|e| IndexingError::ParseFailed(e.to_string()))?;
        language_queries.insert("rust".to_string(), rust_query);
        
        // TypeScript function query
        let ts_query = Query::new(
            &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            r#"
            (function_declaration
                name: (identifier) @name
                parameters: (formal_parameters) @params
                body: (statement_block) @body) @function
            "#,
        ).map_err(|e| IndexingError::ParseFailed(e.to_string()))?;
        language_queries.insert("typescript".to_string(), ts_query);
        
        // Python function query
        let py_query = Query::new(
            &tree_sitter_python::LANGUAGE.into(),
            r#"
            (function_definition
                name: (identifier) @name
                parameters: (parameters) @params
                body: (block) @body) @function
            "#,
        ).map_err(|e| IndexingError::ParseFailed(e.to_string()))?;
        language_queries.insert("python".to_string(), py_query);

        Ok(Self {
            parser,
            language_queries,
        })
    }

    /// Detect language from file extension
    pub fn detect_language(file_path: &Path) -> Option<String> {
        file_path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext {
                "rs" => "rust",
                "ts" | "tsx" => "typescript",
                "py" => "python",
                "js" | "jsx" => "javascript",
                "go" => "go",
                "java" => "java",
                "c" | "cpp" | "h" | "hpp" => "cpp",
                _ => "unknown",
            })
            .map(|s| s.to_string())
    }

    /// Extract chunks from code
    pub fn extract_chunks(
        &mut self,
        code: &str,
        file_path: &Path,
        language: &str,
    ) -> Result<Vec<CodeChunk>> {
        // Check if we have a query for this language
        let has_query = self.language_queries.contains_key(language);
        
        if !has_query {
            // Fallback: return single module chunk
            return Ok(vec![self.create_module_chunk(code, file_path, language)]);
        }

        // For now, use simple line-based chunking as placeholder
        // Tree-sitter integration requires more complex setup
        let chunks = self.simple_chunking(code, file_path, language);
        
        Ok(chunks)
    }

    /// Simple line-based chunking (placeholder for full Tree-sitter)
    fn simple_chunking(&self, code: &str, file_path: &Path, language: &str) -> Vec<CodeChunk> {
        let lines: Vec<&str> = code.lines().collect();
        let mut chunks = Vec::new();
        
        // Chunk by functions (simple heuristic: look for function keywords)
        let mut current_chunk_start = 0;
        let mut in_function = false;
        
        for (i, line) in lines.iter().enumerate() {
            let is_function_start = self.is_function_start(line, language);
            
            if is_function_start {
                // Save previous chunk if exists
                if i > current_chunk_start {
                    let chunk_code = lines[current_chunk_start..i].join("\n");
                    if !chunk_code.trim().is_empty() {
                        chunks.push(self.create_chunk(
                            &chunk_code,
                            file_path,
                            language,
                            current_chunk_start + 1,
                            i,
                        ));
                    }
                }
                current_chunk_start = i;
                in_function = true;
            }
        }
        
        // Add final chunk
        if current_chunk_start < lines.len() {
            let chunk_code = lines[current_chunk_start..].join("\n");
            if !chunk_code.trim().is_empty() {
                chunks.push(self.create_chunk(
                    &chunk_code,
                    file_path,
                    language,
                    current_chunk_start + 1,
                    lines.len(),
                ));
            }
        }
        
        // If no chunks created, create module-level chunk
        if chunks.is_empty() {
            chunks.push(self.create_module_chunk(code, file_path, language));
        }
        
        chunks
    }

    /// Check if line starts a function
    fn is_function_start(&self, line: &str, language: &str) -> bool {
        match language {
            "rust" => line.trim().starts_with("fn "),
            "typescript" | "javascript" => line.trim().starts_with("function ") || line.trim().starts_with("async function"),
            "python" => line.trim().starts_with("def "),
            "go" => line.trim().starts_with("func "),
            "java" => line.contains("(") && (line.contains("public ") || line.contains("private ") || line.contains("void ")),
            _ => false,
        }
    }

    /// Create a chunk
    fn create_chunk(&self, code: &str, file_path: &Path, language: &str, start_line: usize, end_line: usize) -> CodeChunk {
        // Try to extract function name
        let first_line = code.lines().next().unwrap_or("");
        let func_name = self.extract_function_name(first_line, language);
        
        CodeChunk {
            id: uuid::Uuid::new_v4().to_string(),
            file_path: file_path.to_path_buf(),
            line_range: (start_line, end_line),
            content: code.to_string(),
            language: language.to_string(),
            chunk_type: if func_name.is_some() { ChunkType::Function } else { ChunkType::Module },
            metadata: ChunkMetadata {
                function_name: func_name,
                class_name: None,
                signature: String::new(),
                docstring: None,
                imports: vec![],
                callees: vec![],
                type_references: vec![],
            },
            token_count: code.len() / 4,
        }
    }

    /// Extract function name from declaration line
    fn extract_function_name(&self, line: &str, language: &str) -> Option<String> {
        match language {
            "rust" => {
                // fn name(...) -> ...
                line.split("fn ").nth(1)
                    .and_then(|s| s.split('(').next())
                    .map(|s| s.trim().to_string())
            }
            "typescript" | "javascript" => {
                // function name(...) or async function name(...)
                line.split("function ").nth(1)
                    .and_then(|s| s.split('(').next())
                    .map(|s| s.trim().to_string())
            }
            "python" => {
                // def name(...):
                line.split("def ").nth(1)
                    .and_then(|s| s.split('(').next())
                    .map(|s| s.trim().to_string())
            }
            _ => None,
        }
    }

    /// Create a module-level chunk (fallback)
    fn create_module_chunk(&self, code: &str, file_path: &Path, language: &str) -> CodeChunk {
        let line_count = code.lines().count();
        
        CodeChunk {
            id: uuid::Uuid::new_v4().to_string(),
            file_path: file_path.to_path_buf(),
            line_range: (1, line_count),
            content: code.to_string(),
            language: language.to_string(),
            chunk_type: ChunkType::Module,
            metadata: ChunkMetadata {
                function_name: None,
                class_name: None,
                signature: String::new(),
                docstring: None,
                imports: vec![],
                callees: vec![],
                type_references: vec![],
            },
            token_count: code.len() / 4,
        }
    }
}

impl Default for Chunker {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunker_creation() {
        let chunker = Chunker::new();
        assert!(chunker.is_ok());
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(Chunker::detect_language(Path::new("test.rs")), Some("rust".to_string()));
        assert_eq!(Chunker::detect_language(Path::new("test.ts")), Some("typescript".to_string()));
        assert_eq!(Chunker::detect_language(Path::new("test.py")), Some("python".to_string()));
        assert_eq!(Chunker::detect_language(Path::new("test.unknown")), Some("unknown".to_string()));
    }

    #[test]
    fn test_rust_function_chunking() {
        let mut chunker = Chunker::new().unwrap();
        
        let code = r#"
fn hello() {
    println!("world");
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;

        let chunks = chunker.extract_chunks(code, Path::new("test.rs"), "rust").unwrap();
        
        // Should find at least 1 function (may find 2)
        assert!(chunks.len() >= 1);
        
        // Check that we found functions
        let has_function = chunks.iter().any(|c| matches!(c.chunk_type, ChunkType::Function));
        assert!(has_function);
    }

    #[test]
    fn test_module_fallback() {
        let mut chunker = Chunker::new().unwrap();
        
        let code = r#"
// Just a comment, no functions
const X: i32 = 42;
"#;

        let chunks = chunker.extract_chunks(code, Path::new("test.rs"), "rust").unwrap();
        
        // Should create module-level chunk
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::Module));
    }
}
