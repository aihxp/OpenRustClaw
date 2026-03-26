//! Shared utilities for OpenRustClaw E2E tests.
//!
//! This module provides test environment setup, database initialization,
//! mock providers, and helper functions for end-to-end testing.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use futures::Stream;
use openrustclaw_core::error::{Error, ProviderError, Result};
use openrustclaw_core::traits::{LlmProvider, Tool, ToolContext};
use openrustclaw_core::types::{
    CompletionRequest, CompletionResponse, CoreEntry, FinishReason, MemoryEntry, MemoryQuery,
    MemorySource, MemoryType, Message, Role, SkillCapability, StreamChunk, TokenUsage, ToolCall,
    ToolFormat, ToolOutput,
};
use openrustclaw_db::core_memory_store::SqliteCoreMemoryStore;
use openrustclaw_db::memory_store::SqliteMemoryStore;
use openrustclaw_gateway::metrics_endpoint::{install_metrics, metrics_middleware, metrics_routes};
use openrustclaw_gateway::server::{GatewayServer, GatewayState};
use openrustclaw_gateway::sessions::SessionManager;
use openrustclaw_memory::context::ContextManager;
use openrustclaw_memory::core_memory::CoreMemoryManager;
use openrustclaw_memory::policies::MemoryPolicies;
use openrustclaw_memory::recall::RecallMemory;
use openrustclaw_security::OriginValidator;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;
use wiremock::MockServer;

pub mod assertions;
pub mod fixtures;
pub mod http_client;

pub use assertions::E2eAssertions;
pub use fixtures::*;
pub use http_client::{ResponseExt, TestError, TestHttpClient, TestWebSocketClient};

/// Initialize tracing subscriber for tests.
pub fn init_test_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap())
                .add_directive("openrustclaw=debug".parse().unwrap()),
        )
        .with_test_writer()
        .try_init();
}

/// Check if live provider tests should run.
pub fn use_live_providers() -> bool {
    std::env::var("E2E_LIVE").is_ok_and(|v| v == "1" || v == "true")
}

/// Check if a specific provider API key is available.
pub fn has_api_key(provider: &str) -> bool {
    let key = format!("{}_API_KEY", provider.to_uppercase());
    std::env::var(&key).is_ok()
}

// ═════════════════════════════════════════════════════════════════════════════
// Test Environment
// ═════════════════════════════════════════════════════════════════════════════

/// Complete test environment for E2E tests.
pub struct TestEnvironment {
    pub db_pool: SqlitePool,
    pub _temp_dir: tempfile::TempDir,
    pub memory_store: SqliteMemoryStore,
    pub core_memory_store: SqliteCoreMemoryStore,
    pub recall_memory: Arc<RecallMemory>,
    pub core_memory: Arc<CoreMemoryManager>,
    pub context_manager: Arc<ContextManager>,
    pub session_manager: Arc<SessionManager>,
    pub mock_server: Option<MockServer>,
}

impl TestEnvironment {
    /// Create a new test environment with an in-memory database.
    pub async fn new() -> Self {
        init_test_tracing();

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(10)
            .connect(&db_url)
            .await
            .expect("Failed to create test database pool");

        // Run migrations
        openrustclaw_db::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        // Create memory stores
        let memory_store = SqliteMemoryStore::new(pool.clone());
        let core_memory_store = SqliteCoreMemoryStore::new(pool.clone());

        // Create memory managers
        let policies = MemoryPolicies::default();
        let recall_memory = Arc::new(RecallMemory::new(policies));
        let core_memory = Arc::new(CoreMemoryManager::new(500, 20));

        // Create context manager
        let context_manager = Arc::new(ContextManager::new(8192));

        let session_manager = Arc::new(SessionManager::new());

        Self {
            db_pool: pool,
            _temp_dir: temp_dir,
            memory_store,
            core_memory_store,
            recall_memory,
            core_memory,
            context_manager,
            session_manager,
            mock_server: None,
        }
    }

    /// Create a new test environment with a mock HTTP server.
    pub async fn with_mock_server() -> (Self, MockServer) {
        let mut env = Self::new().await;
        let mock_server = MockServer::start().await;
        env.mock_server = Some(MockServer::start().await);
        (env, mock_server)
    }

    /// Create a gateway state for testing.
    pub fn create_gateway_state(&self, require_auth: bool) -> GatewayState {
        GatewayState {
            session_manager: self.session_manager.clone(),
            origin_validator: Arc::new(OriginValidator::new(vec![
                "http://localhost:3000".to_string(),
                "http://127.0.0.1:3000".to_string(),
            ])),
            require_auth,
            internal_api_token: None,
            trusted_proxy_token: None,
            memory_store: Some(Arc::new(self.memory_store.clone())),
            core_memory_store: Some(Arc::new(self.core_memory_store.clone())),
            rag_store: None,
            langsmith: None,
        }
    }

    /// Start a test gateway server.
    pub async fn start_gateway(&self, require_auth: bool) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind to random port");
        let port = listener.local_addr().unwrap().port();
        let addr = format!("127.0.0.1:{}", port);

        let gateway = GatewayServer::new("127.0.0.1".to_string(), port);
        let state = self.create_gateway_state(require_auth);
        let metrics_handle = install_metrics();
        let app = gateway
            .router(state)
            .merge(metrics_routes(metrics_handle))
            .layer(metrics_middleware());

        let handle = tokio::spawn(async move {
            axum::serve(
                tokio::net::TcpListener::from_std(listener.into_std().unwrap()).unwrap(),
                app,
            )
            .await
            .expect("Server failed");
        });

        // Wait for server to be ready
        tokio::time::sleep(Duration::from_millis(100)).await;

        (addr, handle)
    }

    /// Store a test memory entry.
    pub async fn store_memory(&self, entry: MemoryEntry) -> Result<()> {
        use openrustclaw_core::traits::MemoryStore;
        self.memory_store.store(entry).await
    }

    /// Search memories.
    pub async fn search_memories(
        &self,
        query: MemoryQuery,
    ) -> Result<Vec<openrustclaw_core::types::ScoredMemory>> {
        use openrustclaw_core::traits::MemoryStore;
        self.memory_store.search(&query).await
    }

    /// Store a core memory entry.
    pub async fn store_core_memory(&self, user_id: &str, entry: CoreEntry) -> Result<()> {
        use openrustclaw_core::traits::CoreMemoryStore;
        self.core_memory_store.set(user_id, entry).await
    }

    /// Get core memory for a user.
    pub async fn get_core_memory(&self, user_id: &str) -> Result<Vec<CoreEntry>> {
        use openrustclaw_core::traits::CoreMemoryStore;
        self.core_memory_store.get_all(user_id).await
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Mock Provider Implementations
// ═════════════════════════════════════════════════════════════════════════════

/// A mock provider that always succeeds with a configurable response.
pub struct MockSuccessProvider {
    name: String,
    response_content: String,
    tool_calls: Option<Vec<ToolCall>>,
}

impl MockSuccessProvider {
    pub fn new(name: impl Into<String>, response_content: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            response_content: response_content.into(),
            tool_calls: None,
        }
    }

    pub fn with_tool_calls(mut self, tool_calls: Vec<ToolCall>) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }
}

#[async_trait]
impl LlmProvider for MockSuccessProvider {
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
        let mut message = Message::assistant(&self.response_content);
        message.tool_calls = self.tool_calls.clone();

        Ok(CompletionResponse {
            id: format!("{}-response", self.name),
            message,
            model: "mock-model".to_string(),
            usage: TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 10,
                total_tokens: 20,
                cost_usd: None,
            },
            provider: self.name.clone(),
            finish_reason: if self.tool_calls.is_some() {
                FinishReason::ToolUse
            } else {
                FinishReason::Stop
            },
        })
    }

    async fn stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        use futures::stream;

        let content = self.response_content.clone();
        let name = self.name.clone();

        let s = stream::unfold(0, move |state| {
            let content = content.clone();
            let name = name.clone();
            async move {
                match state {
                    0 => {
                        let chunk = StreamChunk::ContentDelta {
                            delta: content.clone(),
                        };
                        Some((Ok(chunk), 1))
                    }
                    1 => {
                        let response = CompletionResponse {
                            id: format!("{}-stream-response", name),
                            message: Message::assistant(content),
                            model: "mock-model".to_string(),
                            usage: TokenUsage::default(),
                            provider: name,
                            finish_reason: FinishReason::Stop,
                        };
                        let chunk = StreamChunk::Done { response };
                        Some((Ok(chunk), 2))
                    }
                    _ => None,
                }
            }
        });

        Ok(Box::pin(s))
    }

    fn model_id(&self) -> &str {
        "mock-model"
    }

    fn max_tokens(&self) -> usize {
        1000
    }

    fn provider_name(&self) -> &str {
        &self.name
    }

    fn supports_strict_tools(&self) -> bool {
        true
    }

    fn supports_streaming_tool_deltas(&self) -> bool {
        false
    }

    fn native_tool_format(&self) -> ToolFormat {
        ToolFormat::OpenAi
    }
}

/// A mock provider that returns a rate limit error.
pub struct MockRateLimitedProvider {
    name: String,
    retry_after_secs: Option<u64>,
}

impl MockRateLimitedProvider {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            retry_after_secs: Some(60),
        }
    }

    pub fn with_retry_after(mut self, secs: u64) -> Self {
        self.retry_after_secs = Some(secs);
        self
    }
}

#[async_trait]
impl LlmProvider for MockRateLimitedProvider {
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
        Err(Error::Provider(ProviderError::RateLimited {
            provider: self.name.clone(),
            retry_after_secs: self.retry_after_secs,
        }))
    }

    async fn stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        Err(Error::Provider(ProviderError::RateLimited {
            provider: self.name.clone(),
            retry_after_secs: self.retry_after_secs,
        }))
    }

    fn model_id(&self) -> &str {
        "mock-model"
    }

    fn max_tokens(&self) -> usize {
        1000
    }

    fn provider_name(&self) -> &str {
        &self.name
    }

    fn supports_strict_tools(&self) -> bool {
        false
    }

    fn supports_streaming_tool_deltas(&self) -> bool {
        false
    }

    fn native_tool_format(&self) -> ToolFormat {
        ToolFormat::OpenAi
    }
}

/// A mock provider that returns an unavailable error.
pub struct MockUnavailableProvider {
    name: String,
    message: String,
}

impl MockUnavailableProvider {
    pub fn new(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            message: message.into(),
        }
    }
}

#[async_trait]
impl LlmProvider for MockUnavailableProvider {
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
        Err(Error::Provider(ProviderError::Unavailable {
            provider: self.name.clone(),
            message: self.message.clone(),
        }))
    }

    async fn stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        Err(Error::Provider(ProviderError::Unavailable {
            provider: self.name.clone(),
            message: self.message.clone(),
        }))
    }

    fn model_id(&self) -> &str {
        "mock-model"
    }

    fn max_tokens(&self) -> usize {
        1000
    }

    fn provider_name(&self) -> &str {
        &self.name
    }

    fn supports_strict_tools(&self) -> bool {
        false
    }

    fn supports_streaming_tool_deltas(&self) -> bool {
        false
    }

    fn native_tool_format(&self) -> ToolFormat {
        ToolFormat::OpenAi
    }
}

/// A mock provider that simulates multi-turn conversation.
pub struct MockConversationalProvider {
    name: String,
    responses: Arc<Mutex<Vec<String>>>,
    call_count: Arc<Mutex<usize>>,
}

impl MockConversationalProvider {
    pub fn new(name: impl Into<String>, responses: Vec<String>) -> Self {
        Self {
            name: name.into(),
            responses: Arc::new(Mutex::new(responses)),
            call_count: Arc::new(Mutex::new(0)),
        }
    }
}

#[async_trait]
impl LlmProvider for MockConversationalProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let mut count = self.call_count.lock().await;
        let responses = self.responses.lock().await;

        let response_text = if *count < responses.len() {
            responses[*count].clone()
        } else {
            "I have nothing more to say.".to_string()
        };

        *count += 1;

        // Check if there are tool results in the messages
        let has_tool_results = request.messages.iter().any(|m| m.role == Role::Tool);

        Ok(CompletionResponse {
            id: format!("{}-response-{}", self.name, count),
            message: Message::assistant(response_text),
            model: "mock-model".to_string(),
            usage: TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 10,
                total_tokens: 20,
                cost_usd: None,
            },
            provider: self.name.clone(),
            finish_reason: if has_tool_results {
                FinishReason::Stop
            } else {
                FinishReason::ToolUse
            },
        })
    }

    async fn stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        Err(Error::Provider(ProviderError::StreamError {
            provider: self.name.clone(),
            message: "Not implemented".to_string(),
        }))
    }

    fn model_id(&self) -> &str {
        "mock-model"
    }

    fn max_tokens(&self) -> usize {
        1000
    }

    fn provider_name(&self) -> &str {
        &self.name
    }

    fn supports_strict_tools(&self) -> bool {
        true
    }

    fn supports_streaming_tool_deltas(&self) -> bool {
        false
    }

    fn native_tool_format(&self) -> ToolFormat {
        ToolFormat::OpenAi
    }
}

/// A mock provider that simulates a tool-calling flow.
pub struct MockToolCallingProvider {
    name: String,
    tool_to_call: String,
    tool_args: Value,
    final_response: String,
}

impl MockToolCallingProvider {
    pub fn new(
        name: impl Into<String>,
        tool_to_call: impl Into<String>,
        tool_args: Value,
        final_response: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            tool_to_call: tool_to_call.into(),
            tool_args,
            final_response: final_response.into(),
        }
    }
}

#[async_trait]
impl LlmProvider for MockToolCallingProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        // Check if we've already seen tool results (second turn)
        let has_tool_results = request.messages.iter().any(|m| m.role == Role::Tool);

        if has_tool_results {
            // Second turn - return final response
            Ok(CompletionResponse {
                id: format!("{}-final", self.name),
                message: Message::assistant(&self.final_response),
                model: "mock-model".to_string(),
                usage: TokenUsage {
                    prompt_tokens: 20,
                    completion_tokens: 10,
                    total_tokens: 30,
                    cost_usd: None,
                },
                provider: self.name.clone(),
                finish_reason: FinishReason::Stop,
            })
        } else {
            // First turn - request tool call
            let tool_call = ToolCall {
                id: format!("{}-call-1", self.name),
                name: self.tool_to_call.clone(),
                arguments: self.tool_args.clone(),
            };
            let mut message = Message::assistant("I'll help you with that.");
            message.tool_calls = Some(vec![tool_call]);

            Ok(CompletionResponse {
                id: format!("{}-tool-request", self.name),
                message,
                model: "mock-model".to_string(),
                usage: TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 15,
                    total_tokens: 25,
                    cost_usd: None,
                },
                provider: self.name.clone(),
                finish_reason: FinishReason::ToolUse,
            })
        }
    }

    async fn stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        Err(Error::Provider(ProviderError::StreamError {
            provider: self.name.clone(),
            message: "Not implemented".to_string(),
        }))
    }

    fn model_id(&self) -> &str {
        "mock-model"
    }

    fn max_tokens(&self) -> usize {
        1000
    }

    fn provider_name(&self) -> &str {
        &self.name
    }

    fn supports_strict_tools(&self) -> bool {
        true
    }

    fn supports_streaming_tool_deltas(&self) -> bool {
        false
    }

    fn native_tool_format(&self) -> ToolFormat {
        ToolFormat::OpenAi
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Mock Tool Implementations
// ═════════════════════════════════════════════════════════════════════════════

/// A mock tool that echoes its input.
pub struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echoes back the input message"
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to echo"
                }
            },
            "required": ["message"]
        })
    }

    fn capabilities_required(&self) -> Vec<SkillCapability> {
        vec![]
    }

    async fn execute(&self, input: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let message = input
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("No message provided");

        Ok(ToolOutput {
            tool_call_id: String::new(),
            content: format!("Echo: {}", message),
            is_error: false,
        })
    }
}

/// A mock tool that simulates a calculation.
pub struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }

    fn description(&self) -> &str {
        "Performs basic arithmetic operations"
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"],
                    "description": "The operation to perform"
                },
                "a": {
                    "type": "number",
                    "description": "First operand"
                },
                "b": {
                    "type": "number",
                    "description": "Second operand"
                }
            },
            "required": ["operation", "a", "b"]
        })
    }

    fn capabilities_required(&self) -> Vec<SkillCapability> {
        vec![]
    }

    async fn execute(&self, input: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let operation = input
            .get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let a = input.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let b = input.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);

        let result = match operation {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => {
                if b == 0.0 {
                    return Ok(ToolOutput {
                        tool_call_id: String::new(),
                        content: "Error: Division by zero".to_string(),
                        is_error: true,
                    });
                }
                a / b
            }
            _ => {
                return Ok(ToolOutput {
                    tool_call_id: String::new(),
                    content: format!("Error: Unknown operation '{}'", operation),
                    is_error: true,
                });
            }
        };

        Ok(ToolOutput {
            tool_call_id: String::new(),
            content: format!("Result: {}", result),
            is_error: false,
        })
    }
}

/// A mock tool that can fail.
pub struct FailingTool {
    pub fail_message: String,
}

#[async_trait]
impl Tool for FailingTool {
    fn name(&self) -> &str {
        "failing_tool"
    }

    fn description(&self) -> &str {
        "A tool that always fails"
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn capabilities_required(&self) -> Vec<SkillCapability> {
        vec![]
    }

    async fn execute(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        Ok(ToolOutput {
            tool_call_id: String::new(),
            content: format!("Error: {}", self.fail_message),
            is_error: true,
        })
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Test Data Builders
// ═════════════════════════════════════════════════════════════════════════════

/// Builder for creating test memory entries.
pub struct MemoryEntryBuilder {
    content: String,
    memory_type: MemoryType,
    source: MemorySource,
    user_id: Option<String>,
    session_id: Option<Uuid>,
    namespace: String,
    importance: f32,
    confidence: f32,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl MemoryEntryBuilder {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            memory_type: MemoryType::Semantic,
            source: MemorySource::ExplicitUserStatement,
            user_id: None,
            session_id: None,
            namespace: "test".to_string(),
            importance: 0.5,
            confidence: 1.0,
            expires_at: None,
        }
    }

    pub fn memory_type(mut self, memory_type: MemoryType) -> Self {
        self.memory_type = memory_type;
        self
    }

    pub fn source(mut self, source: MemorySource) -> Self {
        self.source = source;
        self
    }

    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn session_id(mut self, session_id: Uuid) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    pub fn importance(mut self, importance: f32) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    pub fn confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn expires_at(mut self, expires_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn expires_in(mut self, days: i64) -> Self {
        self.expires_at = Some(Utc::now() + chrono::Duration::days(days));
        self
    }

    pub fn build(self) -> MemoryEntry {
        use sha2::{Digest, Sha256};

        let content_hash = {
            let mut hasher = Sha256::new();
            hasher.update(self.content.as_bytes());
            format!("{:x}", hasher.finalize())
        };

        MemoryEntry {
            id: Uuid::new_v4(),
            memory_type: self.memory_type,
            content: self.content,
            content_hash,
            source: Some(format!("{}", self.source)),
            source_type: None,
            session_id: self.session_id,
            user_id: self.user_id,
            namespace: self.namespace,
            importance: self.importance,
            confidence: self.confidence,
            access_count: 0,
            last_accessed: None,
            created_at: Utc::now(),
            expires_at: self.expires_at,
            metadata: serde_json::json!({}),
        }
    }
}

/// Builder for creating core memory entries.
pub struct CoreEntryBuilder {
    key: String,
    value: String,
    importance: f32,
}

impl CoreEntryBuilder {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            importance: 0.8,
        }
    }

    pub fn importance(mut self, importance: f32) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    pub fn build(self) -> CoreEntry {
        let token_count = (self.key.len() + self.value.len() + 2) / 4 + 1;
        CoreEntry {
            key: self.key,
            value: self.value,
            importance: self.importance,
            token_count,
            updated_at: Utc::now(),
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Helper Functions
// ═════════════════════════════════════════════════════════════════════════════

/// Helper to create a memory query.
pub fn create_memory_query(text: impl Into<String>) -> MemoryQuery {
    MemoryQuery {
        text: text.into(),
        memory_types: vec![],
        source_types: vec![],
        namespace: None,
        limit: 10,
        min_confidence: 0.0,
        recency_weight: 0.0,
    }
}

/// Wait for a condition with a timeout.
pub async fn wait_for<F, Fut>(mut condition: F, timeout_ms: u64)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = tokio::time::Instant::now();
    let timeout = Duration::from_millis(timeout_ms);

    while start.elapsed() < timeout {
        if condition().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    panic!("Timeout waiting for condition after {}ms", timeout_ms);
}

/// Retry an async operation with exponential backoff.
pub async fn retry_with_backoff<F, Fut, T>(mut operation: F, max_retries: u32) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut delay = Duration::from_millis(100);

    for attempt in 0..max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(_) if attempt < max_retries - 1 => {
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
            Err(e) => return Err(e),
        }
    }

    unreachable!()
}

/// Parse JSON response from a string.
pub fn parse_json<T: DeserializeOwned>(json_str: &str) -> Result<T> {
    serde_json::from_str(json_str).map_err(|e| Error::Internal(format!("JSON parse error: {}", e)))
}

/// Create a test JWT token.
pub fn create_test_jwt(sub: &str, secret: &str) -> String {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
    }

    let claims = Claims {
        sub: sub.to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
    };

    let header = Header::new(Algorithm::HS256);
    encode(&header, &claims, &EncodingKey::from_secret(secret.as_ref()))
        .expect("Failed to encode JWT")
}

/// Check if running in CI environment.
pub fn is_ci() -> bool {
    std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok()
}

/// Get test timeout from environment.
pub fn test_timeout_secs() -> u64 {
    std::env::var("E2E_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300)
}
