//! Gateway benchmarks for OpenRustClaw.
//!
//! Measures:
//! - WebSocket message throughput
//! - Connection handling
//! - Auth validation speed

use std::collections::HashMap;

use criterion::{
    criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
};
use openrustclaw_core::types::{SessionType, Platform};
use openrustclaw_gateway::{auth, sessions::SessionManager};
use tokio::runtime::Runtime;

// =============================================================================
// Auth Validation Benchmarks
// =============================================================================

fn bench_auth_token_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("gateway/auth_token_extraction");

    // Valid token
    let valid_header = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

    group.bench_function("valid_token", |b| {
        b.iter(|| {
            let result = auth::extract_token(Some(valid_header));
            criterion::black_box(result);
        });
    });

    // Missing Bearer prefix
    let invalid_header = "Basic dXNlcjpwYXNz";

    group.bench_function("invalid_prefix", |b| {
        b.iter(|| {
            let result = auth::extract_token(Some(invalid_header));
            criterion::black_box(result);
        });
    });

    // Missing header
    group.bench_function("missing_header", |b| {
        b.iter(|| {
            let result: Result<String, _> = auth::extract_token(None);
            criterion::black_box(result);
        });
    });

    group.finish();
}

fn bench_auth_token_validation_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("gateway/auth_token_validation");

    // Simulate JWT validation without actual crypto (for benchmark consistency)
    let valid_tokens: Vec<String> = (0..100)
        .map(|i| format!("valid_token_{}", i))
        .collect();

    let valid_set: std::collections::HashSet<&str> = valid_tokens
        .iter()
        .map(|s| s.as_str())
        .collect();

    group.bench_function("hashset_lookup", |b| {
        let mut i = 0;
        b.iter(|| {
            let token = &valid_tokens[i % valid_tokens.len()];
            let is_valid = valid_set.contains(token.as_str());
            i += 1;
            criterion::black_box(is_valid);
        });
    });

    // Simulate with some invalid tokens
    let mixed_tokens: Vec<String> = (0..100)
        .map(|i| {
            if i % 3 == 0 {
                format!("invalid_token_{}", i)
            } else {
                format!("valid_token_{}", i)
            }
        })
        .collect();

    group.bench_function("mixed_lookup", |b| {
        let mut i = 0;
        b.iter(|| {
            let token = &mixed_tokens[i % mixed_tokens.len()];
            let is_valid = valid_set.contains(token.as_str());
            i += 1;
            criterion::black_box(is_valid);
        });
    });

    group.finish();
}

// =============================================================================
// Session Management Benchmarks
// =============================================================================

fn bench_session_manager_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("gateway/session_manager_get");
    let rt = Runtime::new().unwrap();

    for session_count in [10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(session_count),
            &session_count,
            |b, &count| {
                let manager = SessionManager::new();
                let session_ids: Vec<String> = rt.block_on(async {
                    let mut ids = Vec::with_capacity(count);
                    for i in 0..count {
                        let session = manager
                            .create_session(
                                &format!("user_{}", i),
                                SessionType::Dm,
                                Platform::WebChat,
                            )
                            .await
                            .unwrap();
                        ids.push(session.id.to_string());
                    }
                    ids
                });

                b.to_async(&rt).iter(|| async {
                    // Random access pattern - use a static index to avoid capture issues
                    let idx = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as usize % session_ids.len();
                    let result = manager.get_session(&session_ids[idx]).await;
                    criterion::black_box(result);
                });
            },
        );
    }

    group.finish();
}

fn bench_session_manager_create(c: &mut Criterion) {
    let mut group = c.benchmark_group("gateway/session_manager_create");
    let rt = Runtime::new().unwrap();

    group.bench_function("create_dm", |b| {
        b.to_async(&rt).iter(|| async {
            let manager = SessionManager::new();
            let session = manager
                .create_session(
                    &format!("user_{}", uuid::Uuid::new_v4()),
                    SessionType::Dm,
                    Platform::WebChat,
                )
                .await
                .unwrap();
            criterion::black_box(session);
        });
    });

    group.bench_function("create_group", |b| {
        b.to_async(&rt).iter(|| async {
            let manager = SessionManager::new();
            let session = manager
                .create_session(
                    &format!("user_{}", uuid::Uuid::new_v4()),
                    SessionType::Group,
                    Platform::Discord,
                )
                .await
                .unwrap();
            criterion::black_box(session);
        });
    });

    group.finish();
}

// =============================================================================
// Message Serialization Benchmarks
// =============================================================================

fn bench_websocket_message_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("gateway/ws_message_serialize");

    // Simple text message
    let text_msg = serde_json::json!({
        "type": "message",
        "payload": {
            "content": "Hello, world!",
            "timestamp": "2024-01-01T00:00:00Z"
        }
    });

    group.bench_function("simple_text", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&text_msg).unwrap();
            criterion::black_box(json);
        });
    });

    // Complex message with nested data
    let complex_msg = serde_json::json!({
        "type": "chat_message",
        "payload": {
            "id": uuid::Uuid::new_v4().to_string(),
            "session_id": uuid::Uuid::new_v4().to_string(),
            "user_id": "user_123",
            "content": "This is a longer message with more content that needs to be processed",
            "timestamp": "2024-01-01T00:00:00Z",
            "metadata": {
                "platform": "web_chat",
                "client_version": "1.0.0",
                "features": ["streaming", "tools", "memory"]
            }
        }
    });

    group.bench_function("complex_message", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&complex_msg).unwrap();
            criterion::black_box(json);
        });
    });

    // Batch message
    let messages: Vec<_> = (0..50)
        .map(|i| serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "content": format!("Message {}", i),
            "timestamp": "2024-01-01T00:00:00Z"
        }))
        .collect();

    let batch_msg = serde_json::json!({
        "type": "batch",
        "payload": messages
    });

    group.throughput(Throughput::Elements(50));

    group.bench_function("batch_50_messages", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&batch_msg).unwrap();
            criterion::black_box(json);
        });
    });

    group.finish();
}

fn bench_websocket_message_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("gateway/ws_message_deserialize");

    let json_text = r#"{"type":"message","payload":{"content":"Hello","timestamp":"2024-01-01T00:00:00Z"}}"#;

    group.throughput(Throughput::Bytes(json_text.len() as u64));

    group.bench_function("simple_text", |b| {
        b.iter(|| {
            let msg: serde_json::Value = serde_json::from_str(json_text).unwrap();
            criterion::black_box(msg);
        });
    });

    // Larger message
    let large_json = serde_json::to_string(&serde_json::json!({
        "type": "chat_message",
        "payload": {
            "id": uuid::Uuid::new_v4().to_string(),
            "session_id": uuid::Uuid::new_v4().to_string(),
            "user_id": "user_123",
            "content": "x".repeat(1000),
            "timestamp": "2024-01-01T00:00:00Z",
            "metadata": {
                "platform": "web_chat",
                "features": ["streaming", "tools", "memory"]
            }
        }
    })).unwrap();

    group.throughput(Throughput::Bytes(large_json.len() as u64));

    group.bench_function("large_message", |b| {
        b.iter(|| {
            let msg: serde_json::Value = serde_json::from_str(&large_json).unwrap();
            criterion::black_box(msg);
        });
    });

    group.finish();
}

// =============================================================================
// Connection Handling Benchmarks
// =============================================================================

fn bench_connection_id_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("gateway/connection_id_generation");

    group.bench_function("uuid_v4", |b| {
        b.iter(|| {
            let id = uuid::Uuid::new_v4().to_string();
            criterion::black_box(id);
        });
    });

    // Simulate nanoid-style generation
    group.bench_function("nanoid_style", |b| {
        const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        b.iter(|| {
            let id: String = (0..21)
                .map(|_| ALPHABET[rand::random::<usize>() % ALPHABET.len()] as char)
                .collect();
            criterion::black_box(id);
        });
    });

    group.finish();
}

// =============================================================================
// Rate Limit Simulation Benchmarks
// =============================================================================

fn bench_rate_limit_check(c: &mut Criterion) {
    let mut group = c.benchmark_group("gateway/rate_limit_check");

    use dashmap::DashMap;
    use std::time::{Duration, Instant};

    let limits: DashMap<String, (u64, Instant)> = DashMap::with_capacity(1000);

    // Pre-populate with rate limit entries
    for i in 0..1000 {
        limits.insert(
            format!("user_{}", i),
            (0, Instant::now()),
        );
    }

    group.bench_function("check_and_increment", |b| {
        let mut i = 0;
        b.iter(|| {
            let user_id = format!("user_{}", i % 1000);
            i += 1;

            let allowed = match limits.get_mut(&user_id) {
                Some(mut entry) => {
                    let (count, reset_time) = entry.value_mut();
                    if Instant::now() > *reset_time {
                        *count = 1;
                        *reset_time = Instant::now() + Duration::from_secs(60);
                        true
                    } else if *count < 100 {
                        *count += 1;
                        true
                    } else {
                        false
                    }
                }
                None => {
                    limits.insert(user_id, (1, Instant::now() + Duration::from_secs(60)));
                    true
                }
            };

            criterion::black_box(allowed);
        });
    });

    group.finish();
}

// =============================================================================
// Session Lookup Benchmarks
// =============================================================================

fn bench_session_lookup_by_user(c: &mut Criterion) {
    let mut group = c.benchmark_group("gateway/session_lookup_by_user");
    let rt = Runtime::new().unwrap();

    // Build index of user_id -> session_ids
    let user_sessions: HashMap<String, Vec<String>> = rt.block_on(async {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        let manager = SessionManager::new();

        for i in 0..100 {
            let user_id = format!("user_{}", i % 20);
            let session = manager
                .create_session(&user_id, SessionType::Dm, Platform::WebChat)
                .await
                .unwrap();

            map.entry(user_id)
                .or_default()
                .push(session.id.to_string());
        }

        map
    });

    group.bench_function("lookup", |b| {
        let user_ids: Vec<String> = (0..20).map(|i| format!("user_{}", i)).collect();
        let mut i = 0;

        b.iter(|| {
            let user_id = &user_ids[i % user_ids.len()];
            i += 1;
            let sessions = user_sessions.get(user_id);
            criterion::black_box(sessions);
        });
    });

    group.finish();
}

// =============================================================================
// Message Routing Benchmarks
// =============================================================================

fn bench_message_routing(c: &mut Criterion) {
    let mut group = c.benchmark_group("gateway/message_routing");

    // Simulate routing table
    let routes: HashMap<String, String> = (0..100)
        .map(|i| (format!("session_{}", i), format!("handler_{}", i % 10)))
        .collect();

    group.bench_function("route_lookup", |b| {
        let mut i = 0;
        b.iter(|| {
            let session_id = format!("session_{}", i % 100);
            i += 1;
            let handler = routes.get(&session_id);
            criterion::black_box(handler);
        });
    });

    // Simulate platform routing
    let platform_routes: HashMap<Platform, Vec<String>> = [
        (Platform::WebChat, vec!["handler_1".to_string(), "handler_2".to_string()]),
        (Platform::Discord, vec!["handler_3".to_string()]),
        (Platform::Slack, vec!["handler_4".to_string(), "handler_5".to_string()]),
    ]
    .into_iter()
    .collect();

    group.bench_function("platform_route", |b| {
        let platforms = [Platform::WebChat, Platform::Discord, Platform::Slack];
        let mut i = 0;

        b.iter(|| {
            let platform = platforms[i % platforms.len()];
            i += 1;
            let handlers = platform_routes.get(&platform);
            criterion::black_box(handlers);
        });
    });

    group.finish();
}

// =============================================================================
// WebSocket Frame Processing Benchmarks
// =============================================================================

fn bench_frame_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("gateway/frame_processing");

    // Simulate frame parsing
    let frame_data = "{\"type\":\"ping\",\"timestamp\":1234567890}";

    group.bench_function("parse_ping", |b| {
        b.iter(|| {
            let msg: serde_json::Value = serde_json::from_str(frame_data).unwrap();
            let msg_type = msg.get("type").and_then(|v| v.as_str());
            criterion::black_box(msg_type);
        });
    });

    // Simulate pong response generation
    group.bench_function("generate_pong", |b| {
        b.iter(|| {
            let response = serde_json::json!({
                "type": "pong",
                "timestamp": 1234567890
            });
            let json = serde_json::to_string(&response).unwrap();
            criterion::black_box(json);
        });
    });

    group.finish();
}

// =============================================================================
// Criterion Groups
// =============================================================================

criterion_group!(
    gateway_benches,
    bench_auth_token_extraction,
    bench_auth_token_validation_simulation,
    bench_session_manager_get,
    bench_session_manager_create,
    bench_websocket_message_serialize,
    bench_websocket_message_deserialize,
    bench_connection_id_generation,
    bench_rate_limit_check,
    bench_session_lookup_by_user,
    bench_message_routing,
    bench_frame_processing,
);

criterion_main!(gateway_benches);
