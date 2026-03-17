# Agent Runtime API Reference

This reference documents the agent runtime and execution for OpenRustClaw.

**Crate**: `openrustclaw-agent`

---

## Overview

The agent runtime orchestrates message handling:
1. Receives a message
2. Builds context with core memory
3. Calls the LLM provider
4. Processes tool calls
5. Returns the final response

---

## AgentRuntime

The main agent runtime that orchestrates message handling.

```rust
pub struct AgentRuntime {
    provider: Arc<dyn LlmProvider>,
    tool_registry: Arc<ToolRegistry>,
    agent_name: String,
    max_tool_iterations: usize,
    memory_store: Option<Arc<dyn MemoryStore>>,
    core_memory_store: Option<Arc<dyn CoreMemoryStore>>,
}

pub struct AgentResponse {
    pub message: Message,
    pub usage: TokenUsage,
    pub tool_calls_made: usize,
}
```

### Constructors

```rust
impl AgentRuntime {
    /// Create a new runtime with provider and tool registry
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        tool_registry: Arc<ToolRegistry>,
        agent_name: String,
    ) -> Self;
    
    /// Create a runtime with memory stores pre-configured
    pub fn with_memory_stores(
        provider: Arc<dyn LlmProvider>,
        agent_name: String,
        memory_store: Arc<dyn MemoryStore>,
        core_memory_store: Arc<dyn CoreMemoryStore>,
    ) -> Self;
    
    /// Set maximum tool iterations (default: 10, 0 = disable tools)
    pub fn with_max_tool_iterations(mut self, max: usize) -> Self;
}
```

### Methods

| Method | Description |
|--------|-------------|
| `process(messages, core_memory, session_id, user_id)` | Process conversation and return response |
| `memory_store()` | Get memory store reference (if configured) |
| `core_memory_store()` | Get core memory store reference (if configured) |
| `tool_registry()` | Get tool registry reference |

**Example**:
```rust
use openrustclaw_agent::AgentRuntime;
use openrustclaw_core::types::{Message, Platform, Session};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Create provider and tool registry
    let provider: Arc<dyn LlmProvider> = // ... initialize
    let tool_registry = Arc::new(ToolRegistry::new());
    
    // Create runtime
    let runtime = AgentRuntime::new(
        provider,
        tool_registry,
        "MyAssistant".to_string(),
    );
    
    // Create session
    let session = Session::new_dm("user_42", Platform::WebChat);
    
    // Process message
    let messages = vec![Message::user("What is the weather?")];
    let core_memory = vec![];
    
    let response = runtime.process(
        &messages,
        &core_memory,
        &session.id.to_string(),
        &session.user_id,
    ).await?;
    
    println!("Response: {}", response.message.content);
    println!("Tool calls made: {}", response.tool_calls_made);
    println!("Tokens used: {}", response.usage.total_tokens);
    
    Ok(())
}
```

---

## ToolRegistry

Registry of available tools for the agent.

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}
```

### Methods

| Method | Description |
|--------|-------------|
| `ToolRegistry::new()` | Create empty registry |
| `ToolRegistry::with_memory_tools(memory, core)` | Create with memory tools pre-registered |
| `register(tool)` | Register a tool |
| `definitions()` | Get tool definitions for LLM |
| `execute(call, ctx)` | Execute a tool call |
| `contains(name)` | Check if tool is registered |
| `get(name)` | Get tool by name |
| `len()` / `is_empty()` | Registry size checks |

**Example**:
```rust
use openrustclaw_agent::ToolRegistry;
use openrustclaw_core::traits::Tool;

let mut registry = ToolRegistry::new();

// Register custom tool
registry.register(Arc::new(MyCustomTool::new()));

// Check registration
assert!(registry.contains("my_tool"));

// Get tool definitions for LLM
let definitions = registry.definitions();

// Execute tool call
let output = registry.execute(&tool_call, &tool_context).await?;
```

---

## Tool Trait

The trait for executable tools.

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique tool name (e.g., "memory_search")
    fn name(&self) -> &str;
    
    /// Human-readable description for the LLM
    fn description(&self) -> &str;
    
    /// JSON Schema for input parameters
    fn schema(&self) -> Value;
    
    /// Capabilities this tool requires
    fn capabilities_required(&self) -> Vec<SkillCapability>;
    
    /// Execute the tool
    async fn execute(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput>;
}
```

### ToolContext

Context provided to tools during execution.

```rust
pub struct ToolContext {
    pub session_id: String,
    pub user_id: String,
    pub workspace_path: Option<String>,
}
```

**Example - Custom Tool**:
```rust
use async_trait::async_trait;
use openrustclaw_core::traits::{Tool, ToolContext};
use openrustclaw_core::types::{ToolOutput, SkillCapability};
use serde_json::{Value, json};

pub struct WeatherTool {
    api_key: String,
}

impl WeatherTool {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[async_trait]
impl Tool for WeatherTool {
    fn name(&self) -> &str {
        "get_weather"
    }
    
    fn description(&self) -> &str {
        "Get the current weather for a location"
    }
    
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name or coordinates"
                }
            },
            "required": ["location"]
        })
    }
    
    fn capabilities_required(&self) -> Vec<SkillCapability> {
        vec![SkillCapability::NetworkAccess]
    }
    
    async fn execute(&self, input: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let location = input["location"].as_str()
            .ok_or_else(|| Error::Tool(ToolError::InputValidation {
                tool: "get_weather".to_string(),
                message: "location is required".to_string(),
            }))?;
        
        // Call weather API...
        let weather = fetch_weather(&self.api_key, location).await?;
        
        Ok(ToolOutput {
            tool_call_id: String::new(),  // Set by registry
            content: format!("Weather in {}: {}", location, weather),
            is_error: false,
        })
    }
}
```

---

## ToolFactory

Factory for creating tools with their dependencies injected.

```rust
pub struct ToolFactory {
    memory_store: Arc<dyn MemoryStore>,
    core_memory_store: Arc<dyn CoreMemoryStore>,
}
```

### Methods

| Method | Description |
|--------|-------------|
| `ToolFactory::new(memory, core)` | Create with storage backends |
| `create_memory_search_tool()` | Create MemorySearchTool |
| `create_memory_store_tool()` | Create MemoryStoreTool |
| `create_core_memory_update_tool()` | Create CoreMemoryUpdateTool |
| `register_all(registry)` | Register all memory tools |

**Example**:
```rust
use openrustclaw_agent::ToolFactory;

let factory = ToolFactory::new(memory_store, core_memory_store);

// Create individual tools
let search_tool = factory.create_memory_search_tool();
let store_tool = factory.create_memory_store_tool();

// Or register all at once
let mut registry = ToolRegistry::new();
factory.register_all(&mut registry);
assert_eq!(registry.len(), 3);  // search, store, core_update
```

---

## Memory Tools

Built-in tools for memory operations.

### MemorySearchTool

Searches recall memory.

```rust
pub struct MemorySearchTool {
    memory_store: Arc<dyn MemoryStore>,
}

impl MemorySearchTool {
    pub fn new(memory_store: Arc<dyn MemoryStore>) -> Self;
}
```

**Schema**:
```json
{
  "type": "object",
  "properties": {
    "query": {"type": "string", "description": "Search query"},
    "limit": {"type": "integer", "default": 5},
    "memory_type": {"type": "string", "enum": ["episodic", "semantic", "procedural"]}
  },
  "required": ["query"]
}
```

### MemoryStoreTool

Stores new memories.

```rust
pub struct MemoryStoreTool {
    memory_store: Arc<dyn MemoryStore>,
}

impl MemoryStoreTool {
    pub fn new(memory_store: Arc<dyn MemoryStore>) -> Self;
}
```

**Schema**:
```json
{
  "type": "object",
  "properties": {
    "content": {"type": "string", "description": "Content to remember"},
    "memory_type": {"type": "string", "enum": ["episodic", "semantic", "procedural"]},
    "importance": {"type": "number", "minimum": 0, "maximum": 1}
  },
  "required": ["content", "memory_type"]
}
```

### CoreMemoryUpdateTool

Updates core memory.

```rust
pub struct CoreMemoryUpdateTool {
    core_memory_store: Arc<dyn CoreMemoryStore>,
}

impl CoreMemoryUpdateTool {
    pub fn new(core_memory_store: Arc<dyn CoreMemoryStore>) -> Self;
}
```

**Schema**:
```json
{
  "type": "object",
  "properties": {
    "key": {"type": "string", "description": "Memory key"},
    "value": {"type": "string", "description": "Memory value"},
    "importance": {"type": "number", "minimum": 0, "maximum": 1}
  },
  "required": ["key", "value"]
}
```

---

## Streaming

The runtime supports streaming responses.

```rust
use openrustclaw_agent::streaming::StreamingProcessor;
use futures::StreamExt;

async fn stream_response(runtime: &AgentRuntime) -> Result<()> {
    let stream = runtime.stream(&messages, &core_memory, session_id, user_id).await?;
    
    let mut processor = StreamingProcessor::new();
    
    processor.process(stream, |chunk| {
        match chunk {
            StreamChunk::ContentDelta { delta } => {
                print!("{}", delta);
                std::io::stdout().flush()?;
            }
            StreamChunk::ToolCallDelta { id, name, arguments_delta } => {
                println!("\n[Tool: {:?} - {}]", name, id);
            }
            StreamChunk::Done { response } => {
                println!("\n[Done: {} tokens]", response.usage.total_tokens);
            }
        }
        Ok(())
    }).await?;
    
    Ok(())
}
```

---

## Configuration

### Runtime Configuration

```toml
[agent]
name = "MyAssistant"
max_tool_iterations = 10
temperature = 0.7
max_tokens = 4096

[agent.system_prompt]
base = "You are a helpful AI assistant."
include_tools = true
include_core_memory = true
```

---

## Complete Example

```rust
use openrustclaw_agent::{AgentRuntime, ToolRegistry, ToolFactory};
use openrustclaw_providers::AnthropicProvider;
use openrustclaw_core::types::{Message, Session, Platform, CoreEntry};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize provider
    let provider = Arc::new(AnthropicProvider::new(
        std::env::var("ANTHROPIC_API_KEY")?,
        "claude-sonnet-4-20250514".to_string(),
    ));
    
    // Initialize memory stores (implementations omitted)
    let memory_store: Arc<dyn MemoryStore> = // ...
    let core_memory_store: Arc<dyn CoreMemoryStore> = // ...
    
    // Create runtime with memory
    let runtime = AgentRuntime::with_memory_stores(
        provider,
        "MyAssistant".to_string(),
        memory_store.clone(),
        core_memory_store.clone(),
    );
    
    // Create session with core memory
    let session = Session::new_dm("user_42", Platform::Cli);
    let core_memory = vec![
        CoreEntry {
            key: "name".to_string(),
            value: "Alice".to_string(),
            importance: 0.9,
            token_count: 3,
            updated_at: Utc::now(),
        },
    ];
    
    // Conversation loop
    let mut messages: Vec<Message> = vec![];
    
    loop {
        let input = read_line()?;
        if input == "/quit" {
            break;
        }
        
        messages.push(Message::user(input));
        
        let response = runtime.process(
            &messages,
            &core_memory,
            &session.id.to_string(),
            &session.user_id,
        ).await?;
        
        println!("Assistant: {}", response.message.content);
        
        if response.tool_calls_made > 0 {
            println!("[Used {} tool(s)]", response.tool_calls_made);
        }
        
        messages.push(response.message);
    }
    
    Ok(())
}
```

---

## Error Handling

```rust
use openrustclaw_core::error::{Error, ToolError};

match runtime.process(&messages, &core_memory, session_id, user_id).await {
    Ok(response) => response,
    Err(Error::Tool(ToolError::NotFound(name))) => {
        eprintln!("Tool not found: {}", name);
    }
    Err(Error::Tool(ToolError::CapabilityDenied { tool, capability })) => {
        eprintln!("Tool {} requires {} capability", tool, capability);
    }
    Err(Error::Tool(ToolError::Timeout { tool, timeout_ms })) => {
        eprintln!("Tool {} timed out after {}ms", tool, timeout_ms);
    }
    Err(e) => return Err(e),
}
```
