# Tool Trait API Reference

This reference documents the `Tool` trait and related types for implementing custom tools.

---

## `Tool` Trait

The main trait for executable tools that the agent can invoke.

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique tool name (e.g. "memory_search", "file_read")
    fn name(&self) -> &str;

    /// Human-readable description for the LLM
    fn description(&self) -> &str;

    /// JSON Schema describing the tool's input parameters
    fn schema(&self) -> Value;

    /// Capabilities this tool requires (used for WASM sandbox enforcement)
    fn capabilities_required(&self) -> Vec<SkillCapability>;

    /// Execute the tool with the given input and context
    async fn execute(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput>;
}
```

### Example Implementation

```rust
use async_trait::async_trait;
use openrustclaw_core::traits::{Tool, ToolContext};
use openrustclaw_core::types::{SkillCapability, ToolOutput};
use serde_json::Value;

pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }
    
    fn description(&self) -> &str {
        "Read the contents of a file at the specified path"
    }
    
    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read",
                    "default": 100
                }
            },
            "required": ["path"]
        })
    }
    
    fn capabilities_required(&self) -> Vec<SkillCapability> {
        vec![SkillCapability::FileRead]
    }
    
    async fn execute(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        // Parse input
        let path = input["path"].as_str()
            .ok_or_else(|| Error::invalid_input("path required"))?;
        
        let limit = input["limit"].as_u64().unwrap_or(100);
        
        // Validate path is within workspace
        let full_path = if let Some(workspace) = &ctx.workspace_path {
            PathBuf::from(workspace).join(path)
        } else {
            PathBuf::from(path)
        };
        
        // Read file
        let content = tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|e| Error::execution_failed(format!("Failed to read file: {}", e)))?;
        
        // Apply limit
        let lines: Vec<&str> = content.lines().take(limit as usize).collect();
        let result = lines.join("\n");
        
        Ok(ToolOutput {
            tool_call_id: ctx.tool_call_id.clone(),
            content: result,
            is_error: false,
        })
    }
}
```

---

## `ToolContext`

Context provided to tools during execution.

```rust
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Current session ID
    pub session_id: String,
    
    /// Current user ID
    pub user_id: String,
    
    /// Workspace root path (if applicable)
    pub workspace_path: Option<String>,
    
    /// Tool call ID (for matching responses)
    pub tool_call_id: String,
}

impl ToolContext {
    /// Verify the tool has a required capability
    pub fn verify_capability(&self, capability: SkillCapability) -> Result<()>;
    
    /// Check if tool has a capability without failing
    pub fn has_capability(&self, capability: SkillCapability) -> bool;
}
```

---

## `ToolRegistry`

Registry for managing and invoking tools.

```rust
pub struct ToolRegistry {
    tools: DashMap<String, Arc<dyn Tool>>,
    capability_checker: Arc<dyn CapabilityChecker>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new(capability_checker: Arc<dyn CapabilityChecker>) -> Self;
    
    /// Register a tool
    pub fn register(&self, tool: Arc<dyn Tool>) -> Result<()>;
    
    /// Get a tool by name
    pub fn get(&self, name: &str) -> Result<Arc<dyn Tool>>;
    
    /// List all registered tools
    pub fn list_all(&self) -> Vec<Arc<dyn Tool>>;
    
    /// Get tool definitions for LLM
    pub fn definitions(&self) -> Vec<ToolDefinition>;
    
    /// Execute a tool by name
    pub async fn execute(
        &self,
        name: &str,
        input: Value,
        ctx: &ToolContext,
    ) -> Result<ToolOutput>;
}
```

### Usage Example

```rust
use openrustclaw_agent::ToolRegistry;

// Create registry
let registry = ToolRegistry::new(capability_checker);

// Register tools
registry.register(Arc::new(FileReadTool))?;
registry.register(Arc::new(FileWriteTool))?;
registry.register(Arc::new(MemorySearchTool::new(memory_store)))?;

// Execute tool
let output = registry.execute(
    "file_read",
    json!({"path": "README.md"}),
    &ctx,
).await?;

println!("Result: {}", output.content);
```

---

## `ToolFactory`

Factory for creating tool instances with dependencies.

```rust
pub trait ToolFactory: Send + Sync {
    /// Create a tool instance
    fn create(&self, deps: &ToolDependencies) -> Arc<dyn Tool>;
    
    /// Tool name
    fn name(&self) -> &str;
}

pub struct ToolDependencies {
    pub memory_store: Option<Arc<dyn MemoryStore>>,
    pub core_memory: Option<Arc<dyn CoreMemoryStore>>,
    pub provider: Option<Arc<dyn LlmProvider>>,
    // ... other common dependencies
}
```

---

## Tool Error Types

```rust
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Tool execution failed: {tool}: {message}")]
    ExecutionFailed { tool: String, message: String },

    #[error("Tool input validation error: {tool}: {message}")]
    InputValidation { tool: String, message: String },

    #[error("Tool capability not granted: {tool} requires {capability}")]
    CapabilityDenied { tool: String, capability: String },

    #[error("Tool sandbox violation: {0}")]
    SandboxViolation(String),

    #[error("Tool timeout: {tool} exceeded {timeout_ms}ms")]
    Timeout { tool: String, timeout_ms: u64 },
}
```

---

## Best Practices

### 1. Validate Input Thoroughly

```rust
async fn execute(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
    // Validate required fields
    let required_field = input["required_field"].as_str()
        .ok_or_else(|| Error::InputValidation {
            tool: self.name().into(),
            message: "required_field is required".into(),
        })?;
    
    // Validate types
    let count = input["count"].as_u64()
        .ok_or_else(|| Error::InputValidation {
            tool: self.name().into(),
            message: "count must be a positive integer".into(),
        })?;
    
    // Validate ranges
    if count > 1000 {
        return Err(Error::InputValidation {
            tool: self.name().into(),
            message: "count must be <= 1000".into(),
        });
    }
    
    // ... execute
}
```

### 2. Handle Errors Gracefully

```rust
async fn execute(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
    match self.do_work().await {
        Ok(result) => Ok(ToolOutput {
            tool_call_id: ctx.tool_call_id.clone(),
            content: serde_json::to_string(&result)?,
            is_error: false,
        }),
        Err(e) => Ok(ToolOutput {
            tool_call_id: ctx.tool_call_id.clone(),
            content: format!("Error: {}", e),
            is_error: true,  // Mark as error but don't fail
        }),
    }
}
```

### 3. Use Appropriate Capabilities

```rust
fn capabilities_required(&self) -> Vec<SkillCapability> {
    // Only request what you need
    vec![
        SkillCapability::FileRead,
        // NOT FileWrite if you only read
        // NOT NetworkAccess if you don't make requests
    ]
}
```

### 4. Provide Clear Descriptions

```rust
fn description(&self) -> &str {
    // Good: specific and actionable
    "Search the memory system for relevant information based on a query"
    
    // Bad: vague
    "Does a search"
}
```

### 5. Include Examples in Schema

```rust
fn schema(&self) -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Search query",
                // Include example
                "examples": ["deployment process", "user preferences"]
            }
        }
    })
}
```
