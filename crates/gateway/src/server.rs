//! Axum WebSocket server with observability integration.

use crate::auth::extract_token;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::header::HeaderName;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{
    extract::Path,
    Json, Router,
    routing::{get, post},
};
use futures::StreamExt;
use openrustclaw_core::error::{Error, Result as CoreResult, SecurityError};
use openrustclaw_core::traits::{CoreMemoryStore, MemoryStore};
use openrustclaw_core::types::{MemoryEntry, MemoryQuery, MemoryType, SourceType};
use openrustclaw_db::{SqliteCoreMemoryStore, SqliteMemoryStore};
use openrustclaw_observability::metrics::{
    SimpleTimer, decrement_active_connections, increment_active_connections,
    record_websocket_message,
};
use openrustclaw_security::OriginValidator;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};
use uuid::Uuid;

/// Shared gateway state.
#[derive(Clone)]
pub struct GatewayState {
    pub session_manager: Arc<crate::sessions::SessionManager>,
    pub origin_validator: Arc<OriginValidator>,
    pub require_auth: bool,
    pub internal_api_token: Option<Arc<String>>,
    pub memory_store: Option<Arc<SqliteMemoryStore>>,
    pub core_memory_store: Option<Arc<SqliteCoreMemoryStore>>,
}

/// The gateway WebSocket server.
pub struct GatewayServer {
    host: String,
    port: u16,
}

impl GatewayServer {
    pub fn new(host: String, port: u16) -> Self {
        Self { host, port }
    }

    /// Build the Axum router.
    pub fn router(&self, state: GatewayState) -> Router {
        Router::new()
            .route("/ws", get(ws_handler))
            .route(
                "/health",
                get(health_handler)
                    .post(health_handler)
                    .options(cors_preflight_handler),
            )
            .route("/v1/chat/completions", post(chat_completions_handler))
            .route("/internal/memory/search", post(internal_memory_search_handler))
            .route("/internal/memory/store", post(internal_memory_store_handler))
            .route(
                "/internal/memory/archive/store",
                post(internal_memory_archive_store_handler),
            )
            .route(
                "/internal/memory/archive/delete",
                post(internal_memory_archive_delete_handler),
            )
            .route(
                "/internal/memory/maintenance/old",
                post(internal_memory_maintenance_old_handler),
            )
            .route(
                "/internal/memory/core/{user_id}",
                post(internal_core_memory_render_handler),
            )
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods([
                        axum::http::Method::GET,
                        axum::http::Method::POST,
                        axum::http::Method::OPTIONS,
                    ])
                    .allow_headers(Any),
            )
            .with_state(state)
    }

    /// Get the bind address.
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Response {
    let timer = SimpleTimer::new();

    if let Err(err) = validate_ws_request(&state, &headers) {
        warn!(error = %err, "Rejected websocket connection");
        openrustclaw_observability::metrics::record_request(
            "websocket",
            "/ws",
            match err {
                Error::Security(SecurityError::AuthRequired) => "401",
                Error::Security(SecurityError::TokenInvalid(_)) => "401",
                Error::Security(SecurityError::InvalidOrigin { .. }) => "403",
                _ => "400",
            },
        );
        openrustclaw_observability::metrics::record_request_duration(
            "websocket",
            "/ws",
            timer.elapsed_secs(),
        );
        return gateway_error_response(err);
    }

    // Track successful connection
    increment_active_connections();

    ws.on_upgrade(move |socket| {
        async move {
            handle_socket(socket).await;
            // Connection closed
            decrement_active_connections();
        }
    })
}

async fn health_handler() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "service": "openrustclaw-gateway",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn cors_preflight_handler() -> StatusCode {
    StatusCode::OK
}

async fn chat_completions_handler(
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let content = payload
        .get("messages")
        .and_then(|messages| messages.as_array())
        .and_then(|messages| {
            messages.iter().rev().find_map(|message| {
                let role = message.get("role").and_then(|role| role.as_str());
                if role == Some("user") {
                    message.get("content").and_then(|content| content.as_str())
                } else {
                    None
                }
            })
        })
        .unwrap_or("");

    Json(json!({
        "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": payload.get("model").and_then(|model| model.as_str()).unwrap_or("openrustclaw-mock"),
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": format!("Echo: {}", content),
            },
            "finish_reason": "stop",
        }],
        "usage": {
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "total_tokens": 0,
        }
    }))
}

#[derive(serde::Deserialize)]
struct InternalMemorySearchRequest {
    user_id: String,
    query: String,
    limit: Option<usize>,
}

#[derive(serde::Deserialize)]
struct InternalMemoryStoreRequest {
    user_id: String,
    content: String,
    category: Option<String>,
    importance: Option<f32>,
    session_id: Option<String>,
}

#[derive(serde::Deserialize)]
struct InternalMemoryArchiveStoreRequest {
    id: String,
    summary: String,
    #[serde(default)]
    source_memory_ids: Vec<String>,
    namespace: Option<String>,
    importance: Option<f32>,
    source_type: Option<String>,
}

#[derive(serde::Deserialize)]
struct InternalMemoryArchiveDeleteRequest {
    #[serde(default)]
    memory_ids: Vec<String>,
}

#[derive(serde::Deserialize)]
struct InternalMemoryMaintenanceOldRequest {
    age_days: Option<i64>,
    namespace: Option<String>,
    user_id: Option<String>,
    limit: Option<usize>,
}

async fn internal_memory_search_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<InternalMemorySearchRequest>,
) -> Response {
    if let Err(response) = validate_internal_api(&state, &headers) {
        return response;
    }

    let Some(memory_store) = &state.memory_store else {
        return (StatusCode::SERVICE_UNAVAILABLE, "memory store unavailable").into_response();
    };

    let query = MemoryQuery {
        text: payload.query,
        memory_types: vec![],
        source_types: vec![],
        namespace: Some(payload.user_id),
        limit: payload.limit.unwrap_or(5),
        min_confidence: 0.0,
        recency_weight: 0.0,
    };

    match memory_store.search(&query).await {
        Ok(results) => Json(json!({
            "memories": results.into_iter().map(|scored| json!({
                "id": scored.entry.id,
                "content": scored.entry.content,
                "score": scored.score,
                "importance": scored.entry.importance,
            })).collect::<Vec<_>>()
        }))
        .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("memory search failed: {}", error),
        )
            .into_response(),
    }
}

async fn internal_memory_store_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<InternalMemoryStoreRequest>,
) -> Response {
    if let Err(response) = validate_internal_api(&state, &headers) {
        return response;
    }

    let Some(memory_store) = &state.memory_store else {
        return (StatusCode::SERVICE_UNAVAILABLE, "memory store unavailable").into_response();
    };

    let category = payload.category.unwrap_or_else(|| "semantic".to_string());
    let memory_type = match category.as_str() {
        "episodic" => MemoryType::Episodic,
        "procedural" => MemoryType::Procedural,
        _ => MemoryType::Semantic,
    };

    let mut hasher = Sha256::new();
    hasher.update(payload.content.as_bytes());
    let content_hash = hex::encode(hasher.finalize());

    let entry = MemoryEntry {
        id: Uuid::new_v4(),
        memory_type,
        content: payload.content,
        content_hash,
        source: Some("sidecar_memory_bridge".to_string()),
        source_type: Some(SourceType::Conversation),
        session_id: payload
            .session_id
            .as_deref()
            .and_then(|value| Uuid::parse_str(value).ok()),
        user_id: Some(payload.user_id.clone()),
        namespace: payload.user_id,
        importance: payload.importance.unwrap_or(0.7).clamp(0.0, 1.0),
        confidence: 1.0,
        access_count: 0,
        last_accessed: None,
        created_at: chrono::Utc::now(),
        expires_at: None,
        metadata: json!({
            "source": "sidecar_memory_bridge",
            "category": category,
        }),
    };

    let id = entry.id;
    match memory_store.store(entry).await {
        Ok(()) => Json(json!({"stored": true, "id": id})).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("memory store failed: {}", error),
        )
            .into_response(),
    }
}

async fn internal_core_memory_render_handler(
    Path(user_id): Path<String>,
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = validate_internal_api(&state, &headers) {
        return response;
    }

    let Some(core_memory_store) = &state.core_memory_store else {
        return (StatusCode::SERVICE_UNAVAILABLE, "core memory store unavailable").into_response();
    };

    match core_memory_store.render(&user_id).await {
        Ok(content) => Json(json!({ "content": content })).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("core memory render failed: {}", error),
        )
            .into_response(),
    }
}

async fn internal_memory_archive_store_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<InternalMemoryArchiveStoreRequest>,
) -> Response {
    if let Err(response) = validate_internal_api(&state, &headers) {
        return response;
    }

    let Some(memory_store) = &state.memory_store else {
        return (StatusCode::SERVICE_UNAVAILABLE, "memory store unavailable").into_response();
    };

    match memory_store
        .store_archive_entry(
            &payload.id,
            &payload.summary,
            &payload.source_memory_ids,
            payload.namespace.as_deref(),
            payload.importance,
            payload.source_type.as_deref(),
        )
        .await
    {
        Ok(id) => Json(json!({"stored": true, "id": id})).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("memory archive store failed: {}", error),
        )
            .into_response(),
    }
}

async fn internal_memory_archive_delete_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<InternalMemoryArchiveDeleteRequest>,
) -> Response {
    if let Err(response) = validate_internal_api(&state, &headers) {
        return response;
    }

    let Some(memory_store) = &state.memory_store else {
        return (StatusCode::SERVICE_UNAVAILABLE, "memory store unavailable").into_response();
    };

    match memory_store.delete_many(&payload.memory_ids).await {
        Ok(deleted) => Json(json!({"deleted": deleted, "memory_ids": payload.memory_ids}))
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("memory archive delete failed: {}", error),
        )
            .into_response(),
    }
}

async fn internal_memory_maintenance_old_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<InternalMemoryMaintenanceOldRequest>,
) -> Response {
    if let Err(response) = validate_internal_api(&state, &headers) {
        return response;
    }

    let Some(memory_store) = &state.memory_store else {
        return (StatusCode::SERVICE_UNAVAILABLE, "memory store unavailable").into_response();
    };

    let age_days = payload.age_days.unwrap_or(30).max(0);
    let cutoff = chrono::Utc::now() - chrono::Duration::days(age_days);

    match memory_store
        .list_old_episodic_memories(
            cutoff,
            payload.namespace.as_deref(),
            payload.user_id.as_deref(),
            payload.limit.unwrap_or(100),
        )
        .await
    {
        Ok(entries) => Json(json!({
            "memories": entries.into_iter().map(|entry| json!({
                "id": entry.id,
                "content": entry.content,
                "timestamp": entry.created_at,
                "namespace": entry.namespace,
                "user_id": entry.user_id,
                "importance": entry.importance,
            })).collect::<Vec<_>>()
        }))
        .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("memory maintenance fetch failed: {}", error),
        )
            .into_response(),
    }
}

fn validate_ws_request(state: &GatewayState, headers: &HeaderMap) -> CoreResult<()> {
    let origin = headers
        .get(axum::http::header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            Error::Security(SecurityError::InvalidOrigin {
                origin: "<missing>".to_string(),
            })
        })?;
    state.origin_validator.validate(origin)?;

    if state.require_auth {
        let auth_header = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok());
        extract_token(auth_header)?;
    }

    Ok(())
}

fn validate_internal_api(state: &GatewayState, headers: &HeaderMap) -> std::result::Result<(), Response> {
    let Some(expected_token) = &state.internal_api_token else {
        return Err((StatusCode::SERVICE_UNAVAILABLE, "internal api disabled").into_response());
    };

    let header_name = HeaderName::from_static("x-openrustclaw-internal-token");
    let provided = headers
        .get(header_name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    if provided == expected_token.as_str() {
        Ok(())
    } else {
        Err((StatusCode::UNAUTHORIZED, "invalid internal api token").into_response())
    }
}

fn gateway_error_response(err: Error) -> Response {
    let (status, message) = match err {
        Error::Security(SecurityError::AuthRequired) => (
            StatusCode::UNAUTHORIZED,
            "authorization required".to_string(),
        ),
        Error::Security(SecurityError::TokenInvalid(message)) => {
            (StatusCode::UNAUTHORIZED, message)
        }
        Error::Security(SecurityError::InvalidOrigin { origin }) => {
            (StatusCode::FORBIDDEN, format!("invalid origin: {origin}"))
        }
        other => (StatusCode::BAD_REQUEST, other.to_string()),
    };

    (status, message).into_response()
}

async fn handle_socket(mut socket: WebSocket) {
    let hello = json!({
        "type": "connected",
        "message": "WebSocket connection established",
    });

    info!("WebSocket connection established");
    record_websocket_message("out", "text");

    if socket
        .send(Message::Text(hello.to_string().into()))
        .await
        .is_err()
    {
        return;
    }

    while let Some(Ok(message)) = socket.next().await {
        match message {
            Message::Close(_) => {
                record_websocket_message("in", "close");
                break;
            }
            Message::Ping(payload) => {
                record_websocket_message("in", "ping");
                record_websocket_message("out", "pong");
                if socket.send(Message::Pong(payload)).await.is_err() {
                    break;
                }
            }
            Message::Text(text) => {
                record_websocket_message("in", "text");
                let reply = json!({
                    "type": "ack",
                    "received": text.as_str(),
                });
                if socket
                    .send(Message::Text(reply.to_string().into()))
                    .await
                    .is_err()
                {
                    break;
                }
                record_websocket_message("out", "text");
            }
            Message::Binary(payload) => {
                record_websocket_message("in", "binary");
                if socket.send(Message::Binary(payload)).await.is_err() {
                    break;
                }
                record_websocket_message("out", "binary");
            }
            Message::Pong(_) => {
                record_websocket_message("in", "pong");
            }
        }
    }

    info!("WebSocket connection closed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use http::Request;
    use http::header::{CONTENT_TYPE, HeaderValue};
    use openrustclaw_core::traits::CoreMemoryStore;
    use openrustclaw_core::types::CoreEntry;
    use openrustclaw_db::{SqliteCoreMemoryStore, SqliteMemoryStore, init_pool, run_migrations};
    use tower::ServiceExt;

    fn test_state() -> GatewayState {
        GatewayState {
            session_manager: Arc::new(crate::sessions::SessionManager::new()),
            origin_validator: Arc::new(OriginValidator::new(vec![
                "http://localhost:3000".to_string(),
            ])),
            require_auth: true,
            internal_api_token: None,
            memory_store: None,
            core_memory_store: None,
        }
    }

    #[test]
    fn validate_ws_request_rejects_missing_origin() {
        let headers = HeaderMap::new();
        assert!(validate_ws_request(&test_state(), &headers).is_err());
    }

    #[test]
    fn validate_ws_request_rejects_missing_auth() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::ORIGIN,
            "http://localhost:3000".parse().unwrap(),
        );
        assert!(validate_ws_request(&test_state(), &headers).is_err());
    }

    #[test]
    fn validate_ws_request_accepts_valid_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::ORIGIN,
            "http://localhost:3000".parse().unwrap(),
        );
        headers.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer test-token".parse().unwrap(),
        );
        assert!(validate_ws_request(&test_state(), &headers).is_ok());
    }

    #[tokio::test]
    async fn internal_memory_api_store_search_and_render() {
        let db_path = std::env::temp_dir().join(format!("gateway-memory-{}.db", Uuid::new_v4()));
        let pool = init_pool(&format!("sqlite://{}", db_path.display()), 1)
            .await
            .unwrap();
        run_migrations(&pool).await.unwrap();

        let memory_store = Arc::new(SqliteMemoryStore::new(pool.clone()));
        let core_memory_store = Arc::new(SqliteCoreMemoryStore::new(pool.clone()));
        core_memory_store
            .set(
                "user-1",
                CoreEntry {
                    key: "preference".to_string(),
                    value: "Prefers Rust".to_string(),
                    importance: 0.9,
                    token_count: 4,
                    updated_at: chrono::Utc::now(),
                },
            )
            .await
            .unwrap();

        let state = GatewayState {
            session_manager: Arc::new(crate::sessions::SessionManager::new()),
            origin_validator: Arc::new(OriginValidator::new(vec![
                "http://localhost:3000".to_string(),
            ])),
            require_auth: false,
            internal_api_token: Some(Arc::new("test-token".to_string())),
            memory_store: Some(memory_store),
            core_memory_store: Some(core_memory_store),
        };

        let app = GatewayServer::new("127.0.0.1".to_string(), 0).router(state);

        let store_request = Request::builder()
            .method("POST")
            .uri("/internal/memory/store")
            .header(CONTENT_TYPE, "application/json")
            .header("x-openrustclaw-internal-token", "test-token")
            .body(Body::from(
                r#"{"user_id":"user-1","content":"Rust ownership matters","category":"semantic"}"#,
            ))
            .unwrap();
        let store_response = app.clone().oneshot(store_request).await.unwrap();
        assert_eq!(store_response.status(), StatusCode::OK);

        let search_request = Request::builder()
            .method("POST")
            .uri("/internal/memory/search")
            .header(CONTENT_TYPE, "application/json")
            .header("x-openrustclaw-internal-token", "test-token")
            .body(Body::from(r#"{"user_id":"user-1","query":"ownership","limit":3}"#))
            .unwrap();
        let search_response = app.clone().oneshot(search_request).await.unwrap();
        assert_eq!(search_response.status(), StatusCode::OK);
        let search_body = to_bytes(search_response.into_body(), usize::MAX).await.unwrap();
        let search_json: serde_json::Value = serde_json::from_slice(&search_body).unwrap();
        assert!(search_json["memories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["content"] == "Rust ownership matters"));

        let core_request = Request::builder()
            .method("POST")
            .uri("/internal/memory/core/user-1")
            .header("x-openrustclaw-internal-token", HeaderValue::from_static("test-token"))
            .body(Body::from("{}"))
            .unwrap();
        let core_response = app.oneshot(core_request).await.unwrap();
        assert_eq!(core_response.status(), StatusCode::OK);
        let core_body = to_bytes(core_response.into_body(), usize::MAX).await.unwrap();
        let core_json: serde_json::Value = serde_json::from_slice(&core_body).unwrap();
        assert!(core_json["content"]
            .as_str()
            .unwrap()
            .contains("Prefers Rust"));

        let _ = std::fs::remove_file(db_path);
    }

    #[tokio::test]
    async fn internal_memory_archive_and_maintenance_endpoints_work() {
        let db_path = std::env::temp_dir().join(format!("gateway-archive-{}.db", Uuid::new_v4()));
        let pool = init_pool(&format!("sqlite://{}", db_path.display()), 1)
            .await
            .unwrap();
        run_migrations(&pool).await.unwrap();

        let memory_store = Arc::new(SqliteMemoryStore::new(pool.clone()));
        let core_memory_store = Arc::new(SqliteCoreMemoryStore::new(pool.clone()));

        let old_entry = MemoryEntry {
            id: Uuid::new_v4(),
            memory_type: MemoryType::Episodic,
            content: "Old episodic memory".to_string(),
            content_hash: "hash-old".to_string(),
            source: Some("test".to_string()),
            source_type: Some(SourceType::Conversation),
            session_id: None,
            user_id: Some("user-archive".to_string()),
            namespace: "user-archive".to_string(),
            importance: 0.8,
            confidence: 1.0,
            access_count: 0,
            last_accessed: None,
            created_at: chrono::Utc::now() - chrono::Duration::days(90),
            expires_at: None,
            metadata: json!({}),
        };
        memory_store.store(old_entry.clone()).await.unwrap();

        let state = GatewayState {
            session_manager: Arc::new(crate::sessions::SessionManager::new()),
            origin_validator: Arc::new(OriginValidator::new(vec![
                "http://localhost:3000".to_string(),
            ])),
            require_auth: false,
            internal_api_token: Some(Arc::new("test-token".to_string())),
            memory_store: Some(memory_store),
            core_memory_store: Some(core_memory_store),
        };

        let app = GatewayServer::new("127.0.0.1".to_string(), 0).router(state);

        let old_request = Request::builder()
            .method("POST")
            .uri("/internal/memory/maintenance/old")
            .header(CONTENT_TYPE, "application/json")
            .header("x-openrustclaw-internal-token", "test-token")
            .body(Body::from(
                r#"{"user_id":"user-archive","namespace":"user-archive","age_days":30,"limit":5}"#,
            ))
            .unwrap();
        let old_response = app.clone().oneshot(old_request).await.unwrap();
        assert_eq!(old_response.status(), StatusCode::OK);
        let old_body = to_bytes(old_response.into_body(), usize::MAX).await.unwrap();
        let old_json: serde_json::Value = serde_json::from_slice(&old_body).unwrap();
        assert_eq!(old_json["memories"].as_array().unwrap().len(), 1);

        let archive_request = Request::builder()
            .method("POST")
            .uri("/internal/memory/archive/store")
            .header(CONTENT_TYPE, "application/json")
            .header("x-openrustclaw-internal-token", "test-token")
            .body(Body::from(format!(
                r#"{{"id":"archive-1","summary":"Archive summary","source_memory_ids":["{}"],"namespace":"user-archive","importance":0.8,"source_type":"conversation"}}"#,
                old_entry.id
            )))
            .unwrap();
        let archive_response = app.clone().oneshot(archive_request).await.unwrap();
        assert_eq!(archive_response.status(), StatusCode::OK);

        let delete_request = Request::builder()
            .method("POST")
            .uri("/internal/memory/archive/delete")
            .header(CONTENT_TYPE, "application/json")
            .header("x-openrustclaw-internal-token", "test-token")
            .body(Body::from(format!(
                r#"{{"memory_ids":["{}"]}}"#,
                old_entry.id
            )))
            .unwrap();
        let delete_response = app.clone().oneshot(delete_request).await.unwrap();
        assert_eq!(delete_response.status(), StatusCode::OK);

        let search_request = Request::builder()
            .method("POST")
            .uri("/internal/memory/search")
            .header(CONTENT_TYPE, "application/json")
            .header("x-openrustclaw-internal-token", "test-token")
            .body(Body::from(
                r#"{"user_id":"user-archive","query":"episodic","limit":5}"#,
            ))
            .unwrap();
        let search_response = app.oneshot(search_request).await.unwrap();
        let search_body = to_bytes(search_response.into_body(), usize::MAX).await.unwrap();
        let search_json: serde_json::Value = serde_json::from_slice(&search_body).unwrap();
        assert!(search_json["memories"].as_array().unwrap().is_empty());

        let archive_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memory_archive")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(archive_count, 1);

        let _ = std::fs::remove_file(db_path);
    }
}
