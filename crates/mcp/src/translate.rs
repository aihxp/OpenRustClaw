//! Tool format translation between MCP, Anthropic, and OpenAI schemas.

use serde_json::Value;

use openrustclaw_core::types::{ToolDefinition, ToolFormat};

use crate::client::McpToolDef;

/// Convert an MCP tool definition to a unified ToolDefinition.
pub fn mcp_to_unified(mcp_tool: &McpToolDef) -> ToolDefinition {
    ToolDefinition {
        name: mcp_tool.name.clone(),
        description: mcp_tool.description.clone(),
        parameters: mcp_tool.input_schema.clone(),
        strict: false, // MCP doesn't have strict mode
    }
}

/// Convert a unified ToolDefinition to an MCP tool definition.
pub fn unified_to_mcp(tool: &ToolDefinition, server_name: &str) -> McpToolDef {
    McpToolDef {
        name: tool.name.clone(),
        description: tool.description.clone(),
        input_schema: tool.parameters.clone(),
        server_name: server_name.to_string(),
    }
}

/// Convert a [`ToolDefinition`] into the wire format required by the target [`ToolFormat`].
///
/// The `_from` parameter is accepted for API symmetry but is not used because the
/// unified [`ToolDefinition`] is already provider-agnostic.
pub fn translate(tool: &ToolDefinition, _from: ToolFormat, to: ToolFormat) -> Value {
    match to {
        ToolFormat::Mcp => serde_json::json!({
            "name": tool.name,
            "description": tool.description,
            "inputSchema": tool.parameters,
        }),
        ToolFormat::Anthropic => serde_json::json!({
            "name": tool.name,
            "description": tool.description,
            "input_schema": tool.parameters,
        }),
        ToolFormat::OpenAi => {
            let mut params = tool.parameters.clone();
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tool() -> ToolDefinition {
        ToolDefinition {
            name: "get_weather".to_string(),
            description: "Get current weather".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": { "type": "string" },
                    "units": { "type": "string", "enum": ["celsius", "fahrenheit"] }
                },
                "required": ["location"]
            }),
            strict: false,
        }
    }

    fn strict_tool() -> ToolDefinition {
        ToolDefinition {
            name: "calculate".to_string(),
            description: "Perform calculation".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "expression": { "type": "string" }
                },
                "required": ["expression"]
            }),
            strict: true,
        }
    }

    // --- mcp_to_unified ---

    #[test]
    fn mcp_to_unified_preserves_name() {
        let mcp_tool = McpToolDef {
            name: "search".to_string(),
            description: "Search docs".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
            server_name: "docs-server".to_string(),
        };
        let unified = mcp_to_unified(&mcp_tool);
        assert_eq!(unified.name, "search");
    }

    #[test]
    fn mcp_to_unified_preserves_description() {
        let mcp_tool = McpToolDef {
            name: "search".to_string(),
            description: "Search docs".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
            server_name: "docs-server".to_string(),
        };
        let unified = mcp_to_unified(&mcp_tool);
        assert_eq!(unified.description, "Search docs");
    }

    #[test]
    fn mcp_to_unified_preserves_input_schema_as_parameters() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": { "query": { "type": "string" } }
        });
        let mcp_tool = McpToolDef {
            name: "search".to_string(),
            description: "Search".to_string(),
            input_schema: schema.clone(),
            server_name: "srv".to_string(),
        };
        let unified = mcp_to_unified(&mcp_tool);
        assert_eq!(unified.parameters, schema);
    }

    #[test]
    fn mcp_to_unified_sets_strict_false() {
        let mcp_tool = McpToolDef {
            name: "tool".to_string(),
            description: "desc".to_string(),
            input_schema: serde_json::json!({}),
            server_name: "srv".to_string(),
        };
        let unified = mcp_to_unified(&mcp_tool);
        assert!(!unified.strict);
    }

    // --- unified_to_mcp ---

    #[test]
    fn unified_to_mcp_preserves_name_and_description() {
        let tool = sample_tool();
        let mcp = unified_to_mcp(&tool, "my-server");
        assert_eq!(mcp.name, "get_weather");
        assert_eq!(mcp.description, "Get current weather");
    }

    #[test]
    fn unified_to_mcp_sets_server_name() {
        let tool = sample_tool();
        let mcp = unified_to_mcp(&tool, "weather-server");
        assert_eq!(mcp.server_name, "weather-server");
    }

    #[test]
    fn unified_to_mcp_preserves_parameters_as_input_schema() {
        let tool = sample_tool();
        let mcp = unified_to_mcp(&tool, "srv");
        assert_eq!(mcp.input_schema, tool.parameters);
    }

    // --- round-trip ---

    #[test]
    fn round_trip_mcp_to_unified_to_mcp_preserves_data() {
        let original = McpToolDef {
            name: "file_read".to_string(),
            description: "Read a file".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }),
            server_name: "fs-server".to_string(),
        };
        let unified = mcp_to_unified(&original);
        let back = unified_to_mcp(&unified, &original.server_name);
        assert_eq!(back.name, original.name);
        assert_eq!(back.description, original.description);
        assert_eq!(back.input_schema, original.input_schema);
        assert_eq!(back.server_name, original.server_name);
    }

    // --- translate to MCP format ---

    #[test]
    fn translate_to_mcp_uses_input_schema_key() {
        let tool = sample_tool();
        let result = translate(&tool, ToolFormat::Anthropic, ToolFormat::Mcp);
        assert_eq!(result["name"], "get_weather");
        assert_eq!(result["description"], "Get current weather");
        assert!(result.get("inputSchema").is_some());
        assert!(result.get("input_schema").is_none());
    }

    #[test]
    fn translate_to_mcp_preserves_schema_properties() {
        let tool = sample_tool();
        let result = translate(&tool, ToolFormat::OpenAi, ToolFormat::Mcp);
        assert_eq!(result["inputSchema"]["type"], "object");
        assert!(result["inputSchema"]["properties"]["location"].is_object());
    }

    // --- translate to Anthropic format ---

    #[test]
    fn translate_to_anthropic_uses_input_schema_snake_case() {
        let tool = sample_tool();
        let result = translate(&tool, ToolFormat::Mcp, ToolFormat::Anthropic);
        assert_eq!(result["name"], "get_weather");
        assert!(result.get("input_schema").is_some());
        assert!(result.get("inputSchema").is_none());
    }

    #[test]
    fn translate_to_anthropic_preserves_required_fields() {
        let tool = sample_tool();
        let result = translate(&tool, ToolFormat::Mcp, ToolFormat::Anthropic);
        let required = result["input_schema"]["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "location");
    }

    // --- translate to OpenAI format ---

    #[test]
    fn translate_to_openai_wraps_in_function_object() {
        let tool = sample_tool();
        let result = translate(&tool, ToolFormat::Mcp, ToolFormat::OpenAi);
        assert_eq!(result["type"], "function");
        assert!(result.get("function").is_some());
        assert_eq!(result["function"]["name"], "get_weather");
        assert_eq!(result["function"]["description"], "Get current weather");
    }

    #[test]
    fn translate_to_openai_non_strict_has_strict_false() {
        let tool = sample_tool();
        let result = translate(&tool, ToolFormat::Mcp, ToolFormat::OpenAi);
        assert_eq!(result["function"]["strict"], false);
    }

    #[test]
    fn translate_to_openai_strict_sets_additional_properties_false() {
        let tool = strict_tool();
        let result = translate(&tool, ToolFormat::Mcp, ToolFormat::OpenAi);
        assert_eq!(result["function"]["strict"], true);
        assert_eq!(
            result["function"]["parameters"]["additionalProperties"],
            false
        );
    }

    #[test]
    fn translate_to_openai_non_strict_does_not_set_additional_properties() {
        let tool = sample_tool();
        let result = translate(&tool, ToolFormat::Mcp, ToolFormat::OpenAi);
        assert!(result["function"]["parameters"]
            .get("additionalProperties")
            .is_none());
    }

    #[test]
    fn translate_to_openai_preserves_parameter_schema() {
        let tool = sample_tool();
        let result = translate(&tool, ToolFormat::Mcp, ToolFormat::OpenAi);
        let params = &result["function"]["parameters"];
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["location"].is_object());
        assert!(params["properties"]["units"]["enum"].is_array());
    }

    // --- cross-format consistency ---

    #[test]
    fn all_formats_preserve_tool_name() {
        let tool = sample_tool();
        let mcp = translate(&tool, ToolFormat::Mcp, ToolFormat::Mcp);
        let anthropic = translate(&tool, ToolFormat::Mcp, ToolFormat::Anthropic);
        let openai = translate(&tool, ToolFormat::Mcp, ToolFormat::OpenAi);
        assert_eq!(mcp["name"], "get_weather");
        assert_eq!(anthropic["name"], "get_weather");
        assert_eq!(openai["function"]["name"], "get_weather");
    }

    #[test]
    fn translate_with_empty_parameters() {
        let tool = ToolDefinition {
            name: "no_params".to_string(),
            description: "No parameters".to_string(),
            parameters: serde_json::json!({}),
            strict: false,
        };
        let mcp = translate(&tool, ToolFormat::Mcp, ToolFormat::Mcp);
        assert_eq!(mcp["inputSchema"], serde_json::json!({}));

        let openai = translate(&tool, ToolFormat::Mcp, ToolFormat::OpenAi);
        assert_eq!(openai["function"]["parameters"], serde_json::json!({}));
    }

    #[test]
    fn translate_with_nested_object_schema() {
        let tool = ToolDefinition {
            name: "complex".to_string(),
            description: "Complex schema".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "config": {
                        "type": "object",
                        "properties": {
                            "enabled": { "type": "boolean" },
                            "count": { "type": "integer" }
                        }
                    }
                }
            }),
            strict: false,
        };
        let result = translate(&tool, ToolFormat::Mcp, ToolFormat::Anthropic);
        assert_eq!(
            result["input_schema"]["properties"]["config"]["type"],
            "object"
        );
        assert_eq!(
            result["input_schema"]["properties"]["config"]["properties"]["enabled"]["type"],
            "boolean"
        );
    }
}
