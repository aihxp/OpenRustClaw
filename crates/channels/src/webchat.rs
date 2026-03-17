//! WebChat channel implementation.
//!
//! Served directly by the Axum gateway. Messages are exchanged
//! via WebSocket connections.

use async_trait::async_trait;
use openrustclaw_core::error::{Error, Result};
use openrustclaw_core::traits::Channel;
use openrustclaw_core::types::{IncomingMessage, OutgoingMessage, Platform};
use tokio::sync::mpsc;

/// WebChat channel that communicates via the gateway WebSocket.
pub struct WebChatChannel {
    #[allow(dead_code)]
    incoming_rx: mpsc::Receiver<IncomingMessage>,
    outgoing_tx: mpsc::Sender<OutgoingMessage>,
}

impl WebChatChannel {
    pub fn new() -> (
        Self,
        mpsc::Sender<IncomingMessage>,
        mpsc::Receiver<OutgoingMessage>,
    ) {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);
        let (outgoing_tx, outgoing_rx) = mpsc::channel(256);

        let channel = Self {
            incoming_rx,
            outgoing_tx,
        };

        (channel, incoming_tx, outgoing_rx)
    }
}

#[async_trait]
impl Channel for WebChatChannel {
    fn platform(&self) -> Platform {
        Platform::WebChat
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        self.outgoing_tx
            .send(msg)
            .await
            .map_err(|e| Error::Internal(format!("WebChat send error: {}", e)))
    }

    async fn receive(&self) -> Result<IncomingMessage> {
        // Note: this is simplified - real impl needs &mut self or interior mutability
        Err(Error::Internal("Use incoming_rx directly".to_string()))
    }

    async fn connect(&mut self) -> Result<()> {
        Ok(()) // WebChat is always connected via the gateway
    }

    async fn disconnect(&mut self) -> Result<()> {
        Ok(())
    }
}
