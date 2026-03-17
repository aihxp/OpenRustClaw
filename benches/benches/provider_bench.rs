//! Provider benchmarks for OpenRustClaw.
//!
//! Measures:
//! - Request serialization speed
//! - Response parsing speed
//! - Streaming throughput
//! - Token counting performance

use std::collections::HashMap;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use openrustclaw_core::types::{
    CompletionRequest, CompletionResponse, FinishReason, Message, StreamChunk, TokenUsage,
    ToolCall, ToolDefinition, ToolFormat,
};
use openrustclaw_providers::tool_formats::{
    parse_anthropic_tool_calls, parse_openai_tool_calls, translate_tool_definition,
};
use serde_json::Value;
use tiktoken_rs::p50k_base;
use uuid::Uuid;

// =============================================================================
// Helper Functions
// =============================================================================

fn create_sample_completion_request(msg_count: usize, tool_count: usize) -> CompletionRequest {
    let messages: Vec<Message> = (0..msg_count)
        .map(|i| {
            if i % 2 == 0 {
                Message::user(format!("User message number {} with some content", i))
            } else {
                Message::assistant(format!("Assistant response number {}", i))
            }
        })
        .collect();

    let tools: Vec<ToolDefinition> = (0..tool_count)
        .map(|i| ToolDefinition {
            name: format!("tool_{}", i),
            description: format!("Description for tool {}", i),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "arg": { "type": "string" }
                }
            }),
            strict: false,
        })
        .collect();

    CompletionRequest {
        messages,
        model: Some("gpt-4".to_string()),
        max_tokens: Some(4096),
        temperature: Some(0.7),
        tools: if tools.is_empty() { None } else { Some(tools) },
        system_prompt: Some("You are a helpful assistant.".to_string()),
        stream: false,
    }
}

fn create_sample_completion_response(tool_count: usize) -> CompletionResponse {
    let tool_calls: Vec<ToolCall> = (0..tool_count)
        .map(|i| ToolCall {
            id: format!("call_{}", i),
            name: format!("tool_{}", i),
            arguments: serde_json::json!({"arg": format!("value_{}", i)}),
        })
        .collect();

    let mut message = Message::assistant("I'll help you with that.".to_string());
    if !tool_calls.is_empty() {
        message.tool_calls = Some(tool_calls);
    }

    CompletionResponse {
        id: Uuid::new_v4().to_string(),
        message,
        model: "gpt-4".to_string(),
        usage: TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            cost_usd: Some(0.002),
        },
        provider: "openai".to_string(),
        finish_reason: if tool_count > 0 {
            FinishReason::ToolUse
        } else {
            FinishReason::Stop
        },
    }
}

fn create_anthropic_content_blocks(tool_count: usize) -> Vec<Value> {
    let mut blocks = vec![serde_json::json!({
        "type": "text",
        "text": "Let me check that for you."
    })];

    for i in 0..tool_count {
        blocks.push(serde_json::json!({
            "type": "tool_use",
            "id": format!("toolu_{}", i),
            "name": format!("tool_{}", i),
            "input": { "arg": format!("value_{}", i) }
        }));
    }

    blocks
}

fn create_openai_tool_calls(tool_count: usize) -> Vec<Value> {
    (0..tool_count)
        .map(|i| {
            serde_json::json!({
                "id": format!("call_{}", i),
                "type": "function",
                "function": {
                    "name": format!("tool_{}", i),
                    "arguments": format!("{{\"arg\":\"value_{}\"}}", i)
                }
            })
        })
        .collect()
}

// =============================================================================
// Request Serialization Benchmarks
// =============================================================================

fn bench_request_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("provider/request_serialization");

    for (msg_count, tool_count) in [(10, 0), (50, 5), (100, 20)] {
        let param = format!("{}msgs_{}tools", msg_count, tool_count);

        group.bench_with_input(
            BenchmarkId::new("json", &param),
            &(msg_count, tool_count),
            |b, &(msgs, tools)| {
                let request = create_sample_completion_request(msgs, tools);

                b.iter(|| {
                    let json = serde_json::to_string(&request).unwrap();
                    criterion::black_box(json);
                });
            },
        );
    }

    group.finish();
}

fn bench_request_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("provider/request_deserialization");

    for (msg_count, tool_count) in [(10, 0), (50, 5), (100, 20)] {
        let param = format!("{}msgs_{}tools", msg_count, tool_count);
        let request = create_sample_completion_request(msg_count, tool_count);
        let json = serde_json::to_string(&request).unwrap();

        group.throughput(Throughput::Bytes(json.len() as u64));

        group.bench_with_input(BenchmarkId::new("json", &param), &json, |b, json_str| {
            b.iter(|| {
                let req: CompletionRequest = serde_json::from_str(json_str).unwrap();
                criterion::black_box(req);
            });
        });
    }

    group.finish();
}

// =============================================================================
// Response Parsing Benchmarks
// =============================================================================

fn bench_response_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("provider/response_serialization");

    for tool_count in [0, 5, 20] {
        group.bench_with_input(
            BenchmarkId::from_parameter(tool_count),
            &tool_count,
            |b, &tools| {
                let response = create_sample_completion_response(tools);

                b.iter(|| {
                    let json = serde_json::to_string(&response).unwrap();
                    criterion::black_box(json);
                });
            },
        );
    }

    group.finish();
}

fn bench_response_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("provider/response_deserialization");

    for tool_count in [0, 5, 20] {
        let response = create_sample_completion_response(tool_count);
        let json = serde_json::to_string(&response).unwrap();

        group.throughput(Throughput::Bytes(json.len() as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(tool_count),
            &json,
            |b, json_str| {
                b.iter(|| {
                    let resp: CompletionResponse = serde_json::from_str(json_str).unwrap();
                    criterion::black_box(resp);
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Tool Format Translation Benchmarks
// =============================================================================

fn bench_tool_format_translation(c: &mut Criterion) {
    let mut group = c.benchmark_group("provider/tool_translation");

    let tools: Vec<ToolDefinition> = (0..10)
        .map(|i| ToolDefinition {
            name: format!("tool_{}", i),
            description: format!("Description for tool {}", i),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "arg1": { "type": "string" },
                    "arg2": { "type": "number" },
                    "arg3": { "type": "boolean" }
                },
                "required": ["arg1"]
            }),
            strict: i % 2 == 0,
        })
        .collect();

    for format in [ToolFormat::OpenAi, ToolFormat::Anthropic, ToolFormat::Mcp] {
        let format_name = format!("{:?}", format).to_lowercase();

        group.bench_with_input(
            BenchmarkId::new("translate", &format_name),
            &tools,
            |b, tool_list| {
                b.iter(|| {
                    for tool in tool_list {
                        let translated = translate_tool_definition(tool, format);
                        criterion::black_box(translated);
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_parse_anthropic_tool_calls(c: &mut Criterion) {
    let mut group = c.benchmark_group("provider/parse_anthropic_tools");

    for tool_count in [1, 5, 20, 50] {
        let blocks = create_anthropic_content_blocks(tool_count);

        group.bench_with_input(
            BenchmarkId::from_parameter(tool_count),
            &blocks,
            |b, content_blocks| {
                b.iter(|| {
                    let calls = parse_anthropic_tool_calls(content_blocks);
                    criterion::black_box(calls);
                });
            },
        );
    }

    group.finish();
}

fn bench_parse_openai_tool_calls(c: &mut Criterion) {
    let mut group = c.benchmark_group("provider/parse_openai_tools");

    for tool_count in [1, 5, 20, 50] {
        let tool_calls = create_openai_tool_calls(tool_count);

        group.bench_with_input(
            BenchmarkId::from_parameter(tool_count),
            &tool_calls,
            |b, calls| {
                b.iter(|| {
                    let result = parse_openai_tool_calls(calls).unwrap();
                    criterion::black_box(result);
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Token Counting Benchmarks
// =============================================================================

fn bench_token_counting(c: &mut Criterion) {
    let mut group = c.benchmark_group("provider/token_counting");

    let bpe = p50k_base().unwrap();

    for text_size in [100, 1000, 10000, 100000] {
        group.throughput(Throughput::Bytes(text_size as u64));

        let text: String = (0..text_size)
            .map(|i| format!("word{} ", i % 1000))
            .collect();

        group.bench_with_input(BenchmarkId::new("tiktoken", text_size), &text, |b, txt| {
            b.iter(|| {
                let tokens = bpe.encode_with_special_tokens(txt);
                criterion::black_box(tokens.len());
            });
        });

        // Simple estimation for comparison
        group.bench_with_input(BenchmarkId::new("estimate", text_size), &text, |b, txt| {
            b.iter(|| {
                let estimate = txt.len() / 4;
                criterion::black_box(estimate);
            });
        });
    }

    group.finish();
}

fn bench_message_token_counting(c: &mut Criterion) {
    let mut group = c.benchmark_group("provider/message_token_counting");

    let bpe = p50k_base().unwrap();

    for msg_count in [10, 50, 100, 500] {
        let messages: Vec<Message> = (0..msg_count)
            .map(|i| {
                if i % 2 == 0 {
                    Message::user(format!("User message {} with content", i))
                } else {
                    Message::assistant(format!("Assistant response {}", i))
                }
            })
            .collect();

        group.throughput(Throughput::Elements(msg_count as u64));

        group.bench_with_input(
            BenchmarkId::new("tiktoken", msg_count),
            &messages,
            |b, msgs| {
                b.iter(|| {
                    let total: usize = msgs
                        .iter()
                        .map(|m| bpe.encode_with_special_tokens(&m.content).len())
                        .sum();
                    criterion::black_box(total);
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Streaming Simulation Benchmarks
// =============================================================================

fn bench_stream_chunk_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("provider/stream_processing");

    for chunk_count in [10, 100, 1000] {
        let chunks: Vec<StreamChunk> = (0..chunk_count)
            .map(|i| {
                if i % 10 == 0 {
                    StreamChunk::ToolCallDelta {
                        id: format!("call_{}", i / 10),
                        name: Some("test_tool".to_string()),
                        arguments_delta: format!("{{\"arg\":{}}}", i),
                    }
                } else {
                    StreamChunk::ContentDelta {
                        delta: format!("token{} ", i),
                    }
                }
            })
            .collect();

        group.throughput(Throughput::Elements(chunk_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(chunk_count),
            &chunks,
            |b, chunk_list| {
                b.iter(|| {
                    let mut full_content = String::new();
                    let mut tool_calls: HashMap<String, String> = HashMap::new();

                    for chunk in chunk_list {
                        match chunk {
                            StreamChunk::ContentDelta { delta } => {
                                full_content.push_str(delta);
                            }
                            StreamChunk::ToolCallDelta {
                                id,
                                name: _,
                                arguments_delta,
                            } => {
                                tool_calls
                                    .entry(id.clone())
                                    .or_default()
                                    .push_str(arguments_delta);
                            }
                            _ => {}
                        }
                    }

                    criterion::black_box((full_content, tool_calls));
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// JSON Value Manipulation Benchmarks
// =============================================================================

fn bench_json_value_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("provider/json_clone");

    let messages: Vec<_> = (0..50)
        .map(|i| {
            serde_json::json!({
                "role": if i % 2 == 0 { "user" } else { "assistant" },
                "content": format!("Message content {}", i)
            })
        })
        .collect();

    let tools: Vec<_> = (0..10)
        .map(|i| {
            serde_json::json!({
                "name": format!("tool_{}", i),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "arg": { "type": "string" }
                    }
                }
            })
        })
        .collect();

    let value = serde_json::json!({
        "messages": messages,
        "tools": tools
    });

    group.bench_function("large_value", |b| {
        b.iter(|| {
            let cloned = value.clone();
            criterion::black_box(cloned);
        });
    });

    group.finish();
}

// =============================================================================
// Criterion Groups
// =============================================================================

criterion_group!(
    provider_benches,
    bench_request_serialization,
    bench_request_deserialization,
    bench_response_serialization,
    bench_response_deserialization,
    bench_tool_format_translation,
    bench_parse_anthropic_tool_calls,
    bench_parse_openai_tool_calls,
    bench_token_counting,
    bench_message_token_counting,
    bench_stream_chunk_processing,
    bench_json_value_clone,
);

criterion_main!(provider_benches);
