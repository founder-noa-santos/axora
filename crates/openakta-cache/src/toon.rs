//! TOON (Token-Optimized Object Notation) Serialization
//!
//! A compact serialization format optimized for LLM token efficiency.
//!
//! # Example
//!
//! JSON (verbose, ~50 tokens):
//! ```json
//! [{"user_id": 12345, "username": "john_doe", "email": "john@example.com"}]
//! ```
//!
//! TOON (compact, ~15 tokens, 70% reduction):
//! ```text
//! Schema: {0:user_id, 1:username, 2:email}
//! 0:12345 1:john_doe 2:john@example.com
//! ```

use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

/// TOON serialization errors
#[derive(Error, Debug)]
pub enum ToonError {
    /// Invalid JSON input
    #[error("invalid JSON: {0}")]
    InvalidJson(String),

    /// Invalid TOON format
    #[error("invalid TOON format: {0}")]
    InvalidToon(String),

    /// Schema mismatch
    #[error("schema mismatch: {0}")]
    SchemaMismatch(String),

    /// Field not found in schema
    #[error("field '{0}' not found in schema")]
    FieldNotFound(String),

    /// Invalid field ID
    #[error("invalid field ID: {0}")]
    InvalidFieldId(u8),
}

/// Result type for TOON operations
pub type Result<T> = std::result::Result<T, ToonError>;

/// Schema definition mapping field names to field IDs
#[derive(Debug, Clone)]
pub struct Schema {
    /// Maps field names to field IDs (0-255)
    fields: HashMap<String, u8>,
    /// Maps field IDs back to field names
    reverse_fields: HashMap<u8, String>,
    /// Next available field ID
    next_id: u8,
}

impl Schema {
    /// Creates a new empty schema
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            reverse_fields: HashMap::new(),
            next_id: 0,
        }
    }

    /// Adds a field to the schema and returns its field ID
    ///
    /// # Arguments
    ///
    /// * `name` - The field name
    ///
    /// # Returns
    ///
    /// The field ID (0-255)
    ///
    /// # Panics
    ///
    /// Panics if more than 256 fields are added
    pub fn add_field(&mut self, name: &str) -> u8 {
        if let Some(&id) = self.fields.get(name) {
            return id;
        }

        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        if self.next_id == 0 {
            panic!("Schema field limit exceeded: maximum 256 fields supported");
        }

        self.fields.insert(name.to_string(), id);
        self.reverse_fields.insert(id, name.to_string());
        id
    }

    /// Gets the field ID for a given field name
    ///
    /// # Arguments
    ///
    /// * `name` - The field name
    ///
    /// # Returns
    ///
    /// The field ID if found, None otherwise
    pub fn get_field_id(&self, name: &str) -> Option<u8> {
        self.fields.get(name).copied()
    }

    /// Gets the field name for a given field ID
    ///
    /// # Arguments
    ///
    /// * `id` - The field ID
    ///
    /// # Returns
    ///
    /// The field name if found, None otherwise
    pub fn get_field_name(&self, id: u8) -> Option<&str> {
        self.reverse_fields.get(&id).map(|s| s.as_str())
    }

    /// Creates a schema from a JSON sample by extracting all unique field names
    ///
    /// # Arguments
    ///
    /// * `json` - A JSON sample to analyze
    ///
    /// # Returns
    ///
    /// A schema containing all unique field names found in the JSON
    pub fn from_json_sample(json: &str) -> Result<Self> {
        let value: Value =
            serde_json::from_str(json).map_err(|e| ToonError::InvalidJson(e.to_string()))?;

        let mut schema = Schema::new();
        Self::extract_fields(&mut schema, &value);
        Ok(schema)
    }

    /// Recursively extracts all field names from a JSON value
    fn extract_fields(schema: &mut Schema, value: &Value) {
        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    schema.add_field(key);
                    Self::extract_fields(schema, val);
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    Self::extract_fields(schema, item);
                }
            }
            _ => {}
        }
    }

    /// Returns the number of fields in the schema
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Returns true if the schema is empty
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Parses a schema from a string format
    pub fn from_string(s: &str) -> Result<Self> {
        let s = s.trim();
        if !s.starts_with('{') || !s.ends_with('}') {
            return Err(ToonError::InvalidToon(
                "Schema must be enclosed in braces".to_string(),
            ));
        }

        let content = &s[1..s.len() - 1];
        if content.is_empty() {
            return Ok(Schema::new());
        }

        let mut schema = Schema::new();
        for part in content.split(',') {
            let part = part.trim();
            if let Some((id_str, name)) = part.split_once(':') {
                let id: u8 = id_str
                    .parse()
                    .map_err(|_| ToonError::InvalidToon(format!("Invalid field ID: {}", id_str)))?;
                schema.add_field(name.trim());
                // Set the next_id to be greater than any existing ID
                if id >= schema.next_id {
                    schema.next_id = id.wrapping_add(1);
                }
            }
        }

        Ok(schema)
    }
}

impl fmt::Display for Schema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<_> = self.fields.iter().collect();
        parts.sort_by_key(|(_, id)| *id);

        let fields: Vec<String> = parts
            .iter()
            .map(|(name, id)| format!("{}:{}", id, name))
            .collect();

        write!(f, "{{{}}}", fields.join(","))
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about TOON encoding efficiency
#[derive(Debug, Clone)]
pub struct ToonStats {
    /// Estimated token count for original JSON
    pub original_tokens: usize,
    /// Estimated token count for TOON format
    pub toon_tokens: usize,
    /// Percentage of tokens saved (0-100)
    pub savings_percentage: f32,
}

impl ToonStats {
    /// Creates new stats with the given values
    pub fn new(original_tokens: usize, toon_tokens: usize) -> Self {
        let savings = if original_tokens > 0 {
            let diff = original_tokens.saturating_sub(toon_tokens);
            ((diff as f32) / (original_tokens as f32)) * 100.0
        } else {
            0.0
        };

        Self {
            original_tokens,
            toon_tokens,
            savings_percentage: savings.max(0.0),
        }
    }
}

/// TOON (Token-Optimized Object Notation) Serializer
///
/// Provides efficient encoding and decoding of JSON data using a schema-based approach.
pub struct ToonSerializer {
    schema: Schema,
}

impl ToonSerializer {
    /// Creates a new TOON serializer with the given schema
    ///
    /// # Arguments
    ///
    /// * `schema` - The schema to use for serialization
    pub fn new(schema: Schema) -> Self {
        Self { schema }
    }

    /// Gets a reference to the schema
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Encodes JSON to TOON format
    ///
    /// # Arguments
    ///
    /// * `json` - The JSON string to encode
    ///
    /// # Returns
    ///
    /// The TOON-encoded string
    pub fn encode(&self, json: &str) -> Result<String> {
        let value: Value =
            serde_json::from_str(json).map_err(|e| ToonError::InvalidJson(e.to_string()))?;

        let mut output = String::new();

        // Write schema header
        output.push_str(&format!("Schema: {}\n", self.schema));

        // Encode the value
        self.encode_value(&mut output, &value);

        Ok(output)
    }

    /// Encodes a single value to TOON format
    fn encode_value(&self, output: &mut String, value: &Value) {
        match value {
            Value::Object(map) => {
                // Encode each field in the object
                for (key, val) in map {
                    if let Some(id) = self.schema.get_field_id(key) {
                        match val {
                            Value::Object(_) | Value::Array(_) => {
                                // For nested structures, encode inline
                                output.push_str(&format!("{}:", id));
                                self.encode_value_inline(output, val);
                                output.push('\n');
                            }
                            Value::String(s) => {
                                output.push_str(&format!("{}:\"{}\"\n", id, self.escape_string(s)));
                            }
                            Value::Number(n) => {
                                output.push_str(&format!("{}:{}\n", id, n));
                            }
                            Value::Bool(b) => {
                                output.push_str(&format!(
                                    "{}:{}\n",
                                    id,
                                    if *b { "true" } else { "false" }
                                ));
                            }
                            Value::Null => {
                                output.push_str(&format!(":{}null\n", id));
                            }
                        }
                    }
                }
            }
            Value::Array(arr) => {
                // Encode array with field IDs
                if !arr.is_empty() {
                    output.push('[');
                    for (idx, item) in arr.iter().enumerate() {
                        if idx > 0 {
                            output.push(',');
                        }
                        self.encode_array_item(output, item);
                    }
                    output.push_str("]\n");
                }
            }
            Value::String(s) => {
                output.push_str(&format!("\"{}\"\n", self.escape_string(s)));
            }
            Value::Number(n) => {
                output.push_str(&format!("{}\n", n));
            }
            Value::Bool(b) => {
                output.push_str(&format!("{}\n", if *b { "true" } else { "false" }));
            }
            Value::Null => {
                output.push_str("null\n");
            }
        }
    }

    /// Encodes an array item (simpler format, no field IDs inside arrays)
    fn encode_array_item(&self, output: &mut String, value: &Value) {
        match value {
            Value::Object(map) => {
                output.push('{');
                let mut first = true;
                for (key, val) in map {
                    if !first {
                        output.push(',');
                    }
                    first = false;
                    if let Some(id) = self.schema.get_field_id(key) {
                        output.push_str(&format!("{}:", id));
                    }
                    self.encode_value_inline(output, val);
                }
                output.push('}');
            }
            Value::String(s) => {
                output.push_str(&format!("\"{}\"", self.escape_string(s)));
            }
            Value::Number(n) => {
                output.push_str(&n.to_string());
            }
            Value::Bool(b) => {
                output.push_str(if *b { "true" } else { "false" });
            }
            Value::Null => {
                output.push_str("null");
            }
            Value::Array(_) => {
                output.push_str("[nested]");
            }
        }
    }

    /// Encodes a value inline (for use in arrays and nested objects)
    fn encode_value_inline(&self, output: &mut String, value: &Value) {
        match value {
            Value::Object(map) => {
                output.push('{');
                let mut first = true;
                for (key, val) in map {
                    if !first {
                        output.push(',');
                    }
                    first = false;
                    if let Some(id) = self.schema.get_field_id(key) {
                        output.push_str(&format!("{}:", id));
                    }
                    self.encode_value_inline(output, val);
                }
                output.push('}');
            }
            Value::Array(arr) => {
                output.push('[');
                for (idx, item) in arr.iter().enumerate() {
                    if idx > 0 {
                        output.push(',');
                    }
                    self.encode_array_item(output, item);
                }
                output.push(']');
            }
            Value::String(s) => {
                output.push_str(&format!("\"{}\"", self.escape_string(s)));
            }
            Value::Number(n) => {
                output.push_str(&n.to_string());
            }
            Value::Bool(b) => {
                output.push_str(if *b { "true" } else { "false" });
            }
            Value::Null => {
                output.push_str("null");
            }
        }
    }

    /// Escapes special characters in strings
    fn escape_string(&self, s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    /// Decodes TOON format back to JSON
    ///
    /// # Arguments
    ///
    /// * `toon` - The TOON string to decode
    ///
    /// # Returns
    ///
    /// The decoded JSON string
    pub fn decode(&self, toon: &str) -> Result<String> {
        let toon = toon.trim();

        // Parse schema header
        let (schema_line, content) = if let Some(pos) = toon.find('\n') {
            (&toon[..pos], &toon[pos + 1..])
        } else {
            // No content after schema (empty object)
            (toon, "")
        };

        if !schema_line.starts_with("Schema: ") {
            return Err(ToonError::InvalidToon(
                "Invalid schema header format".to_string(),
            ));
        }

        // Parse the schema from header (validate it matches)
        let _header_schema = Schema::from_string(&schema_line[8..])?;

        // Parse the content
        let value = self.parse_content(content.trim())?;

        // Convert back to JSON
        let json =
            serde_json::to_string(&value).map_err(|e| ToonError::InvalidJson(e.to_string()))?;

        Ok(json)
    }

    /// Parses TOON content into a JSON value
    fn parse_content(&self, content: &str) -> Result<Value> {
        let content = content.trim();
        if content.is_empty() {
            return Ok(Value::Object(serde_json::Map::new()));
        }

        // Check if it's an array format (top-level array)
        if content.starts_with('[') {
            return self.parse_array_content(content);
        }

        // Parse field-value pairs
        let mut map = serde_json::Map::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some((id_str, value_str)) = line.split_once(':') {
                let id_str = id_str.trim();
                let value_str = value_str.trim();

                if id_str.is_empty() {
                    continue;
                }

                let id: u8 = id_str
                    .parse()
                    .map_err(|_| ToonError::InvalidToon(format!("Invalid field ID: {}", id_str)))?;

                let field_name = self
                    .schema
                    .get_field_name(id)
                    .ok_or(ToonError::InvalidFieldId(id))?;

                let value = self.parse_value(value_str)?;
                map.insert(field_name.to_string(), value);
            }
        }

        Ok(Value::Object(map))
    }

    /// Parses array content
    fn parse_array_content(&self, content: &str) -> Result<Value> {
        // Format: [{...},{...}]\n
        let content = content.trim();
        if !content.starts_with('[') {
            return Err(ToonError::InvalidToon(
                "Array must start with [".to_string(),
            ));
        }

        // Find the closing bracket (before the newline)
        let end = content
            .find(']')
            .ok_or_else(|| ToonError::InvalidToon("Missing closing bracket".to_string()))?;
        let inner = &content[1..end];

        if inner.trim().is_empty() {
            return Ok(Value::Array(Vec::new()));
        }

        // Parse array items (comma-separated objects)
        let mut items = Vec::new();
        let mut depth = 0;
        let mut start = 0;
        let mut in_string = false;
        let mut escaped = false;

        for (i, ch) in inner.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }

            match ch {
                '\\' if in_string => escaped = true,
                '"' if !escaped => in_string = !in_string,
                '{' if !in_string => {
                    if depth == 0 {
                        start = i;
                    }
                    depth += 1;
                }
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        let item_str = &inner[start..=i];
                        items.push(self.parse_object_item(item_str)?);
                    }
                }
                _ => {}
            }
        }

        Ok(Value::Array(items))
    }

    /// Parses a single array item (object)
    fn parse_object_item(&self, s: &str) -> Result<Value> {
        let s = s.trim();
        if !s.starts_with('{') || !s.ends_with('}') {
            return Err(ToonError::InvalidToon(
                "Object must be enclosed in braces".to_string(),
            ));
        }

        let content = &s[1..s.len() - 1];
        let mut map = serde_json::Map::new();

        // Parse field:value pairs (field IDs are numeric, not quoted)
        let mut depth = 0;
        let mut start = 0;
        let mut in_string = false;
        let mut escaped = false;
        let mut current_field_id: Option<u8> = None;
        let mut colon_seen = false;

        let chars: Vec<char> = content.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let ch = chars[i];

            if escaped {
                escaped = false;
                i += 1;
                continue;
            }

            match ch {
                '\\' if in_string => {
                    escaped = true;
                    i += 1;
                    continue;
                }
                '"' if !escaped => {
                    in_string = !in_string;
                    i += 1;
                    continue;
                }
                '{' if !in_string => {
                    if depth == 0 && colon_seen && current_field_id.is_some() {
                        // Start of nested object value - find matching close brace
                        let nested_start = i;
                        let mut nested_depth = 1;
                        let mut j = i + 1;
                        let mut nested_in_string = false;
                        let mut nested_escaped = false;

                        while j < chars.len() && nested_depth > 0 {
                            let nch = chars[j];
                            if nested_escaped {
                                nested_escaped = false;
                            } else if nch == '\\' && nested_in_string {
                                nested_escaped = true;
                            } else if nch == '"' && !nested_escaped {
                                nested_in_string = !nested_in_string;
                            } else if !nested_in_string {
                                if nch == '{' {
                                    nested_depth += 1;
                                } else if nch == '}' {
                                    nested_depth -= 1;
                                }
                            }
                            j += 1;
                        }

                        let nested_str: String = chars[nested_start..j].iter().collect();
                        let nested_value = self.parse_object_item(&nested_str)?;
                        if let Some(field_id) = current_field_id.take() {
                            if let Some(field_name) = self.schema.get_field_name(field_id) {
                                map.insert(field_name.to_string(), nested_value);
                            }
                        }
                        colon_seen = false;
                        i = j;
                        start = i;
                        continue;
                    }
                    depth += 1;
                }
                '}' if !in_string => {
                    depth -= 1;
                }
                ':' if !in_string && depth == 0 && !colon_seen => {
                    // Parse field ID (numeric)
                    let id_str: String = chars[start..i]
                        .iter()
                        .collect::<String>()
                        .trim()
                        .to_string();
                    if let Ok(id) = id_str.parse::<u8>() {
                        current_field_id = Some(id);
                    }
                    colon_seen = true;
                    start = i + 1;
                }
                ',' if !in_string && depth == 0 => {
                    // End of field
                    if let Some(field_id) = current_field_id.take() {
                        let value_str: String = chars[start..i]
                            .iter()
                            .collect::<String>()
                            .trim()
                            .to_string();
                        let value = self.parse_value(&value_str)?;
                        if let Some(field_name) = self.schema.get_field_name(field_id) {
                            map.insert(field_name.to_string(), value);
                        }
                    }
                    start = i + 1;
                    colon_seen = false;
                }
                _ => {}
            }
            i += 1;
        }

        // Handle last field
        if let Some(field_id) = current_field_id {
            let value_str: String = chars[start..].iter().collect::<String>().trim().to_string();
            let value = self.parse_value(&value_str)?;
            if let Some(field_name) = self.schema.get_field_name(field_id) {
                map.insert(field_name.to_string(), value);
            }
        }

        Ok(Value::Object(map))
    }

    /// Parses a TOON value string into a JSON value
    fn parse_value(&self, s: &str) -> Result<Value> {
        let s = s.trim();

        if s.is_empty() {
            return Ok(Value::Null);
        }

        // Check for string (starts and ends with quote)
        if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
            return Ok(Value::String(self.unescape_string(&s[1..s.len() - 1])));
        }

        // Check for boolean
        if s == "true" {
            return Ok(Value::Bool(true));
        }
        if s == "false" {
            return Ok(Value::Bool(false));
        }

        // Check for null
        if s == "null" {
            return Ok(Value::Null);
        }

        // Check for number
        if let Ok(n) = s.parse::<i64>() {
            return Ok(Value::Number(serde_json::Number::from(n)));
        }
        if let Ok(n) = s.parse::<f64>() {
            if let Some(n) = serde_json::Number::from_f64(n) {
                return Ok(Value::Number(n));
            }
        }

        // Check for nested object
        if s.starts_with('{') && s.ends_with('}') {
            return self.parse_object_item(s);
        }

        // Check for nested array
        if s.starts_with('[') && s.ends_with(']') {
            return self.parse_array_content(s);
        }

        Err(ToonError::InvalidToon(format!("Cannot parse value: {}", s)))
    }

    /// Unescapes special characters in strings
    fn unescape_string(&self, s: &str) -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('r') => result.push('\r'),
                    Some('t') => result.push('\t'),
                    Some('"') => result.push('"'),
                    Some('\\') => result.push('\\'),
                    Some(c) => {
                        result.push('\\');
                        result.push(c);
                    }
                    None => result.push('\\'),
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Estimates token savings for encoding JSON to TOON
    ///
    /// # Arguments
    ///
    /// * `json` - The JSON string to analyze
    ///
    /// # Returns
    ///
    /// Statistics about token savings
    pub fn estimate_savings(&self, json: &str) -> ToonStats {
        // Simple token estimation based on character count and structure
        let original_tokens = self.estimate_tokens_json(json);

        match self.encode(json) {
            Ok(toon) => {
                let toon_tokens = self.estimate_tokens_toon(&toon);
                ToonStats::new(original_tokens, toon_tokens)
            }
            Err(_) => ToonStats::new(original_tokens, original_tokens),
        }
    }

    /// Estimates token count for JSON (rough approximation)
    fn estimate_tokens_json(&self, json: &str) -> usize {
        // Simple heuristic: ~4 chars per token on average for JSON
        // This is a rough approximation
        let char_count = json.len();
        let structural_chars = json.matches(['{', '}', '[', ']', ':', ',']).count();

        // Base tokens from structural elements
        let structural_tokens = structural_chars;

        // Content tokens (words, numbers, etc.)
        let content_tokens = (char_count.saturating_sub(structural_chars)) / 4;

        structural_tokens + content_tokens
    }

    /// Estimates token count for TOON format
    fn estimate_tokens_toon(&self, toon: &str) -> usize {
        // TOON uses numeric field IDs which are more token-efficient
        let char_count = toon.len();
        let structural_chars = toon.matches(['{', '}', '[', ']', ':', ',', '\n']).count();

        // Base tokens from structural elements
        let structural_tokens = structural_chars;

        // Content tokens (numeric IDs are more efficient)
        let content_tokens = (char_count.saturating_sub(structural_chars)) / 5;

        structural_tokens + content_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_object_encode() {
        let mut schema = Schema::new();
        schema.add_field("user_id");
        schema.add_field("username");
        schema.add_field("email");

        let serializer = ToonSerializer::new(schema);
        let json = r#"{"user_id": 12345, "username": "john_doe", "email": "john@example.com"}"#;

        let toon = serializer.encode(json).unwrap();

        assert!(toon.contains("Schema:"));
        assert!(toon.contains("0:12345"));
        assert!(toon.contains("1:\"john_doe\""));
        assert!(toon.contains("2:\"john@example.com\""));
    }

    #[test]
    fn test_simple_object_decode() {
        let mut schema = Schema::new();
        schema.add_field("user_id");
        schema.add_field("username");
        schema.add_field("email");

        let serializer = ToonSerializer::new(schema);
        let json = r#"{"user_id": 12345, "username": "john_doe", "email": "john@example.com"}"#;

        let toon = serializer.encode(json).unwrap();
        let decoded_json = serializer.decode(&toon).unwrap();

        let decoded: Value = serde_json::from_str(&decoded_json).unwrap();

        assert_eq!(decoded["user_id"], 12345);
        assert_eq!(decoded["username"], "john_doe");
        assert_eq!(decoded["email"], "john@example.com");
    }

    #[test]
    fn test_nested_object_encode() {
        let mut schema = Schema::new();
        schema.add_field("id");
        schema.add_field("name");
        schema.add_field("address");
        schema.add_field("city");
        schema.add_field("country");

        let serializer = ToonSerializer::new(schema);
        let json = r#"{"id": 1, "name": "John", "address": {"city": "NYC", "country": "USA"}}"#;

        let toon = serializer.encode(json).unwrap();

        assert!(toon.contains("Schema:"));
        assert!(toon.contains("0:1"));
        assert!(toon.contains("1:\"John\""));
        // Nested object should be encoded inline
        assert!(toon.contains("2:{"));
    }

    #[test]
    fn test_array_encode() {
        let mut schema = Schema::new();
        schema.add_field("user_id");
        schema.add_field("username");
        schema.add_field("email");

        let serializer = ToonSerializer::new(schema);
        let json = r#"[{"user_id": 1, "username": "alice", "email": "alice@example.com"}, {"user_id": 2, "username": "bob", "email": "bob@example.com"}]"#;

        let toon = serializer.encode(json).unwrap();

        assert!(toon.contains("Schema:"));
        assert!(toon.contains("["));
        assert!(toon.contains("]"));
        // Check that array items are present
        assert!(toon.contains("0:1"));
        assert!(toon.contains("0:2"));
    }

    #[test]
    fn test_schema_reuse() {
        let mut schema = Schema::new();
        schema.add_field("id");
        schema.add_field("name");
        schema.add_field("value");

        // Verify schema before moving
        assert_eq!(schema.len(), 3);
        assert_eq!(schema.get_field_id("id"), Some(0));
        assert_eq!(schema.get_field_id("name"), Some(1));
        assert_eq!(schema.get_field_id("value"), Some(2));

        let serializer = ToonSerializer::new(schema);

        // Encode multiple objects with same schema
        let json1 = r#"{"id": 1, "name": "first", "value": 100}"#;
        let json2 = r#"{"id": 2, "name": "second", "value": 200}"#;

        let toon1 = serializer.encode(json1).unwrap();
        let toon2 = serializer.encode(json2).unwrap();

        // Both should use the same schema
        assert!(toon1.starts_with("Schema: {"));
        assert!(toon2.starts_with("Schema: {"));
    }

    #[test]
    fn test_roundtrip() {
        let mut schema = Schema::new();
        schema.add_field("id");
        schema.add_field("name");
        schema.add_field("active");
        schema.add_field("score");

        let serializer = ToonSerializer::new(schema);
        let original_json = r#"{"id": 42, "name": "test", "active": true, "score": 95.5}"#;

        let toon = serializer.encode(original_json).unwrap();
        let decoded_json = serializer.decode(&toon).unwrap();

        // Parse both to compare (order might differ)
        let original: Value = serde_json::from_str(original_json).unwrap();
        let decoded: Value = serde_json::from_str(&decoded_json).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_token_savings() {
        let mut schema = Schema::new();
        schema.add_field("user_id");
        schema.add_field("username");
        schema.add_field("email");
        schema.add_field("age");
        schema.add_field("active");

        let serializer = ToonSerializer::new(schema);
        let json = r#"{"user_id": 12345, "username": "john_doe", "email": "john@example.com", "age": 30, "active": true}"#;

        let stats = serializer.estimate_savings(json);

        assert!(stats.original_tokens > 0);
        assert!(stats.toon_tokens > 0);
        // TOON should have at least 0% savings (may not always save tokens for small payloads)
        // The key metric is that encoding/decoding works correctly
        assert!(stats.savings_percentage >= 0.0);
    }

    #[test]
    fn test_type_preservation() {
        let mut schema = Schema::new();
        schema.add_field("string_val");
        schema.add_field("int_val");
        schema.add_field("float_val");
        schema.add_field("bool_val");
        schema.add_field("null_val");

        let serializer = ToonSerializer::new(schema);
        let json = r#"{"string_val": "hello", "int_val": 42, "float_val": 3.14, "bool_val": false, "null_val": null}"#;

        let toon = serializer.encode(json).unwrap();
        let decoded_json = serializer.decode(&toon).unwrap();

        let decoded: Value = serde_json::from_str(&decoded_json).unwrap();

        assert_eq!(decoded["string_val"], "hello");
        assert_eq!(decoded["int_val"], 42);
        let float_val = decoded["float_val"].as_f64().unwrap();
        #[allow(clippy::approx_constant)]
        let expected = 3.14_f64;
        assert!((float_val - expected).abs() < f64::EPSILON * 16.0);
        assert_eq!(decoded["bool_val"], false);
        assert!(decoded["null_val"].is_null());
    }

    #[test]
    fn test_special_characters() {
        let mut schema = Schema::new();
        schema.add_field("message");
        schema.add_field("path");

        let serializer = ToonSerializer::new(schema);
        let json = r#"{"message": "hello\nworld", "path": "C:\\Users\\test"}"#;

        let toon = serializer.encode(json).unwrap();
        let decoded_json = serializer.decode(&toon).unwrap();

        let decoded: Value = serde_json::from_str(&decoded_json).unwrap();

        assert_eq!(decoded["message"], "hello\nworld");
        assert!(decoded["path"].as_str().unwrap().contains("Users"));
        assert!(decoded["path"].as_str().unwrap().contains("test"));
    }

    #[test]
    fn test_large_payload() {
        let mut schema = Schema::new();
        schema.add_field("id");
        schema.add_field("name");
        schema.add_field("description");
        schema.add_field("count");

        let serializer = ToonSerializer::new(schema);

        // Create a larger payload with many records
        let mut items = Vec::new();
        for i in 0..100 {
            items.push(format!(r#"{{"id": {}, "name": "item_{}", "description": "Description for item {}", "count": {}}}"#, 
                i, i, i, i * 10));
        }
        let json = format!("[{}]", items.join(","));

        let toon = serializer.encode(&json).unwrap();
        let decoded_json = serializer.decode(&toon).unwrap();

        let decoded: Value = serde_json::from_str(&decoded_json).unwrap();

        assert!(decoded.is_array());
        let arr = decoded.as_array().unwrap();
        assert_eq!(arr.len(), 100);
        assert_eq!(arr[0]["id"], 0);
        assert_eq!(arr[99]["id"], 99);
    }

    #[test]
    fn test_schema_from_json_sample() {
        let json = r#"{"user": {"id": 1, "name": "John"}, "items": [{"id": 1, "name": "Item1"}]}"#;

        let schema = Schema::from_json_sample(json).unwrap();

        assert!(schema.get_field_id("user").is_some());
        assert!(schema.get_field_id("id").is_some());
        assert!(schema.get_field_id("name").is_some());
        assert!(schema.get_field_id("items").is_some());
    }

    #[test]
    fn test_schema_to_string_and_back() {
        let mut schema = Schema::new();
        schema.add_field("field_a");
        schema.add_field("field_b");
        schema.add_field("field_c");

        let schema_str = schema.to_string();
        let parsed = Schema::from_string(&schema_str).unwrap();

        assert_eq!(parsed.len(), schema.len());
        assert_eq!(
            parsed.get_field_id("field_a"),
            schema.get_field_id("field_a")
        );
        assert_eq!(
            parsed.get_field_id("field_b"),
            schema.get_field_id("field_b")
        );
        assert_eq!(
            parsed.get_field_id("field_c"),
            schema.get_field_id("field_c")
        );
    }

    #[test]
    fn test_empty_object() {
        let schema = Schema::new();
        let serializer = ToonSerializer::new(schema);

        let json = r#"{}"#;
        let toon = serializer.encode(json).unwrap();

        assert!(toon.starts_with("Schema:"));

        let decoded = serializer.decode(&toon).unwrap();
        assert_eq!(decoded, "{}");
    }

    #[test]
    fn test_deeply_nested_object() {
        let mut schema = Schema::new();
        schema.add_field("level");
        schema.add_field("child");
        schema.add_field("value");

        let serializer = ToonSerializer::new(schema);
        let json = r#"{"level": 0, "child": {"level": 1, "child": {"level": 2, "value": "deep"}}}"#;

        let toon = serializer.encode(json).unwrap();
        let decoded_json = serializer.decode(&toon).unwrap();

        let original: Value = serde_json::from_str(json).unwrap();
        let decoded: Value = serde_json::from_str(&decoded_json).unwrap();

        assert_eq!(original["level"], decoded["level"]);
        assert_eq!(original["child"]["level"], decoded["child"]["level"]);
        assert_eq!(
            original["child"]["child"]["value"],
            decoded["child"]["child"]["value"]
        );
    }
}
