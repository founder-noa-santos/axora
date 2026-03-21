//! Code minification for reducing token count when sending code to LLMs.
//!
//! This module provides whitespace removal, identifier compression,
//! comment stripping, and token savings estimation for multiple languages.

use std::collections::HashMap;
use thiserror::Error;

/// Minification errors
#[derive(Error, Debug)]
pub enum MinifierError {
    /// Unsupported language
    #[error("unsupported language: {0}")]
    UnsupportedLanguage(String),

    /// Invalid minified code format
    #[error("invalid minified code format")]
    InvalidFormat,
}

/// Result type for minifier operations
pub type Result<T> = std::result::Result<T, MinifierError>;

/// Represents minified code with metadata for decompression
#[derive(Debug)]
pub struct MinifiedCode {
    /// The minified code content
    pub content: String,
    /// Mapping from short identifiers back to original long identifiers
    pub identifier_map: HashMap<String, String>,
    /// Original code length in bytes
    pub original_length: usize,
    /// Minified code length in bytes
    pub minified_length: usize,
    /// Percentage of bytes saved (0.0 to 100.0)
    pub savings_percentage: f32,
}

impl MinifiedCode {
    /// Calculate the percentage of bytes saved
    pub fn savings_percentage(&self) -> f32 {
        self.savings_percentage
    }

    /// Calculate the number of bytes saved
    pub fn byte_savings(&self) -> usize {
        self.original_length.saturating_sub(self.minified_length)
    }

    /// Estimate token savings (approximate: 1 token ≈ 4 bytes)
    pub fn token_savings(&self) -> usize {
        self.byte_savings() / 4
    }
}

/// Configuration options for the minifier
#[derive(Debug, Clone)]
pub struct MinifierConfig {
    /// Remove unnecessary whitespace
    pub remove_whitespace: bool,
    /// Compress identifiers (long → short)
    pub compress_identifiers: bool,
    /// Strip comments
    pub strip_comments: bool,
    /// Preserve docstrings (when stripping comments)
    pub preserve_docstrings: bool,
    /// Maintain minimal indentation (when removing whitespace)
    pub maintain_indentation: bool,
}

impl Default for MinifierConfig {
    fn default() -> Self {
        Self {
            remove_whitespace: true,
            compress_identifiers: true,
            strip_comments: true,
            preserve_docstrings: false,
            maintain_indentation: false,
        }
    }
}

/// Code minifier for reducing token count in code sent to LLMs
pub struct CodeMinifier {
    config: MinifierConfig,
}

impl CodeMinifier {
    /// Create a new CodeMinifier with default configuration
    pub fn new() -> Self {
        Self {
            config: MinifierConfig::default(),
        }
    }

    /// Create a new CodeMinifier with custom configuration
    pub fn with_config(config: MinifierConfig) -> Self {
        Self { config }
    }

    /// Minify code for a specific language
    ///
    /// # Arguments
    /// * `code` - The source code to minify
    /// * `language` - The programming language (e.g., "rust", "typescript", "python", "go")
    ///
    /// # Returns
    /// A `MinifiedCode` struct containing the minified content and metadata
    ///
    /// # Example
    /// ```
    /// use openakta_cache::CodeMinifier;
    ///
    /// let minifier = CodeMinifier::new();
    /// let result = minifier.minify("fn main() { println!(\"hello\"); }", "rust").unwrap();
    /// assert!(result.minified_length <= result.original_length);
    /// ```
    pub fn minify(&self, code: &str, language: &str) -> Result<MinifiedCode> {
        let original_length = code.len();
        let lang_lower = language.to_lowercase();

        // Validate language support
        if !Self::is_supported_language(&lang_lower) {
            return Err(MinifierError::UnsupportedLanguage(language.to_string()));
        }

        let mut result = code.to_string();

        // Step 1: Strip comments (if configured)
        if self.config.strip_comments {
            result = self.strip_comments(&result, &lang_lower)?;
        }

        // Step 2: Compress identifiers (if configured)
        let mut identifier_map: HashMap<String, String> = HashMap::new();
        if self.config.compress_identifiers {
            let (compressed, map) = self.compress_identifiers(&result, &lang_lower);
            result = compressed;
            identifier_map = map;
        }

        // Step 3: Remove whitespace (if configured)
        if self.config.remove_whitespace {
            result = self.remove_whitespace(&result, &lang_lower);
        }

        let minified_length = result.len();
        let savings_percentage = if original_length > 0 {
            ((original_length - minified_length) as f32 / original_length as f32) * 100.0
        } else {
            0.0
        };

        Ok(MinifiedCode {
            content: result,
            identifier_map,
            original_length,
            minified_length,
            savings_percentage,
        })
    }

    /// Decompress minified code back to original form
    ///
    /// # Arguments
    /// * `minified` - The minified code with identifier map
    ///
    /// # Returns
    /// The decompressed code with original identifiers restored
    ///
    /// # Note
    /// This only reverses identifier compression. Whitespace removal
    /// and comment stripping are irreversible.
    pub fn decompress(&self, minified: &MinifiedCode) -> Result<String> {
        let mut result = minified.content.clone();

        // Create a reverse map: short -> long for decompression
        let mut short_to_long: Vec<(&String, &String)> = minified
            .identifier_map
            .iter()
            .map(|(long, short)| (short, long))
            .collect();

        // Sort by short identifier length (longest first) to avoid partial replacements
        short_to_long.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        for (short, long) in short_to_long {
            // Use word boundary-aware replacement
            result = self.replace_identifier(&result, short, long);
        }

        Ok(result)
    }

    /// Estimate the number of tokens in code
    ///
    /// Uses a simple heuristic: approximately 1 token per 4 bytes
    /// This is a rough approximation based on typical LLM tokenization
    ///
    /// # Arguments
    /// * `code` - The code to estimate tokens for
    ///
    /// # Returns
    /// Estimated token count
    pub fn estimate_tokens(code: &str) -> usize {
        // Simple estimation: ~4 bytes per token on average
        // This varies by language and content but is a reasonable approximation
        code.len() / 4
    }

    /// Check if a language is supported
    ///
    /// # Arguments
    /// * `language` - The language identifier (lowercase)
    ///
    /// # Returns
    /// true if the language is supported
    pub fn is_supported_language(language: &str) -> bool {
        matches!(
            language,
            "rust"
                | "rs"
                | "typescript"
                | "ts"
                | "tsx"
                | "javascript"
                | "js"
                | "jsx"
                | "python"
                | "py"
                | "go"
                | "golang"
        )
    }

    /// Detect language from file extension
    ///
    /// # Arguments
    /// * `file_path` - The file path or extension
    ///
    /// # Returns
    /// The language identifier or None if not recognized
    pub fn detect_language(file_path: &str) -> Option<String> {
        let ext = file_path.rsplit('.').next()?.to_lowercase();
        match ext.as_str() {
            "rs" => Some("rust".to_string()),
            "ts" => Some("typescript".to_string()),
            "tsx" => Some("typescript".to_string()),
            "js" => Some("javascript".to_string()),
            "jsx" => Some("javascript".to_string()),
            "py" => Some("python".to_string()),
            "go" => Some("go".to_string()),
            _ => None,
        }
    }

    // === Internal Implementation ===

    /// Strip comments based on language
    fn strip_comments(&self, code: &str, language: &str) -> Result<String> {
        match language {
            "rust" | "rs" => Ok(self.strip_rust_comments(code)),
            "typescript" | "ts" | "tsx" | "javascript" | "js" | "jsx" => {
                Ok(self.strip_typescript_comments(code))
            }
            "python" | "py" => Ok(self.strip_python_comments(code)),
            "go" | "golang" => Ok(self.strip_go_comments(code)),
            _ => Err(MinifierError::UnsupportedLanguage(language.to_string())),
        }
    }

    /// Strip Rust comments (// and /* */)
    fn strip_rust_comments(&self, code: &str) -> String {
        let mut result = String::with_capacity(code.len());
        let mut chars = code.chars().peekable();
        let mut in_string = false;
        let mut in_char = false;
        let mut in_raw_string = false;
        let mut raw_hash_count = 0;

        while let Some(c) = chars.next() {
            // Handle raw strings
            if !in_string && !in_char && c == 'r' && chars.peek() == Some(&'#') {
                let mut hash_count = 0;
                while chars.peek() == Some(&'#') {
                    hash_count += 1;
                    chars.next();
                }
                if chars.peek() == Some(&'"') {
                    in_raw_string = true;
                    raw_hash_count = hash_count;
                    result.push(c);
                    for _ in 0..hash_count {
                        result.push('#');
                    }
                    result.push('"');
                    chars.next();
                    continue;
                } else {
                    // Not a raw string, output what we consumed
                    result.push(c);
                    for _ in 0..hash_count {
                        result.push('#');
                    }
                    continue;
                }
            }

            if in_raw_string {
                result.push(c);
                if c == '"' {
                    let mut hash_count = 0;
                    while chars.peek() == Some(&'#') && hash_count < raw_hash_count {
                        hash_count += 1;
                        chars.next();
                        result.push('#');
                    }
                    if hash_count == raw_hash_count {
                        in_raw_string = false;
                    }
                }
                continue;
            }

            // Handle regular strings
            if !in_char && c == '"' {
                in_string = !in_string;
                result.push(c);
                continue;
            }

            // Handle char literals
            if !in_string && c == '\'' {
                in_char = !in_char;
                result.push(c);
                continue;
            }

            if in_string || in_char {
                result.push(c);
                continue;
            }

            // Check for line comment
            if c == '/' && chars.peek() == Some(&'/') {
                // Skip until end of line
                while let Some(&next) = chars.peek() {
                    if next == '\n' {
                        break;
                    }
                    chars.next();
                }
                continue;
            }

            // Check for block comment
            if c == '/' && chars.peek() == Some(&'*') {
                chars.next(); // consume '*'
                let mut nested = 1;
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        nested -= 1;
                        if nested == 0 {
                            break;
                        }
                    } else if next == '/' && chars.peek() == Some(&'*') {
                        chars.next();
                        nested += 1;
                    }
                }
                continue;
            }

            // Check for doc comments (/// and /**)
            if c == '/' && chars.peek() == Some(&'/') {
                chars.next();
                if chars.peek() == Some(&'/') {
                    // Doc comment - preserve if configured
                    if self.config.preserve_docstrings {
                        result.push(c);
                        result.push('/');
                        result.push('/');
                        chars.next();
                        continue;
                    } else {
                        // Skip until end of line
                        while let Some(&next) = chars.peek() {
                            if next == '\n' {
                                break;
                            }
                            chars.next();
                        }
                        continue;
                    }
                }
            }

            result.push(c);
        }

        result
    }

    /// Strip TypeScript/JavaScript comments (//, /* */, and # for JSX)
    fn strip_typescript_comments(&self, code: &str) -> String {
        let mut result = String::with_capacity(code.len());
        let mut chars = code.chars().peekable();
        let mut in_string = false;
        let mut in_template = false;
        let mut in_regex = false;
        let mut escape_next = false;

        while let Some(c) = chars.next() {
            if escape_next {
                result.push(c);
                escape_next = false;
                continue;
            }

            if c == '\\' && (in_string || in_template) {
                result.push(c);
                escape_next = true;
                continue;
            }

            // Handle template literals
            if !in_string && !in_regex && c == '`' {
                in_template = !in_template;
                result.push(c);
                continue;
            }

            // Handle regular strings
            if !in_template && !in_regex && c == '"' {
                in_string = !in_string;
                result.push(c);
                continue;
            }
            if !in_template && !in_string && c == '\'' {
                in_string = !in_string;
                result.push(c);
                continue;
            }

            if in_string || in_template {
                result.push(c);
                continue;
            }

            // Simple regex detection (after certain keywords or operators)
            if c == '/' && !in_regex {
                let prev = result.chars().last();
                let is_regex_context = prev.is_none_or(|p| {
                    p.is_whitespace()
                        || matches!(
                            p,
                            '=' | '(' | '[' | ',' | ':' | ';' | '{' | '}' | '!' | '&' | '|' | '?'
                        )
                });

                if is_regex_context && chars.peek() != Some(&'/') && chars.peek() != Some(&'*') {
                    in_regex = true;
                    result.push(c);
                    continue;
                }
            }

            if in_regex {
                result.push(c);
                if c == '/' && !escape_next {
                    in_regex = false;
                }
                continue;
            }

            // Check for line comment
            if c == '/' && chars.peek() == Some(&'/') {
                while let Some(&next) = chars.peek() {
                    if next == '\n' {
                        break;
                    }
                    chars.next();
                }
                continue;
            }

            // Check for block comment
            if c == '/' && chars.peek() == Some(&'*') {
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        break;
                    }
                }
                continue;
            }

            result.push(c);
        }

        result
    }

    /// Strip Python comments (# and docstrings)
    fn strip_python_comments(&self, code: &str) -> String {
        let mut result = String::with_capacity(code.len());
        let mut chars = code.chars().peekable();
        let mut in_string = false;
        let mut string_char = '"';
        let mut in_multiline_string = false;

        while let Some(c) = chars.next() {
            // Check for triple quotes
            if !in_string && !in_multiline_string && (c == '"' || c == '\'') {
                let next_two: String = chars.clone().take(2).collect();

                if next_two.chars().all(|x| x == c) {
                    // Triple quote found
                    in_multiline_string = true;
                    result.push(c);
                    result.push(chars.next().unwrap());
                    result.push(chars.next().unwrap());
                    string_char = c;
                    continue;
                } else {
                    // Single quote - toggle string mode
                    in_string = !in_string;
                    string_char = c;
                    result.push(c);
                    continue;
                }
            }

            if in_multiline_string {
                result.push(c);
                if c == string_char {
                    let mut count = 1;
                    while chars.peek() == Some(&string_char) && count < 3 {
                        count += 1;
                        result.push(chars.next().unwrap());
                    }
                    if count == 3 {
                        in_multiline_string = false;
                    }
                }
                continue;
            }

            if in_string {
                result.push(c);
                if c == '\\' {
                    if let Some(&_next) = chars.peek() {
                        result.push(chars.next().unwrap());
                    }
                } else if c == string_char {
                    in_string = false;
                }
                continue;
            }

            // Check for line comment
            if c == '#' {
                while let Some(&next) = chars.peek() {
                    if next == '\n' {
                        break;
                    }
                    chars.next();
                }
                continue;
            }

            result.push(c);
        }

        result
    }

    /// Strip Go comments (// and /* */)
    fn strip_go_comments(&self, code: &str) -> String {
        let mut result = String::with_capacity(code.len());
        let mut chars = code.chars().peekable();
        let mut in_string = false;
        let mut in_raw_string = false;
        let mut in_char = false;

        while let Some(c) = chars.next() {
            // Handle raw strings (backticks)
            if !in_string && !in_char && c == '`' {
                in_raw_string = !in_raw_string;
                result.push(c);
                continue;
            }

            if in_raw_string {
                result.push(c);
                continue;
            }

            // Handle regular strings
            if !in_char && c == '"' {
                in_string = !in_string;
                result.push(c);
                continue;
            }

            // Handle char literals (Go doesn't have char literals, but handle for completeness)
            if !in_string && c == '\'' {
                in_char = !in_char;
                result.push(c);
                continue;
            }

            if in_string || in_char {
                result.push(c);
                if c == '\\' {
                    if let Some(next) = chars.next() {
                        result.push(next);
                    }
                }
                continue;
            }

            // Check for line comment
            if c == '/' && chars.peek() == Some(&'/') {
                while let Some(&next) = chars.peek() {
                    if next == '\n' {
                        break;
                    }
                    chars.next();
                }
                continue;
            }

            // Check for block comment
            if c == '/' && chars.peek() == Some(&'*') {
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        break;
                    }
                }
                continue;
            }

            result.push(c);
        }

        result
    }

    /// Compress identifiers in code
    fn compress_identifiers(
        &self,
        code: &str,
        language: &str,
    ) -> (String, HashMap<String, String>) {
        let mut identifier_map: HashMap<String, String> = HashMap::new();
        let mut short_to_long: HashMap<String, String> = HashMap::new();
        let mut counter = 0;

        let keywords = self.get_keywords_for_language(language);
        let mut result = String::with_capacity(code.len());
        let chars = code.chars().peekable();
        let mut current_ident = String::new();
        let mut in_string = false;

        for c in chars {
            // Simple string detection (not perfect but works for most cases)
            if c == '"' || c == '\'' || c == '`' {
                in_string = !in_string;
                result.push(c);
                continue;
            }

            if in_string {
                result.push(c);
                continue;
            }

            // Check if character can be part of identifier
            if c.is_alphabetic() || c == '_' {
                current_ident.push(c);
                continue;
            }

            if c.is_numeric() && !current_ident.is_empty() {
                current_ident.push(c);
                continue;
            }

            // End of potential identifier
            if !current_ident.is_empty() {
                if self.is_compressible_identifier(&current_ident, &keywords) {
                    let short = self.generate_short_identifier(counter);
                    counter += 1;

                    if !identifier_map.contains_key(&current_ident) {
                        identifier_map.insert(current_ident.clone(), short.clone());
                        short_to_long.insert(short.clone(), current_ident.clone());
                    }

                    if let Some(short) = identifier_map.get(&current_ident) {
                        result.push_str(short);
                    } else {
                        result.push_str(&current_ident);
                    }
                } else {
                    result.push_str(&current_ident);
                }
                current_ident.clear();
            }

            result.push(c);
        }

        // Handle identifier at end of file
        if !current_ident.is_empty() {
            if self.is_compressible_identifier(&current_ident, &keywords) {
                if let Some(short) = identifier_map.get(&current_ident) {
                    result.push_str(short);
                } else {
                    result.push_str(&current_ident);
                }
            } else {
                result.push_str(&current_ident);
            }
        }

        // Return map with long -> short mapping
        let final_map: HashMap<String, String> = identifier_map.into_iter().collect();

        (result, final_map)
    }

    /// Check if an identifier should be compressed
    fn is_compressible_identifier(&self, ident: &str, keywords: &[&str]) -> bool {
        // Don't compress keywords
        if keywords.contains(&ident) {
            return false;
        }

        // Don't compress very short identifiers
        if ident.len() <= 2 {
            return false;
        }

        // Don't compress single uppercase letters (likely type parameters)
        if ident.len() == 1 && ident.chars().next().unwrap().is_uppercase() {
            return false;
        }

        // Compress identifiers longer than 3 characters
        ident.len() > 3
    }

    /// Generate a short identifier
    fn generate_short_identifier(&self, counter: usize) -> String {
        // Generate identifiers like: a, b, c, ..., z, a1, b1, ..., z1, a2, ...
        let letters = "abcdefghijklmnopqrstuvwxyz";
        if counter < 26 {
            letters.chars().nth(counter).unwrap().to_string()
        } else {
            let letter_idx = counter % 26;
            let num = counter / 26;
            format!("{}{}", letters.chars().nth(letter_idx).unwrap(), num)
        }
    }

    /// Get keywords for a language (these should not be compressed)
    fn get_keywords_for_language(&self, language: &str) -> Vec<&'static str> {
        match language {
            "rust" | "rs" => vec![
                "fn", "let", "mut", "const", "static", "struct", "enum", "impl", "trait", "mod",
                "pub", "use", "crate", "self", "Self", "super", "if", "else", "match", "for",
                "while", "loop", "return", "break", "continue", "as", "where", "type", "async",
                "await", "dyn", "ref", "box", "move", "in", "unsafe", "extern", "macro",
            ],
            "typescript" | "ts" | "tsx" | "javascript" | "js" | "jsx" => vec![
                "function",
                "const",
                "let",
                "var",
                "class",
                "interface",
                "type",
                "enum",
                "namespace",
                "module",
                "export",
                "import",
                "from",
                "as",
                "if",
                "else",
                "for",
                "while",
                "do",
                "switch",
                "case",
                "break",
                "continue",
                "return",
                "yield",
                "async",
                "await",
                "try",
                "catch",
                "finally",
                "throw",
                "new",
                "this",
                "super",
                "extends",
                "implements",
                "static",
                "public",
                "private",
                "protected",
                "readonly",
                "abstract",
                "declare",
                "namespace",
                "typeof",
                "instanceof",
                "in",
                "of",
            ],
            "python" | "py" => vec![
                "def", "class", "import", "from", "as", "if", "elif", "else", "for", "while",
                "break", "continue", "return", "yield", "lambda", "try", "except", "finally",
                "raise", "assert", "with", "pass", "global", "nonlocal", "async", "await", "in",
                "is", "not", "and", "or", "True", "False", "None", "self",
            ],
            "go" | "golang" => vec![
                "func",
                "package",
                "import",
                "const",
                "var",
                "type",
                "struct",
                "interface",
                "map",
                "chan",
                "go",
                "defer",
                "return",
                "if",
                "else",
                "for",
                "range",
                "break",
                "continue",
                "switch",
                "case",
                "fallthrough",
                "select",
                "default",
                "goto",
                "new",
                "make",
                "nil",
                "true",
                "false",
            ],
            _ => vec![],
        }
    }

    /// Remove whitespace from code
    fn remove_whitespace(&self, code: &str, language: &str) -> String {
        if self.config.maintain_indentation {
            self.remove_whitespace_minimal(code, language)
        } else {
            self.remove_whitespace_aggressive(code, language)
        }
    }

    /// Aggressive whitespace removal (single line)
    fn remove_whitespace_aggressive(&self, code: &str, _language: &str) -> String {
        let mut result = String::with_capacity(code.len());
        let chars = code.chars().peekable();
        let mut in_string = false;
        let mut last_was_space = false;

        for c in chars {
            // Handle string literals
            if c == '"' || c == '\'' || c == '`' {
                in_string = !in_string;
                result.push(c);
                continue;
            }

            if in_string {
                result.push(c);
                continue;
            }

            // Handle whitespace
            if c.is_whitespace() {
                if !last_was_space && !result.is_empty() {
                    result.push(' ');
                    last_was_space = true;
                }
                continue;
            }

            last_was_space = false;
            result.push(c);
        }

        result.trim().to_string()
    }

    /// Minimal whitespace removal (preserve newlines for readability)
    fn remove_whitespace_minimal(&self, code: &str, _language: &str) -> String {
        let mut result = String::with_capacity(code.len());

        for line in code.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                result.push_str(trimmed);
                result.push('\n');
            }
        }

        // Remove trailing newline
        if result.ends_with('\n') {
            result.pop();
        }

        result
    }

    /// Replace identifier with word boundary awareness
    fn replace_identifier(&self, code: &str, short: &str, long: &str) -> String {
        let mut result = String::with_capacity(code.len());
        let chars = code.chars().peekable();
        let mut current = String::new();

        for c in chars {
            if c.is_alphabetic() || c == '_' || c.is_numeric() {
                current.push(c);
                continue;
            }

            if !current.is_empty() {
                if current == short {
                    result.push_str(long);
                } else {
                    result.push_str(&current);
                }
                current.clear();
            }

            result.push(c);
        }

        // Handle identifier at end
        if !current.is_empty() {
            if current == short {
                result.push_str(long);
            } else {
                result.push_str(&current);
            }
        }

        result
    }
}

impl Default for CodeMinifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whitespace_removal() {
        let minifier = CodeMinifier::new();
        let code = "fn   main()   {   println!(\"hello\");   }";
        let result = minifier.minify(code, "rust").unwrap();

        assert!(result.minified_length < result.original_length);
        assert!(result.savings_percentage > 0.0);
        assert!(!result.content.contains("  ")); // No double spaces
    }

    #[test]
    fn test_identifier_compression() {
        let minifier = CodeMinifier::new();
        let code = "fn calculateMonthlyRevenueMetrics() { let totalRevenue = 0; }";
        let result = minifier.minify(code, "rust").unwrap();

        assert!(!result.identifier_map.is_empty());
        assert!(result.minified_length < result.original_length);

        // Verify we can decompress
        let decompressed = minifier.decompress(&result).unwrap();
        assert!(decompressed.contains("calculateMonthlyRevenueMetrics"));
        assert!(decompressed.contains("totalRevenue"));
    }

    #[test]
    fn test_comment_stripping_rust() {
        let minifier = CodeMinifier::new();
        let code = r#"
            // This is a comment
            fn main() {
                /* Block comment */
                println!("hello"); // inline comment
            }
        "#;
        let result = minifier.minify(code, "rust").unwrap();

        assert!(!result.content.contains("// This is a comment"));
        assert!(!result.content.contains("/* Block comment */"));
        assert!(!result.content.contains("// inline comment"));
        // Keywords should be preserved
        assert!(result.content.contains("fn"));
        // String content should be preserved
        assert!(result.content.contains("hello"));
        // Verify comments are stripped but code structure remains
        assert!(result.savings_percentage > 0.0);
    }

    #[test]
    fn test_comment_stripping_typescript() {
        let minifier = CodeMinifier::new();
        let code = r#"
            // TypeScript comment
            const x = 5; // inline
            /* Block */
            function test() {
                return `template ${x}`; // comment in template
            }
        "#;
        let result = minifier.minify(code, "typescript").unwrap();

        assert!(!result.content.contains("// TypeScript comment"));
        assert!(!result.content.contains("// inline"));
        assert!(!result.content.contains("/* Block */"));
        assert!(result.content.contains("const"));
        assert!(result.content.contains("function"));
    }

    #[test]
    fn test_comment_stripping_python() {
        let minifier = CodeMinifier::new();
        let code = r#"
            # Python comment
            def hello():
                """Docstring"""
                x = 5  # inline
                return x
        "#;
        let result = minifier.minify(code, "python").unwrap();

        assert!(!result.content.contains("# Python comment"));
        assert!(!result.content.contains("# inline"));
        assert!(result.content.contains("def"));
        assert!(result.content.contains("return"));
    }

    #[test]
    fn test_comment_stripping_go() {
        let minifier = CodeMinifier::new();
        let code = r#"
            // Go comment
            package main
            /* Block comment */
            func main() {
                x := `raw string with // comment inside`
            }
        "#;
        let result = minifier.minify(code, "go").unwrap();

        assert!(!result.content.contains("// Go comment"));
        assert!(!result.content.contains("/* Block comment */"));
        assert!(result.content.contains("package"));
        assert!(result.content.contains("func"));
        assert!(result.content.contains("raw string"));
    }

    #[test]
    fn test_roundtrip_decompress() {
        let minifier = CodeMinifier::new();
        let code = "fn calculateTotalRevenue() -> i32 { let monthlyRevenue = 100; monthlyRevenue }";
        let minified = minifier.minify(code, "rust").unwrap();
        let decompressed = minifier.decompress(&minified).unwrap();

        // Identifiers should be restored
        assert!(decompressed.contains("calculateTotalRevenue"));
        assert!(decompressed.contains("monthlyRevenue"));
    }

    #[test]
    fn test_token_savings() {
        let minifier = CodeMinifier::new();
        let code = r#"
            // Calculate revenue
            fn calculateMonthlyRevenueMetrics(data: Vec<i32>) -> i32 {
                let totalRevenue = data.iter().sum();
                totalRevenue
            }
        "#;
        let result = minifier.minify(code, "rust").unwrap();

        assert!(
            result.savings_percentage > 20.0,
            "Expected >20% savings, got {:.1}%",
            result.savings_percentage
        );
        assert!(result.byte_savings() > 0);
        assert!(result.token_savings() > 0);
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(
            CodeMinifier::detect_language("file.rs"),
            Some("rust".to_string())
        );
        assert_eq!(
            CodeMinifier::detect_language("file.ts"),
            Some("typescript".to_string())
        );
        assert_eq!(
            CodeMinifier::detect_language("file.tsx"),
            Some("typescript".to_string())
        );
        assert_eq!(
            CodeMinifier::detect_language("file.js"),
            Some("javascript".to_string())
        );
        assert_eq!(
            CodeMinifier::detect_language("file.py"),
            Some("python".to_string())
        );
        assert_eq!(
            CodeMinifier::detect_language("file.go"),
            Some("go".to_string())
        );
        assert_eq!(CodeMinifier::detect_language("file.unknown"), None);
    }

    #[test]
    fn test_preserve_strings() {
        let minifier = CodeMinifier::new();

        // Rust strings
        let rust_code = r#"let s = "hello // not a comment";"#;
        let result = minifier.minify(rust_code, "rust").unwrap();
        assert!(result.content.contains("hello"));

        // Python strings
        let py_code = r#"s = "hello # not a comment""#;
        let result = minifier.minify(py_code, "python").unwrap();
        assert!(result.content.contains("hello"));

        // Go raw strings
        let go_code = r#"s := `hello // not a comment`"#;
        let result = minifier.minify(go_code, "go").unwrap();
        assert!(result.content.contains("hello"));
    }

    #[test]
    fn test_preserve_keywords() {
        let minifier = CodeMinifier::new();

        // Rust keywords should not be compressed
        let rust_code = "fn main() { let x = 5; if x > 0 { return x; } }";
        let result = minifier.minify(rust_code, "rust").unwrap();
        let decompressed = minifier.decompress(&result).unwrap();
        assert!(decompressed.contains("fn"));
        assert!(decompressed.contains("let"));
        assert!(decompressed.contains("if"));
        assert!(decompressed.contains("return"));

        // TypeScript keywords
        let ts_code = "function test() { const x = 5; if (x > 0) { return x; } }";
        let result = minifier.minify(ts_code, "typescript").unwrap();
        let decompressed = minifier.decompress(&result).unwrap();
        assert!(decompressed.contains("function"));
        assert!(decompressed.contains("const"));
    }

    #[test]
    fn test_unsupported_language() {
        let minifier = CodeMinifier::new();
        let result = minifier.minify("code", "cobol");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MinifierError::UnsupportedLanguage(_)
        ));
    }

    #[test]
    fn test_estimate_tokens() {
        let code = "fn main() { println!(\"hello world\"); }";
        let tokens = CodeMinifier::estimate_tokens(code);
        assert!(tokens > 0);
        assert!(tokens < code.len()); // Should be less than byte count
    }

    #[test]
    fn test_is_supported_language() {
        assert!(CodeMinifier::is_supported_language("rust"));
        assert!(CodeMinifier::is_supported_language("rs"));
        assert!(CodeMinifier::is_supported_language("typescript"));
        assert!(CodeMinifier::is_supported_language("ts"));
        assert!(CodeMinifier::is_supported_language("python"));
        assert!(CodeMinifier::is_supported_language("py"));
        assert!(CodeMinifier::is_supported_language("go"));
        assert!(!CodeMinifier::is_supported_language("cobol"));
        assert!(!CodeMinifier::is_supported_language("fortran"));
    }

    #[test]
    fn test_config_options() {
        // Test with whitespace removal disabled
        let config = MinifierConfig {
            remove_whitespace: false,
            ..Default::default()
        };
        let minifier = CodeMinifier::with_config(config);
        let code = "fn   main()   {   }";
        let result = minifier.minify(code, "rust").unwrap();
        assert!(result.content.contains("   ")); // Should preserve extra spaces

        // Test with identifier compression disabled
        let config = MinifierConfig {
            compress_identifiers: false,
            ..Default::default()
        };
        let minifier = CodeMinifier::with_config(config);
        let code = "fn calculateTotalRevenue() { }";
        let result = minifier.minify(code, "rust").unwrap();
        assert!(result.content.contains("calculateTotalRevenue"));
        assert!(result.identifier_map.is_empty());
    }

    #[test]
    fn test_nested_comments_rust() {
        let minifier = CodeMinifier::new();
        let code = r#"
            /* Outer /* nested */ outer */
            fn main() { }
        "#;
        let result = minifier.minify(code, "rust").unwrap();
        assert!(!result.content.contains("Outer"));
        assert!(!result.content.contains("nested"));
        assert!(result.content.contains("fn"));
    }

    #[test]
    fn test_empty_code() {
        let minifier = CodeMinifier::new();
        let result = minifier.minify("", "rust").unwrap();
        assert_eq!(result.original_length, 0);
        assert_eq!(result.minified_length, 0);
        assert_eq!(result.savings_percentage, 0.0);
    }

    #[test]
    fn test_maintain_indentation() {
        let config = MinifierConfig {
            maintain_indentation: true,
            ..Default::default()
        };
        let minifier = CodeMinifier::with_config(config);
        let code = "fn main() {\n    println!(\"hello\");\n}";
        let result = minifier.minify(code, "rust").unwrap();

        // Should preserve newlines but trim line whitespace
        assert!(result.content.contains('\n'));
        assert!(!result.content.contains("    ")); // Leading spaces should be trimmed
    }

    #[test]
    fn test_benchmark_real_world_code() {
        // Test with realistic Rust code
        let rust_code = r#"
            /// Calculate the total revenue for a given period
            /// 
            /// # Arguments
            /// * `start_date` - The start of the period
            /// * `end_date` - The end of the period
            pub fn calculateTotalRevenueForPeriod(start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Decimal, RevenueError> {
                // Fetch all transactions in the period
                let transactions = self.fetchTransactionsInRange(start_date, end_date)?;
                
                // Calculate the sum
                let totalRevenue = transactions
                    .iter()
                    .filter(|t| t.status == TransactionStatus::Completed)
                    .map(|t| t.amount)
                    .sum::<Decimal>();
                
                Ok(totalRevenue)
            }
        "#;

        let minifier = CodeMinifier::new();
        let result = minifier.minify(rust_code, "rust").unwrap();

        println!("Rust Code Minification Benchmark:");
        println!("  Original: {} bytes", result.original_length);
        println!("  Minified: {} bytes", result.minified_length);
        println!(
            "  Savings: {:.1}% ({:.0} bytes)",
            result.savings_percentage,
            result.byte_savings()
        );
        println!("  Token savings: ~{} tokens", result.token_savings());

        // Should achieve at least 30% savings with comments + whitespace removal
        assert!(
            result.savings_percentage >= 30.0,
            "Expected >=30% savings, got {:.1}%",
            result.savings_percentage
        );

        // Verify decompression works
        let decompressed = minifier.decompress(&result).unwrap();
        assert!(decompressed.contains("calculateTotalRevenueForPeriod"));
        assert!(decompressed.contains("totalRevenue"));
    }

    #[test]
    fn test_typescript_code_savings() {
        let ts_code = r#"
            // Calculate user metrics for dashboard
            interface UserMetrics {
                totalUsers: number;
                activeUsers: number;
                newUsersThisMonth: number;
            }
            
            async function fetchUserMetrics(startDate: Date, endDate: Date): Promise<UserMetrics> {
                // Fetch from API
                const response = await fetch('/api/metrics/users');
                const data = await response.json();
                
                return {
                    totalUsers: data.total,
                    activeUsers: data.active,
                    newUsersThisMonth: data.new
                };
            }
        "#;

        let minifier = CodeMinifier::new();
        let result = minifier.minify(ts_code, "typescript").unwrap();

        println!("\nTypeScript Code Minification Benchmark:");
        println!("  Original: {} bytes", result.original_length);
        println!("  Minified: {} bytes", result.minified_length);
        println!("  Savings: {:.1}%", result.savings_percentage);

        assert!(result.savings_percentage >= 25.0);
    }

    #[test]
    fn test_python_code_savings() {
        let py_code = r#"
            # Calculate monthly revenue metrics
            def calculate_monthly_revenue_metrics(year: int, month: int) -> Dict[str, float]:
                """
                Calculate comprehensive revenue metrics for a given month.
                
                Args:
                    year: The year to calculate metrics for
                    month: The month to calculate metrics for
                    
                Returns:
                    Dictionary containing revenue metrics
                """
                # Fetch transactions from database
                transactions = fetch_transactions_for_period(year, month)
                
                # Calculate total revenue
                total_revenue = sum(t.amount for t in transactions)
                
                return {
                    'total': total_revenue,
                    'count': len(transactions)
                }
        "#;

        let minifier = CodeMinifier::new();
        let result = minifier.minify(py_code, "python").unwrap();

        println!("\nPython Code Minification Benchmark:");
        println!("  Original: {} bytes", result.original_length);
        println!("  Minified: {} bytes", result.minified_length);
        println!("  Savings: {:.1}%", result.savings_percentage);

        assert!(result.savings_percentage >= 35.0);
    }
}
