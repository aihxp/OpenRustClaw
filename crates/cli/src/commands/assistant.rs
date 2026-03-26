//! Shared assistant runtime and session helpers.

use openrustclaw_agent::runtime::AgentRuntime;
use openrustclaw_core::traits::{CoreMemoryStore, LlmProvider, MemoryStore};
use openrustclaw_core::types::Platform;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const ASSISTANT_NAME: &str = "OpenRustClaw Assistant";
pub const ASSISTANT_IDENTITY: &str = "primary";
pub const DEFAULT_ASSISTANT_MAX_TOOL_ITERATIONS: usize = 4;

pub fn build_runtime(
    provider: Arc<dyn LlmProvider>,
    memory_store: Arc<dyn MemoryStore>,
    core_memory_store: Arc<dyn CoreMemoryStore>,
    workspace_path: impl Into<PathBuf>,
) -> AgentRuntime {
    AgentRuntime::with_memory_stores(
        provider,
        ASSISTANT_NAME.to_string(),
        memory_store,
        core_memory_store,
    )
    .with_workspace_path(workspace_path)
    .with_max_tool_iterations(DEFAULT_ASSISTANT_MAX_TOOL_ITERATIONS)
}

pub fn cli_route_key(user_id: &str, workspace_root: &Path) -> String {
    let digest = Sha256::digest(workspace_root.display().to_string().as_bytes());
    format!(
        "cli:assistant:{}:{}",
        user_id,
        hex::encode(&digest)[..12].to_string()
    )
}

pub fn session_metadata(
    surface: &str,
    route_key: Option<&str>,
    workspace_root: Option<&Path>,
    extra: Value,
) -> Value {
    let mut metadata = Map::new();
    metadata.insert(
        "assistant_identity".to_string(),
        Value::String(ASSISTANT_IDENTITY.to_string()),
    );
    metadata.insert(
        "assistant_surface".to_string(),
        Value::String(surface.to_string()),
    );
    metadata.insert(
        "assistant_session_model".to_string(),
        Value::String("persisted".to_string()),
    );
    if let Some(route_key) = route_key {
        metadata.insert(
            "route_key".to_string(),
            Value::String(route_key.to_string()),
        );
    }
    if let Some(workspace_root) = workspace_root {
        metadata.insert(
            "workspace_root".to_string(),
            Value::String(workspace_root.display().to_string()),
        );
    }

    if let Some(extra_object) = extra.as_object() {
        for (key, value) in extra_object {
            metadata.insert(key.clone(), value.clone());
        }
    }

    Value::Object(metadata)
}

pub fn session_metadata_for_platform(
    platform: Platform,
    route_key: Option<&str>,
    workspace_root: Option<&Path>,
    extra: Value,
) -> Value {
    let Some(surface) = assistant_surface_for_platform(platform) else {
        return extra;
    };
    session_metadata(surface, route_key, workspace_root, extra)
}

pub fn startup_handoff_message(active_cli_session: bool) -> String {
    if active_cli_session {
        "Assistant handoff: run `openrustclaw assistant` to resume your persisted CLI session."
            .to_string()
    } else {
        "Assistant handoff: run `openrustclaw assistant` to start a persisted CLI session for this workspace."
            .to_string()
    }
}

pub fn startup_handoff_json(active_cli_session: bool) -> Value {
    json!({
        "active_cli_session": active_cli_session,
        "command": "openrustclaw assistant",
        "message": startup_handoff_message(active_cli_session),
    })
}

fn assistant_surface_for_platform(platform: Platform) -> Option<&'static str> {
    match platform {
        Platform::Cli => Some("cli"),
        Platform::Api => Some("api"),
        Platform::WebChat => Some("webchat"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_route_key_is_stable_for_workspace_and_user() {
        let root = Path::new("/tmp/project-a");
        assert_eq!(cli_route_key("alice", root), cli_route_key("alice", root));
    }

    #[test]
    fn session_metadata_marks_primary_assistant_surface() {
        let metadata = session_metadata(
            "cli",
            Some("cli:assistant:alice:123"),
            Some(Path::new("/tmp/project-a")),
            json!({"workspace_id": "project-a"}),
        );

        assert_eq!(metadata["assistant_identity"], "primary");
        assert_eq!(metadata["assistant_surface"], "cli");
        assert_eq!(metadata["assistant_session_model"], "persisted");
        assert_eq!(metadata["route_key"], "cli:assistant:alice:123");
        assert_eq!(metadata["workspace_id"], "project-a");
    }

    #[test]
    fn session_metadata_for_platform_only_tags_assistant_surfaces() {
        let webchat = session_metadata_for_platform(
            Platform::WebChat,
            Some("webchat:user-1"),
            Some(Path::new("/tmp/project-a")),
            json!({"spawned_by": "mcp"}),
        );
        assert_eq!(webchat["assistant_surface"], "webchat");
        assert_eq!(webchat["spawned_by"], "mcp");

        let telegram = session_metadata_for_platform(
            Platform::Telegram,
            Some("telegram:chat-1"),
            Some(Path::new("/tmp/project-a")),
            json!({"spawned_by": "mcp"}),
        );
        assert_eq!(telegram["spawned_by"], "mcp");
        assert!(telegram.get("assistant_surface").is_none());
    }

    #[test]
    fn startup_handoff_message_changes_when_session_exists() {
        assert!(startup_handoff_message(true).contains("resume"));
        assert!(startup_handoff_message(false).contains("start"));
    }
}
