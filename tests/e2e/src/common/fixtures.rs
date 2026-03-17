//! Test fixtures for E2E tests.
//!
//! Provides consistent test data and scenarios for all E2E test types.

use openrustclaw_core::types::*;
use serde_json::json;
use uuid::Uuid;

/// Standard test users
pub struct TestUsers;

impl TestUsers {
    pub fn alice() -> TestUser {
        TestUser {
            id: "user-alice-001".to_string(),
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        }
    }

    pub fn bob() -> TestUser {
        TestUser {
            id: "user-bob-002".to_string(),
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
        }
    }

    pub fn admin() -> TestUser {
        TestUser {
            id: "user-admin-000".to_string(),
            name: "Admin".to_string(),
            email: "admin@example.com".to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TestUser {
    pub id: String,
    pub name: String,
    pub email: String,
}

/// Standard test sessions
pub struct TestSessions;

impl TestSessions {
    pub fn create() -> Session {
        Session {
            id: Uuid::new_v4(),
            user_id: TestUsers::alice().id,
            created_at: chrono::Utc::now(),
            last_active: chrono::Utc::now(),
            metadata: json!({}),
        }
    }

    pub fn for_user(user: &TestUser) -> Session {
        Session {
            id: Uuid::new_v4(),
            user_id: user.id.clone(),
            created_at: chrono::Utc::now(),
            last_active: chrono::Utc::now(),
            metadata: json!({"test": true}),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Session {
    pub id: Uuid,
    pub user_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_active: chrono::DateTime<chrono::Utc>,
    pub metadata: serde_json::Value,
}

/// Standard test messages
pub struct TestMessages;

impl TestMessages {
    pub fn greeting() -> Message {
        Message::user("Hello, how can you help me today?")
    }

    pub fn with_tool_request() -> Message {
        Message::user("Please calculate 2 + 2")
    }

    pub fn with_context(user_name: &str) -> Message {
        Message::user(format!("What do you know about me, {}?", user_name))
    }

    pub fn complex_request() -> Message {
        Message::user("Search my memories about work, then summarize what projects I'm working on")
    }

    pub fn assistant_response(content: impl Into<String>) -> Message {
        Message::assistant(content)
    }

    pub fn tool_result(tool_call_id: &str, content: impl Into<String>) -> Message {
        Message::tool(tool_call_id, content)
    }
}

/// Standard test memory entries
pub struct TestMemories;

impl TestMemories {
    pub fn work_project() -> MemoryEntry {
        MemoryEntry {
            id: Uuid::new_v4(),
            memory_type: MemoryType::Semantic,
            content: "Working on OpenRustClaw v2.0 with E2E testing framework".to_string(),
            content_hash: "abc123".to_string(),
            source: Some("user_statement".to_string()),
            source_type: None,
            session_id: None,
            user_id: Some(TestUsers::alice().id),
            namespace: "work".to_string(),
            importance: 0.8,
            confidence: 1.0,
            access_count: 0,
            last_accessed: None,
            created_at: chrono::Utc::now(),
            expires_at: None,
            metadata: json!({"project": "OpenRustClaw"}),
        }
    }

    pub fn personal_fact() -> MemoryEntry {
        MemoryEntry {
            id: Uuid::new_v4(),
            memory_type: MemoryType::Semantic,
            content: "Alice prefers Rust over Python for systems programming".to_string(),
            content_hash: "def456".to_string(),
            source: Some("user_statement".to_string()),
            source_type: None,
            session_id: None,
            user_id: Some(TestUsers::alice().id),
            namespace: "preferences".to_string(),
            importance: 0.6,
            confidence: 0.95,
            access_count: 0,
            last_accessed: None,
            created_at: chrono::Utc::now(),
            expires_at: None,
            metadata: json!({}),
        }
    }

    pub fn ephemeral_fact() -> MemoryEntry {
        MemoryEntry {
            id: Uuid::new_v4(),
            memory_type: MemoryType::Episodic,
            content: "Current temperature is 72°F".to_string(),
            content_hash: "ghi789".to_string(),
            source: Some("tool_result".to_string()),
            source_type: None,
            session_id: None,
            user_id: Some(TestUsers::alice().id),
            namespace: "weather".to_string(),
            importance: 0.3,
            confidence: 1.0,
            access_count: 0,
            last_accessed: None,
            created_at: chrono::Utc::now(),
            expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
            metadata: json!({"ttl_hours": 1}),
        }
    }

    pub fn core_preference() -> CoreEntry {
        CoreEntry {
            key: "name".to_string(),
            value: "Alice".to_string(),
            importance: 1.0,
            token_count: 2,
            updated_at: chrono::Utc::now(),
        }
    }

    pub fn core_goal() -> CoreEntry {
        CoreEntry {
            key: "goal".to_string(),
            value: "Build reliable AI systems".to_string(),
            importance: 0.9,
            token_count: 5,
            updated_at: chrono::Utc::now(),
        }
    }
}

/// Standard completion requests
pub struct TestRequests;

impl TestRequests {
    pub fn simple_chat() -> CompletionRequest {
        CompletionRequest {
            messages: vec![TestMessages::greeting()],
            model: Some("mock-model".to_string()),
            max_tokens: Some(100),
            temperature: Some(0.7),
            tools: None,
            system_prompt: None,
            stream: false,
        }
    }

    pub fn with_tools() -> CompletionRequest {
        CompletionRequest {
            messages: vec![TestMessages::with_tool_request()],
            model: Some("mock-model".to_string()),
            max_tokens: Some(100),
            temperature: Some(0.7),
            tools: Some(vec![ToolDefinition {
                name: "calculator".to_string(),
                description: "Performs calculations".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "expression": {"type": "string"}
                    },
                    "required": ["expression"]
                }),
                strict: false,
            }]),
            system_prompt: None,
            stream: false,
        }
    }

    pub fn streaming() -> CompletionRequest {
        CompletionRequest {
            messages: vec![TestMessages::greeting()],
            model: Some("mock-model".to_string()),
            max_tokens: Some(100),
            temperature: Some(0.7),
            tools: None,
            system_prompt: None,
            stream: true,
        }
    }
}

/// Standard tool definitions
pub struct TestTools;

impl TestTools {
    pub fn calculator() -> ToolDefinition {
        ToolDefinition {
            name: "calculator".to_string(),
            description: "Perform arithmetic operations".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["add", "subtract", "multiply", "divide"]
                    },
                    "a": {"type": "number"},
                    "b": {"type": "number"}
                },
                "required": ["operation", "a", "b"]
            }),
            strict: false,
        }
    }

    pub fn search() -> ToolDefinition {
        ToolDefinition {
            name: "search_memories".to_string(),
            description: "Search user memories".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer", "default": 5}
                },
                "required": ["query"]
            }),
            strict: false,
        }
    }

    pub fn weather() -> ToolDefinition {
        ToolDefinition {
            name: "get_weather".to_string(),
            description: "Get weather for a location".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "location": {"type": "string"},
                    "units": {"type": "string", "enum": ["celsius", "fahrenheit"], "default": "celsius"}
                },
                "required": ["location"]
            }),
            strict: false,
        }
    }
}

/// Test scenarios for horizontal tests
pub struct TestScenarios;

impl TestScenarios {
    /// Complete chat session with context
    pub fn chat_with_memory() -> Scenario {
        Scenario {
            name: "Chat with Memory Context".to_string(),
            steps: vec![
                ScenarioStep::StoreMemory {
                    content: "User is working on OpenRustClaw project".to_string(),
                },
                ScenarioStep::SendMessage {
                    content: "What am I working on?".to_string(),
                },
                ScenarioStep::ExpectResponse {
                    contains: "OpenRustClaw".to_string(),
                },
            ],
        }
    }

    /// Multi-turn conversation with tool calling
    pub fn multi_turn_with_tools() -> Scenario {
        Scenario {
            name: "Multi-turn Tool Conversation".to_string(),
            steps: vec![
                ScenarioStep::SendMessage {
                    content: "Calculate 10 * 5".to_string(),
                },
                ScenarioStep::ExpectToolCall {
                    tool: "calculator".to_string(),
                },
                ScenarioStep::SendToolResult {
                    result: "50".to_string(),
                },
                ScenarioStep::ExpectResponse {
                    contains: "50".to_string(),
                },
            ],
        }
    }

    /// Session isolation test
    pub fn session_isolation() -> Scenario {
        Scenario {
            name: "Session Isolation".to_string(),
            steps: vec![
                ScenarioStep::CreateSession {
                    user_id: TestUsers::alice().id,
                },
                ScenarioStep::StoreMemory {
                    content: "Alice's secret: loves Rust".to_string(),
                },
                ScenarioStep::CreateSession {
                    user_id: TestUsers::bob().id,
                },
                ScenarioStep::SendMessage {
                    content: "What do you know about me?".to_string(),
                },
                ScenarioStep::ExpectResponse {
                    contains: "nothing".to_string(),
                },
            ],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Scenario {
    pub name: String,
    pub steps: Vec<ScenarioStep>,
}

#[derive(Clone, Debug)]
pub enum ScenarioStep {
    StoreMemory { content: String },
    SendMessage { content: String },
    ExpectResponse { contains: String },
    ExpectToolCall { tool: String },
    SendToolResult { result: String },
    CreateSession { user_id: String },
}

/// Health check expectations
pub struct HealthExpectations;

impl HealthExpectations {
    pub fn all_healthy() -> serde_json::Value {
        json!({
            "status": "healthy",
            "components": {
                "database": "ok",
                "gateway": "ok",
                "memory": "ok",
                "providers": "ok"
            }
        })
    }

    pub fn degraded(component: &str) -> serde_json::Value {
        json!({
            "status": "degraded",
            "components": {
                component: "degraded"
            }
        })
    }
}
