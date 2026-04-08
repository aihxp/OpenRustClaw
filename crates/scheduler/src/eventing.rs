//! Durable event bus for lifecycle hooks and event-triggered workflows.

use openrustclaw_core::types::Event;
use openrustclaw_db::SqlitePool;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::broadcast;
use tracing::debug;

use crate::Result;
use crate::persistence::queue_runtime_event;

/// Event published to subscribers after durable persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedRuntimeEvent {
    pub event_id: String,
    pub event_name: String,
    pub event_type: String,
    pub session_id: Option<String>,
    pub payload: Value,
    pub queued_dispatches: usize,
}

/// Durable runtime event bus backed by SQLite.
#[derive(Clone)]
pub struct DurableEventBus {
    pool: SqlitePool,
    sender: broadcast::Sender<PublishedRuntimeEvent>,
}

impl DurableEventBus {
    pub fn new(pool: SqlitePool, capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { pool, sender }
    }

    pub async fn publish(&self, event: Event) -> Result<PublishedRuntimeEvent> {
        let envelope = event_envelope(event);
        self.publish_named(
            &envelope.event_name,
            &envelope.event_type,
            envelope.session_id.as_deref(),
            &envelope.payload,
            None,
        )
        .await
    }

    pub async fn publish_named(
        &self,
        event_name: &str,
        event_type: &str,
        session_id: Option<&str>,
        payload: &Value,
        dedupe_key: Option<&str>,
    ) -> Result<PublishedRuntimeEvent> {
        let (record, queued_dispatches) = queue_runtime_event(
            &self.pool, event_name, event_type, session_id, payload, dedupe_key,
        )
        .await?;
        let published = PublishedRuntimeEvent {
            event_id: record.id,
            event_name: record.event_name,
            event_type: record.event_type,
            session_id: record.session_id,
            payload: record.payload,
            queued_dispatches,
        };
        debug!(
            event_id = %published.event_id,
            event_name = %published.event_name,
            queued_dispatches = published.queued_dispatches,
            "Published durable runtime event"
        );
        let _ = self.sender.send(published.clone());
        Ok(published)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<PublishedRuntimeEvent> {
        self.sender.subscribe()
    }
}

struct EventEnvelope {
    event_name: String,
    event_type: String,
    session_id: Option<String>,
    payload: Value,
}

fn event_envelope(event: Event) -> EventEnvelope {
    match event {
        Event::MessageReceived {
            session_id,
            message,
        } => EventEnvelope {
            event_name: "message.received".to_string(),
            event_type: "message_received".to_string(),
            session_id: Some(session_id.to_string()),
            payload: serde_json::json!({ "message": message }),
        },
        Event::MessageSent {
            session_id,
            message,
        } => EventEnvelope {
            event_name: "message.sent".to_string(),
            event_type: "message_sent".to_string(),
            session_id: Some(session_id.to_string()),
            payload: serde_json::json!({ "message": message }),
        },
        Event::ToolExecuted {
            session_id,
            tool_name,
            output,
            duration_ms,
        } => EventEnvelope {
            event_name: "tool.executed".to_string(),
            event_type: "tool_executed".to_string(),
            session_id: Some(session_id.to_string()),
            payload: serde_json::json!({
                "tool_name": tool_name,
                "output": output,
                "duration_ms": duration_ms,
            }),
        },
        Event::MemoryStored {
            entry_id,
            memory_type,
            source,
        } => EventEnvelope {
            event_name: "memory.stored".to_string(),
            event_type: "memory_stored".to_string(),
            session_id: None,
            payload: serde_json::json!({
                "entry_id": entry_id,
                "memory_type": memory_type,
                "source": source,
            }),
        },
        Event::MemorySearched {
            query,
            namespace,
            result_count,
            recall_pack,
        } => EventEnvelope {
            event_name: "memory.searched".to_string(),
            event_type: "memory_searched".to_string(),
            session_id: None,
            payload: serde_json::json!({
                "query": query,
                "namespace": namespace,
                "result_count": result_count,
                "recall_pack": recall_pack,
            }),
        },
        Event::SessionCreated { session } => EventEnvelope {
            event_name: "session.created".to_string(),
            event_type: "session_created".to_string(),
            session_id: Some(session.id.to_string()),
            payload: serde_json::json!({ "session": session }),
        },
        Event::SessionClosed { session_id } => EventEnvelope {
            event_name: "session.closed".to_string(),
            event_type: "session_closed".to_string(),
            session_id: Some(session_id.to_string()),
            payload: serde_json::json!({}),
        },
        Event::SessionLifecycle {
            session_id,
            hook,
            metadata,
        } => EventEnvelope {
            event_name: hook.clone(),
            event_type: "session_lifecycle".to_string(),
            session_id: Some(session_id.to_string()),
            payload: metadata,
        },
        Event::SchedulerJobFired { job_id, job_name } => EventEnvelope {
            event_name: "scheduler.job_fired".to_string(),
            event_type: "scheduler_job_fired".to_string(),
            session_id: None,
            payload: serde_json::json!({
                "job_id": job_id,
                "job_name": job_name,
            }),
        },
        Event::ApprovalRequested {
            request_id,
            session_id,
            description,
        } => EventEnvelope {
            event_name: "approval.requested".to_string(),
            event_type: "approval_requested".to_string(),
            session_id: Some(session_id.to_string()),
            payload: serde_json::json!({
                "request_id": request_id,
                "description": description,
            }),
        },
        Event::ApprovalReceived {
            request_id,
            approved,
            reviewer,
        } => EventEnvelope {
            event_name: "approval.received".to_string(),
            event_type: "approval_received".to_string(),
            session_id: None,
            payload: serde_json::json!({
                "request_id": request_id,
                "approved": approved,
                "reviewer": reviewer,
            }),
        },
        Event::Error {
            message,
            session_id,
            severity,
        } => EventEnvelope {
            event_name: "runtime.error".to_string(),
            event_type: "error".to_string(),
            session_id: session_id.map(|id| id.to_string()),
            payload: serde_json::json!({
                "message": message,
                "severity": severity,
            }),
        },
        Event::Custom { name, payload } => EventEnvelope {
            event_name: name,
            event_type: "custom".to_string(),
            session_id: None,
            payload,
        },
    }
}
