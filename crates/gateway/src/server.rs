//! Axum WebSocket server with observability integration.

use crate::auth::extract_token;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::header::HeaderName;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{
    Json, Router,
    extract::Path,
    routing::{get, post},
};
use futures::StreamExt;
use openrustclaw_core::error::{Error, Result as CoreResult, SecurityError};
use openrustclaw_core::traits::{CoreMemoryStore, MemoryStore};
use openrustclaw_core::types::{MemoryEntry, MemoryQuery, MemoryType, SourceType};
use openrustclaw_db::{RagChunkInput, SqliteCoreMemoryStore, SqliteMemoryStore, SqliteRagStore};
use openrustclaw_observability::LangSmithClient;
use openrustclaw_observability::langsmith::{RunType, TraceRun};
use openrustclaw_observability::metrics::{
    SimpleTimer, decrement_active_connections, increment_active_connections, record_auth_attempt,
    record_memory_maintenance, record_memory_maintenance_duration, record_origin_check,
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
    pub trusted_proxy_token: Option<Arc<String>>,
    pub memory_store: Option<Arc<SqliteMemoryStore>>,
    pub core_memory_store: Option<Arc<SqliteCoreMemoryStore>>,
    pub rag_store: Option<Arc<SqliteRagStore>>,
    pub langsmith: Option<LangSmithClient>,
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
            .route(
                "/internal/memory/search",
                post(internal_memory_search_handler),
            )
            .route(
                "/internal/memory/store",
                post(internal_memory_store_handler),
            )
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
            .route(
                "/internal/memory/core/set",
                post(internal_core_memory_set_handler),
            )
            .route("/internal/rag/store", post(internal_rag_store_handler))
            .route("/internal/rag/load", post(internal_rag_load_handler))
            .route("/internal/rag/list", post(internal_rag_list_handler))
            .route("/internal/rag/delete", post(internal_rag_delete_handler))
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
    State(state): State<GatewayState>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let mut trace = gateway_trace(
        state.langsmith.as_ref(),
        "gateway_chat_completions",
        RunType::Chain,
        json!({"request": payload.clone()}),
    );
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

    let response = json!({
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
    });
    complete_gateway_trace(
        state.langsmith.as_ref(),
        trace.as_mut(),
        Some(json!({"response": response.clone()})),
        None,
    )
    .await;
    Json(response)
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

#[derive(serde::Deserialize)]
struct InternalRagChunkRequest {
    id: String,
    source_id: Option<String>,
    content: String,
    #[serde(default)]
    metadata: serde_json::Value,
    chunk_index: Option<i64>,
}

#[derive(serde::Deserialize)]
struct InternalRagStoreRequest {
    collection_name: String,
    #[serde(default)]
    chunks: Vec<InternalRagChunkRequest>,
}

#[derive(serde::Deserialize)]
struct InternalRagLoadRequest {
    collection_name: String,
    limit: Option<usize>,
}

#[derive(serde::Deserialize)]
struct InternalRagListRequest {
    limit: Option<usize>,
}

#[derive(serde::Deserialize)]
struct InternalRagDeleteRequest {
    collection_name: String,
}

#[derive(serde::Deserialize)]
struct InternalCoreMemorySetRequest {
    user_id: String,
    key: String,
    value: String,
    importance: Option<f32>,
}

async fn internal_memory_search_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<InternalMemorySearchRequest>,
) -> Response {
    if let Err(response) = validate_internal_api(&state, &headers) {
        return response;
    }
    let mut trace = gateway_trace(
        state.langsmith.as_ref(),
        "internal_memory_search",
        RunType::Retriever,
        json!({
            "user_id": payload.user_id.clone(),
            "query": payload.query.clone(),
            "limit": payload.limit,
        }),
    );

    let Some(memory_store) = &state.memory_store else {
        complete_gateway_trace(
            state.langsmith.as_ref(),
            trace.as_mut(),
            None,
            Some("memory store unavailable".to_string()),
        )
        .await;
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
        Ok(results) => {
            let body = json!({
                "memories": results.into_iter().map(|scored| json!({
                    "id": scored.entry.id,
                    "content": scored.entry.content,
                    "score": scored.score,
                    "importance": scored.entry.importance,
                })).collect::<Vec<_>>()
            });
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(body.clone()),
                None,
            )
            .await;
            Json(body).into_response()
        }
        Err(error) => {
            let error_message = format!("memory search failed: {}", error);
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                None,
                Some(error_message.clone()),
            )
            .await;
            (StatusCode::INTERNAL_SERVER_ERROR, error_message).into_response()
        }
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
    let mut trace = gateway_trace(
        state.langsmith.as_ref(),
        "internal_memory_store",
        RunType::Tool,
        json!({
            "user_id": payload.user_id.clone(),
            "category": payload.category.clone(),
            "importance": payload.importance,
            "session_id": payload.session_id.clone(),
        }),
    );

    let Some(memory_store) = &state.memory_store else {
        complete_gateway_trace(
            state.langsmith.as_ref(),
            trace.as_mut(),
            None,
            Some("memory store unavailable".to_string()),
        )
        .await;
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
        Ok(()) => {
            let body = json!({"stored": true, "id": id});
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(body.clone()),
                None,
            )
            .await;
            Json(body).into_response()
        }
        Err(error) => {
            let error_message = format!("memory store failed: {}", error);
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                None,
                Some(error_message.clone()),
            )
            .await;
            (StatusCode::INTERNAL_SERVER_ERROR, error_message).into_response()
        }
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
    let mut trace = gateway_trace(
        state.langsmith.as_ref(),
        "internal_core_memory_render",
        RunType::Retriever,
        json!({"user_id": user_id}),
    );

    let Some(core_memory_store) = &state.core_memory_store else {
        complete_gateway_trace(
            state.langsmith.as_ref(),
            trace.as_mut(),
            None,
            Some("core memory store unavailable".to_string()),
        )
        .await;
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "core memory store unavailable",
        )
            .into_response();
    };

    match core_memory_store.render(&user_id).await {
        Ok(content) => {
            let body = json!({ "content": content });
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(body.clone()),
                None,
            )
            .await;
            Json(body).into_response()
        }
        Err(error) => {
            let error_message = format!("core memory render failed: {}", error);
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                None,
                Some(error_message.clone()),
            )
            .await;
            (StatusCode::INTERNAL_SERVER_ERROR, error_message).into_response()
        }
    }
}

async fn internal_core_memory_set_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<InternalCoreMemorySetRequest>,
) -> Response {
    if let Err(response) = validate_internal_api(&state, &headers) {
        return response;
    }
    let mut trace = gateway_trace(
        state.langsmith.as_ref(),
        "internal_core_memory_set",
        RunType::Tool,
        json!({
            "user_id": payload.user_id.clone(),
            "key": payload.key.clone(),
            "importance": payload.importance,
        }),
    );

    let Some(core_memory_store) = &state.core_memory_store else {
        complete_gateway_trace(
            state.langsmith.as_ref(),
            trace.as_mut(),
            None,
            Some("core memory store unavailable".to_string()),
        )
        .await;
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "core memory store unavailable",
        )
            .into_response();
    };

    let entry = openrustclaw_db::CoreEntryBuilder::new(&payload.key, &payload.value)
        .importance(payload.importance.unwrap_or(0.8).clamp(0.0, 1.0))
        .build();

    match core_memory_store.set(&payload.user_id, entry).await {
        Ok(()) => {
            let body = json!({
                "stored": true,
                "user_id": payload.user_id,
                "key": payload.key,
            });
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(body.clone()),
                None,
            )
            .await;
            Json(body).into_response()
        }
        Err(error) => {
            let error_message = format!("core memory set failed: {}", error);
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                None,
                Some(error_message.clone()),
            )
            .await;
            (StatusCode::INTERNAL_SERVER_ERROR, error_message).into_response()
        }
    }
}

async fn internal_memory_archive_store_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<InternalMemoryArchiveStoreRequest>,
) -> Response {
    let timer = SimpleTimer::new();
    if let Err(response) = validate_internal_api(&state, &headers) {
        return response;
    }
    let mut trace = gateway_trace(
        state.langsmith.as_ref(),
        "internal_memory_archive_store",
        RunType::Tool,
        json!({
            "archive_id": payload.id.clone(),
            "source_memory_ids": payload.source_memory_ids.clone(),
            "namespace": payload.namespace.clone(),
            "importance": payload.importance,
            "source_type": payload.source_type.clone(),
        }),
    );

    let Some(memory_store) = &state.memory_store else {
        complete_gateway_trace(
            state.langsmith.as_ref(),
            trace.as_mut(),
            None,
            Some("memory store unavailable".to_string()),
        )
        .await;
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
        Ok(id) => {
            record_memory_maintenance("archive_store", "success", 1);
            record_memory_maintenance_duration("archive_store", timer.elapsed_secs());
            let body = json!({"stored": true, "id": id});
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(body.clone()),
                None,
            )
            .await;
            Json(body).into_response()
        }
        Err(error) => {
            record_memory_maintenance("archive_store", "error", 0);
            record_memory_maintenance_duration("archive_store", timer.elapsed_secs());
            let error_message = format!("memory archive store failed: {}", error);
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                None,
                Some(error_message.clone()),
            )
            .await;
            (StatusCode::INTERNAL_SERVER_ERROR, error_message).into_response()
        }
    }
}

async fn internal_memory_archive_delete_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<InternalMemoryArchiveDeleteRequest>,
) -> Response {
    let timer = SimpleTimer::new();
    if let Err(response) = validate_internal_api(&state, &headers) {
        return response;
    }
    let mut trace = gateway_trace(
        state.langsmith.as_ref(),
        "internal_memory_archive_delete",
        RunType::Tool,
        json!({
            "memory_ids": payload.memory_ids.clone(),
        }),
    );

    let Some(memory_store) = &state.memory_store else {
        complete_gateway_trace(
            state.langsmith.as_ref(),
            trace.as_mut(),
            None,
            Some("memory store unavailable".to_string()),
        )
        .await;
        return (StatusCode::SERVICE_UNAVAILABLE, "memory store unavailable").into_response();
    };

    match memory_store.delete_many(&payload.memory_ids).await {
        Ok(deleted) => {
            record_memory_maintenance(
                "archive_delete",
                "success",
                usize::try_from(deleted).unwrap_or(usize::MAX),
            );
            record_memory_maintenance_duration("archive_delete", timer.elapsed_secs());
            let body = json!({"deleted": deleted, "memory_ids": payload.memory_ids});
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(body.clone()),
                None,
            )
            .await;
            Json(body).into_response()
        }
        Err(error) => {
            record_memory_maintenance("archive_delete", "error", 0);
            record_memory_maintenance_duration("archive_delete", timer.elapsed_secs());
            let error_message = format!("memory archive delete failed: {}", error);
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                None,
                Some(error_message.clone()),
            )
            .await;
            (StatusCode::INTERNAL_SERVER_ERROR, error_message).into_response()
        }
    }
}

async fn internal_memory_maintenance_old_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<InternalMemoryMaintenanceOldRequest>,
) -> Response {
    let timer = SimpleTimer::new();
    if let Err(response) = validate_internal_api(&state, &headers) {
        return response;
    }
    let mut trace = gateway_trace(
        state.langsmith.as_ref(),
        "internal_memory_maintenance_old",
        RunType::Retriever,
        json!({
            "age_days": payload.age_days,
            "namespace": payload.namespace.clone(),
            "user_id": payload.user_id.clone(),
            "limit": payload.limit,
        }),
    );

    let Some(memory_store) = &state.memory_store else {
        complete_gateway_trace(
            state.langsmith.as_ref(),
            trace.as_mut(),
            None,
            Some("memory store unavailable".to_string()),
        )
        .await;
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
        Ok(entries) => {
            record_memory_maintenance("fetch_old", "success", entries.len());
            record_memory_maintenance_duration("fetch_old", timer.elapsed_secs());
            let body = json!({
                "memories": entries.into_iter().map(|entry| json!({
                    "id": entry.id,
                    "content": entry.content,
                    "timestamp": entry.created_at,
                    "namespace": entry.namespace,
                    "user_id": entry.user_id,
                    "importance": entry.importance,
                })).collect::<Vec<_>>()
            });
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(body.clone()),
                None,
            )
            .await;
            Json(body).into_response()
        }
        Err(error) => {
            record_memory_maintenance("fetch_old", "error", 0);
            record_memory_maintenance_duration("fetch_old", timer.elapsed_secs());
            let error_message = format!("memory maintenance fetch failed: {}", error);
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                None,
                Some(error_message.clone()),
            )
            .await;
            (StatusCode::INTERNAL_SERVER_ERROR, error_message).into_response()
        }
    }
}

async fn internal_rag_store_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<InternalRagStoreRequest>,
) -> Response {
    if let Err(response) = validate_internal_api(&state, &headers) {
        return response;
    }
    let mut trace = gateway_trace(
        state.langsmith.as_ref(),
        "internal_rag_store",
        RunType::Tool,
        json!({
            "collection_name": payload.collection_name.clone(),
            "chunk_count": payload.chunks.len(),
        }),
    );

    let Some(rag_store) = &state.rag_store else {
        complete_gateway_trace(
            state.langsmith.as_ref(),
            trace.as_mut(),
            None,
            Some("rag store unavailable".to_string()),
        )
        .await;
        return (StatusCode::SERVICE_UNAVAILABLE, "rag store unavailable").into_response();
    };

    let chunks: Vec<RagChunkInput> = payload
        .chunks
        .into_iter()
        .map(|chunk| RagChunkInput {
            chunk_id: chunk.id.clone(),
            source_id: chunk.source_id.unwrap_or(chunk.id),
            chunk_index: chunk.chunk_index.unwrap_or(0),
            content: chunk.content,
            metadata: chunk.metadata,
        })
        .collect();

    match rag_store
        .replace_collection(&payload.collection_name, &chunks)
        .await
    {
        Ok(stored_chunks) => {
            let body = json!({
                "collection_name": payload.collection_name,
                "stored_chunks": stored_chunks,
            });
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(body.clone()),
                None,
            )
            .await;
            Json(body).into_response()
        }
        Err(error) => {
            let error_message = format!("rag store failed: {}", error);
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                None,
                Some(error_message.clone()),
            )
            .await;
            (StatusCode::INTERNAL_SERVER_ERROR, error_message).into_response()
        }
    }
}

async fn internal_rag_load_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<InternalRagLoadRequest>,
) -> Response {
    if let Err(response) = validate_internal_api(&state, &headers) {
        return response;
    }
    let mut trace = gateway_trace(
        state.langsmith.as_ref(),
        "internal_rag_load",
        RunType::Retriever,
        json!({
            "collection_name": payload.collection_name.clone(),
            "limit": payload.limit,
        }),
    );

    let Some(rag_store) = &state.rag_store else {
        complete_gateway_trace(
            state.langsmith.as_ref(),
            trace.as_mut(),
            None,
            Some("rag store unavailable".to_string()),
        )
        .await;
        return (StatusCode::SERVICE_UNAVAILABLE, "rag store unavailable").into_response();
    };

    match rag_store
        .load_collection(&payload.collection_name, payload.limit)
        .await
    {
        Ok(chunks) => {
            let body = json!({
                "collection_name": payload.collection_name,
                "chunks": chunks.into_iter().map(|chunk| json!({
                    "id": chunk.chunk_id,
                    "source_id": chunk.source_id,
                    "chunk_index": chunk.chunk_index,
                    "content": chunk.content,
                    "metadata": chunk.metadata,
                })).collect::<Vec<_>>(),
            });
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(body.clone()),
                None,
            )
            .await;
            Json(body).into_response()
        }
        Err(error) => {
            let error_message = format!("rag load failed: {}", error);
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                None,
                Some(error_message.clone()),
            )
            .await;
            (StatusCode::INTERNAL_SERVER_ERROR, error_message).into_response()
        }
    }
}

async fn internal_rag_list_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<InternalRagListRequest>,
) -> Response {
    if let Err(response) = validate_internal_api(&state, &headers) {
        return response;
    }
    let mut trace = gateway_trace(
        state.langsmith.as_ref(),
        "internal_rag_list",
        RunType::Retriever,
        json!({
            "limit": payload.limit,
        }),
    );

    let Some(rag_store) = &state.rag_store else {
        complete_gateway_trace(
            state.langsmith.as_ref(),
            trace.as_mut(),
            None,
            Some("rag store unavailable".to_string()),
        )
        .await;
        return (StatusCode::SERVICE_UNAVAILABLE, "rag store unavailable").into_response();
    };

    match rag_store.list_collection_stats(payload.limit).await {
        Ok(collections) => {
            let body = json!({
                "collections": collections.into_iter().map(|stats| json!({
                    "collection_name": stats.collection_name,
                    "chunk_count": stats.chunk_count,
                    "source_count": stats.source_count,
                    "total_content_bytes": stats.total_content_bytes,
                    "last_updated_at": stats.last_updated_at,
                })).collect::<Vec<_>>(),
            });
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(body.clone()),
                None,
            )
            .await;
            Json(body).into_response()
        }
        Err(error) => {
            let error_message = format!("rag list failed: {}", error);
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                None,
                Some(error_message.clone()),
            )
            .await;
            (StatusCode::INTERNAL_SERVER_ERROR, error_message).into_response()
        }
    }
}

async fn internal_rag_delete_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<InternalRagDeleteRequest>,
) -> Response {
    if let Err(response) = validate_internal_api(&state, &headers) {
        return response;
    }
    let mut trace = gateway_trace(
        state.langsmith.as_ref(),
        "internal_rag_delete",
        RunType::Tool,
        json!({
            "collection_name": payload.collection_name.clone(),
        }),
    );

    let Some(rag_store) = &state.rag_store else {
        complete_gateway_trace(
            state.langsmith.as_ref(),
            trace.as_mut(),
            None,
            Some("rag store unavailable".to_string()),
        )
        .await;
        return (StatusCode::SERVICE_UNAVAILABLE, "rag store unavailable").into_response();
    };

    match rag_store.delete_collection(&payload.collection_name).await {
        Ok(deleted_chunks) => {
            let body = json!({
                "collection_name": payload.collection_name,
                "deleted_chunks": deleted_chunks,
            });
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                Some(body.clone()),
                None,
            )
            .await;
            Json(body).into_response()
        }
        Err(error) => {
            let error_message = format!("rag delete failed: {}", error);
            complete_gateway_trace(
                state.langsmith.as_ref(),
                trace.as_mut(),
                None,
                Some(error_message.clone()),
            )
            .await;
            (StatusCode::INTERNAL_SERVER_ERROR, error_message).into_response()
        }
    }
}

fn gateway_trace(
    client: Option<&LangSmithClient>,
    name: &str,
    run_type: RunType,
    inputs: serde_json::Value,
) -> Option<TraceRun> {
    let client = client?;
    Some(client.new_run(name, run_type, inputs))
}

async fn complete_gateway_trace(
    client: Option<&LangSmithClient>,
    trace: Option<&mut TraceRun>,
    outputs: Option<serde_json::Value>,
    error: Option<String>,
) {
    let (Some(client), Some(trace)) = (client, trace) else {
        return;
    };

    trace.outputs = outputs;
    trace.error = error;
    trace.end_time = Some(chrono::Utc::now());

    if let Err(trace_error) = client.trace_run(trace).await {
        warn!(error = %trace_error, trace_name = %trace.name, "Failed to send LangSmith gateway trace");
    }
}

fn validate_ws_request(state: &GatewayState, headers: &HeaderMap) -> CoreResult<()> {
    let trusted_proxy = trusted_proxy_authorized(state, headers);
    let origin = headers
        .get(axum::http::header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .or_else(|| {
            trusted_proxy.then(|| {
                headers
                    .get(HeaderName::from_static("x-forwarded-origin"))
                    .and_then(|value| value.to_str().ok())
            })?
        })
        .ok_or_else(|| {
            record_origin_check("denied");
            Error::Security(SecurityError::InvalidOrigin {
                origin: "<missing>".to_string(),
            })
        })?;
    if let Err(error) = state.origin_validator.validate(origin) {
        record_origin_check("denied");
        return Err(error);
    }
    record_origin_check("allowed");

    if state.require_auth {
        let auth_header = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok());
        if let Err(error) = extract_token(auth_header) {
            if trusted_proxy {
                record_auth_attempt("trusted_proxy", "success");
                return Ok(());
            }
            record_auth_attempt("bearer", "failure");
            return Err(error);
        }
        record_auth_attempt("bearer", "success");
    }

    Ok(())
}

fn trusted_proxy_authorized(state: &GatewayState, headers: &HeaderMap) -> bool {
    let Some(expected_token) = state.trusted_proxy_token.as_deref() else {
        return false;
    };

    let header_name = HeaderName::from_static("x-openrustclaw-trusted-proxy-token");
    let provided = headers
        .get(header_name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    if provided.is_empty() {
        return false;
    }

    if provided == expected_token.as_str() {
        true
    } else {
        record_auth_attempt("trusted_proxy", "failure");
        false
    }
}

#[allow(clippy::result_large_err)]
fn validate_internal_api(
    state: &GatewayState,
    headers: &HeaderMap,
) -> std::result::Result<(), Response> {
    let Some(expected_token) = &state.internal_api_token else {
        record_auth_attempt("internal_token", "disabled");
        return Err((StatusCode::SERVICE_UNAVAILABLE, "internal api disabled").into_response());
    };

    let header_name = HeaderName::from_static("x-openrustclaw-internal-token");
    let provided = headers
        .get(header_name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    if provided == expected_token.as_str() {
        record_auth_attempt("internal_token", "success");
        Ok(())
    } else {
        record_auth_attempt("internal_token", "failure");
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
    use openrustclaw_db::{
        SqliteCoreMemoryStore, SqliteMemoryStore, SqliteRagStore, init_pool, run_migrations,
    };
    use tower::ServiceExt;

    fn test_state() -> GatewayState {
        GatewayState {
            session_manager: Arc::new(crate::sessions::SessionManager::new()),
            origin_validator: Arc::new(OriginValidator::new(vec![
                "http://localhost:3000".to_string(),
            ])),
            require_auth: true,
            internal_api_token: None,
            trusted_proxy_token: None,
            memory_store: None,
            core_memory_store: None,
            rag_store: None,
            langsmith: None,
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

    #[test]
    fn validate_ws_request_accepts_trusted_proxy_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-forwarded-origin"),
            "http://localhost:3000".parse().unwrap(),
        );
        headers.insert(
            HeaderName::from_static("x-openrustclaw-trusted-proxy-token"),
            "proxy-secret".parse().unwrap(),
        );

        let mut state = test_state();
        state.trusted_proxy_token = Some(Arc::new("proxy-secret".to_string()));
        assert!(validate_ws_request(&state, &headers).is_ok());
    }

    #[test]
    fn validate_ws_request_rejects_invalid_trusted_proxy_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-forwarded-origin"),
            "http://localhost:3000".parse().unwrap(),
        );
        headers.insert(
            HeaderName::from_static("x-openrustclaw-trusted-proxy-token"),
            "wrong-secret".parse().unwrap(),
        );

        let mut state = test_state();
        state.trusted_proxy_token = Some(Arc::new("proxy-secret".to_string()));
        assert!(validate_ws_request(&state, &headers).is_err());
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
            trusted_proxy_token: None,
            memory_store: Some(memory_store),
            core_memory_store: Some(core_memory_store),
            rag_store: Some(Arc::new(SqliteRagStore::new(pool.clone()))),
            langsmith: None,
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
            .body(Body::from(
                r#"{"user_id":"user-1","query":"ownership","limit":3}"#,
            ))
            .unwrap();
        let search_response = app.clone().oneshot(search_request).await.unwrap();
        assert_eq!(search_response.status(), StatusCode::OK);
        let search_body = to_bytes(search_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let search_json: serde_json::Value = serde_json::from_slice(&search_body).unwrap();
        assert!(
            search_json["memories"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["content"] == "Rust ownership matters")
        );

        let core_request = Request::builder()
            .method("POST")
            .uri("/internal/memory/core/user-1")
            .header(
                "x-openrustclaw-internal-token",
                HeaderValue::from_static("test-token"),
            )
            .body(Body::from("{}"))
            .unwrap();
        let core_response = app.clone().oneshot(core_request).await.unwrap();
        assert_eq!(core_response.status(), StatusCode::OK);
        let core_body = to_bytes(core_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let core_json: serde_json::Value = serde_json::from_slice(&core_body).unwrap();
        assert!(
            core_json["content"]
                .as_str()
                .unwrap()
                .contains("Prefers Rust")
        );

        let set_request = Request::builder()
            .method("POST")
            .uri("/internal/memory/core/set")
            .header(CONTENT_TYPE, "application/json")
            .header("x-openrustclaw-internal-token", "test-token")
            .body(Body::from(
                r#"{"user_id":"user-1","key":"language","value":"Rust","importance":0.95}"#,
            ))
            .unwrap();
        let set_response = app.clone().oneshot(set_request).await.unwrap();
        assert_eq!(set_response.status(), StatusCode::OK);

        let updated_core_request = Request::builder()
            .method("POST")
            .uri("/internal/memory/core/user-1")
            .header(
                "x-openrustclaw-internal-token",
                HeaderValue::from_static("test-token"),
            )
            .body(Body::from("{}"))
            .unwrap();
        let updated_core_response = app.oneshot(updated_core_request).await.unwrap();
        let updated_core_body = to_bytes(updated_core_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let updated_core_json: serde_json::Value =
            serde_json::from_slice(&updated_core_body).unwrap();
        assert!(
            updated_core_json["content"]
                .as_str()
                .unwrap()
                .contains("language")
        );
        assert!(
            updated_core_json["content"]
                .as_str()
                .unwrap()
                .contains("Rust")
        );

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
            trusted_proxy_token: None,
            memory_store: Some(memory_store),
            core_memory_store: Some(core_memory_store),
            rag_store: Some(Arc::new(SqliteRagStore::new(pool.clone()))),
            langsmith: None,
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
        let old_body = to_bytes(old_response.into_body(), usize::MAX)
            .await
            .unwrap();
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
        let search_body = to_bytes(search_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let search_json: serde_json::Value = serde_json::from_slice(&search_body).unwrap();
        assert!(search_json["memories"].as_array().unwrap().is_empty());

        let archive_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memory_archive")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(archive_count, 1);

        let _ = std::fs::remove_file(db_path);
    }

    #[tokio::test]
    async fn internal_rag_endpoints_store_and_load_chunks() {
        let db_path = std::env::temp_dir().join(format!("gateway-rag-{}.db", Uuid::new_v4()));
        let pool = init_pool(&format!("sqlite://{}", db_path.display()), 1)
            .await
            .unwrap();
        run_migrations(&pool).await.unwrap();

        let state = GatewayState {
            session_manager: Arc::new(crate::sessions::SessionManager::new()),
            origin_validator: Arc::new(OriginValidator::new(vec![
                "http://localhost:3000".to_string(),
            ])),
            require_auth: false,
            internal_api_token: Some(Arc::new("test-token".to_string())),
            trusted_proxy_token: None,
            memory_store: Some(Arc::new(SqliteMemoryStore::new(pool.clone()))),
            core_memory_store: Some(Arc::new(SqliteCoreMemoryStore::new(pool.clone()))),
            rag_store: Some(Arc::new(SqliteRagStore::new(pool.clone()))),
            langsmith: None,
        };

        let app = GatewayServer::new("127.0.0.1".to_string(), 0).router(state);

        let store_request = Request::builder()
            .method("POST")
            .uri("/internal/rag/store")
            .header(CONTENT_TYPE, "application/json")
            .header("x-openrustclaw-internal-token", "test-token")
            .body(Body::from(
                r#"{"collection_name":"docs","chunks":[{"id":"chunk-1","content":"Rust uses ownership.","metadata":{"type":"text"},"chunk_index":0}]}"#,
            ))
            .unwrap();
        let store_response = app.clone().oneshot(store_request).await.unwrap();
        assert_eq!(store_response.status(), StatusCode::OK);

        let load_request = Request::builder()
            .method("POST")
            .uri("/internal/rag/load")
            .header(CONTENT_TYPE, "application/json")
            .header("x-openrustclaw-internal-token", "test-token")
            .body(Body::from(r#"{"collection_name":"docs","limit":5}"#))
            .unwrap();
        let load_response = app.clone().oneshot(load_request).await.unwrap();
        assert_eq!(load_response.status(), StatusCode::OK);
        let load_body = to_bytes(load_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let load_json: serde_json::Value = serde_json::from_slice(&load_body).unwrap();
        let chunks = load_json["chunks"].as_array().unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0]["source_id"], "chunk-1");
        assert_eq!(chunks[0]["content"], "Rust uses ownership.");

        let list_request = Request::builder()
            .method("POST")
            .uri("/internal/rag/list")
            .header(CONTENT_TYPE, "application/json")
            .header("x-openrustclaw-internal-token", "test-token")
            .body(Body::from(r#"{"limit":10}"#))
            .unwrap();
        let list_response = app.clone().oneshot(list_request).await.unwrap();
        assert_eq!(list_response.status(), StatusCode::OK);
        let list_body = to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let list_json: serde_json::Value = serde_json::from_slice(&list_body).unwrap();
        assert_eq!(list_json["collections"][0]["collection_name"], "docs");
        assert_eq!(list_json["collections"][0]["chunk_count"], 1);
        assert_eq!(list_json["collections"][0]["source_count"], 1);
        assert!(
            list_json["collections"][0]["total_content_bytes"]
                .as_i64()
                .unwrap()
                > 0
        );
        assert!(list_json["collections"][0]["last_updated_at"].is_string());

        let delete_request = Request::builder()
            .method("POST")
            .uri("/internal/rag/delete")
            .header(CONTENT_TYPE, "application/json")
            .header("x-openrustclaw-internal-token", "test-token")
            .body(Body::from(r#"{"collection_name":"docs"}"#))
            .unwrap();
        let delete_response = app.clone().oneshot(delete_request).await.unwrap();
        assert_eq!(delete_response.status(), StatusCode::OK);
        let delete_body = to_bytes(delete_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let delete_json: serde_json::Value = serde_json::from_slice(&delete_body).unwrap();
        assert_eq!(delete_json["deleted_chunks"], 1);

        let _ = std::fs::remove_file(db_path);
    }
}
