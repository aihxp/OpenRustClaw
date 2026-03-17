//! Agent runtime benchmarks for OpenRustClaw.
//!
//! Measures:
//! - Tool registry lookup
//! - Message processing latency
//! - Context building
//! - Memory injection overhead

use std::collections::HashMap;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use openrustclaw_agent::{prompt::build_system_prompt, tools::ToolRegistry};
use openrustclaw_core::traits::ToolContext;
use openrustclaw_core::types::{
    CoreEntry, MemoryQuery, MemoryType, Message, Platform, Role, Session, ToolCall, ToolDefinition,
    ToolOutput,
};
use serde_json::json;
use uuid::Uuid;

// =============================================================================
// Tool Registry Benchmarks
// =============================================================================

fn bench_tool_registry_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent/tool_registry_lookup");

    for tool_count in [10, 50, 100, 500] {
        // Use DashMap or HashMap directly for benchmark
        let mut map: HashMap<String, String> = HashMap::with_capacity(tool_count);
        for i in 0..tool_count {
            map.insert(format!("tool_{}", i), format!("tool_impl_{}", i));
        }

        group.throughput(Throughput::Elements(tool_count as u64));

        group.bench_with_input(BenchmarkId::from_parameter(tool_count), &map, |b, m| {
            let keys: Vec<String> = (0..tool_count).map(|i| format!("tool_{}", i)).collect();
            let mut key_iter = keys.iter().cycle();

            b.iter(|| {
                let key = key_iter.next().unwrap();
                let result = m.get(key);
                criterion::black_box(result);
            });
        });
    }

    group.finish();
}

fn bench_tool_registry_definitions(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent/tool_registry_definitions");

    for tool_count in [5, 10, 20, 50] {
        // Create registry with tools
        let registry = ToolRegistry::new();

        group.bench_with_input(
            BenchmarkId::from_parameter(tool_count),
            &registry,
            |b, reg| {
                b.iter(|| {
                    let defs = reg.definitions();
                    criterion::black_box(defs);
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// System Prompt Building Benchmarks
// =============================================================================

fn bench_build_system_prompt(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent/build_system_prompt");

    for (memory_count, tool_count) in [(5, 5), (10, 20), (20, 50)] {
        let param = format!("{}mem_{}tools", memory_count, tool_count);

        let core_memory: Vec<CoreEntry> = (0..memory_count)
            .map(|i| CoreEntry {
                key: format!("key_{}", i),
                value: format!("This is the value for key {} with some content", i),
                importance: 0.5 + (i as f32 / memory_count as f32) * 0.5,
                token_count: 20,
                updated_at: chrono::Utc::now(),
            })
            .collect();

        let tools: Vec<ToolDefinition> = (0..tool_count)
            .map(|i| ToolDefinition {
                name: format!("tool_{}", i),
                description: format!("This is tool {} that does something useful", i),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "arg1": { "type": "string" },
                        "arg2": { "type": "number" }
                    }
                }),
                strict: false,
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("prompt", &param),
            &(&core_memory, &tools),
            |b, (mem, t)| {
                b.iter(|| {
                    let prompt = build_system_prompt("OpenRustClaw", mem, t);
                    criterion::black_box(prompt);
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Message Processing Benchmarks
// =============================================================================

fn bench_message_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent/message_processing");

    for msg_count in [10, 50, 100, 500] {
        let messages: Vec<Message> = (0..msg_count)
            .map(|i| {
                if i % 3 == 0 {
                    Message::user(format!("User question number {}", i))
                } else if i % 3 == 1 {
                    Message::assistant(format!("Assistant response to question {}", i))
                } else {
                    let mut msg = Message::new(Role::Tool, format!("Tool result for call {}", i));
                    msg.tool_call_id = Some(format!("call_{}", i));
                    msg
                }
            })
            .collect();

        group.throughput(Throughput::Elements(msg_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(msg_count),
            &messages,
            |b, msgs| {
                b.iter(|| {
                    // Simulate message processing:
                    // 1. Filter relevant messages
                    // 2. Extract tool calls
                    // 3. Build conversation context
                    let user_msgs: usize =
                        msgs.iter().filter(|m| matches!(m.role, Role::User)).count();
                    let tool_calls: usize = msgs
                        .iter()
                        .filter_map(|m| m.tool_calls.as_ref())
                        .flatten()
                        .count();

                    criterion::black_box((user_msgs, tool_calls));
                });
            },
        );
    }

    group.finish();
}

fn bench_message_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent/message_clone");

    for content_size in [100, 1000, 10000] {
        let message = Message::user("x".repeat(content_size));

        group.throughput(Throughput::Bytes(content_size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(content_size),
            &message,
            |b, msg| {
                b.iter(|| {
                    let cloned = msg.clone();
                    criterion::black_box(cloned);
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Context Building Benchmarks
// =============================================================================

fn bench_context_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent/context_building");

    for (msg_count, core_mem_count) in [(10, 5), (50, 10), (100, 20)] {
        let param = format!("{}msgs_{}core", msg_count, core_mem_count);

        let messages: Vec<Message> = (0..msg_count)
            .map(|i| Message::user(format!("Message {} with some content here", i)))
            .collect();

        let core_memory: Vec<CoreEntry> = (0..core_mem_count)
            .map(|i| CoreEntry {
                key: format!("preference_{}", i),
                value: format!("User preference value {}", i),
                importance: 0.8,
                token_count: 15,
                updated_at: chrono::Utc::now(),
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("build", &param),
            &(&messages, &core_memory),
            |b, (msgs, mem)| {
                b.iter(|| {
                    // Simulate building conversation context
                    let mut context = String::with_capacity(4096);

                    // Add core memory
                    context.push_str("[Core Memory]\n");
                    for entry in mem.iter() {
                        context.push_str(&format!("{}: {}\n", entry.key, entry.value));
                    }
                    context.push('\n');

                    // Add messages
                    context.push_str("[Conversation]\n");
                    for msg in msgs.iter() {
                        context.push_str(&format!(
                            "{:?}: {}\n",
                            msg.role,
                            &msg.content[..msg.content.len().min(100)]
                        ));
                    }

                    criterion::black_box(context);
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Session Management Benchmarks
// =============================================================================

fn bench_session_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent/session_creation");

    group.bench_function("new_dm", |b| {
        b.iter(|| {
            let session = Session::new_dm(format!("user_{}", Uuid::new_v4()), Platform::WebChat);
            criterion::black_box(session);
        });
    });

    group.bench_function("new_with_metadata", |b| {
        b.iter(|| {
            let mut session =
                Session::new_dm(format!("user_{}", Uuid::new_v4()), Platform::WebChat);
            session.workspace_id = Some("workspace_123".to_string());
            session.metadata = json!({
                "theme": "dark",
                "language": "en",
                "preferences": {
                    "notifications": true,
                    "sound": false
                }
            });
            criterion::black_box(session);
        });
    });

    group.finish();
}

// =============================================================================
// Tool Call Processing Benchmarks
// =============================================================================

fn bench_tool_call_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent/tool_call_parsing");

    for call_count in [1, 5, 10, 20] {
        let tool_calls: Vec<ToolCall> = (0..call_count)
            .map(|i| ToolCall {
                id: format!("call_{}", i),
                name: format!("tool_{}", i % 5),
                arguments: json!({
                    "arg1": format!("value_{}", i),
                    "arg2": i as i64,
                    "arg3": i % 2 == 0
                }),
            })
            .collect();

        group.throughput(Throughput::Elements(call_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(call_count),
            &tool_calls,
            |b, calls| {
                b.iter(|| {
                    for call in calls {
                        // Simulate parsing and validation
                        let _name = &call.name;
                        let args = &call.arguments;
                        let _arg1 = args.get("arg1").and_then(|v| v.as_str());
                        let _arg2 = args.get("arg2").and_then(|v| v.as_i64());
                        criterion::black_box((_arg1, _arg2));
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_tool_output_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent/tool_output_creation");

    for content_size in [100, 1000, 10000] {
        let content = "x".repeat(content_size);

        group.throughput(Throughput::Bytes(content_size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(content_size),
            &content,
            |b, txt| {
                b.iter(|| {
                    let output = ToolOutput {
                        tool_call_id: "call_123".to_string(),
                        content: txt.clone(),
                        is_error: false,
                    };
                    criterion::black_box(output);
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Memory Tools Benchmarks
// =============================================================================

fn bench_memory_query_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent/memory_query_building");

    for query_len in [10, 50, 100, 500] {
        let text: String = (0..query_len).map(|i| format!("word{} ", i)).collect();

        group.throughput(Throughput::Bytes(query_len as u64));

        group.bench_with_input(BenchmarkId::from_parameter(query_len), &text, |b, txt| {
            b.iter(|| {
                let query = MemoryQuery {
                    text: txt.clone(),
                    memory_types: vec![MemoryType::Semantic, MemoryType::Episodic],
                    source_types: vec![],
                    namespace: Some("default".to_string()),
                    limit: 10,
                    min_confidence: 0.5,
                    recency_weight: 0.3,
                };
                criterion::black_box(query);
            });
        });
    }

    group.finish();
}

// =============================================================================
// Tool Context Benchmarks
// =============================================================================

fn bench_tool_context_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent/tool_context_clone");

    let ctx = ToolContext {
        session_id: Uuid::new_v4().to_string(),
        user_id: "user_123".to_string(),
        workspace_path: Some("/path/to/workspace".to_string()),
    };

    group.bench_function("clone", |b| {
        b.iter(|| {
            let cloned = ctx.clone();
            criterion::black_box(cloned);
        });
    });

    group.finish();
}

// =============================================================================
// JSON Argument Processing Benchmarks
// =============================================================================

fn bench_json_argument_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent/json_argument_processing");

    // Test extracting various argument types
    let args = json!({
        "string_arg": "hello world",
        "number_arg": 42,
        "bool_arg": true,
        "array_arg": [1, 2, 3, 4, 5],
        "object_arg": {
            "nested": "value",
            "deep": {
                "deeper": "data"
            }
        }
    });

    group.bench_function("extract_all_types", |b| {
        b.iter(|| {
            let string_val = args.get("string_arg").and_then(|v| v.as_str());
            let number_val = args.get("number_arg").and_then(|v| v.as_i64());
            let bool_val = args.get("bool_arg").and_then(|v| v.as_bool());
            let array_len = args
                .get("array_arg")
                .and_then(|v| v.as_array())
                .map(|a| a.len());
            let nested = args
                .get("object_arg")
                .and_then(|v| v.as_object())
                .and_then(|o| o.get("deep"))
                .and_then(|v| v.as_object())
                .and_then(|o| o.get("deeper"))
                .and_then(|v| v.as_str());

            criterion::black_box((string_val, number_val, bool_val, array_len, nested));
        });
    });

    group.finish();
}

// =============================================================================
// Criterion Groups
// =============================================================================

criterion_group!(
    agent_benches,
    bench_tool_registry_lookup,
    bench_tool_registry_definitions,
    bench_build_system_prompt,
    bench_message_processing,
    bench_message_clone,
    bench_context_building,
    bench_session_creation,
    bench_tool_call_parsing,
    bench_tool_output_creation,
    bench_memory_query_building,
    bench_tool_context_clone,
    bench_json_argument_processing,
);

criterion_main!(agent_benches);
