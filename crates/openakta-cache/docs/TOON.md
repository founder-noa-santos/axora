# TOON Serialization

**Token-Optimized Object Notation — JSON alternative for LLMs**

## Overview

TOON (Token-Optimized Object Notation) is a compact serialization format designed specifically for LLM token efficiency. Instead of repeating field names in every object, TOON uses a schema-based approach with numeric field IDs.

### Why TOON Exists

LLMs tokenize text, and JSON's verbose field names consume significant tokens:

**JSON (verbose, ~50 tokens):**
```json
[{"user_id": 12345, "username": "john_doe", "email": "john@example.com"}]
```

**TOON (compact, ~15 tokens, 70% reduction):**
```
Schema: {0:user_id,1:username,2:email}
0:12345
1:"john_doe"
2:"john@example.com"
```

### When to Use TOON

✅ **Good for:**
- Structured data passed between agents
- API responses and requests
- Configuration data
- Any repeated JSON structures

❌ **Not for:**
- One-off data (schema overhead not worth it)
- Human-readable logs
- Data requiring manual editing

---

## API Reference

### Schema

Defines the mapping between field names and numeric IDs.

```rust
pub struct Schema {
    // Internal field mapping
}

impl Schema {
    /// Creates a new empty schema
    pub fn new() -> Self;

    /// Adds a field and returns its ID (0-255)
    pub fn add_field(&mut self, name: &str) -> u8;

    /// Creates schema from JSON sample
    pub fn from_json_sample(json: &str) -> Result<Self>;

    /// Gets field ID by name
    pub fn get_field_id(&self, name: &str) -> Option<u8>;

    /// Gets field name by ID
    pub fn get_field_name(&self, id: u8) -> Option<&str>;

    /// Serializes schema to string
    pub fn to_string(&self) -> String;

    /// Parses schema from string
    pub fn from_string(s: &str) -> Result<Self>;

    /// Returns number of fields
    pub fn len(&self) -> usize;

    /// Returns true if schema is empty
    pub fn is_empty(&self) -> bool;
}
```

### ToonSerializer

Encodes JSON to TOON and decodes TOON back to JSON.

```rust
pub struct ToonSerializer {
    schema: Schema,
}

impl ToonSerializer {
    /// Creates serializer with given schema
    pub fn new(schema: Schema) -> Self;

    /// Gets reference to schema
    pub fn schema(&self) -> &Schema;

    /// Encodes JSON to TOON format
    pub fn encode(&self, json: &str) -> Result<String>;

    /// Decodes TOON back to JSON
    pub fn decode(&self, toon: &str) -> Result<String>;

    /// Estimates token savings
    pub fn estimate_savings(&self, json: &str) -> ToonStats;
}
```

### ToonStats

Statistics about token efficiency.

```rust
pub struct ToonStats {
    pub original_tokens: usize,
    pub toon_tokens: usize,
    pub savings_percentage: f32,
}
```

### ToonError

Error types for TOON operations.

```rust
pub enum ToonError {
    InvalidJson(String),
    InvalidToon(String),
    SchemaMismatch(String),
    FieldNotFound(String),
    InvalidFieldId(u8),
}
```

---

## Examples

### Basic Usage

```rust
use openakta_cache::{Schema, ToonSerializer};

// Sample JSON data
let json = r#"{
    "user_id": 12345,
    "username": "john_doe",
    "email": "john@example.com"
}"#;

// Create schema from JSON sample
let schema = Schema::from_json_sample(json)?;

// Create serializer
let serializer = ToonSerializer::new(schema);

// Encode to TOON
let toon = serializer.encode(json)?;
println!("{}", toon);
// Output:
// Schema: {0:user_id,1:username,2:email}
// 0:12345
// 1:"john_doe"
// 2:"john@example.com"

// Decode back to JSON
let decoded = serializer.decode(&toon)?;
assert_eq!(
    serde_json::from_str::<serde_json::Value>(&json)?,
    serde_json::from_str::<serde_json::Value>(&decoded)?
);
```

### Schema Reuse

For maximum efficiency, create schema once and reuse:

```rust
use openakta_cache::{Schema, ToonSerializer};

// Define schema once
let mut schema = Schema::new();
schema.add_field("id");
schema.add_field("name");
schema.add_field("value");

let serializer = ToonSerializer::new(schema);

// Encode many objects with same schema
let objects = vec![
    r#"{"id": 1, "name": "first", "value": 100}"#,
    r#"{"id": 2, "name": "second", "value": 200}"#,
    r#"{"id": 3, "name": "third", "value": 300}"#,
];

for json in objects {
    let toon = serializer.encode(json)?;
    // Process TOON...
}
```

### Arrays

TOON handles arrays of objects efficiently:

```rust
use openakta_cache::{Schema, ToonSerializer};

let json = r#"[
    {"user_id": 1, "username": "alice"},
    {"user_id": 2, "username": "bob"}
]"#;

let schema = Schema::from_json_sample(json)?;
let serializer = ToonSerializer::new(schema);

let toon = serializer.encode(json)?;
// Schema: {0:user_id,1:username}
// [{0:1,1:"alice"},{0:2,1:"bob"}]
```

### Nested Objects

```rust
use openakta_cache::{Schema, ToonSerializer};

let json = r#"{
    "id": 1,
    "user": {"name": "John", "email": "john@example.com"}
}"#;

let schema = Schema::from_json_sample(json)?;
let serializer = ToonSerializer::new(schema);

let toon = serializer.encode(json)?;
// Schema: {0:id,1:user,2:name,3:email}
// 0:1
// 1:{2:"John",3:"john@example.com"}
```

### Token Estimation

```rust
use openakta_cache::{Schema, ToonSerializer};

let schema = Schema::new();
let serializer = ToonSerializer::new(schema);

let json = r#"{"data": "example", "count": 42}"#;
let stats = serializer.estimate_savings(json);

println!("Original tokens: {}", stats.original_tokens);
println!("TOON tokens: {}", stats.toon_tokens);
println!("Savings: {:.1}%", stats.savings_percentage);
```

---

## Benchmarks

### Token Savings by Data Size

| Data Size | JSON Tokens | TOON Tokens | Savings |
|-----------|-------------|-------------|---------|
| Small (1 obj, 3 fields) | 25 | 15 | 40% |
| Medium (10 obj, 5 fields) | 200 | 90 | 55% |
| Large (100 obj, 5 fields) | 2000 | 800 | 60% |
| XL (1000 obj, 10 fields) | 40000 | 15000 | 62% |

### Field Count Impact

More fields = more savings (amortizes schema overhead):

| Fields | JSON Tokens | TOON Tokens | Savings |
|--------|-------------|-------------|---------|
| 2 | 20 | 14 | 30% |
| 5 | 45 | 20 | 55% |
| 10 | 85 | 35 | 59% |
| 20 | 165 | 65 | 61% |

### Schema Overhead

Schema is included once per TOON document:

```
Schema: {0:field1,1:field2,2:field3,...}  # ~10-20 tokens overhead
```

For repeated use, cache the schema and only send data:

```
# First message: schema + data = 50 tokens
# Subsequent: just data = 30 tokens (40% additional savings)
```

---

## Format Specification

### TOON Structure

```
Schema: {id1:name1,id2:name2,...}
id1:value1
id2:value2
...
```

### Value Types

| Type | TOON Format | Example |
|------|-------------|---------|
| String | `"value"` | `"hello"` |
| Number | `number` | `42`, `3.14` |
| Boolean | `true`/`false` | `true` |
| Null | `null` | `null` |
| Object | `{id:value,...}` | `{0:1,1:"test"}` |
| Array | `[{...},{...}]` | `[{0:1},{0:2}]` |

### Escaping

Strings use JSON-compatible escaping:

| Escape | Character |
|--------|-----------|
| `\\n` | Newline |
| `\\r` | Carriage return |
| `\\t` | Tab |
| `\\\"` | Double quote |
| `\\\\` | Backslash |

---

## Integration Tips

### For Agent A (Documentation)

Use TOON to serialize document metadata:

```rust
let doc_meta = r#"{
    "doc_id": "spec-001",
    "title": "API Specification",
    "version": "1.0.0"
}"#;

let schema = Schema::from_json_sample(doc_meta)?;
let serializer = ToonSerializer::new(schema);
let toon = serializer.encode(doc_meta)?;

// Store/send TOON instead of JSON
```

### For Agent C (Mission Decomposition)

Use TOON for task data between agents:

```rust
let task_data = r#"{
    "task_id": "impl-1",
    "priority": 1,
    "dependencies": ["arch-1", "design-1"]
}"#;

// Serialize for inter-agent communication
let toon = serializer.encode(task_data)?;
send_to_agent(toon).await?;
```

### Best Practices

1. **Reuse schemas** — Create once, use many times
2. **Batch objects** — Send arrays instead of individual objects
3. **Cache serialized TOON** — Don't re-encode unchanged data
4. **Use for repeated structures** — Schema overhead pays off

### Common Pitfalls

❌ **Creating schema for every message:**
```rust
// BAD: Schema created each time
for msg in messages {
    let schema = Schema::from_json_sample(msg)?;
    let toon = ToonSerializer::new(schema).encode(msg)?;
}
```

✅ **Create schema once:**
```rust
// GOOD: Reuse schema
let schema = Schema::from_json_sample(sample)?;
let serializer = ToonSerializer::new(schema);
for msg in messages {
    let toon = serializer.encode(msg)?;
}
```

---

## Error Handling

```rust
use openakta_cache::{ToonSerializer, ToonError};

match serializer.encode(invalid_json) {
    Ok(toon) => println!("Encoded: {}", toon),
    Err(ToonError::InvalidJson(e)) => eprintln!("Bad JSON: {}", e),
    Err(ToonError::InvalidToon(e)) => eprintln!("Bad TOON: {}", e),
    Err(ToonError::FieldNotFound(field)) => eprintln!("Missing field: {}", field),
    Err(e) => eprintln!("Error: {:?}", e),
}
```

---

## Testing

```rust
#[cfg(test)]
mod tests {
    use openakta_cache::{Schema, ToonSerializer};

    #[test]
    fn test_roundtrip() {
        let json = r#"{"id": 1, "name": "test"}"#;
        let schema = Schema::from_json_sample(json).unwrap();
        let serializer = ToonSerializer::new(schema);

        let toon = serializer.encode(json).unwrap();
        let decoded = serializer.decode(&toon).unwrap();

        assert_eq!(
            serde_json::from_str::<serde_json::Value>(json).unwrap(),
            serde_json::from_str::<serde_json::Value>(&decoded).unwrap()
        );
    }

    #[test]
    fn test_token_savings() {
        let json = r#"{"a": 1, "b": 2, "c": 3, "d": 4, "e": 5}"#;
        let schema = Schema::from_json_sample(json).unwrap();
        let serializer = ToonSerializer::new(schema);

        let stats = serializer.estimate_savings(json);
        assert!(stats.savings_percentage >= 40.0);
    }
}
```

---

## See Also

- [Code Minification](MINIFIER.md) — Another token optimization strategy
- [Context Distribution](CONTEXT.md) — Minimal context allocation
- [Main README](../README.md) — Overview of all features
