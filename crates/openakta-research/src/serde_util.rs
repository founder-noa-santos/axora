//! Lenient JSON field helpers so provider responses never panic the daemon on odd types.

use serde::Deserialize;

/// Deserialize a value that should become a `String`: accepts `null`, strings, numbers, and bools;
/// objects/arrays become empty strings.
pub(crate) fn lenient_string_or_default<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(deserializer)?;
    Ok(match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s,
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => String::new(),
    })
}
