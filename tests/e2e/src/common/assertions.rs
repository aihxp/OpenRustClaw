//! Custom assertions for E2E tests.

use openrustclaw_core::types::{CompletionResponse, MemoryEntry, Message, ToolCall};
use serde_json::Value;

/// Assertion helpers for E2E tests
pub struct E2eAssertions;

impl E2eAssertions {
    /// Assert that a response contains expected text
    pub fn response_contains(response: &CompletionResponse, expected: &str) {
        let content = &response.message.content;
        assert!(
            content.to_lowercase().contains(&expected.to_lowercase()),
            "Expected response to contain '{}', but got: {}",
            expected,
            content
        );
    }

    /// Assert that a response is from a specific provider
    pub fn response_from_provider(response: &CompletionResponse, provider: &str) {
        assert_eq!(
            response.provider, provider,
            "Expected provider '{}', but got: {}",
            provider, response.provider
        );
    }

    /// Assert that a response has tool calls
    pub fn response_has_tool_calls(response: &CompletionResponse) {
        assert!(
            response.message.tool_calls.is_some(),
            "Expected response to have tool calls, but none found"
        );
        assert!(
            !response.message.tool_calls.as_ref().unwrap().is_empty(),
            "Expected response to have tool calls, but list is empty"
        );
    }

    /// Assert that a specific tool was called
    pub fn tool_was_called<'a>(response: &'a CompletionResponse, tool_name: &str) -> &'a ToolCall {
        let tool_calls = response
            .message
            .tool_calls
            .as_ref()
            .expect("No tool calls in response");

        let found = tool_calls
            .iter()
            .find(|tc| tc.name == tool_name)
            .unwrap_or_else(|| {
                panic!(
                    "Expected tool '{}' to be called, but got: {:?}",
                    tool_name,
                    tool_calls.iter().map(|tc| &tc.name).collect::<Vec<_>>()
                )
            });

        found
    }

    /// Assert that a tool call has specific arguments
    pub fn tool_has_args(tool_call: &ToolCall, expected_args: Value) {
        assert_eq!(
            tool_call.arguments, expected_args,
            "Tool arguments don't match. Expected: {}, Got: {}",
            expected_args, tool_call.arguments
        );
    }

    /// Assert that a memory entry contains expected content
    pub fn memory_contains(entry: &MemoryEntry, expected: &str) {
        assert!(
            entry.content.to_lowercase().contains(&expected.to_lowercase()),
            "Expected memory to contain '{}', but got: {}",
            expected,
            entry.content
        );
    }

    /// Assert that a memory entry has correct namespace
    pub fn memory_in_namespace(entry: &MemoryEntry, namespace: &str) {
        assert_eq!(
            entry.namespace, namespace,
            "Expected memory in namespace '{}', but got: {}",
            namespace,
            entry.namespace
        );
    }

    /// Assert that a message has the expected role
    pub fn message_has_role(message: &Message, role: &str) {
        let role_str = format!("{:?}", message.role).to_lowercase();
        assert_eq!(
            role_str, role.to_lowercase(),
            "Expected message role '{}', but got: {:?}",
            role, message.role
        );
    }

    /// Assert that a health response is healthy
    pub fn is_healthy(health: &Value) {
        let status = health
            .get("status")
            .and_then(|v| v.as_str())
            .expect("Health response missing status");

        assert!(
            status == "healthy" || status == "ok",
            "Expected healthy status, but got: {}",
            status
        );
    }

    /// Assert that a health response has all components ok
    pub fn all_components_healthy(health: &Value) {
        let components = health
            .get("components")
            .expect("Health response missing components");

        if let Some(obj) = components.as_object() {
            for (name, status) in obj {
                let status_str = status.as_str().unwrap_or("unknown");
                assert!(
                    status_str == "ok" || status_str == "healthy",
                    "Component '{}' is not healthy: {}",
                    name,
                    status_str
                );
            }
        }
    }

    /// Assert that an error response contains expected message
    pub fn error_contains(error: &openrustclaw_core::error::Error, expected: &str) {
        let error_string = format!("{}", error).to_lowercase();
        assert!(
            error_string.contains(&expected.to_lowercase()),
            "Expected error to contain '{}', but got: {}",
            expected,
            error
        );
    }

    /// Assert that a JSON value has a specific field
    pub fn json_has_field(json: &Value, field: &str) {
        assert!(
            json.get(field).is_some(),
            "Expected JSON to have field '{}', but it doesn't. JSON: {}",
            field,
            json
        );
    }

    /// Assert that a JSON value has a field with expected value
    pub fn json_field_equals(json: &Value, field: &str, expected: Value) {
        let actual = json
            .get(field)
            .unwrap_or_else(|| panic!("Field '{}' not found in JSON: {}", field, json));

        assert_eq!(
            actual, &expected,
            "Field '{}' value mismatch. Expected: {}, Got: {}",
            field, expected, actual
        );
    }

    /// Assert that a list has the expected length
    pub fn list_has_length<T>(list: &[T], expected: usize) {
        assert_eq!(
            list.len(),
            expected,
            "Expected list length {}, but got {}",
            expected,
            list.len()
        );
    }

    /// Assert that a list is not empty
    pub fn list_not_empty<T>(list: &[T]) {
        assert!(!list.is_empty(), "Expected list to not be empty");
    }

    /// Assert that a duration is within expected range
    pub fn duration_within(actual_ms: u64, min_ms: u64, max_ms: u64) {
        assert!(
            actual_ms >= min_ms && actual_ms <= max_ms,
            "Expected duration between {}ms and {}ms, but got {}ms",
            min_ms,
            max_ms,
            actual_ms
        );
    }

    /// Assert that a token usage is within expected range
    pub fn token_usage_within(response: &CompletionResponse, max_tokens: usize) {
        assert!(
            response.usage.total_tokens <= max_tokens,
            "Expected token usage <= {}, but got {}",
            max_tokens,
            response.usage.total_tokens
        );
    }

    /// Assert that a stream completed successfully
    pub fn stream_completed_successfully(chunks: &[openrustclaw_core::types::StreamChunk]) {
        let last = chunks
            .last()
            .expect("Stream should have at least one chunk");

        match last {
            openrustclaw_core::types::StreamChunk::Done { .. } => {}
            _ => panic!("Expected stream to end with Done chunk, but got: {:?}", last),
        }
    }
}

/// Macro for asserting response contains text
#[macro_export]
macro_rules! assert_response_contains {
    ($response:expr, $expected:expr) => {
        $crate::common::assertions::E2eAssertions::response_contains($response, $expected)
    };
}

/// Macro for asserting tool was called
#[macro_export]
macro_rules! assert_tool_called {
    ($response:expr, $tool_name:expr) => {
        $crate::common::assertions::E2eAssertions::tool_was_called($response, $tool_name)
    };
}

/// Macro for asserting health is ok
#[macro_export]
macro_rules! assert_healthy {
    ($health:expr) => {
        $crate::common::assertions::E2eAssertions::is_healthy($health);
        $crate::common::assertions::E2eAssertions::all_components_healthy($health);
    };
}
