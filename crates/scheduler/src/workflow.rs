//! Rust-native workflow registry and compatibility dispatcher.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{Context, Result as AnyhowResult};
use async_trait::async_trait;
use chrono::{Datelike, Utc};
use openrustclaw_agent::runtime::AgentRuntime;
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::error::SchedulerError;
use openrustclaw_core::traits::{CoreMemoryStore, LlmProvider};
use openrustclaw_core::types::{CoreEntry, Message, OutgoingMessage, Platform, SourceType, ToolFormat};
use openrustclaw_db::{SqliteCoreMemoryStore, SqliteMemoryStore, SqlitePool, SqliteRagStore};
use openrustclaw_langbridge::{LangBridgeClient, WorkflowInvocation};
use openrustclaw_providers::{
    AnthropicProvider, OllamaProvider, OpenAiProvider, OpenRouterProvider, ProviderChain,
    openrouter::RouteStrategy,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tracing::warn;
use uuid::Uuid;

use crate::Result;
use crate::eventing::DurableEventBus;
use crate::persistence::save_workflow_checkpoint;

/// Unified execution tier for registered workflows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTier {
    RustNative,
    CompatSidecar,
    ExperimentalLanggraph,
}

/// Registered workflow metadata surfaced to operators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub workflow_id: String,
    pub tier: WorkflowTier,
    pub description: String,
}

/// Outcome returned by a workflow dispatcher.
#[derive(Debug, Clone)]
pub struct WorkflowDispatchResult {
    pub status: String,
    pub output: Value,
    pub error: Option<String>,
    pub trace_id: Option<String>,
}

/// Pluggable execution backend for scheduled workflows and event dispatches.
#[async_trait]
pub trait WorkflowDispatcher {
    async fn dispatch(&mut self, invocation: WorkflowInvocation) -> Result<WorkflowDispatchResult>;
}

/// Delivery backend used by the Rust-native reminder workflow.
#[async_trait]
pub trait ReminderSender: Send + Sync {
    async fn send(&self, platform: Platform, message: OutgoingMessage) -> Result<()>;
    fn available_platforms(&self) -> Vec<Platform>;
}

#[async_trait]
impl WorkflowDispatcher for LangBridgeClient {
    async fn dispatch(&mut self, invocation: WorkflowInvocation) -> Result<WorkflowDispatchResult> {
        let response = self
            .execute_invocation(invocation)
            .await
            .map_err(|e| SchedulerError::WorkflowFailed(e.to_string()))?;

        let output =
            serde_json::from_str(&response.output).unwrap_or_else(|_| serde_json::json!({}));
        let error = if response.error.is_empty() {
            None
        } else {
            Some(response.error)
        };
        let trace_id = if response.trace_id.is_empty() {
            None
        } else {
            Some(response.trace_id)
        };

        Ok(WorkflowDispatchResult {
            status: response.status,
            output,
            error,
            trace_id,
        })
    }
}

/// Rust-owned workflow dispatcher with optional sidecar compatibility fallback.
pub struct RustWorkflowDispatcher {
    pool: SqlitePool,
    memory_store: Arc<SqliteMemoryStore>,
    core_memory_store: Arc<SqliteCoreMemoryStore>,
    rag_store: Arc<SqliteRagStore>,
    agent_runtime: Option<Arc<AgentRuntime>>,
    compat_sidecar_addr: Option<String>,
    reminder_sender: Option<Arc<dyn ReminderSender>>,
    event_bus: Option<DurableEventBus>,
    definitions: Vec<WorkflowDefinition>,
}

impl RustWorkflowDispatcher {
    pub fn from_runtime_parts(
        pool: SqlitePool,
        memory_store: Arc<SqliteMemoryStore>,
        core_memory_store: Arc<SqliteCoreMemoryStore>,
        rag_store: Arc<SqliteRagStore>,
        agent_runtime: Option<Arc<AgentRuntime>>,
        compat_sidecar_addr: Option<String>,
        reminder_sender: Option<Arc<dyn ReminderSender>>,
        event_bus: Option<DurableEventBus>,
    ) -> Self {
        Self {
            pool,
            memory_store,
            core_memory_store,
            rag_store,
            agent_runtime,
            compat_sidecar_addr,
            reminder_sender,
            event_bus,
            definitions: vec![
                WorkflowDefinition {
                    workflow_id: "agent".to_string(),
                    tier: WorkflowTier::RustNative,
                    description: "Rust-native agent execution over the core runtime".to_string(),
                },
                WorkflowDefinition {
                    workflow_id: "memory_maintenance".to_string(),
                    tier: WorkflowTier::RustNative,
                    description:
                        "Rust-native episodic memory consolidation and archive maintenance"
                            .to_string(),
                },
                WorkflowDefinition {
                    workflow_id: "rag".to_string(),
                    tier: WorkflowTier::RustNative,
                    description: "Rust-native RAG indexing, retrieval, and context assembly"
                        .to_string(),
                },
                WorkflowDefinition {
                    workflow_id: "scheduler".to_string(),
                    tier: WorkflowTier::RustNative,
                    description:
                        "Rust-native scheduled workflow envelope and inner workflow dispatch"
                            .to_string(),
                },
                WorkflowDefinition {
                    workflow_id: "reminder".to_string(),
                    tier: WorkflowTier::RustNative,
                    description:
                        "Rust-native reminder delivery with channel-aware fallback policies"
                            .to_string(),
                },
            ],
        }
    }

    pub fn from_config(
        pool: SqlitePool,
        config: &AppConfig,
        compat_sidecar_addr: Option<String>,
        reminder_sender: Option<Arc<dyn ReminderSender>>,
        event_bus: Option<DurableEventBus>,
    ) -> Result<Self> {
        let memory_store = Arc::new(SqliteMemoryStore::new(pool.clone()));
        let core_memory_store = Arc::new(SqliteCoreMemoryStore::new(pool.clone()));
        let rag_store = Arc::new(SqliteRagStore::new(pool.clone()));
        let agent_runtime = build_agent_runtime(config, memory_store.clone(), core_memory_store.clone())
            .map(Some)
            .unwrap_or_else(|error| {
                warn!(error = %error, "Rust-native agent workflow is disabled until a provider is available");
                None
            });

        Ok(Self::from_runtime_parts(
            pool,
            memory_store,
            core_memory_store,
            rag_store,
            agent_runtime,
            compat_sidecar_addr,
            reminder_sender,
            event_bus,
        ))
    }

    pub fn definitions(&self) -> &[WorkflowDefinition] {
        &self.definitions
    }

    fn has_rust_workflow(&self, workflow_id: &str) -> bool {
        matches!(
            workflow_id,
            "agent" | "memory_maintenance" | "rag" | "scheduler" | "reminder"
        )
    }

    async fn checkpoint(
        &self,
        invocation: &WorkflowInvocation,
        step: i64,
        state: Value,
    ) -> Result<()> {
        save_workflow_checkpoint(
            &self.pool,
            &invocation.workflow_id,
            &invocation.thread_id,
            step,
            &state,
        )
        .await
    }

    async fn dispatch_rust(
        &self,
        invocation: WorkflowInvocation,
    ) -> Result<WorkflowDispatchResult> {
        match invocation.workflow_id.as_str() {
            "agent" => self.execute_agent(invocation).await,
            "memory_maintenance" => self.execute_memory_maintenance(invocation).await,
            "rag" => self.execute_rag(invocation).await,
            "scheduler" => self.execute_scheduler(invocation).await,
            "reminder" => self.execute_reminder(invocation).await,
            other => Err(SchedulerError::WorkflowFailed(format!(
                "unknown rust-native workflow: {other}"
            ))),
        }
    }

    async fn dispatch_compat(
        &self,
        invocation: WorkflowInvocation,
    ) -> Result<WorkflowDispatchResult> {
        let Some(addr) = &self.compat_sidecar_addr else {
            return Err(SchedulerError::WorkflowFailed(format!(
                "workflow '{}' is not registered in Rust and no compatibility sidecar is configured",
                invocation.workflow_id
            )));
        };

        let mut client = LangBridgeClient::connect(addr)
            .await
            .map_err(|e| SchedulerError::WorkflowFailed(format!("sidecar connect failed: {e}")))?;
        client.dispatch(invocation).await
    }

    async fn execute_scheduler(
        &self,
        invocation: WorkflowInvocation,
    ) -> Result<WorkflowDispatchResult> {
        self.checkpoint(&invocation, 0, serde_json::json!({"status": "received"}))
            .await?;

        let inner_workflow = invocation
            .input
            .get("workflow_id")
            .and_then(Value::as_str)
            .or_else(|| invocation.input.get("job_type").and_then(Value::as_str))
            .map(ToString::to_string);

        if let Some(inner_workflow) = inner_workflow {
            if inner_workflow != "scheduler" {
                self.checkpoint(
                    &invocation,
                    1,
                    serde_json::json!({"status": "dispatching", "inner_workflow": inner_workflow}),
                )
                .await?;

                let nested_input = invocation
                    .input
                    .get("input")
                    .cloned()
                    .unwrap_or_else(|| invocation.input.clone());
                let nested = WorkflowInvocation::new(
                    inner_workflow.clone(),
                    invocation.thread_id.clone(),
                    nested_input,
                )
                .with_metadata(invocation.metadata.clone())
                .with_configurable(invocation.configurable.clone());

                let result = match inner_workflow.as_str() {
                    "agent" => self.execute_agent(nested).await?,
                    "memory_maintenance" => self.execute_memory_maintenance(nested).await?,
                    "rag" => self.execute_rag(nested).await?,
                    "reminder" => self.execute_reminder(nested).await?,
                    _ => self.dispatch_compat(nested).await?,
                };

                self.checkpoint(
                    &invocation,
                    2,
                    serde_json::json!({
                        "status": result.status,
                        "inner_workflow": inner_workflow,
                    }),
                )
                .await?;
                return Ok(result);
            }
        }

        self.checkpoint(&invocation, 1, serde_json::json!({"status": "completed"}))
            .await?;
        Ok(WorkflowDispatchResult {
            status: "success".to_string(),
            output: serde_json::json!({
                "message": "scheduler envelope handled by Rust runtime",
                "thread_id": invocation.thread_id,
                "input": invocation.input,
            }),
            error: None,
            trace_id: None,
        })
    }

    async fn execute_agent(
        &self,
        invocation: WorkflowInvocation,
    ) -> Result<WorkflowDispatchResult> {
        let Some(runtime) = self.agent_runtime.as_ref() else {
            return Err(SchedulerError::WorkflowFailed(
                "rust-native agent workflow is unavailable because no provider could be initialized"
                    .to_string(),
            ));
        };

        self.checkpoint(
            &invocation,
            0,
            serde_json::json!({"status": "loading_input"}),
        )
        .await?;

        let user_id = invocation
            .input
            .get("user_id")
            .and_then(Value::as_str)
            .or_else(|| {
                invocation
                    .configurable
                    .get("user_id")
                    .and_then(Value::as_str)
            })
            .unwrap_or("scheduler")
            .to_string();

        let session_id = invocation
            .input
            .get("session_id")
            .and_then(Value::as_str)
            .unwrap_or(&invocation.thread_id)
            .to_string();

        let messages = parse_messages(invocation.input.get("messages"));
        let core_memory = if let Some(entries_value) = invocation.input.get("core_memory") {
            serde_json::from_value::<Vec<CoreEntry>>(entries_value.clone()).unwrap_or_default()
        } else {
            self.core_memory_store
                .get_all(&user_id)
                .await
                .map_err(|e| SchedulerError::WorkflowFailed(e.to_string()))?
        };

        self.checkpoint(
            &invocation,
            1,
            serde_json::json!({
                "status": "executing",
                "message_count": messages.len(),
                "core_memory_entries": core_memory.len(),
            }),
        )
        .await?;

        let response = runtime
            .process(&messages, &core_memory, &session_id, &user_id)
            .await
            .map_err(|e| SchedulerError::WorkflowFailed(e.to_string()))?;

        let output = serde_json::json!({
            "message": response.message,
            "usage": response.usage,
            "tool_calls_made": response.tool_calls_made,
            "session_id": session_id,
            "user_id": user_id,
        });

        self.checkpoint(
            &invocation,
            2,
            serde_json::json!({
                "status": "completed",
                "tool_calls_made": response.tool_calls_made,
            }),
        )
        .await?;

        Ok(WorkflowDispatchResult {
            status: "success".to_string(),
            output,
            error: None,
            trace_id: None,
        })
    }

    async fn execute_memory_maintenance(
        &self,
        invocation: WorkflowInvocation,
    ) -> Result<WorkflowDispatchResult> {
        let age_days = invocation
            .input
            .get("age_days")
            .and_then(Value::as_i64)
            .unwrap_or(30)
            .max(1) as i64;
        let limit = invocation
            .input
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(100)
            .max(1) as usize;
        let namespace = invocation.input.get("namespace").and_then(Value::as_str);
        let user_id = invocation.input.get("user_id").and_then(Value::as_str);
        let delete_archived = invocation
            .input
            .get("delete_archived")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let cutoff = Utc::now() - chrono::Duration::days(age_days);

        self.checkpoint(
            &invocation,
            0,
            serde_json::json!({
                "status": "loading_old_memories",
                "age_days": age_days,
                "limit": limit,
                "namespace": namespace,
                "user_id": user_id,
            }),
        )
        .await?;

        let memories = self
            .memory_store
            .list_old_episodic_memories(cutoff, namespace, user_id, limit)
            .await
            .map_err(|e| SchedulerError::WorkflowFailed(e.to_string()))?;

        if memories.is_empty() {
            self.checkpoint(
                &invocation,
                1,
                serde_json::json!({"status": "completed", "archived_count": 0}),
            )
            .await?;
            return Ok(WorkflowDispatchResult {
                status: "success".to_string(),
                output: serde_json::json!({
                    "archived_count": 0,
                    "consolidated_count": 0,
                    "archive_entries": [],
                    "archived_memory_ids": [],
                }),
                error: None,
                trace_id: None,
            });
        }

        let groups = group_memories_for_archive(&memories);
        self.checkpoint(
            &invocation,
            1,
            serde_json::json!({
                "status": "summarizing",
                "memory_count": memories.len(),
                "group_count": groups.len(),
            }),
        )
        .await?;

        let mut archive_entries = Vec::new();
        let mut archived_memory_ids = Vec::new();

        for group in groups {
            let source_ids: Vec<String> = group.iter().map(|entry| entry.id.to_string()).collect();
            let summary = build_archive_summary(&group);
            let namespace = group.iter().find_map(|entry| {
                (!entry.namespace.is_empty()).then_some(entry.namespace.as_str())
            });
            let source_type = group
                .iter()
                .find_map(|entry| entry.source_type.as_ref())
                .map(source_type_to_str);
            let archive_id = Uuid::new_v4().to_string();

            self.memory_store
                .store_archive_entry(
                    &archive_id,
                    &summary,
                    &source_ids,
                    namespace,
                    Some(0.6),
                    source_type,
                )
                .await
                .map_err(|e| SchedulerError::WorkflowFailed(e.to_string()))?;

            archived_memory_ids.extend(source_ids.iter().cloned());
            archive_entries.push(serde_json::json!({
                "id": archive_id,
                "summary": summary,
                "source_memory_ids": source_ids,
                "namespace": namespace,
                "source_type": source_type,
            }));
        }

        self.checkpoint(
            &invocation,
            2,
            serde_json::json!({
                "status": "archiving",
                "archive_entry_count": archive_entries.len(),
                "archived_memory_count": archived_memory_ids.len(),
            }),
        )
        .await?;

        let deleted_count = if delete_archived {
            self.memory_store
                .delete_many(&archived_memory_ids)
                .await
                .map_err(|e| SchedulerError::WorkflowFailed(e.to_string()))?
        } else {
            0
        };

        let output = serde_json::json!({
            "archived_count": archived_memory_ids.len(),
            "consolidated_count": archive_entries.len(),
            "deleted_count": deleted_count,
            "archive_entries": archive_entries,
            "archived_memory_ids": archived_memory_ids,
        });

        self.checkpoint(
            &invocation,
            3,
            serde_json::json!({
                "status": "completed",
                "deleted_count": deleted_count,
            }),
        )
        .await?;

        Ok(WorkflowDispatchResult {
            status: "success".to_string(),
            output,
            error: None,
            trace_id: None,
        })
    }

    async fn execute_rag(&self, invocation: WorkflowInvocation) -> Result<WorkflowDispatchResult> {
        let collection_name = invocation
            .input
            .get("collection_name")
            .and_then(Value::as_str)
            .unwrap_or("rag_documents");
        let mut output = Map::new();

        if let Some(chunks_value) = invocation.input.get("chunks").and_then(Value::as_array) {
            self.checkpoint(
                &invocation,
                0,
                serde_json::json!({
                    "status": "indexing",
                    "collection_name": collection_name,
                    "chunk_count": chunks_value.len(),
                }),
            )
            .await?;
            let chunks = parse_rag_chunks(chunks_value)?;
            let stored = self
                .rag_store
                .replace_collection(collection_name, &chunks)
                .await
                .map_err(|e| SchedulerError::WorkflowFailed(e.to_string()))?;
            output.insert("stored_chunk_count".to_string(), Value::from(stored as u64));
        }

        if let Some(query) = invocation.input.get("query").and_then(Value::as_str) {
            self.checkpoint(
                &invocation,
                1,
                serde_json::json!({
                    "status": "retrieving",
                    "collection_name": collection_name,
                    "query": query,
                }),
            )
            .await?;

            let loaded = self
                .rag_store
                .load_collection(collection_name, None)
                .await
                .map_err(|e| SchedulerError::WorkflowFailed(e.to_string()))?;

            let retrieval = retrieve_rag_chunks(query, &loaded, &invocation.input);
            output.insert("results".to_string(), Value::Array(retrieval.results));
            output.insert("context".to_string(), Value::String(retrieval.context));
            output.insert("retrieval_summary".to_string(), retrieval.summary);
        }

        self.checkpoint(
            &invocation,
            2,
            serde_json::json!({
                "status": "completed",
                "has_query": invocation.input.get("query").is_some(),
                "has_chunks": invocation.input.get("chunks").is_some(),
            }),
        )
        .await?;

        Ok(WorkflowDispatchResult {
            status: "success".to_string(),
            output: Value::Object(output),
            error: None,
            trace_id: None,
        })
    }

    async fn execute_reminder(
        &self,
        invocation: WorkflowInvocation,
    ) -> Result<WorkflowDispatchResult> {
        let Some(sender) = self.reminder_sender.as_ref() else {
            return Err(SchedulerError::WorkflowFailed(
                "rust-native reminder workflow requires at least one connected channel"
                    .to_string(),
            ));
        };

        let content = invocation
            .input
            .get("content")
            .or_else(|| invocation.input.get("message"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                SchedulerError::WorkflowFailed(
                    "reminder workflow requires non-empty input.content or input.message"
                        .to_string(),
                )
            })?
            .to_string();

        let session_id = invocation
            .input
            .get("session_id")
            .and_then(Value::as_str)
            .and_then(|raw| Uuid::parse_str(raw).ok())
            .unwrap_or_else(Uuid::new_v4);
        let user_id = invocation
            .input
            .get("user_id")
            .and_then(Value::as_str)
            .or_else(|| invocation.configurable.get("user_id").and_then(Value::as_str))
            .unwrap_or("scheduler");
        let policy = ReminderDeliveryPolicy::from_input(&invocation.input)?;
        let selected_platforms =
            resolve_delivery_platforms(&policy, sender.available_platforms())?;

        self.checkpoint(
            &invocation,
            0,
            serde_json::json!({
                "status": "queued_for_delivery",
                "target_platforms": selected_platforms.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "delivery_mode": policy.mode.as_str(),
            }),
        )
        .await?;

        if let Some(event_bus) = &self.event_bus {
            let _ = event_bus
                .publish_named(
                    "reminder.triggered",
                    "reminder_triggered",
                    Some(&session_id.to_string()),
                    &serde_json::json!({
                        "content": content,
                        "user_id": user_id,
                        "platforms": selected_platforms.iter().map(ToString::to_string).collect::<Vec<_>>(),
                    }),
                    None,
                )
                .await;
        }

        let mut deliveries = Vec::new();
        let mut failures = Vec::new();
        let mut delivered_count = 0_usize;

        for platform in selected_platforms {
            let metadata = policy.metadata_for(platform, &invocation.input);
            let outgoing = OutgoingMessage {
                session_id,
                content: content.clone(),
                metadata: metadata.clone(),
            };

            match sender.send(platform, outgoing).await {
                Ok(()) => {
                    delivered_count += 1;
                    deliveries.push(serde_json::json!({
                        "platform": platform.to_string(),
                        "metadata": metadata,
                    }));
                    if let Some(event_bus) = &self.event_bus {
                        let _ = event_bus
                            .publish_named(
                                "reminder.delivered",
                                "reminder_delivered",
                                Some(&session_id.to_string()),
                                &serde_json::json!({
                                    "platform": platform.to_string(),
                                    "user_id": user_id,
                                    "content": content,
                                }),
                                None,
                            )
                            .await;
                    }
                    if matches!(policy.mode, ReminderDeliveryMode::FirstSuccess)
                        || delivered_count >= policy.max_deliveries
                    {
                        break;
                    }
                }
                Err(error) => {
                    failures.push(serde_json::json!({
                        "platform": platform.to_string(),
                        "error": error.to_string(),
                    }));
                    if let Some(event_bus) = &self.event_bus {
                        let _ = event_bus
                            .publish_named(
                                "reminder.delivery_failed",
                                "reminder_delivery_failed",
                                Some(&session_id.to_string()),
                                &serde_json::json!({
                                    "platform": platform.to_string(),
                                    "user_id": user_id,
                                    "error": error.to_string(),
                                }),
                                None,
                            )
                            .await;
                    }
                    if !policy.continue_on_failure {
                        break;
                    }
                }
            }
        }

        let success = match policy.mode {
            ReminderDeliveryMode::FirstSuccess => delivered_count >= 1,
            ReminderDeliveryMode::Broadcast => delivered_count >= 1,
        };
        let status = if success { "success" } else { "failed" };

        self.checkpoint(
            &invocation,
            1,
            serde_json::json!({
                "status": status,
                "delivered_count": delivered_count,
                "failure_count": failures.len(),
            }),
        )
        .await?;

        Ok(WorkflowDispatchResult {
            status: status.to_string(),
            output: serde_json::json!({
                "delivered": success,
                "delivered_count": delivered_count,
                "delivery_mode": policy.mode.as_str(),
                "deliveries": deliveries,
                "failures": failures,
            }),
            error: (!success).then_some("reminder delivery failed".to_string()),
            trace_id: None,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum ReminderDeliveryMode {
    FirstSuccess,
    Broadcast,
}

impl ReminderDeliveryMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::FirstSuccess => "first_success",
            Self::Broadcast => "broadcast",
        }
    }
}

#[derive(Debug, Clone)]
struct ReminderDeliveryPolicy {
    mode: ReminderDeliveryMode,
    preferred_channels: Vec<Platform>,
    fallback_channels: Vec<Platform>,
    max_deliveries: usize,
    continue_on_failure: bool,
    default_metadata: Value,
    channel_metadata: Map<String, Value>,
}

impl ReminderDeliveryPolicy {
    fn from_input(input: &Value) -> Result<Self> {
        let policy = input
            .get("delivery_policy")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        let mode = match policy
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("first_success")
        {
            "broadcast" | "all" => ReminderDeliveryMode::Broadcast,
            _ => ReminderDeliveryMode::FirstSuccess,
        };

        let max_deliveries = policy
            .get("max_deliveries")
            .and_then(Value::as_u64)
            .map(|value| value.max(1) as usize)
            .unwrap_or_else(|| match mode {
                ReminderDeliveryMode::FirstSuccess => 1,
                ReminderDeliveryMode::Broadcast => usize::MAX,
            });

        let default_metadata = policy
            .get("default_metadata")
            .cloned()
            .or_else(|| input.get("metadata").cloned())
            .unwrap_or_else(|| serde_json::json!({}));
        let channel_metadata = policy
            .get("channel_metadata")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        Ok(Self {
            mode,
            preferred_channels: parse_platform_list(policy.get("preferred_channels"))?,
            fallback_channels: parse_platform_list(policy.get("fallback_channels"))?,
            max_deliveries,
            continue_on_failure: policy
                .get("continue_on_failure")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            default_metadata,
            channel_metadata,
        })
    }

    fn metadata_for(&self, platform: Platform, input: &Value) -> Value {
        let mut metadata = match self.default_metadata.clone() {
            Value::Object(map) => map,
            _ => Map::new(),
        };

        if let Some(Value::Object(map)) = self.channel_metadata.get(&platform.to_string()) {
            for (key, value) in map {
                metadata.insert(key.clone(), value.clone());
            }
        }
        if let Some(session_id) = input.get("session_id").and_then(Value::as_str) {
            metadata
                .entry("session_id".to_string())
                .or_insert_with(|| Value::String(session_id.to_string()));
        }
        metadata.insert(
            "reminder_platform".to_string(),
            Value::String(platform.to_string()),
        );
        Value::Object(metadata)
    }
}

fn resolve_delivery_platforms(
    policy: &ReminderDeliveryPolicy,
    available_platforms: Vec<Platform>,
) -> Result<Vec<Platform>> {
    let available: HashSet<Platform> = available_platforms.iter().copied().collect();
    if available.is_empty() {
        return Err(SchedulerError::WorkflowFailed(
            "no connected channels are available for reminder delivery".to_string(),
        ));
    }

    let mut selected = Vec::new();
    let requested = if policy.preferred_channels.is_empty() && policy.fallback_channels.is_empty() {
        available_platforms
    } else {
        let mut ordered = policy.preferred_channels.clone();
        ordered.extend(policy.fallback_channels.clone());
        ordered
    };

    for platform in requested {
        if available.contains(&platform) && !selected.contains(&platform) {
            selected.push(platform);
        }
    }

    if selected.is_empty() {
        return Err(SchedulerError::WorkflowFailed(
            "reminder delivery policy did not match any connected channels".to_string(),
        ));
    }

    Ok(selected)
}

fn parse_platform_list(value: Option<&Value>) -> Result<Vec<Platform>> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(parse_platform)
                .collect()
        })
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn parse_platform(raw: &str) -> Result<Platform> {
    match raw {
        "web_chat" | "webchat" => Ok(Platform::WebChat),
        "telegram" => Ok(Platform::Telegram),
        "discord" => Ok(Platform::Discord),
        "slack" => Ok(Platform::Slack),
        "whatsapp" => Ok(Platform::WhatsApp),
        "teams" => Ok(Platform::Teams),
        "google_chat" => Ok(Platform::GoogleChat),
        "gmail" => Ok(Platform::Gmail),
        "twilio" => Ok(Platform::Twilio),
        "signal" => Ok(Platform::Signal),
        "matrix" => Ok(Platform::Matrix),
        "x" | "twitter" => Ok(Platform::X),
        "messenger" => Ok(Platform::Messenger),
        "instagram" => Ok(Platform::Instagram),
        "imessage" => Ok(Platform::IMessage),
        "line" => Ok(Platform::Line),
        "viber" => Ok(Platform::Viber),
        "wechat" => Ok(Platform::WeChat),
        "cli" => Ok(Platform::Cli),
        "api" => Ok(Platform::Api),
        other => Err(SchedulerError::WorkflowFailed(format!(
            "unsupported reminder delivery platform: {other}"
        ))),
    }
}

#[async_trait]
impl WorkflowDispatcher for RustWorkflowDispatcher {
    async fn dispatch(&mut self, invocation: WorkflowInvocation) -> Result<WorkflowDispatchResult> {
        if self.has_rust_workflow(&invocation.workflow_id) {
            self.dispatch_rust(invocation).await
        } else {
            self.dispatch_compat(invocation).await
        }
    }
}

fn parse_messages(raw: Option<&Value>) -> Vec<Message> {
    match raw {
        Some(Value::Array(items)) => {
            let mut messages = Vec::new();
            for item in items {
                if let Ok(message) = serde_json::from_value::<Message>(item.clone()) {
                    messages.push(message);
                } else if let Some(content) = item.as_str() {
                    messages.push(Message::user(content));
                }
            }
            if messages.is_empty() {
                vec![Message::user("Continue")]
            } else {
                messages
            }
        }
        Some(Value::String(content)) => vec![Message::user(content)],
        _ => vec![Message::user("Continue")],
    }
}

fn group_memories_for_archive(
    memories: &[openrustclaw_core::types::MemoryEntry],
) -> Vec<Vec<openrustclaw_core::types::MemoryEntry>> {
    let mut grouped: HashMap<(String, i32, u32), Vec<openrustclaw_core::types::MemoryEntry>> =
        HashMap::new();
    for memory in memories {
        let week = memory.created_at.iso_week();
        grouped
            .entry((memory.namespace.clone(), week.year(), week.week()))
            .or_default()
            .push(memory.clone());
    }
    grouped.into_values().collect()
}

fn build_archive_summary(memories: &[openrustclaw_core::types::MemoryEntry]) -> String {
    let mut parts = Vec::new();
    for memory in memories.iter().take(6) {
        let mut content = memory.content.replace('\n', " ").trim().to_string();
        if content.len() > 160 {
            content.truncate(157);
            content.push_str("...");
        }
        parts.push(format!("- {}", content));
    }
    format!(
        "Consolidated {} episodic memories\n{}",
        memories.len(),
        parts.join("\n")
    )
}

fn source_type_to_str(source_type: &SourceType) -> &'static str {
    match source_type {
        SourceType::Document => "document",
        SourceType::Code => "code",
        SourceType::Config => "config",
        SourceType::Conversation => "conversation",
        SourceType::Runbook => "runbook",
        SourceType::ToolSchema => "tool_schema",
    }
}

fn parse_rag_chunks(chunks_value: &[Value]) -> Result<Vec<openrustclaw_db::RagChunkInput>> {
    chunks_value
        .iter()
        .cloned()
        .map(|value| {
            serde_json::from_value(value).map_err(|e| {
                SchedulerError::WorkflowFailed(format!("invalid rag chunk payload: {e}"))
            })
        })
        .collect()
}

struct RetrievalOutput {
    results: Vec<Value>,
    context: String,
    summary: Value,
}

fn retrieve_rag_chunks(
    query: &str,
    chunks: &[openrustclaw_db::RagChunkRecord],
    input: &Value,
) -> RetrievalOutput {
    let top_k = input.get("top_k").and_then(Value::as_u64).unwrap_or(5) as usize;
    let max_chars = input
        .get("max_context_chars")
        .or_else(|| input.get("max_chars"))
        .and_then(Value::as_u64)
        .unwrap_or(4000) as usize;
    let min_score = input
        .get("min_score")
        .and_then(Value::as_f64)
        .unwrap_or(0.05) as f32;
    let required_source_ids = json_string_set(input.get("required_source_ids"));
    let preferred_source_ids = json_string_set(input.get("preferred_source_ids"));
    let excluded_source_ids = json_string_set(input.get("excluded_source_ids"));
    let required_source_types = json_string_set(input.get("required_source_types"));
    let preferred_source_types = json_string_set(input.get("preferred_source_types"));
    let excluded_source_types = json_string_set(input.get("excluded_source_types"));

    let query_terms = tokenize(query);
    let mut scored = Vec::new();
    for chunk in chunks {
        if excluded_source_ids.contains(&chunk.source_id) {
            continue;
        }

        let source_type = chunk
            .metadata
            .get("source_type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        if !required_source_ids.is_empty() && !required_source_ids.contains(&chunk.source_id) {
            continue;
        }
        if !required_source_types.is_empty() && !required_source_types.contains(&source_type) {
            continue;
        }
        if excluded_source_types.contains(&source_type) {
            continue;
        }

        let chunk_terms = tokenize(&chunk.content);
        let overlap = query_terms.intersection(&chunk_terms).count() as f32;
        if overlap == 0.0 {
            continue;
        }

        let coverage = overlap / query_terms.len().max(1) as f32;
        let density = overlap / chunk_terms.len().max(1) as f32;
        let mut score = coverage * 0.65 + density * 0.35;
        if preferred_source_ids.contains(&chunk.source_id) {
            score += 0.15;
        }
        if preferred_source_types.contains(&source_type) {
            score += 0.10;
        }
        if score < min_score {
            continue;
        }

        scored.push((score, chunk, overlap as usize, source_type));
    }

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut seen_sources = HashSet::new();
    let mut results = Vec::new();
    let mut context = String::new();
    let mut total_score = 0.0_f32;
    for (score, chunk, overlap, source_type) in scored.into_iter().take(top_k * 3) {
        let source_key = format!("{}::{}", chunk.source_id, chunk.chunk_id);
        if !seen_sources.insert(source_key) {
            continue;
        }
        let entry = serde_json::json!({
            "chunk_id": chunk.chunk_id,
            "source_id": chunk.source_id,
            "chunk_index": chunk.chunk_index,
            "content": chunk.content,
            "metadata": chunk.metadata,
            "score": score,
            "overlap_count": overlap,
            "source_type": source_type,
        });
        let line = format!(
            "[{}:{}] {}\n",
            chunk.source_id, chunk.chunk_index, chunk.content
        );
        if context.len() + line.len() > max_chars && !context.is_empty() {
            break;
        }
        total_score += score;
        context.push_str(&line);
        results.push(entry);
        if results.len() >= top_k {
            break;
        }
    }

    let average_score = if results.is_empty() {
        0.0
    } else {
        total_score / results.len() as f32
    };

    RetrievalOutput {
        summary: serde_json::json!({
            "query": query,
            "requested_top_k": top_k,
            "returned_results": results.len(),
            "average_score": average_score,
            "required_source_ids": required_source_ids.into_iter().collect::<Vec<_>>(),
            "preferred_source_ids": preferred_source_ids.into_iter().collect::<Vec<_>>(),
            "excluded_source_ids": excluded_source_ids.into_iter().collect::<Vec<_>>(),
        }),
        context,
        results,
    }
}

fn json_string_set(value: Option<&Value>) -> HashSet<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn tokenize(text: &str) -> HashSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter_map(|token| {
            let token = token.trim().to_lowercase();
            (token.len() >= 3).then_some(token)
        })
        .collect()
}

fn build_agent_runtime(
    config: &AppConfig,
    memory_store: Arc<SqliteMemoryStore>,
    core_memory_store: Arc<SqliteCoreMemoryStore>,
) -> AnyhowResult<Arc<AgentRuntime>> {
    let provider = build_provider(config)?;
    Ok(Arc::new(AgentRuntime::with_memory_stores(
        provider,
        "OpenRustClaw".to_string(),
        memory_store,
        core_memory_store,
    )))
}

fn build_provider(config: &AppConfig) -> AnyhowResult<Arc<dyn LlmProvider>> {
    let mut provider_names = Vec::new();
    provider_names.push(config.providers.default_provider.clone());
    provider_names.extend(config.providers.fallback_chain.clone());

    let mut seen = HashSet::new();
    let mut providers = Vec::new();

    for provider_name in provider_names {
        if !seen.insert(provider_name.clone()) {
            continue;
        }

        match create_provider_from_config(&provider_name, config) {
            Ok(provider) => providers.push(provider),
            Err(error) if providers.is_empty() => return Err(error),
            Err(error) => {
                warn!(
                    provider = %provider_name,
                    error = %error,
                    "Skipping fallback provider that could not be initialized for workflow runtime"
                );
            }
        }
    }

    let primary = providers
        .first()
        .cloned()
        .context("No workflow providers could be initialized")?;

    if providers.len() == 1 {
        return Ok(primary);
    }

    Ok(Arc::new(WorkflowProviderChain::new(providers, primary)))
}

fn create_provider_from_config(
    provider_name: &str,
    config: &AppConfig,
) -> AnyhowResult<Arc<dyn LlmProvider>> {
    match provider_name.to_lowercase().as_str() {
        "anthropic" => {
            let api_key =
                std::env::var("ANTHROPIC_API_KEY").context("ANTHROPIC_API_KEY not set")?;
            Ok(Arc::new(AnthropicProvider::new(
                api_key,
                config.providers.anthropic.model.clone(),
            )))
        }
        "openai" => {
            let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
            Ok(Arc::new(OpenAiProvider::new(
                api_key,
                config.providers.openai.model.clone(),
            )))
        }
        "openrouter" => {
            let api_key =
                std::env::var("OPENROUTER_API_KEY").context("OPENROUTER_API_KEY not set")?;
            let strategy = match config.providers.openrouter.route_strategy.as_str() {
                "price" => RouteStrategy::Price,
                "throughput" => RouteStrategy::Throughput,
                "web_search" | "online" => RouteStrategy::WebSearch,
                _ => RouteStrategy::Quality,
            };
            Ok(Arc::new(OpenRouterProvider::with_strategy(
                api_key,
                "anthropic/claude-sonnet-4".to_string(),
                strategy,
            )))
        }
        "ollama" => Ok(Arc::new(OllamaProvider::with_base_url(
            config.providers.ollama.model.clone(),
            config.providers.ollama.base_url.clone(),
        ))),
        other => anyhow::bail!("Unknown workflow provider '{}'", other),
    }
}

struct WorkflowProviderChain {
    chain: ProviderChain,
    primary: Arc<dyn LlmProvider>,
}

impl WorkflowProviderChain {
    fn new(providers: Vec<Arc<dyn LlmProvider>>, primary: Arc<dyn LlmProvider>) -> Self {
        Self {
            chain: ProviderChain::new(providers),
            primary,
        }
    }
}

#[async_trait]
impl LlmProvider for WorkflowProviderChain {
    async fn complete(
        &self,
        request: openrustclaw_core::types::CompletionRequest,
    ) -> openrustclaw_core::error::Result<openrustclaw_core::types::CompletionResponse> {
        self.chain.complete(request).await
    }

    async fn stream(
        &self,
        _request: openrustclaw_core::types::CompletionRequest,
    ) -> openrustclaw_core::error::Result<
        std::pin::Pin<
            Box<
                dyn futures::Stream<
                        Item = openrustclaw_core::error::Result<
                            openrustclaw_core::types::StreamChunk,
                        >,
                    > + Send,
            >,
        >,
    > {
        Err(openrustclaw_core::error::Error::Provider(
            openrustclaw_core::error::ProviderError::StreamError {
                provider: self.primary.provider_name().to_string(),
                message: "Streaming is not implemented for workflow provider chains".to_string(),
            },
        ))
    }

    fn model_id(&self) -> &str {
        self.primary.model_id()
    }

    fn max_tokens(&self) -> usize {
        self.primary.max_tokens()
    }

    fn provider_name(&self) -> &str {
        self.primary.provider_name()
    }

    fn supports_strict_tools(&self) -> bool {
        self.primary.supports_strict_tools()
    }

    fn supports_streaming_tool_deltas(&self) -> bool {
        self.primary.supports_streaming_tool_deltas()
    }

    fn native_tool_format(&self) -> ToolFormat {
        self.primary.native_tool_format()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use openrustclaw_db::{init_pool, run_migrations};

    #[derive(Default)]
    struct StubReminderSender {
        available: Vec<Platform>,
        deliveries: tokio::sync::Mutex<Vec<(Platform, OutgoingMessage)>>,
    }

    #[async_trait]
    impl ReminderSender for StubReminderSender {
        async fn send(&self, platform: Platform, message: OutgoingMessage) -> Result<()> {
            self.deliveries.lock().await.push((platform, message));
            Ok(())
        }

        fn available_platforms(&self) -> Vec<Platform> {
            self.available.clone()
        }
    }

    #[tokio::test]
    async fn reminder_workflow_delivers_to_first_available_preferred_channel() {
        let pool = init_pool("sqlite::memory:", 1).await.unwrap();
        run_migrations(&pool).await.unwrap();
        let memory_store = Arc::new(SqliteMemoryStore::new(pool.clone()));
        let core_memory_store = Arc::new(SqliteCoreMemoryStore::new(pool.clone()));
        let rag_store = Arc::new(SqliteRagStore::new(pool.clone()));
        let sender = Arc::new(StubReminderSender {
            available: vec![Platform::Telegram, Platform::Slack],
            deliveries: tokio::sync::Mutex::new(Vec::new()),
        });
        let event_bus = DurableEventBus::new(pool.clone(), 16);
        let mut dispatcher = RustWorkflowDispatcher::from_runtime_parts(
            pool.clone(),
            memory_store,
            core_memory_store,
            rag_store,
            None,
            None,
            Some(sender.clone()),
            Some(event_bus),
        );

        let result = dispatcher
            .dispatch(
                WorkflowInvocation::new(
                    "reminder",
                    "thread-1",
                    serde_json::json!({
                        "content": "Take the daily standup notes",
                        "session_id": Uuid::nil().to_string(),
                        "delivery_policy": {
                            "mode": "first_success",
                            "preferred_channels": ["telegram", "slack"],
                            "channel_metadata": {
                                "telegram": {"telegram_chat_id": "chat-123"},
                                "slack": {"slack_channel": "C123"}
                            }
                        }
                    }),
                ),
            )
            .await
            .unwrap();

        assert_eq!(result.status, "success");
        assert_eq!(result.output["delivered_count"], 1);
        assert_eq!(result.output["deliveries"][0]["platform"], "telegram");

        let deliveries = sender.deliveries.lock().await;
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].0, Platform::Telegram);
        assert_eq!(deliveries[0].1.metadata["telegram_chat_id"], "chat-123");

        let event_names: Vec<String> =
            sqlx::query_scalar("SELECT event_name FROM runtime_events ORDER BY created_at ASC")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert!(event_names.contains(&"reminder.triggered".to_string()));
        assert!(event_names.contains(&"reminder.delivered".to_string()));
    }
}
