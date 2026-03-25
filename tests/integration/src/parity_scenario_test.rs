//! Documented parity scenarios exercised through shipped command/data APIs.

use std::fs;

use openrustclaw_cli::commands::{
    channels::{
        ChannelBindingSpec, ChannelRouteStatus, approve as approve_channel_account,
        init as init_channels, load_registry as load_channel_registry, preview_route,
        upsert_binding,
    },
    control,
    media::{MediaExtractTextRequest, MediaInspectRequest, extract_text_with_config, inspect_data},
    mobile::{
        DeviceCommandKind, MobileCommandDecisionRequest, MobileCommandDispatchRequest,
        MobilePairRequest, approve_command_data, dispatch_command_data, pair_node_data,
    },
};
use openrustclaw_core::config::AppConfig;
use openrustclaw_core::types::{IncomingMessage, Message, Platform, Session};
use openrustclaw_db::{SessionStatus, SqliteSessionStore};
use serde_json::json;
use tempfile::tempdir;
use uuid::Uuid;

use crate::common::{create_test_db_file, init_test_tracing};

fn incoming_message(
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
        user_id: "fixture-user".to_string(),
        content: "fixture hello".to_string(),
        platform,
        metadata,
    }
}

#[test]
fn documented_channel_routing_and_pairing_scenario() {
    init_test_tracing();

    let workspace = tempdir().unwrap();
    let control_root = workspace.path().join(".claw/control");
    control::init(Some(&control_root.to_string_lossy())).unwrap();

    let channel_root = workspace.path().join(".claw/channels");
    let channel_root_str = channel_root.to_string_lossy().to_string();
    init_channels(Some(&channel_root_str)).unwrap();

    upsert_binding(
        Some(&channel_root_str),
        ChannelBindingSpec {
            id: "slack-thread-main".to_string(),
            platform: "slack".to_string(),
            enabled: true,
            priority: 100,
            workspace_match: Some("T123".to_string()),
            account_match: Some("slack:T123:fixture-user".to_string()),
            channel_match: Some("slack_thread_ts=171234.000100".to_string()),
            workspace_target: Some("workspace-thread".to_string()),
            agent_id: Some("main".to_string()),
            activation_mode: Some("mention".to_string()),
            direct_strategy: None,
            group_strategy: Some("shared_channel".to_string()),
            send_policy: None,
            metadata: json!({ "scenario": "documented_parity" }),
        },
    )
    .unwrap();

    let mut policy = AppConfig::default().session_routing;
    policy.pairing_approval_required = true;

    let incoming = incoming_message(
        Platform::Slack,
        json!({
            "slack_team_id": "T123",
            "slack_channel": "C123",
            "slack_thread_ts": "171234.000100",
            "slack_is_group": true
        }),
        true,
    );

    let mut registry = load_channel_registry(channel_root).unwrap();
    let pending = preview_route(&mut registry, &incoming, &policy).unwrap();
    assert_eq!(pending.status, ChannelRouteStatus::PendingApproval);
    assert_eq!(pending.account_id, "slack:T123:fixture-user");

    approve_channel_account(Some(&channel_root_str), "slack:T123:fixture-user").unwrap();

    let channel_root = workspace.path().join(".claw/channels");
    let mut registry = load_channel_registry(channel_root).unwrap();
    let allowed = preview_route(&mut registry, &incoming, &policy).unwrap();
    assert_eq!(allowed.status, ChannelRouteStatus::Allowed);
    assert!(allowed.should_respond);
    assert_eq!(allowed.binding_id.as_deref(), Some("slack-thread-main"));
    assert_eq!(allowed.workspace_id.as_deref(), Some("T123"));
    assert_eq!(allowed.agent_id.as_deref(), Some("main"));
}

#[tokio::test]
async fn documented_mobile_operator_scenario() {
    init_test_tracing();

    let workspace = tempdir().unwrap();
    pair_node_data(
        workspace.path(),
        MobilePairRequest {
            id: "scenario-node".to_string(),
            gateway_url: "ws://127.0.0.1:18789/ws".to_string(),
            auth_token_env: "PATH".to_string(),
            device_name: Some("Scenario Phone".to_string()),
            platform: Some("ios".to_string()),
            capabilities: vec!["mobile".to_string(), "notifications".to_string()],
            enabled: true,
            sync: None,
            notifications: None,
            metadata: json!({ "scenario": "documented_parity" }),
        },
    )
    .unwrap();

    let pending = dispatch_command_data(
        workspace.path(),
        MobileCommandDispatchRequest {
            node_id: "scenario-node".to_string(),
            command: DeviceCommandKind::PushNotification,
            payload: json!({
                "title": "Scenario push",
                "body": "Operator approval flow"
            }),
            approved_by: None,
            require_approval: Some(true),
        },
    )
    .await
    .unwrap();

    assert_eq!(pending.status, "pending_approval");

    let executed = approve_command_data(
        workspace.path(),
        &pending.id,
        MobileCommandDecisionRequest {
            decided_by: "scenario-operator".to_string(),
            reason: Some("documented mobile command approval".to_string()),
        },
    )
    .await
    .unwrap();

    assert_eq!(executed.status, "executed");
    assert_eq!(executed.command, DeviceCommandKind::PushNotification);
}

#[tokio::test]
async fn documented_media_and_session_scenario() {
    init_test_tracing();

    let workspace = tempdir().unwrap();
    let note_path = workspace.path().join("scenario-note.txt");
    fs::write(&note_path, "scenario media text\nsecond line\n").unwrap();

    let inspect = inspect_data(MediaInspectRequest {
        path: note_path.display().to_string(),
    })
    .await
    .unwrap();
    assert_eq!(inspect.media_kind, "document");
    assert!(inspect.text_extractable);

    let extracted = extract_text_with_config(
        &AppConfig::default(),
        workspace.path(),
        MediaExtractTextRequest {
            path: note_path.display().to_string(),
            provider: None,
            model: None,
            language: None,
            prompt: None,
            max_audio_bytes: None,
            timeout_secs: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(extracted.text, "scenario media text\nsecond line\n");

    let (pool, _db_dir) = create_test_db_file().await;
    let store = SqliteSessionStore::new(pool);
    let session = Session::new_dm("scenario-user", Platform::WebChat);
    let session_id = session.id.to_string();

    store
        .create_or_update(
            &session,
            Some("webchat:scenario-user"),
            SessionStatus::Active,
        )
        .await
        .unwrap();
    store
        .append_message(&session_id, &Message::user("documented request"))
        .await
        .unwrap();
    store
        .append_message(&session_id, &Message::assistant("documented response"))
        .await
        .unwrap();
    store
        .set_status(
            &session_id,
            SessionStatus::Archived,
            Some("scenario archived"),
        )
        .await
        .unwrap();

    let history = store.list_history(&session_id, 10).await.unwrap();
    assert_eq!(history.len(), 2);

    let archived = store
        .list_sessions(Some(SessionStatus::Archived), 10)
        .await
        .unwrap();
    assert_eq!(archived.len(), 1);
    assert_eq!(
        archived[0].session.metadata["status_reason"],
        json!("scenario archived")
    );
}
