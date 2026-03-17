//! Tool and tool choice types for Bedrock.

use serde::{Deserialize, Serialize};

/// A tool specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// The tool specification.
    pub tool_spec: ToolSpecification,
}

impl Tool {
    /// Create a new tool.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            tool_spec: ToolSpecification {
                name: name.into(),
                description: description.into(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: None,
                    required: None,
                },
            },
        }
    }

    /// Set the input schema.
    pub fn with_schema(mut self, schema: ToolInputSchema) -> Self {
        self.tool_spec.input_schema = schema;
        self
    }

    /// Add a property to the input schema.
    pub fn with_property(
        mut self,
        name: impl Into<String>,
        property_type: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let name = name.into();
        let mut prop = serde_json::Map::new();
        prop.insert(
            "type".to_string(),
            serde_json::Value::String(property_type.into()),
        );
        prop.insert(
            "description".to_string(),
            serde_json::Value::String(description.into()),
        );

        self.tool_spec
            .input_schema
            .properties
            .get_or_insert_with(serde_json::Map::new)
            .insert(name, serde_json::Value::Object(prop));
        self
    }

    /// Make a property required.
    pub fn with_required(mut self, name: impl Into<String>) -> Self {
        self.tool_spec
            .input_schema
            .required
            .get_or_insert_with(Vec::new)
            .push(name.into());
        self
    }
}

/// Tool specification details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSpecification {
    /// The tool name.
    pub name: String,
    /// The tool description.
    pub description: String,
    /// The JSON schema for tool input.
    pub input_schema: ToolInputSchema,
}

/// Tool input schema (JSON Schema).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInputSchema {
    /// The schema type (usually "object").
    #[serde(rename = "type")]
    pub schema_type: String,
    /// Property definitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<serde_json::Map<String, serde_json::Value>>,
    /// Required properties.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

impl ToolInputSchema {
    /// Create a new object schema.
    pub fn object() -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: Some(serde_json::Map::new()),
            required: Some(Vec::new()),
        }
    }

    /// Add a string property.
    pub fn with_string(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        let mut prop = serde_json::Map::new();
        prop.insert("type".to_string(), "string".into());
        prop.insert("description".to_string(), description.into().into());

        self.properties
            .as_mut()
            .expect("properties always initialized in object()")
            .insert(name.clone(), prop.into());
        self
    }

    /// Add an integer property.
    pub fn with_integer(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        let mut prop = serde_json::Map::new();
        prop.insert("type".to_string(), "integer".into());
        prop.insert("description".to_string(), description.into().into());

        self.properties
            .as_mut()
            .expect("properties always initialized in object()")
            .insert(name.clone(), prop.into());
        self
    }

    /// Add a number property.
    pub fn with_number(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        let mut prop = serde_json::Map::new();
        prop.insert("type".to_string(), "number".into());
        prop.insert("description".to_string(), description.into().into());

        self.properties
            .as_mut()
            .expect("properties always initialized in object()")
            .insert(name.clone(), prop.into());
        self
    }

    /// Add a boolean property.
    pub fn with_boolean(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        let mut prop = serde_json::Map::new();
        prop.insert("type".to_string(), "boolean".into());
        prop.insert("description".to_string(), description.into().into());

        self.properties
            .as_mut()
            .expect("properties always initialized in object()")
            .insert(name.clone(), prop.into());
        self
    }

    /// Make a property required.
    pub fn required(mut self, name: impl Into<String>) -> Self {
        self.required
            .as_mut()
            .expect("required always initialized in object()")
            .push(name.into());
        self
    }

    /// Add an enum property.
    pub fn with_enum(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        values: Vec<String>,
    ) -> Self {
        let name = name.into();
        let mut prop = serde_json::Map::new();
        prop.insert("type".to_string(), "string".into());
        prop.insert("description".to_string(), description.into().into());
        let enum_values: Vec<serde_json::Value> = values.into_iter().map(|s| s.into()).collect();
        prop.insert("enum".to_string(), enum_values.into());

        self.properties
            .as_mut()
            .expect("properties always initialized in object()")
            .insert(name.clone(), prop.into());
        self
    }
}

/// Tool choice configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    /// The model can choose to use any tool, no tool, or multiple tools.
    Auto(AutoToolChoice),
    /// The model must use a tool (any available tool).
    Any(AnyToolChoice),
    /// The model must use the specified tool.
    Tool(SpecificToolChoice),
}

impl ToolChoice {
    /// Auto tool choice.
    pub fn auto() -> Self {
        ToolChoice::Auto(AutoToolChoice)
    }

    /// Force any tool to be used.
    pub fn any() -> Self {
        ToolChoice::Any(AnyToolChoice)
    }

    /// Force a specific tool to be used.
    pub fn tool(name: impl Into<String>) -> Self {
        ToolChoice::Tool(SpecificToolChoice { name: name.into() })
    }
}

/// Auto tool choice.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoToolChoice;

/// Any tool choice (force tool use).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnyToolChoice;

/// Specific tool choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecificToolChoice {
    /// The name of the tool to use.
    pub name: String,
}

/// Tool use block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolUseBlock {
    /// The tool use ID.
    pub tool_use_id: String,
    /// The tool name.
    pub name: String,
    /// The tool input.
    pub input: serde_json::Value,
}

/// Tool result block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResultBlock {
    /// The tool use ID.
    pub tool_use_id: String,
    /// The result content.
    pub content: Vec<super::ToolResultContent>,
    /// Whether this result represents an error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_creation() {
        let tool = Tool::new(
            "calculator",
            "A calculator tool for mathematical expressions",
        )
        .with_property(
            "expression",
            "string",
            "The mathematical expression to evaluate",
        )
        .with_required("expression");

        assert_eq!(tool.tool_spec.name, "calculator");
        assert_eq!(
            tool.tool_spec.description,
            "A calculator tool for mathematical expressions"
        );
        assert!(tool.tool_spec.input_schema.properties.is_some());
        assert!(tool.tool_spec.input_schema.required.is_some());
    }

    #[test]
    fn test_tool_input_schema() {
        let schema = ToolInputSchema::object()
            .with_string("name", "The person's name")
            .with_integer("age", "The person's age")
            .with_boolean("active", "Whether the person is active")
            .with_enum(
                "status",
                "The status",
                vec!["active".to_string(), "inactive".to_string()],
            )
            .required("name")
            .required("age");

        assert_eq!(schema.schema_type, "object");
        assert!(schema.properties.as_ref().unwrap().contains_key("name"));
        assert!(schema.properties.as_ref().unwrap().contains_key("age"));
        assert!(schema.properties.as_ref().unwrap().contains_key("active"));
        assert!(schema.properties.as_ref().unwrap().contains_key("status"));
        assert_eq!(schema.required.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_tool_choice() {
        let auto = ToolChoice::auto();
        match auto {
            ToolChoice::Auto(_) => {}
            _ => panic!("Expected auto tool choice"),
        }

        let any = ToolChoice::any();
        match any {
            ToolChoice::Any(_) => {}
            _ => panic!("Expected any tool choice"),
        }

        let specific = ToolChoice::tool("calculator");
        match specific {
            ToolChoice::Tool(t) => assert_eq!(t.name, "calculator"),
            _ => panic!("Expected specific tool choice"),
        }
    }

    #[test]
    fn test_tool_use_block() {
        let block = ToolUseBlock {
            tool_use_id: "tool_123".to_string(),
            name: "search".to_string(),
            input: serde_json::json!({"query": "rust programming"}),
        };
        assert_eq!(block.tool_use_id, "tool_123");
        assert_eq!(block.name, "search");
    }

    #[test]
    fn test_tool_result_block() {
        let block = ToolResultBlock {
            tool_use_id: "tool_123".to_string(),
            content: vec![super::super::ToolResultContent::text("Result here")],
            is_error: Some(false),
        };
        assert_eq!(block.tool_use_id, "tool_123");
        assert_eq!(block.is_error, Some(false));
    }
}
