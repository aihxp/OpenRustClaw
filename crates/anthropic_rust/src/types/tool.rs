//! Tool types for the Anthropic API.

use serde::{Deserialize, Serialize};

/// A tool definition that can be provided to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// The name of the tool.
    pub name: String,

    /// A description of what the tool does.
    pub description: String,

    /// The JSON schema for the tool's input.
    #[serde(rename = "input_schema")]
    pub input_schema: ToolInputSchema,
}

impl Tool {
    /// Create a new tool definition.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: ToolInputSchema,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }

    /// Create a tool with a JSON schema.
    pub fn with_schema(
        name: impl Into<String>,
        description: impl Into<String>,
        schema: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: ToolInputSchema { schema },
        }
    }

    /// Builder for creating a tool with parameters.
    pub fn builder(name: impl Into<String>, description: impl Into<String>) -> ToolBuilder {
        ToolBuilder::new(name, description)
    }
}

/// The input schema for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInputSchema {
    /// The JSON schema object.
    #[serde(flatten)]
    pub schema: serde_json::Value,
}

impl ToolInputSchema {
    /// Create a new object schema with the given properties.
    pub fn object(properties: Vec<(&str, serde_json::Value)>, required: Vec<&str>) -> Self {
        let props: serde_json::Map<String, serde_json::Value> = properties
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();

        Self {
            schema: serde_json::json!({
                "type": "object",
                "properties": props,
                "required": required,
            }),
        }
    }

    /// Create a simple schema that accepts any object.
    pub fn any_object() -> Self {
        Self {
            schema: serde_json::json!({
                "type": "object",
            }),
        }
    }
}

/// Builder for tool definitions.
#[derive(Debug, Clone)]
pub struct ToolBuilder {
    name: String,
    description: String,
    properties: Vec<(&'static str, serde_json::Value)>,
    required: Vec<&'static str>,
}

impl ToolBuilder {
    /// Create a new tool builder.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            properties: Vec::new(),
            required: Vec::new(),
        }
    }

    /// Add a string property.
    pub fn string_property(
        mut self,
        name: &'static str,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let prop = serde_json::json!({
            "type": "string",
            "description": description.into(),
        });
        self.properties.push((name, prop));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add an integer property.
    pub fn integer_property(
        mut self,
        name: &'static str,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let prop = serde_json::json!({
            "type": "integer",
            "description": description.into(),
        });
        self.properties.push((name, prop));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add a number property.
    pub fn number_property(
        mut self,
        name: &'static str,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let prop = serde_json::json!({
            "type": "number",
            "description": description.into(),
        });
        self.properties.push((name, prop));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add a boolean property.
    pub fn boolean_property(
        mut self,
        name: &'static str,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let prop = serde_json::json!({
            "type": "boolean",
            "description": description.into(),
        });
        self.properties.push((name, prop));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add an enum property.
    pub fn enum_property(
        mut self,
        name: &'static str,
        description: impl Into<String>,
        variants: Vec<&'static str>,
        required: bool,
    ) -> Self {
        let prop = serde_json::json!({
            "type": "string",
            "description": description.into(),
            "enum": variants,
        });
        self.properties.push((name, prop));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add a custom property.
    pub fn property(mut self, name: &'static str, schema: serde_json::Value, required: bool) -> Self {
        self.properties.push((name, schema));
        if required {
            self.required.push(name);
        }
        self
    }

    /// Build the tool definition.
    pub fn build(self) -> Tool {
        Tool {
            name: self.name,
            description: self.description,
            input_schema: ToolInputSchema::object(self.properties, self.required),
        }
    }
}

/// A tool use request from the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    /// Unique identifier for this tool use.
    pub id: String,

    /// The name of the tool to use.
    pub name: String,

    /// The input parameters for the tool.
    pub input: serde_json::Value,
}

impl ToolUse {
    /// Create a new tool use.
    pub fn new(id: impl Into<String>, name: impl Into<String>, input: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            input,
        }
    }

    /// Get a string parameter from the input.
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.input.get(key).and_then(|v| v.as_str())
    }

    /// Get an integer parameter from the input.
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.input.get(key).and_then(|v| v.as_i64())
    }

    /// Get a number parameter from the input.
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.input.get(key).and_then(|v| v.as_f64())
    }

    /// Get a boolean parameter from the input.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.input.get(key).and_then(|v| v.as_bool())
    }

    /// Get a parameter by key.
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.input.get(key)
    }
}

/// A tool result to send back to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The ID of the tool use this is responding to.
    #[serde(rename = "tool_use_id")]
    pub tool_use_id: String,

    /// The result content.
    pub content: String,

    /// Whether the tool execution resulted in an error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl ToolResult {
    /// Create a successful tool result.
    pub fn success(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            content: content.into(),
            is_error: Some(false),
        }
    }

    /// Create an error tool result.
    pub fn error(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            content: content.into(),
            is_error: Some(true),
        }
    }

    /// Create a tool result without explicit success/error flag.
    pub fn new(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            content: content.into(),
            is_error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_builder() {
        let tool = Tool::builder("get_weather", "Get the current weather")
            .string_property("location", "The city and state", true)
            .enum_property(
                "unit",
                "Temperature unit",
                vec!["celsius", "fahrenheit"],
                false,
            )
            .build();

        assert_eq!(tool.name, "get_weather");
        assert_eq!(tool.input_schema.schema["type"], "object");
        assert!(tool.input_schema.schema["required"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("location")));
    }

    #[test]
    fn test_tool_use_accessors() {
        let tool_use = ToolUse::new(
            "tool_123",
            "get_weather",
            serde_json::json!({
                "location": "San Francisco",
                "unit": "celsius",
                "days": 7
            }),
        );

        assert_eq!(tool_use.get_string("location"), Some("San Francisco"));
        assert_eq!(tool_use.get_i64("days"), Some(7));
    }

    #[test]
    fn test_tool_result() {
        let success = ToolResult::success("tool_123", "Sunny, 22°C");
        assert_eq!(success.tool_use_id, "tool_123");
        assert_eq!(success.is_error, Some(false));

        let error = ToolResult::error("tool_123", "Network error");
        assert_eq!(error.is_error, Some(true));
    }

    #[test]
    fn test_tool_serialization() {
        let tool = Tool::builder("test", "A test tool")
            .string_property("foo", "A foo value", true)
            .build();

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("input_schema"));
        assert!(json.contains("test"));
    }
}
