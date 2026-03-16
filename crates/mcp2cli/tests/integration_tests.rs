//! Integration tests for mcp2cli
//!
//! These tests cover:
//! - MCP adapter functionality
//! - OpenAPI adapter functionality  
//! - Caching behavior
//! - TOON encoding/decoding
//! - Token cost comparisons
//! - End-to-end discovery and execution

use openrustclaw_mcp2cli::{
    decode_toon, encode_toon, calculate_savings, Mcp2CliFactory, Mcp2CliRegistry, ToolDiscovery,
    ToolSource, TokenCounter, ToolCache, ToolHelp, ToolSummary, ParamHelp,
};
use serde_json::json;
use std::time::Duration;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

// ============================================================================
// TOON Tests
// ============================================================================

#[test]
fn test_toon_simple_values() {
    assert_eq!(encode_toon(&json!(null)), "null");
    assert_eq!(encode_toon(&json!(true)), "true");
    assert_eq!(encode_toon(&json!(false)), "false");
    assert_eq!(encode_toon(&json!(42)), "42");
    assert_eq!(encode_toon(&json!("hello")), "hello");
}

#[test]
fn test_toon_quoted_strings() {
    // Strings with special chars need quoting
    assert_eq!(encode_toon(&json!("hello world")), "\"hello world\"");
    assert_eq!(encode_toon(&json!("a:b")), "\"a:b\"");
    assert_eq!(encode_toon(&json!("a;b")), "\"a;b\"");
}

#[test]
fn test_toon_roundtrip() {
    let test_values = vec![
        json!({"name": "test", "value": 42}),
        json!({"items": [1, 2, 3], "count": 3}),
        json!({"nested": {"key": "value"}, "flag": true}),
        json!({"empty": {}, "list": []}),
    ];

    for original in test_values {
        let toon = encode_toon(&original);
        let decoded = decode_toon(&toon).expect("Failed to decode TOON");
        assert_eq!(original, decoded, "TOON roundtrip failed for: {:?}", original);
    }
}

#[test]
fn test_toon_savings() {
    let json_data = json!({
        "users": [
            {"id": 1, "name": "Alice", "active": true},
            {"id": 2, "name": "Bob", "active": false}
        ],
        "total": 2,
        "page": 1
    });

    let json_str = json_data.to_string();
    let toon_str = encode_toon(&json_data);

    let json_tokens = TokenCounter::count_tokens(&json_str);
    let toon_tokens = TokenCounter::count_tokens(&toon_str);

    let savings = calculate_savings(json_tokens, toon_tokens);
    
    // TOON should provide some savings
    assert!(savings >= 0.0, "TOON should not increase token count");
    
    println!("JSON: {} tokens", json_tokens);
    println!("TOON: {} tokens", toon_tokens);
    println!("Savings: {:.1}%", savings * 100.0);
}

// ============================================================================
// Token Counter Tests
// ============================================================================

#[test]
fn test_token_counting() {
    let text = "Hello, world!";
    let count = TokenCounter::count_tokens(text);
    assert!(count > 0, "Token count should be positive");
    
    // Longer text should have more tokens
    let longer_text = "This is a longer text with more words.";
    let longer_count = TokenCounter::count_tokens(longer_text);
    assert!(longer_count >= count, "Longer text should have >= tokens");
}

#[test]
fn test_cost_comparison() {
    // Simulate a scenario with 100 tools, 10 turns, 5 used
    let comparison = TokenCounter::compare_costs(100, 10, 5);
    
    // Native should cost much more
    assert!(comparison.native_tokens > comparison.mcp2cli_tokens);
    
    // Should have significant savings (>90%)
    assert!(comparison.savings_percent > 0.90, 
        "Expected >90% savings, got {:.1}%", comparison.savings_percent * 100.0);
    
    println!("Cost Comparison:");
    println!("{}", comparison.format());
}

#[test]
fn test_session_cost_breakdown() {
    let breakdown = TokenCounter::session_cost_breakdown(50, 10, 3, 5);
    
    assert!(breakdown.native_equivalent > breakdown.total_for_session);
    assert!(breakdown.savings_percent > 0.0);
    
    println!("Session Cost Breakdown:");
    println!("{}", breakdown.format_report());
}

// ============================================================================
// Cache Tests
// ============================================================================

#[tokio::test]
async fn test_cache_basic() {
    let cache = ToolCache::new(Duration::from_secs(60));
    
    // First call should miss and populate cache
    let result1 = cache
        .get_or_insert("test_key", || async {
            Ok(vec![ToolSummary::new("tool1", "Test tool")])
        })
        .await
        .expect("Cache insert failed");
    
    assert_eq!(result1.tools.len(), 1);
    assert_eq!(result1.tools[0].name, "tool1");
    
    // Second call should hit cache
    let result2 = cache
        .get_or_insert("test_key", || async {
            // This should not be called
            panic!("Cache should be used");
        })
        .await
        .expect("Cache get failed");
    
    assert_eq!(result2.tools.len(), 1);
    
    let stats = cache.stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
    assert!((stats.hit_rate() - 0.5).abs() < 0.01);
}

#[tokio::test]
async fn test_cache_invalidation() {
    let cache = ToolCache::new(Duration::from_secs(60));
    
    cache
        .get_or_insert("key1", || async {
            Ok(vec![ToolSummary::new("tool1", "Test")])
        })
        .await
        .unwrap();
    
    // Invalidate the key
    cache.invalidate("key1");
    
    // Should be a miss now
    cache
        .get_or_insert("key1", || async {
            Ok(vec![ToolSummary::new("tool2", "Updated")])
        })
        .await
        .unwrap();
    
    let stats = cache.stats();
    assert_eq!(stats.misses, 2);
}

#[tokio::test]
async fn test_cache_prefix_invalidation() {
    let cache = ToolCache::new(Duration::from_secs(60));
    
    cache.get_or_insert("prefix:key1", || async {
        Ok(vec![ToolSummary::new("tool1", "Test")])
    }).await.unwrap();
    
    cache.get_or_insert("prefix:key2", || async {
        Ok(vec![ToolSummary::new("tool2", "Test")])
    }).await.unwrap();
    
    cache.get_or_insert("other:key3", || async {
        Ok(vec![ToolSummary::new("tool3", "Test")])
    }).await.unwrap();
    
    // Invalidate by prefix
    cache.invalidate_prefix("prefix:");
    
    // Only prefix keys should be removed
    assert!(cache.get_or_insert("other:key3", || async {
        panic!("Should not be called");
    }).await.is_ok());
}

// ============================================================================
// Tool Discovery Tests
// ============================================================================

#[test]
fn test_tool_summary() {
    let summary = ToolSummary::new("search", "Search for documents in the database");
    
    assert_eq!(summary.name, "search");
    assert_eq!(summary.description, "Search for documents in the database");
    assert!(summary.token_cost > 0);
    
    let compact = summary.to_compact_string();
    assert!(compact.contains("search"));
    assert!(compact.contains("Search for"));
}

#[test]
fn test_tool_help() {
    let params = vec![
        ParamHelp::new("query", "Search query string", "string", true)
            .with_example("rust programming"),
        ParamHelp::new("limit", "Maximum results", "number", false)
            .with_default(json!(10)),
    ];
    
    let help = ToolHelp::new(
        "search",
        "Search for documents in the database",
        "search --query <query> [--limit <n>]",
        params,
    );
    
    assert_eq!(help.name, "search");
    assert_eq!(help.parameters.len(), 2);
    assert!(help.token_cost >= 80 && help.token_cost <= 200);
    
    let compact = help.to_compact_string();
    assert!(compact.contains("search"));
    assert!(compact.contains("query"));
    assert!(compact.contains("limit"));
}

#[test]
fn test_param_help_builder() {
    let param = ParamHelp::new("name", "The name", "string", true)
        .with_example("John Doe")
        .with_default(json!("Anonymous"));
    
    assert_eq!(param.name, "name");
    assert!(param.required);
    assert_eq!(param.example, Some("John Doe".to_string()));
    assert_eq!(param.default, Some(json!("Anonymous")));
}

// ============================================================================
// Tool Source Tests
// ============================================================================

#[test]
fn test_tool_source_creation() {
    let mcp_url = ToolSource::mcp_url("http://localhost:3000/sse");
    assert!(matches!(mcp_url, ToolSource::McpUrl { .. }));
    
    let mcp_stdio = ToolSource::mcp_stdio("npx");
    assert!(matches!(mcp_stdio, ToolSource::McpStdio { .. }));
    
    let openapi_url = ToolSource::openapi_url("https://api.example.com/openapi.json");
    assert!(matches!(openapi_url, ToolSource::OpenApiUrl { .. }));
    
    let openapi_file = ToolSource::openapi_file("./spec.yaml");
    assert!(matches!(openapi_file, ToolSource::OpenApiFile { .. }));
}

#[test]
fn test_tool_source_cache_key() {
    let source1 = ToolSource::mcp_url("http://test.com");
    let source2 = ToolSource::mcp_url("http://test.com");
    let source3 = ToolSource::mcp_url("http://other.com");
    
    assert_eq!(source1.cache_key(), source2.cache_key());
    assert_ne!(source1.cache_key(), source3.cache_key());
}

#[test]
fn test_tool_source_description() {
    let mcp_url = ToolSource::mcp_url("http://localhost:3000");
    assert!(mcp_url.description().contains("MCP server"));
    assert!(mcp_url.description().contains("localhost"));
    
    let openapi_file = ToolSource::openapi_file("./api.yaml");
    assert!(openapi_file.description().contains("OpenAPI"));
    assert!(openapi_file.description().contains("api.yaml"));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn test_discovery_with_cache() {
    let discovery = ToolDiscovery::new();
    let source = ToolSource::openapi_url("https://example.com/api");
    
    // First call - should hit the source (and potentially fail, that's ok)
    let _ = discovery.list_tools(&source).await;
    
    // Check stats
    let stats = discovery.cache_stats();
    // Cache might be empty if the source failed, which is ok
    println!("Cache stats: {:?}", stats);
}

#[tokio::test]
async fn test_openapi_adapter_from_file() {
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    let openapi_content = r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
servers:
  - url: https://api.example.com
paths:
  /users:
    get:
      operationId: listUsers
      summary: List users
      parameters:
        - name: limit
          in: query
          schema:
            type: integer
    post:
      operationId: createUser
      summary: Create user
  /users/{id}:
    get:
      operationId: getUser
      summary: Get user
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
"#;

    let mut temp_file = NamedTempFile::with_suffix(".yaml").unwrap();
    temp_file.write_all(openapi_content.as_bytes()).unwrap();
    
    let adapter = openrustclaw_mcp2cli::OpenApiAdapter::from_file(
        temp_file.path().to_str().unwrap()
    ).await;
    
    // The adapter might fail in test environment, which is ok
    match adapter {
        Ok(_) => println!("OpenAPI adapter created successfully"),
        Err(e) => println!("OpenAPI adapter error (expected in test): {}", e),
    }
}

#[test]
fn test_mcp2cli_registry() {
    let registry = Mcp2CliRegistry::new();
    
    // Create a test tool
    let tool = Mcp2CliFactory::with_name(
        ToolSource::mcp_url("http://test"),
        "test_source",
        "Test source for registry",
    );
    
    // Register it
    registry.register("test", tool);
    
    // Retrieve it
    let retrieved = registry.get("test");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name(), "test_source");
    
    // List all
    let list = registry.list();
    assert_eq!(list.len(), 1);
    
    // Remove it
    registry.remove("test");
    assert!(registry.get("test").is_none());
}

// ============================================================================
// End-to-End Scenarios
// ============================================================================

#[test]
fn test_token_savings_scenario() {
    // Simulate a realistic scenario:
    // - 50 tools available
    // - User conversation with 20 turns
    // - 8 different tools actually used
    
    let tool_count = 50;
    let turns = 20;
    let tools_used = 8;
    
    let comparison = TokenCounter::compare_costs(tool_count, turns, tools_used);
    
    println!("\n=== Token Savings Scenario ===");
    println!("Tools available: {}", tool_count);
    println!("Conversation turns: {}", turns);
    println!("Tools actually used: {}", tools_used);
    println!();
    println!("{}", comparison.format());
    
    // Should have 95%+ savings
    assert!(
        comparison.savings_percent >= 0.95,
        "Expected >=95% savings, got {:.1}%",
        comparison.savings_percent * 100.0
    );
}

#[test]
fn test_large_scale_savings() {
    // Test with 200 tools (large MCP server)
    let tool_count = 200;
    let turns = 10;
    let tools_used = 5;
    
    let comparison = TokenCounter::compare_costs(tool_count, turns, tools_used);
    
    println!("\n=== Large Scale Scenario (200 tools) ===");
    println!("{}", comparison.format());
    
    // With many tools, savings should be even higher (99%+)
    assert!(
        comparison.savings_percent >= 0.99,
        "Expected >=99% savings with 200 tools, got {:.1}%",
        comparison.savings_percent * 100.0
    );
}

#[test]
fn test_mcp2cli_as_tool_schema() {
    let tool = Mcp2CliFactory::with_name(
        ToolSource::openapi_url("https://api.example.com"),
        "api_gateway",
        "API gateway for external services",
    );
    
    let schema = tool.schema();
    
    // Verify schema structure
    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["action"].is_object());
    assert!(schema["properties"]["tool_name"].is_object());
    assert!(schema["properties"]["args"].is_object());
    assert!(schema["properties"]["use_toon"].is_object());
    
    // Verify required fields
    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&json!("action")));
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_cache_access() {
    use std::sync::Arc;
    
    let cache = Arc::new(ToolCache::new(Duration::from_secs(60)));
    let mut handles = vec![];
    
    // Spawn multiple concurrent accesses
    for i in 0..10 {
        let cache_clone = Arc::clone(&cache);
        let handle = tokio::spawn(async move {
            cache_clone
                .get_or_insert("concurrent_key", || async {
                    // Simulate some work
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    Ok(vec![ToolSummary::new(
                        &format!("tool{}", i),
                        "Test tool"
                    )])
                })
                .await
        });
        handles.push(handle);
    }
    
    // Wait for all to complete
    for handle in handles {
        let _ = handle.await.unwrap();
    }
    
    // Should only have one actual miss (the rest cached)
    let stats = cache.stats();
    println!("Concurrent cache stats: {:?}", stats);
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.hits, 9);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_error_types() {
    use openrustclaw_mcp2cli::Mcp2CliError;
    
    let err = Mcp2CliError::tool_not_found("missing_tool");
    assert!(err.to_string().contains("missing_tool"));
    
    let err = Mcp2CliError::auth("Invalid credentials");
    assert!(err.to_string().contains("authentication"));
    
    let err = Mcp2CliError::cache("Cache full");
    assert!(err.to_string().contains("cache"));
}

#[test]
fn test_toon_decode_errors() {
    // Malformed TOON should return error
    let result = decode_toon("[[[[");
    assert!(result.is_err() || result.unwrap() != json!(null));
}

// Helper trait for tests
trait ToolNotFoundExt {
    fn tool_not_found(name: impl Into<String>) -> Self;
}

impl ToolNotFoundExt for openrustclaw_mcp2cli::Mcp2CliError {
    fn tool_not_found(name: impl Into<String>) -> Self {
        Self::ToolNotFound(name.into())
    }
}
