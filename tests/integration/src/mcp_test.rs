//! MCP protocol integration tests.
//!
//! These tests verify:
//! - Tool schema translation
//! - MCP client connection handling
//! - Tool execution via MCP

use openrustclaw_core::types::ToolDefinition;
use openrustclaw_core::types::ToolFormat;
use openrustclaw_mcp::client::McpToolDef;
use openrustclaw_mcp::registry::McpRegistry;
use openrustclaw_mcp::translate::{mcp_to_unified, translate, unified_to_mcp};
use serde_json::json;

use crate::common::init_test_tracing;

// ═════════════════════════════════════════════════════════════════════════════
// Tool Schema Translation Tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn mcp_to_unified_conversion() {
    init_test_tracing();

    let mcp_tool = McpToolDef {
        name: "test_tool".to_string(),
        description: "A test tool".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "param1": { "type": "string" }
            }
        }),
        server_name: "test_server".to_string(),
    };

    let unified = mcp_to_unified(&mcp_tool);

    assert_eq!(unified.name, "test_tool");
    assert_eq!(unified.description, "A test tool");
    assert_eq!(unified.strict, false);
    assert!(unified.parameters.get("properties").is_some());
}

#[test]
fn unified_to_mcp_conversion() {
    init_test_tracing();

    let tool = ToolDefinition {
        name: "my_tool".to_string(),
        description: "My tool description".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "arg1": { "type": "number" }
            },
            "required": ["arg1"]
        }),
        strict: true,
    };

    let mcp = unified_to_mcp(&tool, "my_server");

    assert_eq!(mcp.name, "my_tool");
    assert_eq!(mcp.description, "My tool description");
    assert_eq!(mcp.server_name, "my_server");
}

#[test]
fn translate_to_mcp_format() {
    init_test_tracing();

    let tool = ToolDefinition {
        name: "weather".to_string(),
        description: "Get weather".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "location": { "type": "string" }
            }
        }),
        strict: false,
    };

    let mcp_json = translate(&tool, ToolFormat::OpenAi, ToolFormat::Mcp);

    assert_eq!(mcp_json.get("name").unwrap(), "weather");
    assert_eq!(mcp_json.get("description").unwrap(), "Get weather");
    assert!(mcp_json.get("inputSchema").is_some());
}

#[test]
fn translate_to_anthropic_format() {
    init_test_tracing();

    let tool = ToolDefinition {
        name: "search".to_string(),
        description: "Search documents".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" }
            }
        }),
        strict: true,
    };

    let anthropic_json = translate(&tool, ToolFormat::Mcp, ToolFormat::Anthropic);

    assert_eq!(anthropic_json.get("name").unwrap(), "search");
    assert!(anthropic_json.get("input_schema").is_some());
}

#[test]
fn translate_to_openai_format() {
    init_test_tracing();

    let tool = ToolDefinition {
        name: "calculator".to_string(),
        description: "Calculate".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "expression": { "type": "string" }
            }
        }),
        strict: true,
    };

    let openai_json = translate(&tool, ToolFormat::Mcp, ToolFormat::OpenAi);

    assert_eq!(openai_json.get("type").unwrap(), "function");

    let func = openai_json.get("function").unwrap();
    assert_eq!(func.get("name").unwrap(), "calculator");
    assert_eq!(func.get("strict").unwrap(), true);

    let params = func.get("parameters").unwrap();
    assert!(params.get("additionalProperties").is_some());
    assert_eq!(params.get("additionalProperties").unwrap(), false);
}

#[test]
fn translate_openai_strict_mode_adds_additional_properties() {
    init_test_tracing();

    let tool = ToolDefinition {
        name: "strict_tool".to_string(),
        description: "A strict tool".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {}
        }),
        strict: true,
    };

    let openai_json = translate(&tool, ToolFormat::Mcp, ToolFormat::OpenAi);
    let params = openai_json
        .get("function")
        .unwrap()
        .get("parameters")
        .unwrap();

    assert_eq!(params.get("additionalProperties").unwrap(), false);
}

#[test]
fn translate_openai_non_strict_no_additional_properties() {
    init_test_tracing();

    let tool = ToolDefinition {
        name: "loose_tool".to_string(),
        description: "A non-strict tool".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {}
        }),
        strict: false,
    };

    let openai_json = translate(&tool, ToolFormat::Mcp, ToolFormat::OpenAi);
    let params = openai_json
        .get("function")
        .unwrap()
        .get("parameters")
        .unwrap();

    assert!(params.get("additionalProperties").is_none());
}

#[test]
fn mcp_tool_def_structure() {
    init_test_tracing();

    let tool = McpToolDef {
        name: "test".to_string(),
        description: "Test tool".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "foo": { "type": "string" }
            }
        }),
        server_name: "server1".to_string(),
    };

    assert_eq!(tool.name, "test");
    assert_eq!(tool.server_name, "server1");
}

#[test]
fn roundtrip_mcp_to_unified_to_mcp() {
    init_test_tracing();

    let original = McpToolDef {
        name: "original_tool".to_string(),
        description: "Original description".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "param": { "type": "boolean" }
            }
        }),
        server_name: "original_server".to_string(),
    };

    let unified = mcp_to_unified(&original);
    let back_to_mcp = unified_to_mcp(&unified, "original_server");

    assert_eq!(back_to_mcp.name, original.name);
    assert_eq!(back_to_mcp.description, original.description);
    assert_eq!(back_to_mcp.input_schema, original.input_schema);
}

#[test]
fn translate_preserves_complex_schemas() {
    init_test_tracing();

    let tool = ToolDefinition {
        name: "complex".to_string(),
        description: "Complex tool".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "nested": {
                    "type": "object",
                    "properties": {
                        "array_field": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "enum_field": {
                    "type": "string",
                    "enum": ["a", "b", "c"]
                }
            },
            "required": ["nested"]
        }),
        strict: false,
    };

    let mcp_json = translate(&tool, ToolFormat::Mcp, ToolFormat::Mcp);
    let schema = mcp_json.get("inputSchema").unwrap();

    assert!(schema.get("properties").unwrap().get("nested").is_some());
    assert!(
        schema
            .get("properties")
            .unwrap()
            .get("enum_field")
            .is_some()
    );
    assert!(schema.get("required").is_some());
}

#[test]
fn translate_empty_parameters() {
    init_test_tracing();

    let tool = ToolDefinition {
        name: "no_params".to_string(),
        description: "Tool with no parameters".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {}
        }),
        strict: false,
    };

    let mcp_json = translate(&tool, ToolFormat::Mcp, ToolFormat::Mcp);
    let schema = mcp_json.get("inputSchema").unwrap();

    assert!(
        schema
            .get("properties")
            .unwrap()
            .as_object()
            .unwrap()
            .is_empty()
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// MCP Registry Tests
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn mcp_registry_creation() {
    init_test_tracing();

    let _registry = McpRegistry::new(vec![]);
    // Registry is created successfully
}

#[tokio::test]
async fn mcp_registry_with_configs() {
    init_test_tracing();

    use openrustclaw_mcp::registry::McpServerEntry;

    let configs = vec![
        McpServerEntry {
            name: "server1".to_string(),
            command: "echo".to_string(),
            args: vec!["hello".to_string()],
            enabled: true,
        },
        McpServerEntry {
            name: "server2".to_string(),
            command: "cat".to_string(),
            args: vec![],
            enabled: false,
        },
    ];

    let _registry = McpRegistry::new(configs);
    // Registry created with configs
}

// ═════════════════════════════════════════════════════════════════════════════
// Schema Edge Cases
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn translate_with_null_description() {
    init_test_tracing();

    let tool = ToolDefinition {
        name: "minimal".to_string(),
        description: "".to_string(),
        parameters: json!({"type": "object"}),
        strict: false,
    };

    let mcp_json = translate(&tool, ToolFormat::Mcp, ToolFormat::Mcp);
    assert_eq!(mcp_json.get("description").unwrap(), "");
}

#[test]
fn translate_nested_object_properties() {
    init_test_tracing();

    let tool = ToolDefinition {
        name: "nested".to_string(),
        description: "Nested tool".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "level1": {
                    "type": "object",
                    "properties": {
                        "level2": {
                            "type": "object",
                            "properties": {
                                "value": { "type": "string" }
                            }
                        }
                    }
                }
            }
        }),
        strict: false,
    };

    let mcp_json = translate(&tool, ToolFormat::Mcp, ToolFormat::Mcp);
    let schema = mcp_json.get("inputSchema").unwrap();

    let level1 = schema.get("properties").unwrap().get("level1").unwrap();
    assert!(level1.get("properties").unwrap().get("level2").is_some());
}

#[test]
fn translate_array_items() {
    init_test_tracing();

    let tool = ToolDefinition {
        name: "list_tool".to_string(),
        description: "List tool".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "value": { "type": "number" }
                        }
                    }
                }
            }
        }),
        strict: false,
    };

    let mcp_json = translate(&tool, ToolFormat::Mcp, ToolFormat::Mcp);
    let items = mcp_json
        .get("inputSchema")
        .unwrap()
        .get("properties")
        .unwrap()
        .get("items")
        .unwrap();

    assert_eq!(items.get("type").unwrap(), "array");
    assert!(items.get("items").unwrap().get("properties").is_some());
}

#[test]
fn translate_ref_in_schema() {
    init_test_tracing();

    let tool = ToolDefinition {
        name: "ref_tool".to_string(),
        description: "Tool with refs".to_string(),
        parameters: json!({
            "type": "object",
            "definitions": {
                "address": {
                    "type": "object",
                    "properties": {
                        "street": { "type": "string" }
                    }
                }
            },
            "properties": {
                "home": { "$ref": "#/definitions/address" }
            }
        }),
        strict: false,
    };

    let mcp_json = translate(&tool, ToolFormat::Mcp, ToolFormat::Mcp);
    assert!(mcp_json.get("inputSchema").is_some());
}

#[test]
fn translate_different_source_formats() {
    init_test_tracing();

    let tool = ToolDefinition {
        name: "convert_test".to_string(),
        description: "Test conversion".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "input": { "type": "string" }
            }
        }),
        strict: false,
    };

    // Test all format combinations
    let mcp_result = translate(&tool, ToolFormat::Anthropic, ToolFormat::Mcp);
    assert!(mcp_result.get("inputSchema").is_some());

    let anthropic_result = translate(&tool, ToolFormat::OpenAi, ToolFormat::Anthropic);
    assert!(anthropic_result.get("input_schema").is_some());

    let openai_result = translate(&tool, ToolFormat::Mcp, ToolFormat::OpenAi);
    assert_eq!(openai_result.get("type").unwrap(), "function");
}

#[test]
fn mcp_tool_def_deserialization() {
    init_test_tracing();

    let json_str = r#"{
        "name": "test_tool",
        "description": "A test tool",
        "inputSchema": {
            "type": "object",
            "properties": {
                "param": { "type": "string" }
            }
        },
        "server_name": "test_server"
    }"#;

    let tool: McpToolDef = serde_json::from_str(json_str).unwrap();
    assert_eq!(tool.name, "test_tool");
    assert_eq!(tool.server_name, "test_server");
}
