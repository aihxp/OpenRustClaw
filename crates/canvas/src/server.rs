//! WebSocket server for Canvas real-time communication
//!
//! Provides HTTP routes and WebSocket handlers for canvas connections.

use crate::canvas::Canvas;
use crate::error::{CanvasError, CanvasResult};
use crate::protocol::{CanvasCommand, CanvasMessage, CanvasSnapshot, Interaction};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use dashmap::DashMap;
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, trace, warn};
use uuid::Uuid;

/// Shared state for the canvas server
#[derive(Clone)]
pub struct CanvasServerState {
    /// Map of canvas IDs to canvas instances
    canvases: Arc<DashMap<Uuid, Canvas>>,
    /// Default canvas for clients without explicit ID
    default_canvas: Arc<RwLock<Option<Uuid>>>,
}

impl CanvasServerState {
    /// Create new server state
    pub fn new() -> Self {
        Self {
            canvases: Arc::new(DashMap::new()),
            default_canvas: Arc::new(RwLock::new(None)),
        }
    }

    /// Get or create a canvas by ID
    pub async fn get_or_create_canvas(&self, id: Option<Uuid>) -> Canvas {
        if let Some(canvas_id) = id {
            if let Some(canvas) = self.canvases.get(&canvas_id) {
                canvas.clone()
            } else {
                let canvas = Canvas::with_id(canvas_id, format!("Canvas {}", canvas_id));
                self.canvases.insert(canvas_id, canvas.clone());
                canvas
            }
        } else {
            // Check for default canvas
            let default = *self.default_canvas.read().await;
            if let Some(default_id) = default {
                if let Some(canvas) = self.canvases.get(&default_id) {
                    return canvas.clone();
                }
            }

            // Create new default canvas
            let canvas = Canvas::new("Live Canvas");
            let id = canvas.id();
            *self.default_canvas.write().await = Some(id);
            self.canvases.insert(id, canvas.clone());
            canvas
        }
    }

    /// Get a canvas by ID
    pub fn get_canvas(&self, id: Uuid) -> Option<Canvas> {
        self.canvases.get(&id).map(|c| c.clone())
    }

    /// Remove a canvas
    pub fn remove_canvas(&self, id: Uuid) -> Option<Canvas> {
        self.canvases.remove(&id).map(|(_, c)| c)
    }

    /// List all canvas IDs
    pub fn list_canvases(&self) -> Vec<Uuid> {
        self.canvases.iter().map(|entry| *entry.key()).collect()
    }

    /// Get canvas count
    pub fn canvas_count(&self) -> usize {
        self.canvases.len()
    }
}

impl Default for CanvasServerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Canvas WebSocket server
pub struct CanvasServer {
    state: CanvasServerState,
}

impl CanvasServer {
    /// Create a new canvas server
    pub fn new() -> Self {
        Self {
            state: CanvasServerState::new(),
        }
    }

    /// Create a server with existing state
    pub fn with_state(state: CanvasServerState) -> Self {
        Self { state }
    }

    /// Get the server state
    pub fn state(&self) -> &CanvasServerState {
        &self.state
    }

    /// Create router with all routes
    pub fn router(&self) -> Router {
        Router::new()
            .route("/canvas/ws", get(ws_handler))
            .route("/canvas", post(create_canvas))
            .route("/canvas/{id}", get(get_canvas))
            .route("/canvas/{id}/snapshot", get(get_snapshot))
            .route("/canvases", get(list_canvases))
            .with_state(self.state.clone())
    }

    /// Create a canvas programmatically
    pub async fn create_canvas(&self, title: impl Into<String>) -> Canvas {
        let canvas = Canvas::new(title);
        let id = canvas.id();
        self.state.canvases.insert(id, canvas.clone());
        info!(canvas_id = %id, "Created new canvas");
        canvas
    }

    /// Get or create canvas
    pub async fn get_or_create_canvas(&self, id: Option<Uuid>) -> Canvas {
        self.state.get_or_create_canvas(id).await
    }
}

impl Default for CanvasServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Query parameters for WebSocket connection
#[derive(Debug, Deserialize)]
struct WsQuery {
    canvas_id: Option<Uuid>,
}

/// WebSocket upgrade handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<CanvasServerState>,
) -> impl IntoResponse {
    let canvas_id = query.canvas_id;
    ws.on_upgrade(move |socket| handle_socket(socket, state, canvas_id))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: CanvasServerState, canvas_id: Option<Uuid>) {
    let canvas = state.get_or_create_canvas(canvas_id).await;
    let canvas_id = canvas.id();

    info!(canvas_id = %canvas_id, "New WebSocket connection");

    let (mut sender, mut receiver) = socket.split();
    let mut update_rx = canvas.subscribe();

    // Send initial snapshot
    let snapshot = CanvasSnapshot::new(canvas_id, canvas.title(), canvas.elements().to_vec());
    let snapshot_msg = CanvasMessage::Snapshot { canvas: snapshot };

    if let Ok(json) = serde_json::to_string(&snapshot_msg) {
        if sender.send(Message::Text(json.into())).await.is_err() {
            warn!(canvas_id = %canvas_id, "Failed to send initial snapshot");
            return;
        }
    }

    // Store canvas reference for updates
    state.canvases.insert(canvas_id, canvas);

    // Spawn task to forward canvas updates to WebSocket
    let forward_task = tokio::spawn(async move {
        loop {
            match update_rx.recv().await {
                Ok(update) => {
                    let msg = CanvasMessage::Update { update };
                    if let Ok(json) = serde_json::to_string(&msg) {
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            trace!("Client disconnected, stopping update forward");
                            break;
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    trace!("Broadcast channel closed");
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    warn!("Client lagged behind, some updates may be lost");
                    continue;
                }
            }
        }
    });

    // Handle incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                trace!("Received WebSocket text message");
                if let Err(e) = handle_message(&text, &state, canvas_id).await {
                    warn!(error = %e, "Failed to handle message");
                }
            }
            Message::Binary(_) => {
                trace!("Received WebSocket binary message (ignored)");
            }
            Message::Ping(_) => {
                trace!("Received Ping");
            }
            Message::Pong(_) => {
                trace!("Received Pong");
            }
            Message::Close(frame) => {
                info!(?frame, "WebSocket connection closed by client");
                break;
            }
        }
    }

    forward_task.abort();
    debug!(canvas_id = %canvas_id, "WebSocket connection ended");
}

/// Handle a single WebSocket message
async fn handle_message(
    text: &str,
    state: &CanvasServerState,
    canvas_id: Uuid,
) -> CanvasResult<()> {
    let msg: CanvasMessage = serde_json::from_str(text)
        .map_err(|e| CanvasError::invalid_message(format!("JSON parse error: {}", e)))?;

    trace!(?msg, "Processing canvas message");

    match msg {
        CanvasMessage::Interact {
            element_id,
            interaction,
        } => {
            handle_interaction(state, canvas_id, element_id, interaction).await;
        }
        CanvasMessage::Command { command } => {
            handle_command(state, canvas_id, command).await?;
        }
        CanvasMessage::SetState { key, value } => {
            if let Some(mut canvas) = state.canvases.get_mut(&canvas_id) {
                canvas.set_state(key, value);
            }
        }
        CanvasMessage::Ping => {
            // Client should handle Pong response
            trace!("Received ping, client expects pong");
        }
        _ => {
            // Other messages are either client->server or handled elsewhere
            trace!("Unhandled message type");
        }
    }

    Ok(())
}

/// Handle user interaction
async fn handle_interaction(
    _state: &CanvasServerState,
    canvas_id: Uuid,
    element_id: Uuid,
    interaction: Interaction,
) {
    debug!(
        canvas_id = %canvas_id,
        element_id = %element_id,
        ?interaction,
        "Handling user interaction"
    );

    // Here you would typically:
    // 1. Validate the interaction
    // 2. Update canvas state if needed
    // 3. Notify subscribers
    // 4. Trigger any registered callbacks

    // For now, just log the interaction
    match interaction {
        Interaction::Submit { data } => {
            info!(canvas_id = %canvas_id, ?data, "Form submitted");
        }
        Interaction::Click => {
            trace!(element_id = %element_id, "Element clicked");
        }
        Interaction::Input { value } => {
            trace!(element_id = %element_id, value = %value, "Input received");
        }
        Interaction::Select { option } => {
            trace!(element_id = %element_id, option = %option, "Option selected");
        }
        Interaction::Toggle { checked } => {
            trace!(element_id = %element_id, checked = checked, "Toggle changed");
        }
        Interaction::Custom { payload } => {
            trace!(element_id = %element_id, ?payload, "Custom interaction");
        }
    }
}

/// Handle canvas command
async fn handle_command(
    state: &CanvasServerState,
    canvas_id: Uuid,
    command: CanvasCommand,
) -> CanvasResult<()> {
    

    let mut entry = state
        .canvases
        .entry(canvas_id)
        .or_insert_with(|| Canvas::with_id(canvas_id, "Auto-created"));

    match command {
        CanvasCommand::Push { elements } => {
            entry.push(elements);
        }
        CanvasCommand::Reset => {
            entry.reset();
        }
        CanvasCommand::Update {
            element_id,
            element,
        } => {
            entry.update(element_id, element)?;
        }
        CanvasCommand::Remove { element_id } => {
            entry.remove(element_id)?;
        }
        CanvasCommand::Eval { script } => {
            entry.eval(script);
        }
        CanvasCommand::GetSnapshot => {
            // Snapshot is handled by the update channel
        }
        CanvasCommand::SetTitle { title } => {
            entry.set_title(title);
        }
    }

    Ok(())
}

/// Request body for creating a canvas
#[derive(Debug, Deserialize)]
struct CreateCanvasRequest {
    title: Option<String>,
    id: Option<Uuid>,
}

/// Response for canvas creation
#[derive(Debug, Serialize)]
struct CreateCanvasResponse {
    id: Uuid,
    title: String,
}

/// HTTP handler: Create a new canvas
async fn create_canvas(
    State(state): State<CanvasServerState>,
    Json(req): Json<CreateCanvasRequest>,
) -> Result<Json<CreateCanvasResponse>, CanvasError> {
    let canvas = if let Some(id) = req.id {
        Canvas::with_id(id, req.title.unwrap_or_else(|| "Canvas".to_string()))
    } else {
        Canvas::new(req.title.unwrap_or_else(|| "Canvas".to_string()))
    };

    let id = canvas.id();
    let title = canvas.title().to_string();

    state.canvases.insert(id, canvas);

    info!(canvas_id = %id, "Canvas created via API");

    Ok(Json(CreateCanvasResponse { id, title }))
}

/// HTTP handler: Get canvas info
async fn get_canvas(
    Path(id): Path<Uuid>,
    State(state): State<CanvasServerState>,
) -> Result<Json<CanvasSnapshot>, CanvasError> {
    let canvas = state
        .get_canvas(id)
        .ok_or_else(|| CanvasError::canvas_not_found(id.to_string()))?;

    Ok(Json(CanvasSnapshot::new(
        id,
        canvas.title(),
        canvas.elements().to_vec(),
    )))
}

/// HTTP handler: Get canvas snapshot
async fn get_snapshot(
    Path(id): Path<Uuid>,
    State(state): State<CanvasServerState>,
) -> Result<Json<CanvasSnapshot>, CanvasError> {
    get_canvas(Path(id), State(state)).await
}

/// Response for listing canvases
#[derive(Debug, Serialize)]
struct ListCanvasesResponse {
    canvases: Vec<CanvasInfo>,
    total: usize,
}

/// Canvas info for listing
#[derive(Debug, Serialize)]
struct CanvasInfo {
    id: Uuid,
    title: String,
    element_count: usize,
    subscriber_count: usize,
}

/// HTTP handler: List all canvases
async fn list_canvases(State(state): State<CanvasServerState>) -> Json<ListCanvasesResponse> {
    let canvases: Vec<CanvasInfo> = state
        .canvases
        .iter()
        .map(|entry| {
            let canvas = entry.value();
            CanvasInfo {
                id: canvas.id(),
                title: canvas.title().to_string(),
                element_count: canvas.len(),
                subscriber_count: canvas.subscriber_count(),
            }
        })
        .collect();

    let total = canvases.len();

    Json(ListCanvasesResponse { canvases, total })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::TextStyle;

    #[test]
    fn test_server_state_new() {
        let state = CanvasServerState::new();
        assert_eq!(state.canvas_count(), 0);
    }

    #[tokio::test]
    async fn test_server_state_get_or_create() {
        let state = CanvasServerState::new();

        let canvas1 = state.get_or_create_canvas(None).await;
        let canvas2 = state.get_or_create_canvas(Some(canvas1.id())).await;

        assert_eq!(canvas1.id(), canvas2.id());
    }

    #[tokio::test]
    async fn test_server_create_canvas() {
        let server = CanvasServer::new();
        let canvas = server.create_canvas("Test Canvas").await;

        assert_eq!(canvas.title(), "Test Canvas");
        assert_eq!(server.state().canvas_count(), 1);
    }

    #[test]
    fn test_canvas_info_serialization() {
        let info = CanvasInfo {
            id: Uuid::new_v4(),
            title: "Test".to_string(),
            element_count: 5,
            subscriber_count: 2,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("Test"));
        assert!(json.contains("5"));
    }

    #[tokio::test]
    async fn test_list_canvases() {
        let state = CanvasServerState::new();
        let canvas = Canvas::new("Test");
        state.canvases.insert(canvas.id(), canvas);

        let response = list_canvases(State(state)).await;
        assert_eq!(response.total, 1);
    }
}
