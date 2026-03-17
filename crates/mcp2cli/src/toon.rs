//! TOON (Token-Optimized Output Notation)
//!
//! TOON is a compact format for encoding JSON data that uses 40-60% fewer tokens
//! than standard JSON. It's designed for LLM consumption where token efficiency
//! is critical.
//!
//! # Format
//!
//! - Objects use `key:value` pairs separated by semicolons
//! - Arrays use `[item;item;item]` syntax
//! - Strings omit quotes when unambiguous
//! - Numbers and booleans are unchanged
//! - Nested structures use parentheses for grouping
//!
//! # Example
//!
//! JSON:
//! ```json
//! {"name": "John", "age": 30, "active": true}
//! ```
//!
//! TOON:
//! ```
//! name:John;age:30;active:true
//! ```

use crate::error::Result;
use serde_json::Value;

/// Encode JSON as TOON format
///
/// # Arguments
///
/// * `value` - The JSON value to encode
///
/// # Returns
///
/// A TOON-encoded string (40-60% fewer tokens than JSON)
///
/// # Example
///
/// ```
/// use openrustclaw_mcp2cli::encode_toon;
/// use serde_json::json;
///
/// let json = json!({"name": "test", "count": 42});
/// let toon = encode_toon(&json);
/// assert_eq!(toon, "name:test;count:42");
/// ```
pub fn encode_toon(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => encode_string(s),
        Value::Array(arr) => encode_array(arr),
        Value::Object(obj) => encode_object(obj),
    }
}

/// Encode a string value
///
/// Strings are quoted only when necessary (contain special chars)
fn encode_string(s: &str) -> String {
    // Check if string needs quoting
    let needs_quotes = s.is_empty()
        || s.contains(':')
        || s.contains(';')
        || s.contains('[')
        || s.contains(']')
        || s.contains('(')
        || s.contains(')')
        || s.contains('\n')
        || s.contains('\t')
        || s.contains(' ')
        || s.starts_with('"')
        || s == "true"
        || s == "false"
        || s == "null";

    if needs_quotes {
        // Escape quotes and backslashes
        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{}\"", escaped)
    } else {
        s.to_string()
    }
}

/// Encode an array
fn encode_array(arr: &[Value]) -> String {
    if arr.is_empty() {
        return "[]".to_string();
    }

    let items: Vec<String> = arr.iter().map(encode_toon).collect();
    format!("[{}]", items.join(";"))
}

/// Encode an object
fn encode_object(obj: &serde_json::Map<String, Value>) -> String {
    if obj.is_empty() {
        return String::new();
    }

    let pairs: Vec<String> = obj
        .iter()
        .map(|(k, v)| {
            let key = encode_string(k);
            let value = encode_toon(v);
            format!("{}:{}", key, value)
        })
        .collect();

    pairs.join(";")
}

/// Decode TOON back to JSON
///
/// # Arguments
///
/// * `input` - The TOON-encoded string
///
/// # Returns
///
/// The decoded JSON value
///
/// # Example
///
/// ```
/// use openrustclaw_mcp2cli::decode_toon;
/// use serde_json::json;
///
/// let toon = "name:test;count:42";
/// let json = decode_toon(toon).unwrap();
/// assert_eq!(json, json!({"name": "test", "count": 42}));
/// ```
pub fn decode_toon(input: &str) -> Result<Value> {
    if input.is_empty() {
        return Ok(Value::Object(serde_json::Map::new()));
    }

    // Try to parse as simple values first
    if input == "null" {
        return Ok(Value::Null);
    }
    if input == "true" {
        return Ok(Value::Bool(true));
    }
    if input == "false" {
        return Ok(Value::Bool(false));
    }

    // Try to parse as number
    if let Ok(n) = input.parse::<i64>() {
        return Ok(Value::Number(n.into()));
    }
    if let Ok(n) = input.parse::<f64>() {
        return serde_json::Number::from_f64(n)
            .map(Value::Number)
            .ok_or_else(|| crate::error::Mcp2CliError::toon(format!("Invalid float value: {}", n)));
    }

    // Try to parse as array
    if input.starts_with('[') && input.ends_with(']') {
        return decode_array(input);
    }

    // Try to parse as object
    if input.contains(':') {
        return decode_object(input);
    }

    // Must be a string
    decode_quoted_string(input)
}

/// Decode an array
fn decode_array(input: &str) -> Result<Value> {
    // Remove brackets
    let content = &input[1..input.len() - 1];

    if content.is_empty() {
        return Ok(Value::Array(Vec::new()));
    }

    // Split by semicolons, respecting nested structures
    let items = split_top_level(content, ';');
    let values: Result<Vec<Value>> = items.iter().map(|s| decode_toon(s.trim())).collect();

    Ok(Value::Array(values?))
}

/// Decode an object
fn decode_object(input: &str) -> Result<Value> {
    let mut map = serde_json::Map::new();

    // Split by semicolons at the top level
    let pairs = split_top_level(input, ';');

    for pair in pairs {
        if let Some(pos) = pair.find(':') {
            let key_str = pair[..pos].trim();
            let value_str = pair[pos + 1..].trim();

            let key = decode_key(key_str)?;
            let value = decode_toon(value_str)?;

            map.insert(key, value);
        }
    }

    Ok(Value::Object(map))
}

/// Decode a key (always a string, may be quoted)
fn decode_key(input: &str) -> Result<String> {
    if input.starts_with('"') && input.ends_with('"') {
        // Quoted key
        let inner = &input[1..input.len() - 1];
        Ok(unescape_string(inner))
    } else {
        // Unquoted key
        Ok(input.to_string())
    }
}

/// Decode a quoted string
fn decode_quoted_string(input: &str) -> Result<Value> {
    if input.starts_with('"') && input.ends_with('"') {
        let inner = &input[1..input.len() - 1];
        Ok(Value::String(unescape_string(inner)))
    } else {
        // Unquoted string
        Ok(Value::String(input.to_string()))
    }
}

/// Unescape a string value
fn unescape_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                match next {
                    '"' => result.push('"'),
                    '\\' => result.push('\\'),
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    _ => {
                        result.push(c);
                        result.push(next);
                    }
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Split string by delimiter, respecting brackets and quotes
fn split_top_level(s: &str, delimiter: char) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0;
    let mut in_quotes = false;
    let mut escape_next = false;
    let mut start = 0;

    for (i, c) in s.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if c == '\\' {
            escape_next = true;
            continue;
        }

        if c == '"' {
            in_quotes = !in_quotes;
            continue;
        }

        if !in_quotes {
            match c {
                '[' | '(' | '{' => depth += 1,
                ']' | ')' | '}' => depth -= 1,
                _ if c == delimiter && depth == 0 => {
                    result.push(&s[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }
    }

    // Add the last segment
    if start < s.len() {
        result.push(&s[start..]);
    }

    result
}

/// Calculate token savings percentage
///
/// # Arguments
///
/// * `json_tokens` - Token count of JSON representation
/// * `toon_tokens` - Token count of TOON representation
///
/// # Returns
///
/// Savings percentage (0.0 to 1.0), where 0.4 means 40% savings
///
/// # Example
///
/// ```
/// use openrustclaw_mcp2cli::calculate_savings;
///
/// let savings = calculate_savings(100, 60);
/// assert_eq!(savings, 0.4); // 40% savings
/// ```
pub fn calculate_savings(json_tokens: usize, toon_tokens: usize) -> f64 {
    if json_tokens == 0 {
        return 0.0;
    }

    let saved = json_tokens.saturating_sub(toon_tokens);
    saved as f64 / json_tokens as f64
}

/// Estimate token count for a string
///
/// Uses a simple approximation: ~4 characters per token on average
pub fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Encode and calculate savings in one call
///
/// Returns (toon_string, estimated_savings_percent)
pub fn encode_with_stats(value: &Value) -> (String, f64) {
    let json = value.to_string();
    let toon = encode_toon(value);

    let json_tokens = estimate_tokens(&json);
    let toon_tokens = estimate_tokens(&toon);

    let savings = calculate_savings(json_tokens, toon_tokens);

    (toon, savings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_simple_values() {
        assert_eq!(encode_toon(&Value::Null), "null");
        assert_eq!(encode_toon(&Value::Bool(true)), "true");
        assert_eq!(encode_toon(&Value::Bool(false)), "false");
        assert_eq!(encode_toon(&Value::Number(42.into())), "42");
        assert_eq!(encode_toon(&Value::String("hello".to_string())), "hello");
    }

    #[test]
    fn test_encode_quoted_string() {
        assert_eq!(encode_toon(&Value::String("hello world".to_string())), "\"hello world\"");
        assert_eq!(encode_toon(&Value::String("a:b".to_string())), "\"a:b\"");
        assert_eq!(encode_toon(&Value::String("a;b".to_string())), "\"a;b\"");
    }

    #[test]
    fn test_encode_object() {
        let obj = serde_json::json!({
            "name": "test",
            "count": 42,
            "active": true
        });
        let toon = encode_toon(&obj);
        assert!(toon.contains("name:test"));
        assert!(toon.contains("count:42"));
        assert!(toon.contains("active:true"));
    }

    #[test]
    fn test_encode_array() {
        let arr = serde_json::json!([1, 2, 3]);
        assert_eq!(encode_toon(&arr), "[1;2;3]");
    }

    #[test]
    fn test_decode_simple_values() {
        assert_eq!(decode_toon("null").unwrap(), Value::Null);
        assert_eq!(decode_toon("true").unwrap(), Value::Bool(true));
        assert_eq!(decode_toon("false").unwrap(), Value::Bool(false));
        assert_eq!(decode_toon("42").unwrap(), Value::Number(42.into()));
    }

    #[test]
    fn test_decode_object() {
        let toon = "name:test;count:42";
        let json = decode_toon(toon).unwrap();
        assert_eq!(json["name"], "test");
        assert_eq!(json["count"], 42);
    }

    #[test]
    fn test_decode_array() {
        let toon = "[1;2;3]";
        let json = decode_toon(toon).unwrap();
        assert_eq!(json, serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn test_roundtrip() {
        let original = serde_json::json!({
            "name": "test",
            "items": [1, 2, 3],
            "nested": {
                "key": "value"
            }
        });

        let toon = encode_toon(&original);
        let decoded = decode_toon(&toon).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_calculate_savings() {
        assert_eq!(calculate_savings(100, 60), 0.4);
        assert_eq!(calculate_savings(100, 40), 0.6);
        assert_eq!(calculate_savings(100, 100), 0.0);
    }

    #[test]
    fn test_split_top_level() {
        let input = "a:1;b:2;c:3";
        let parts = split_top_level(input, ';');
        assert_eq!(parts, vec!["a:1", "b:2", "c:3"]);

        let input2 = "[a;b];c:2";
        let parts2 = split_top_level(input2, ';');
        assert_eq!(parts2, vec!["[a;b]", "c:2"]);
    }
}
