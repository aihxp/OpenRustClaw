//! Fixture-driven channel routing and mention activation tests.

use openrustclaw_cli::commands::channels::{
    ChannelBindingSpec, ChannelRouteStatus, init, load_registry, preview_route, upsert_binding,
};
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::types::{IncomingMessage, Platform};
use serde_json::json;
use tempfile::tempdir;
use uuid::Uuid;

use crate::common::init_test_tracing;

fn message(
    user_id: &str,
    platform: Platform,
    metadata: serde_json::Value,
    mentioned: bool,
) -> IncomingMessage {
    let mut metadata = metadata;
    if let Some(object) = metadata.as_object_mut() {
        object.insert(
            match platform {
                Platform::Slack => "slack_bot_mentioned",
                Platform::Discord => "discord_bot_mentioned",
                Platform::Telegram => "telegram_bot_mentioned",
                Platform::WhatsApp => "whatsapp_bot_mentioned",
                Platform::IMessage => "imessage_bot_mentioned",
                Platform::Signal => "signal_bot_mentioned",
                Platform::Mattermost => "mattermost_bot_mentioned",
                Platform::Teams => "teams_bot_mentioned",
                Platform::GoogleChat => "google_chat_bot_mentioned",
                Platform::WebChat => "webchat_bot_mentioned",
                _ => "slack_bot_mentioned",
            }
            .to_string(),
            json!(mentioned),
        );
    }

    IncomingMessage {
        session_id: Uuid::new_v4(),
        user_id: user_id.to_string(),
        content: "fixture hello".to_string(),
        platform,
        metadata,
    }
}

#[test]
fn channel_routing_fixture_prefers_more_specific_binding() {
    init_test_tracing();

    let temp = tempdir().unwrap();
    let registry_root = temp.path().join(".claw/channels");
    let root_str = registry_root.to_string_lossy().to_string();
    init(Some(&root_str)).unwrap();

    openrustclaw_cli::commands::channels::create_account(
        Some(&root_str),
        "slack:T123:U123",
        "slack",
        "U123",
        Some("Fixture Slack"),
        Some("T123"),
        Some("C123"),
        Some("workspace-account"),
        Some("agent-account"),
        true,
        false,
        true,
        Some("mention"),
    )
    .unwrap();

    upsert_binding(
        Some(&root_str),
        ChannelBindingSpec {
            id: "workspace-binding".to_string(),
            platform: "slack".to_string(),
            enabled: true,
            priority: 100,
            workspace_match: Some("T123".to_string()),
            account_match: None,
            channel_match: None,
            workspace_target: Some("workspace-generic".to_string()),
            agent_id: Some("agent-generic".to_string()),
            activation_mode: Some("mention".to_string()),
            direct_strategy: None,
            group_strategy: Some("shared_channel".to_string()),
            send_policy: None,
            metadata: json!({}),
        },
    )
    .unwrap();
    upsert_binding(
        Some(&root_str),
        ChannelBindingSpec {
            id: "thread-binding".to_string(),
            platform: "slack".to_string(),
            enabled: true,
            priority: 100,
            workspace_match: Some("T123".to_string()),
            account_match: Some("slack:T123:U123".to_string()),
            channel_match: Some("slack_thread_ts=171234.000100".to_string()),
            workspace_target: Some("workspace-thread".to_string()),
            agent_id: Some("agent-thread".to_string()),
            activation_mode: Some("always".to_string()),
            direct_strategy: None,
            group_strategy: Some("shared_channel".to_string()),
            send_policy: None,
            metadata: json!({}),
        },
    )
    .unwrap();

    let mut registry = load_registry(registry_root).unwrap();
    let preview = preview_route(
        &mut registry,
        &message(
            "U123",
            Platform::Slack,
            json!({
                "slack_team_id": "T123",
                "slack_channel": "C123",
                "slack_thread_ts": "171234.000100",
                "slack_is_group": true
            }),
            false,
        ),
        &AppConfig::default().session_routing,
    )
    .unwrap();

    assert_eq!(preview.status, ChannelRouteStatus::Allowed);
    assert_eq!(preview.binding_id.as_deref(), Some("thread-binding"));
    assert_eq!(preview.workspace_id.as_deref(), Some("workspace-account"));
    assert_eq!(preview.agent_id.as_deref(), Some("agent-account"));
    assert_eq!(preview.activation_mode, "mention");
    assert!(preview.route_key.contains("account=slack:T123:U123"));
    assert!(preview.route_key.contains("slack_thread_ts=171234.000100"));
}

#[test]
fn group_mention_fixture_honors_activation_mode() {
    init_test_tracing();

    let temp = tempdir().unwrap();
    let registry_root = temp.path().join(".claw/channels");
    let root_str = registry_root.to_string_lossy().to_string();
    init(Some(&root_str)).unwrap();

    openrustclaw_cli::commands::channels::create_account(
        Some(&root_str),
        "google_chat:spaces/AAA:users/123",
        "google_chat",
        "users/123",
        Some("Fixture Chat"),
        Some("spaces/AAA"),
        Some("spaces/AAA"),
        Some("workspace-chat"),
        Some("agent-chat"),
        true,
        false,
        true,
        Some("mention"),
    )
    .unwrap();

    let mut registry = load_registry(registry_root).unwrap();
    let not_mentioned = preview_route(
        &mut registry,
        &message(
            "users/123",
            Platform::GoogleChat,
            json!({
                "google_chat_space": "spaces/AAA",
                "google_chat_thread": "spaces/AAA/threads/BBB",
                "google_chat_is_group": true
            }),
            false,
        ),
        &AppConfig::default().session_routing,
    )
    .unwrap();
    assert_eq!(not_mentioned.status, ChannelRouteStatus::Allowed);
    assert_eq!(not_mentioned.activation_mode, "mention");
    assert!(!not_mentioned.should_respond);

    let mentioned = preview_route(
        &mut registry,
        &message(
            "users/123",
            Platform::GoogleChat,
            json!({
                "google_chat_space": "spaces/AAA",
                "google_chat_thread": "spaces/AAA/threads/BBB",
                "google_chat_is_group": true
            }),
            true,
        ),
        &AppConfig::default().session_routing,
    )
    .unwrap();
    assert_eq!(mentioned.status, ChannelRouteStatus::Allowed);
    assert!(mentioned.should_respond);
}
