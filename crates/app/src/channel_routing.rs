use openrustclaw_core::config::SessionRoutingConfig;
use openrustclaw_core::types::{IncomingMessage, Platform};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelSendPolicy {
    pub mode: String,
    pub max_chunk_chars: usize,
    pub chunk_delay_ms: u64,
    pub coalesce_below_chars: Option<usize>,
    pub preview_chars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelIdentity {
    pub platform: Platform,
    pub account_id: String,
    pub external_user_id: String,
    pub workspace_id: Option<String>,
    pub channel_scope: Option<String>,
    pub is_group: bool,
    pub bot_mentioned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelAccountRouteSpec {
    pub id: String,
    pub blocked: bool,
    pub enabled: bool,
    pub approved: bool,
    pub workspace_target: Option<String>,
    pub agent_id: Option<String>,
    pub activation_mode: Option<String>,
    pub direct_strategy: Option<String>,
    pub group_strategy: Option<String>,
    pub send_policy: Option<ChannelSendPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelBindingRouteSpec {
    pub id: String,
    pub platform: String,
    pub enabled: bool,
    pub priority: i32,
    pub workspace_match: Option<String>,
    pub account_match: Option<String>,
    pub channel_match: Option<String>,
    pub workspace_target: Option<String>,
    pub agent_id: Option<String>,
    pub activation_mode: Option<String>,
    pub direct_strategy: Option<String>,
    pub group_strategy: Option<String>,
    pub send_policy: Option<ChannelSendPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelRouteStatus {
    Allowed,
    PendingApproval,
    Blocked,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelRoutePreview {
    pub status: ChannelRouteStatus,
    pub route_key: String,
    pub workspace_id: Option<String>,
    pub agent_id: Option<String>,
    pub activation_mode: String,
    pub send_policy: ChannelSendPolicy,
    pub account_id: String,
    pub binding_id: Option<String>,
    pub should_respond: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ChannelRoutingService;

impl ChannelRoutingService {
    pub fn new() -> Self {
        Self
    }

    pub fn identity_from_message(&self, message: &IncomingMessage) -> ChannelIdentity {
        let workspace_id = self.message_workspace_id(message);
        let channel_scope = self.message_channel_scope(message);
        let is_group = self.message_is_group(message);
        let bot_mentioned = self.message_bot_mentioned(message);
        let external_user_id = message.user_id.clone();
        let account_id = format!(
            "{}:{}:{}",
            message.platform,
            workspace_id.as_deref().unwrap_or("direct"),
            external_user_id
        );

        ChannelIdentity {
            platform: message.platform,
            account_id,
            external_user_id,
            workspace_id,
            channel_scope,
            is_group,
            bot_mentioned,
        }
    }

    pub fn message_workspace_id(&self, message: &IncomingMessage) -> Option<String> {
        let keys = [
            "workspace_id",
            "slack_team_id",
            "discord_guild_id",
            "teams_conversation_id",
            "mattermost_team_id",
            "google_chat_space",
            "matrix_room_id",
            "telegram_chat_id",
            "whatsapp_workspace_id",
            "imessage_workspace_id",
            "signal_group_id",
            "webchat_workspace_id",
        ];
        self.lookup_string(&message.metadata, &keys)
    }

    pub fn message_channel_scope(&self, message: &IncomingMessage) -> Option<String> {
        let keys = [
            "slack_thread_ts",
            "slack_channel",
            "discord_thread_id",
            "discord_channel_id",
            "teams_conversation_id",
            "mattermost_root_id",
            "mattermost_channel_id",
            "google_chat_thread",
            "google_chat_space",
            "matrix_thread_root",
            "matrix_room_id",
            "whatsapp_group_id",
            "imessage_chat_guid",
            "signal_group_id",
            "signal_source_number",
            "signal_source_uuid",
            "telegram_chat_id",
            "webchat_room_id",
        ];
        self.lookup_string(&message.metadata, &keys)
    }

    pub fn message_is_group(&self, message: &IncomingMessage) -> bool {
        let metadata = &message.metadata;
        [
            ("telegram_is_group", true),
            ("slack_is_group", true),
            ("webchat_is_group", true),
            ("whatsapp_is_group", true),
            ("imessage_is_group", true),
            ("signal_is_group", true),
            ("mattermost_is_group", true),
            ("teams_is_group", true),
            ("google_chat_is_group", true),
            ("matrix_is_group", true),
        ]
        .iter()
        .find_map(|(key, as_is)| {
            metadata
                .get(*key)
                .and_then(|value| value.as_bool())
                .map(|value| if *as_is { value } else { !value })
        })
        .or_else(|| {
            metadata
                .get("discord_is_dm")
                .and_then(|value| value.as_bool())
                .map(|value| !value)
        })
        .unwrap_or(false)
    }

    pub fn message_bot_mentioned(&self, message: &IncomingMessage) -> bool {
        [
            "slack_bot_mentioned",
            "discord_bot_mentioned",
            "telegram_bot_mentioned",
            "whatsapp_bot_mentioned",
            "imessage_bot_mentioned",
            "signal_bot_mentioned",
            "mattermost_bot_mentioned",
            "teams_bot_mentioned",
            "google_chat_bot_mentioned",
            "webchat_bot_mentioned",
        ]
        .iter()
        .any(|key| message.metadata.get(*key).and_then(|value| value.as_bool()) == Some(true))
    }

    pub fn channel_scope_from_metadata(
        &self,
        metadata: &Value,
        thread_overrides_channel: bool,
    ) -> Option<String> {
        let primary_keys: &[&str] = if thread_overrides_channel {
            &[
                "slack_thread_ts",
                "slack_channel",
                "telegram_chat_id",
                "discord_thread_id",
                "discord_channel_id",
                "google_chat_thread",
                "google_chat_space",
                "teams_conversation_id",
                "matrix_room_id",
                "whatsapp_chat_id",
                "line_room_id",
                "meta_thread_id",
            ]
        } else {
            &[
                "slack_channel",
                "telegram_chat_id",
                "discord_channel_id",
                "google_chat_space",
                "teams_conversation_id",
                "matrix_room_id",
                "whatsapp_chat_id",
                "line_room_id",
                "meta_thread_id",
            ]
        };

        for key in primary_keys {
            if let Some(value) = metadata.get(*key) {
                if let Some(text) = value.as_str() {
                    return Some(format!("{key}={text}"));
                }
                if let Some(number) = value.as_i64() {
                    return Some(format!("{key}={number}"));
                }
                if let Some(number) = value.as_u64() {
                    return Some(format!("{key}={number}"));
                }
            }
        }

        None
    }

    pub fn parent_channel_scope_from_metadata(&self, metadata: &Value) -> Option<String> {
        for key in ["discord_parent_channel_id", "slack_channel"] {
            if let Some(value) = metadata.get(key) {
                if let Some(text) = value.as_str() {
                    return Some(format!("{key}={text}"));
                }
                if let Some(number) = value.as_i64() {
                    return Some(format!("{key}={number}"));
                }
                if let Some(number) = value.as_u64() {
                    return Some(format!("{key}={number}"));
                }
            }
        }
        None
    }

    pub fn channel_scope_candidates(
        &self,
        metadata: &Value,
        thread_overrides_channel: bool,
    ) -> Vec<String> {
        let mut candidates = Vec::new();
        if let Some(primary) = self.channel_scope_from_metadata(metadata, thread_overrides_channel)
        {
            candidates.push(primary);
        }
        if let Some(parent) = self.parent_channel_scope_from_metadata(metadata)
            && !candidates.iter().any(|existing| existing == &parent)
        {
            candidates.push(parent);
        }
        candidates
    }

    pub fn default_send_policy(&self, policy: &SessionRoutingConfig) -> ChannelSendPolicy {
        ChannelSendPolicy {
            mode: policy.default_send_mode.clone(),
            max_chunk_chars: policy.default_chunk_chars,
            chunk_delay_ms: policy.default_chunk_delay_ms,
            coalesce_below_chars: Some(320),
            preview_chars: 280,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn route_key_with_binding(
        &self,
        message: &IncomingMessage,
        direct_strategy: &str,
        group_strategy: &str,
        thread_overrides_channel: bool,
        workspace_id: Option<&str>,
        agent_id: Option<&str>,
        account_id: Option<&str>,
    ) -> String {
        let mut prefix = vec![message.platform.to_string()];
        if let Some(workspace_id) = workspace_id {
            prefix.push(format!("workspace={workspace_id}"));
        }
        if let Some(agent_id) = agent_id {
            prefix.push(format!("agent={agent_id}"));
        }
        if let Some(account_id) = account_id {
            prefix.push(format!("account={account_id}"));
        }

        if let Some(scope) =
            self.channel_scope_from_metadata(&message.metadata, thread_overrides_channel)
        {
            if group_strategy == "shared_channel" {
                prefix.push(scope);
                prefix.push("shared".to_string());
                return prefix.join(":");
            }
            prefix.push(scope);
            prefix.push(message.user_id.clone());
            return prefix.join(":");
        }

        match direct_strategy {
            "shared_main" => {
                prefix.push("main".to_string());
                prefix.join(":")
            }
            _ => {
                prefix.push("direct".to_string());
                prefix.push(message.user_id.clone());
                prefix.join(":")
            }
        }
    }

    pub fn resolve_channel_binding<'a>(
        &self,
        bindings: &'a [ChannelBindingRouteSpec],
        platform: Platform,
        workspace_id: Option<&str>,
        account_id: Option<&str>,
        channel_scopes: &[String],
    ) -> Option<&'a ChannelBindingRouteSpec> {
        bindings
            .iter()
            .filter(|binding| binding.enabled && binding.platform == platform.to_string())
            .filter(|binding| {
                binding
                    .workspace_match
                    .as_deref()
                    .map(|value| workspace_id == Some(value))
                    .unwrap_or(true)
            })
            .filter(|binding| {
                binding
                    .account_match
                    .as_deref()
                    .map(|value| account_id == Some(value))
                    .unwrap_or(true)
            })
            .filter(|binding| {
                binding
                    .channel_match
                    .as_deref()
                    .map(|value| channel_scopes.iter().any(|scope| scope == value))
                    .unwrap_or(true)
            })
            .max_by_key(|binding| {
                let specificity = usize::from(binding.workspace_match.is_some())
                    + usize::from(binding.account_match.is_some())
                    + usize::from(binding.channel_match.is_some());
                (specificity, -(binding.priority as isize))
            })
    }

    pub fn preview_route(
        &self,
        incoming: &IncomingMessage,
        account: &ChannelAccountRouteSpec,
        bindings: &[ChannelBindingRouteSpec],
        policy: &SessionRoutingConfig,
        identity: &ChannelIdentity,
    ) -> ChannelRoutePreview {
        let channel_scopes =
            self.channel_scope_candidates(&incoming.metadata, policy.thread_overrides_channel);
        let binding = self.resolve_channel_binding(
            bindings,
            incoming.platform,
            identity.workspace_id.as_deref(),
            Some(account.id.as_str()),
            &channel_scopes,
        );

        let direct_strategy = account
            .direct_strategy
            .clone()
            .or_else(|| binding.and_then(|value| value.direct_strategy.clone()))
            .unwrap_or_else(|| policy.direct_strategy.clone());
        let group_strategy = account
            .group_strategy
            .clone()
            .or_else(|| binding.and_then(|value| value.group_strategy.clone()))
            .unwrap_or_else(|| policy.group_strategy.clone());
        let activation_mode = account
            .activation_mode
            .clone()
            .or_else(|| binding.and_then(|value| value.activation_mode.clone()))
            .unwrap_or_else(|| policy.default_group_activation.clone());
        let send_policy = account
            .send_policy
            .clone()
            .or_else(|| binding.and_then(|value| value.send_policy.clone()))
            .unwrap_or_else(|| self.default_send_policy(policy));
        let workspace_id = account
            .workspace_target
            .clone()
            .or_else(|| binding.and_then(|value| value.workspace_target.clone()))
            .or_else(|| identity.workspace_id.clone());
        let agent_id = account
            .agent_id
            .clone()
            .or_else(|| binding.and_then(|value| value.agent_id.clone()));
        let route_key = self.route_key_with_binding(
            incoming,
            &direct_strategy,
            &group_strategy,
            policy.thread_overrides_channel,
            workspace_id.as_deref(),
            agent_id.as_deref(),
            Some(account.id.as_str()),
        );

        let status = if account.blocked {
            ChannelRouteStatus::Blocked
        } else if !account.enabled {
            ChannelRouteStatus::Disabled
        } else if !account.approved {
            ChannelRouteStatus::PendingApproval
        } else {
            ChannelRouteStatus::Allowed
        };
        let should_respond =
            !identity.is_group || activation_mode != "mention" || identity.bot_mentioned;

        ChannelRoutePreview {
            status,
            route_key,
            workspace_id,
            agent_id,
            activation_mode,
            send_policy,
            account_id: account.id.clone(),
            binding_id: binding.map(|value| value.id.clone()),
            should_respond,
        }
    }

    fn lookup_string(&self, metadata: &Value, keys: &[&str]) -> Option<String> {
        keys.iter().find_map(|key| {
            metadata
                .get(*key)
                .and_then(|value| value.as_str().map(ToString::to_string))
                .or_else(|| {
                    metadata
                        .get(*key)
                        .and_then(|value| value.as_i64().map(|value| value.to_string()))
                })
                .or_else(|| {
                    metadata
                        .get(*key)
                        .and_then(|value| value.as_u64().map(|value| value.to_string()))
                })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openrustclaw_core::config::AppConfig;

    #[test]
    fn message_identity_uses_workspace_and_user() {
        let service = ChannelRoutingService::new();
        let identity = service.identity_from_message(&IncomingMessage {
            session_id: uuid::Uuid::new_v4(),
            user_id: "U123".to_string(),
            content: "hello".to_string(),
            platform: Platform::Slack,
            metadata: serde_json::json!({
                "slack_team_id": "T123",
                "slack_channel": "C123",
                "slack_is_group": true,
                "slack_bot_mentioned": true
            }),
        });

        assert_eq!(identity.account_id, "slack:T123:U123");
        assert_eq!(identity.channel_scope.as_deref(), Some("C123"));
        assert!(identity.is_group);
        assert!(identity.bot_mentioned);
    }

    #[test]
    fn route_preview_marks_unapproved_accounts_pending() {
        let service = ChannelRoutingService::new();
        let mut policy = AppConfig::default().session_routing;
        policy.pairing_approval_required = true;
        let incoming = IncomingMessage {
            session_id: uuid::Uuid::new_v4(),
            user_id: "U111".to_string(),
            content: "hello".to_string(),
            platform: Platform::Slack,
            metadata: serde_json::json!({
                "slack_team_id": "T111"
            }),
        };
        let identity = service.identity_from_message(&incoming);
        let preview = service.preview_route(
            &incoming,
            &ChannelAccountRouteSpec {
                id: identity.account_id.clone(),
                blocked: false,
                enabled: true,
                approved: false,
                workspace_target: None,
                agent_id: None,
                activation_mode: Some("mention".to_string()),
                direct_strategy: None,
                group_strategy: None,
                send_policy: None,
            },
            &[],
            &policy,
            &identity,
        );
        assert_eq!(preview.status, ChannelRouteStatus::PendingApproval);
    }
}
