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
