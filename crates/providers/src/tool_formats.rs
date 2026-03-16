//! Tool schema translation between provider formats (MCP, Anthropic, OpenAI).
//!
//! Each LLM provider expects tool definitions in a slightly different JSON
//! structure. This module translates the unified [`ToolDefinition`] into the
//! wire format expected by the target provider, and parses provider-specific
//! tool call responses back into the unified [`ToolCall`] type.

use openrustclaw_core::types::{ToolCall, ToolDefinition, ToolFormat};
use serde_json::Value;

/// Translate a unified [`ToolDefinition`] to a provider-specific JSON value.
pub fn translate_tool_definition(tool: &ToolDefinition, target: ToolFormat) -> Value {
    match target {
        ToolFormat::Anthropic => to_anthropic_tool(tool),
        ToolFormat::OpenAi => to_openai_tool(tool),
        ToolFormat::Mcp => to_mcp_tool(tool),
    }
}

/// Convert to Anthropic's native tool format.
///
/// Anthropic expects:
/// ```json
/// {
///   "name": "...",
///   "description": "...",
///   "input_schema": { ... }
/// }
/// ```
fn to_anthropic_tool(tool: &ToolDefinition) -> Value {
    serde_json::json!({
        "name": tool.name,
        "description": tool.description,
        "input_schema": tool.parameters,
    })
}

/// Convert to OpenAI's function-calling format.
///
/// OpenAI expects:
/// ```json
/// {
///   "type": "function",
///   "function": {
///     "name": "...",
///     "description": "...",
///     "parameters": { ... },
///     "strict": true/false
///   }
/// }
/// ```
///
/// When `strict` is enabled, OpenAI requires `additionalProperties: false`
/// on all object schemas.
fn to_openai_tool(tool: &ToolDefinition) -> Value {
    let mut params = tool.parameters.clone();
    // OpenAI strict mode requires additionalProperties: false on all objects
    if tool.strict {
        if let Some(obj) = params.as_object_mut() {
            obj.insert(
                "additionalProperties".to_string(),
                Value::Bool(false),
            );
        }
    }
    serde_json::json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description,
            "parameters": params,
            "strict": tool.strict,
        }
    })
}

/// Convert to MCP (Model Context Protocol) format.
///
/// MCP expects:
/// ```json
/// {
///   "name": "...",
///   "description": "...",
///   "inputSchema": { ... }
/// }
/// ```
fn to_mcp_tool(tool: &ToolDefinition) -> Value {
    serde_json::json!({
        "name": tool.name,
        "description": tool.description,
        "inputSchema": tool.parameters,
    })
}

/// Parse Anthropic's content blocks into unified [`ToolCall`] values.
///
/// Anthropic returns tool invocations as content blocks with `type: "tool_use"`:
/// ```json
/// {
///   "type": "tool_use",
///   "id": "toolu_...",
///   "name": "tool_name",
///   "input": { ... }
/// }
/// ```
pub fn parse_anthropic_tool_calls(content_blocks: &[Value]) -> Vec<ToolCall> {
    content_blocks
        .iter()
        .filter(|block| {
            block.get("type").and_then(|t| t.as_str()) == Some("tool_use")
        })
        .filter_map(|block| {
            Some(ToolCall {
                id: block.get("id")?.as_str()?.to_string(),
                name: block.get("name")?.as_str()?.to_string(),
                arguments: block.get("input").cloned().unwrap_or(Value::Null),
            })
        })
        .collect()
}

/// Parse OpenAI's `tool_calls` array into unified [`ToolCall`] values.
///
/// OpenAI returns tool invocations on the assistant message:
/// ```json
/// {
///   "id": "call_...",
///   "type": "function",
///   "function": {
///     "name": "tool_name",
///     "arguments": "{ ... }"   // note: JSON as a string
///   }
/// }
/// ```
pub fn parse_openai_tool_calls(tool_calls: &[Value]) -> Vec<ToolCall> {
    tool_calls
        .iter()
        .filter_map(|tc| {
            let function = tc.get("function")?;
            Some(ToolCall {
                id: tc.get("id")?.as_str()?.to_string(),
                name: function.get("name")?.as_str()?.to_string(),
                arguments: function
                    .get("arguments")
                    .and_then(|a| a.as_str())
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or(Value::Null),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tool() -> ToolDefinition {
        ToolDefinition {
            name: "get_weather".to_string(),
            description: "Get the current weather for a location".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "City name"
                    }
                },
                "required": ["location"]
            }),
            strict: false,
        }
    }

    #[test]
    fn anthropic_tool_format() {
        let tool = sample_tool();
        let result = translate_tool_definition(&tool, ToolFormat::Anthropic);
        assert_eq!(result["name"], "get_weather");
        assert_eq!(result["input_schema"]["type"], "object");
        assert!(result.get("type").is_none()); // no wrapper type
    }

    #[test]
    fn openai_tool_format() {
        let tool = sample_tool();
        let result = translate_tool_definition(&tool, ToolFormat::OpenAi);
        assert_eq!(result["type"], "function");
        assert_eq!(result["function"]["name"], "get_weather");
        assert_eq!(result["function"]["strict"], false);
    }

    #[test]
    fn openai_strict_adds_additional_properties() {
        let mut tool = sample_tool();
        tool.strict = true;
        let result = translate_tool_definition(&tool, ToolFormat::OpenAi);
        assert_eq!(result["function"]["strict"], true);
        assert_eq!(
            result["function"]["parameters"]["additionalProperties"],
            false
        );
    }

    #[test]
    fn mcp_tool_format() {
        let tool = sample_tool();
        let result = translate_tool_definition(&tool, ToolFormat::Mcp);
        assert_eq!(result["name"], "get_weather");
        assert_eq!(result["inputSchema"]["type"], "object");
    }

    #[test]
    fn parse_anthropic_content_blocks() {
        let blocks = vec![
            serde_json::json!({
                "type": "text",
                "text": "Let me check the weather."
            }),
            serde_json::json!({
                "type": "tool_use",
                "id": "toolu_123",
                "name": "get_weather",
                "input": { "location": "San Francisco" }
            }),
        ];
        let calls = parse_anthropic_tool_calls(&blocks);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "toolu_123");
        assert_eq!(calls[0].name, "get_weather");
        assert_eq!(calls[0].arguments["location"], "San Francisco");
    }

    #[test]
    fn parse_openai_tool_calls_array() {
        let tool_calls = vec![serde_json::json!({
            "id": "call_abc",
            "type": "function",
            "function": {
                "name": "get_weather",
                "arguments": "{\"location\":\"New York\"}"
            }
        })];
        let calls = parse_openai_tool_calls(&tool_calls);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_abc");
        assert_eq!(calls[0].name, "get_weather");
        assert_eq!(calls[0].arguments["location"], "New York");
    }

    #[test]
    fn parse_openai_malformed_arguments() {
        let tool_calls = vec![serde_json::json!({
            "id": "call_bad",
            "type": "function",
            "function": {
                "name": "some_tool",
                "arguments": "not valid json"
            }
        })];
        let calls = parse_openai_tool_calls(&tool_calls);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].arguments, Value::Null);
    }
}
