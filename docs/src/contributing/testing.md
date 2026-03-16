# Testing Guide

This guide covers the testing philosophy, structure, and best practices for OpenRustClaw.

---

## 🎯 Testing Philosophy

OpenRustClaw follows these testing principles:

1. **Test behavior, not implementation** — Tests should verify what code does, not how
2. **Fast feedback** — Unit tests should run in milliseconds
3. **Isolation** — Tests should not depend on external services
4. **Coverage** — Critical paths should have thorough test coverage

---

## 🏗️ Test Structure

```
OpenRustClaw/
├── crates/
│   ├── core/
│   │   └── src/
│   │       ├── lib.rs
│   │       └── types.rs
│   │           #[cfg(test)]
│   │           mod tests {
│   │               // Unit tests inline
│   │           }
│   ├── memory/
│   │   └── src/
│   │       ├── lib.rs
│   │       └── search.rs
│   │           #[cfg(test)]
│   │           mod tests {
│   │               // Unit tests inline
│   │           }
│   └── ...
└── tests/
    └── integration/
        ├── Cargo.toml
        └── src/
            ├── main.rs
            ├── test_memory.rs      # Integration tests
            ├── test_providers.rs
            └── test_end_to_end.rs
```

---

## 🧪 Unit Tests

### Inline Unit Tests

Unit tests live in the same file as the code they test:

```rust
// crates/core/src/types.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub role: Role,
    pub content: String,
    // ...
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn message_new_user() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "Hello");
        assert!(msg.tool_calls.is_none());
    }
    
    #[test]
    fn message_new_assistant() {
        let msg = Message::assistant("Hi there");
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.content, "Hi there");
    }
    
    #[test]
    fn message_tool() {
        let msg = Message::tool("call_123", "result");
        assert_eq!(msg.role, Role::Tool);
        assert_eq!(msg.tool_call_id, Some("call_123".into()));
    }
}
```

### Running Unit Tests

```bash
# Run all unit tests
cargo test --workspace --lib

# Run tests for specific crate
cargo test -p openrustclaw-core --lib

# Run specific test
cargo test message_new_user

# Run with output
cargo test -- --nocapture
```

---

## 🔗 Integration Tests

Integration tests are in the `tests/integration/` crate and test multiple components together.

### Test Organization

```rust
// tests/integration/src/test_memory.rs

use openrustclaw_memory::{MemoryStore, MemoryQuery};
use openrustclaw_db::SqliteStore;

#[tokio::test]
async fn test_memory_store_and_retrieve() {
    // Arrange
    let db = SqliteStore::new_in_memory().await.unwrap();
    let memory = MemoryStore::new(db);
    
    let entry = MemoryEntry {
        content: "Test memory".into(),
        memory_type: MemoryType::Semantic,
        ..Default::default()
    };
    
    // Act
    memory.store(entry.clone()).await.unwrap();
    let retrieved = memory.get(&entry.id.to_string()).await.unwrap();
    
    // Assert
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().content, "Test memory");
}

#[tokio::test]
async fn test_memory_search() {
    // Arrange
    let memory = create_test_store().await;
    
    memory.store(MemoryEntry {
        content: "Rust is a systems language".into(),
        memory_type: MemoryType::Semantic,
        ..Default::default()
    }).await.unwrap();
    
    memory.store(MemoryEntry {
        content: "Python is great for scripting".into(),
        memory_type: MemoryType::Semantic,
        ..Default::default()
    }).await.unwrap();
    
    // Act
    let results = memory.search(&MemoryQuery {
        text: "programming language".into(),
        limit: 10,
        ..Default::default()
    }).await.unwrap();
    
    // Assert
    assert_eq!(results.len(), 2);
    assert!(results[0].score > 0.5);
}
```

### Test Fixtures

```rust
// tests/integration/src/fixtures.rs

use openrustclaw_db::SqliteStore;
use openrustclaw_memory::MemoryStore;

pub async fn create_test_store() -> MemoryStore {
    let db = SqliteStore::new_in_memory().await.unwrap();
    MemoryStore::new(db)
}

pub async fn seed_test_data(store: &MemoryStore) {
    let entries = vec![
        MemoryEntry {
            content: "OpenRustClaw is an AI agent framework".into(),
            memory_type: MemoryType::Semantic,
            ..Default::default()
        },
        MemoryEntry {
            content: "Built with Rust and Python".into(),
            memory_type: MemoryType::Semantic,
            ..Default::default()
        },
    ];
    
    for entry in entries {
        store.store(entry).await.unwrap();
    }
}
```

---

## 🎭 Mock Providers

Use mocks to test without hitting real APIs:

```rust
// crates/providers/src/mock.rs

use async_trait::async_trait;
use openrustclaw_core::traits::LlmProvider;
use openrustclaw_core::types::*;
use std::sync::Mutex;

pub struct MockProvider {
    responses: Mutex<Vec<CompletionResponse>>,
    request_log: Mutex<Vec<CompletionRequest>>,
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            responses: Mutex::new(vec![]),
            request_log: Mutex::new(vec![]),
        }
    }
    
    pub fn queue_response(&self, response: CompletionResponse) {
        self.responses.lock().unwrap().push(response);
    }
    
    pub fn get_requests(&self) -> Vec<CompletionRequest> {
        self.request_log.lock().unwrap().clone()
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        self.request_log.lock().unwrap().push(request);
        
        self.responses.lock()
            .unwrap()
            .pop()
            .ok_or_else(|| Error::Provider(ProviderError::Request("No mock response".into())))
    }
    
    fn model_id(&self) -> &str { "mock-model" }
    fn provider_name(&self) -> &str { "mock" }
    fn supports_strict_tools(&self) -> bool { true }
    fn native_tool_format(&self) -> ToolFormat { ToolFormat::OpenAi }
}
```

### Using Mocks in Tests

```rust
#[tokio::test]
async fn test_agent_with_mock_provider() {
    // Arrange
    let mock = Arc::new(MockProvider::new());
    mock.queue_response(CompletionResponse {
        message: Message::assistant("Hello!"),
        ..Default::default()
    });
    
    let agent = Agent::builder()
        .provider(mock.clone())
        .build()
        .await
        .unwrap();
    
    // Act
    let response = agent
        .chat(&session_id, Message::user("Hi"))
        .await
        .unwrap();
    
    // Assert
    assert_eq!(response.content, "Hello!");
    
    let requests = mock.get_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].messages[0].content, "Hi");
}
```

---

## 🌐 HTTP Mocking

Use `wiremock` for HTTP-based provider tests:

```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, header};

#[tokio::test]
async fn test_anthropic_provider() {
    // Arrange
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "id": "msg_123",
                "content": [{"type": "text", "text": "Hello!"}],
                "model": "claude-3",
                "usage": {"input_tokens": 10, "output_tokens": 5}
            })))
        .mount(&mock_server)
        .await;
    
    let provider = AnthropicProvider::new(
        "test-key".into(),
        "claude-3".into(),
    ).with_base_url(mock_server.uri());
    
    // Act
    let response = provider.complete(CompletionRequest {
        messages: vec![Message::user("Hi")],
        ..Default::default()
    }).await.unwrap();
    
    // Assert
    assert_eq!(response.message.content, "Hello!");
    assert_eq!(response.usage.prompt_tokens, 10);
}
```

---

## 🗄️ Database Testing

Use in-memory SQLite for fast database tests:

```rust
impl SqliteStore {
    /// Create an in-memory database for testing
    pub async fn new_in_memory() -> Result<Self> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        
        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await?;
        
        Ok(Self { pool })
    }
}

#[tokio::test]
async fn test_database_operations() {
    let db = SqliteStore::new_in_memory().await.unwrap();
    
    // Test operations...
}
```

---

## 🔁 Async Testing

### Basic Async Test

```rust
#[tokio::test]
async fn test_async_function() {
    let result = some_async_operation().await;
    assert!(result.is_ok());
}
```

### Test with Timeout

```rust
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_with_timeout() {
    let result = timeout(
        Duration::from_secs(5),
        slow_operation()
    ).await;
    
    assert!(result.is_ok(), "Operation timed out");
}
```

### Test with Task

```rust
#[tokio::test]
async fn test_concurrent_operations() {
    let handle1 = tokio::spawn(async {
        operation_one().await
    });
    
    let handle2 = tokio::spawn(async {
        operation_two().await
    });
    
    let (result1, result2) = tokio::join!(handle1, handle2);
    
    assert!(result1.unwrap().is_ok());
    assert!(result2.unwrap().is_ok());
}
```

---

## 📊 Test Coverage

### Running Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage for workspace
cargo tarpaulin --workspace

# Generate HTML report
cargo tarpaulin --workspace --out Html

# Exclude certain files
cargo tarpaulin --workspace --exclude-files "*/tests/*"
```

### Coverage Goals

| Component | Target Coverage |
|-----------|----------------|
| Core types | 90%+ |
| Memory system | 85%+ |
| Provider SDKs | 80%+ |
| Security | 90%+ |
| CLI | 70%+ |

---

## 🎓 Test Best Practices

### 1. Use Descriptive Names

```rust
// Bad
#[test]
fn test1() {}

// Good
#[test]
fn memory_store_rejects_duplicate_content() {}

// Good
#[test]
fn provider_fallback_activates_on_rate_limit() {}
```

### 2. Follow Arrange-Act-Assert

```rust
#[tokio::test]
async fn test_example() {
    // Arrange
    let input = create_test_input();
    let expected = create_expected_output();
    
    // Act
    let result = function_under_test(input).await;
    
    // Assert
    assert_eq!(result, expected);
}
```

### 3. Test Edge Cases

```rust
#[test]
fn parse_empty_string() {}

#[test]
fn parse_max_length_string() {}

#[test]
fn parse_unicode_string() {}

#[test]
fn parse_malformed_input() {}
```

### 4. Use Parameterized Tests

```rust
use rstest::rstest;

#[rstest]
#[case("hello", 5)]
#[case("", 0)]
#[case("Rust", 4)]
#[case("🦀", 4)]  // Unicode counts as 4 bytes
fn string_length(#[case] input: &str, #[case] expected: usize) {
    assert_eq!(input.len(), expected);
}
```

### 5. Clean Up Resources

```rust
#[tokio::test]
async fn test_with_cleanup() {
    let temp_dir = tempfile::tempdir().unwrap();
    
    // Test code...
    
    // Cleanup happens automatically when temp_dir is dropped
    drop(temp_dir);
}
```

---

## 🔧 Continuous Integration

Tests run automatically on pull requests:

```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: dtolnay/rust-action@stable
      
      - name: Run tests
        run: cargo test --workspace
      
      - name: Run clippy
        run: cargo clippy --workspace -- -D warnings
      
      - name: Check formatting
        run: cargo fmt --all -- --check
```

---

## 🐛 Debugging Tests

```bash
# Print all output
cargo test -- --nocapture

# Run single test
cargo test test_name -- --exact

# Run with backtrace
RUST_BACKTRACE=1 cargo test test_name

# Run with debugger
rust-gdb target/debug/deps/test_name-abc123
```
