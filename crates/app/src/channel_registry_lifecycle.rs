use async_trait::async_trait;
use openrustclaw_core::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelSendPolicyRequest {
    pub mode: String,
    pub max_chunk_chars: usize,
    pub chunk_delay_ms: u64,
    pub coalesce_below_chars: Option<usize>,
    pub preview_chars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChannelRegistryAccountUpsertRequest {
    pub id: String,
    pub platform: String,
    pub external_user_id: String,
    pub display_name: Option<String>,
    pub workspace_id: Option<String>,
    pub channel_scope: Option<String>,
    pub enabled: bool,
    pub approved: bool,
    pub blocked: bool,
    pub workspace_target: Option<String>,
    pub agent_id: Option<String>,
    pub activation_mode: Option<String>,
    pub direct_strategy: Option<String>,
    pub group_strategy: Option<String>,
    pub send_policy: Option<ChannelSendPolicyRequest>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChannelRegistryBindingUpsertRequest {
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
    pub send_policy: Option<ChannelSendPolicyRequest>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChannelRegistryBindRequest {
    pub id: String,
    pub platform: String,
    pub workspace_match: Option<String>,
    pub account_match: Option<String>,
    pub channel_match: Option<String>,
    pub workspace_target: Option<String>,
    pub agent_id: Option<String>,
    pub activation_mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelRegistryAccountMutationAction {
    Approve,
    Block,
    Activation { mode: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelRegistryMutationReport {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

#[async_trait]
pub trait ChannelRegistryLifecycleSource {
    async fn upsert_account(&self, request: ChannelRegistryAccountUpsertRequest) -> Result<()>;
    async fn delete_account(&self, id: &str) -> Result<()>;
    async fn mutate_account(
        &self,
        id: &str,
        action: ChannelRegistryAccountMutationAction,
    ) -> Result<()>;
    async fn upsert_binding(&self, request: ChannelRegistryBindingUpsertRequest) -> Result<()>;
    async fn delete_binding(&self, id: &str) -> Result<()>;
}

pub struct ChannelRegistryLifecycleService<S> {
    source: S,
}

impl<S> ChannelRegistryLifecycleService<S> {
    pub fn new(source: S) -> Self {
        Self { source }
    }

    fn validate_expected_id(expected_id: Option<&str>, actual_id: &str, kind: &str) -> Result<()> {
        if let Some(expected_id) = expected_id
            && expected_id != actual_id
        {
            return Err(Error::Internal(format!("{kind} id does not match path")));
        }
        Ok(())
    }

    pub fn build_binding_request(
        &self,
        request: ChannelRegistryBindRequest,
    ) -> ChannelRegistryBindingUpsertRequest {
        ChannelRegistryBindingUpsertRequest {
            id: request.id,
            platform: request.platform,
            enabled: true,
            priority: 100,
            workspace_match: request.workspace_match,
            account_match: request.account_match,
            channel_match: request.channel_match,
            workspace_target: request.workspace_target,
            agent_id: request.agent_id,
            activation_mode: request.activation_mode,
            direct_strategy: None,
            group_strategy: None,
            send_policy: None,
            metadata: serde_json::json!({}),
        }
    }
}

impl<S> ChannelRegistryLifecycleService<S>
where
    S: ChannelRegistryLifecycleSource,
{
    pub async fn upsert_account(
        &self,
        expected_id: Option<&str>,
        request: ChannelRegistryAccountUpsertRequest,
    ) -> Result<ChannelRegistryMutationReport> {
        Self::validate_expected_id(expected_id, &request.id, "account")?;
        self.source.upsert_account(request.clone()).await?;
        Ok(ChannelRegistryMutationReport {
            status: "ok".to_string(),
            account_id: Some(request.id),
            binding_id: None,
            action: None,
        })
    }

    pub async fn delete_account(&self, id: &str) -> Result<ChannelRegistryMutationReport> {
        self.source.delete_account(id).await?;
        Ok(ChannelRegistryMutationReport {
            status: "ok".to_string(),
            account_id: Some(id.to_string()),
            binding_id: None,
            action: Some("delete".to_string()),
        })
    }

    pub async fn approve_account(&self, id: &str) -> Result<ChannelRegistryMutationReport> {
        self.source
            .mutate_account(id, ChannelRegistryAccountMutationAction::Approve)
            .await?;
        Ok(ChannelRegistryMutationReport {
            status: "ok".to_string(),
            account_id: Some(id.to_string()),
            binding_id: None,
            action: Some("approve".to_string()),
        })
    }

    pub async fn block_account(&self, id: &str) -> Result<ChannelRegistryMutationReport> {
        self.source
            .mutate_account(id, ChannelRegistryAccountMutationAction::Block)
            .await?;
        Ok(ChannelRegistryMutationReport {
            status: "ok".to_string(),
            account_id: Some(id.to_string()),
            binding_id: None,
            action: Some("block".to_string()),
        })
    }

    pub async fn activate_account(
        &self,
        id: &str,
        mode: String,
    ) -> Result<ChannelRegistryMutationReport> {
        self.source
            .mutate_account(
                id,
                ChannelRegistryAccountMutationAction::Activation { mode },
            )
            .await?;
        Ok(ChannelRegistryMutationReport {
            status: "ok".to_string(),
            account_id: Some(id.to_string()),
            binding_id: None,
            action: Some("activation".to_string()),
        })
    }

    pub async fn bind_channel(
        &self,
        request: ChannelRegistryBindRequest,
    ) -> Result<ChannelRegistryMutationReport> {
        let binding = self.build_binding_request(request);
        self.source.upsert_binding(binding.clone()).await?;
        Ok(ChannelRegistryMutationReport {
            status: "ok".to_string(),
            account_id: None,
            binding_id: Some(binding.id),
            action: None,
        })
    }

    pub async fn upsert_binding(
        &self,
        expected_id: Option<&str>,
        request: ChannelRegistryBindingUpsertRequest,
    ) -> Result<ChannelRegistryMutationReport> {
        Self::validate_expected_id(expected_id, &request.id, "binding")?;
        self.source.upsert_binding(request.clone()).await?;
        Ok(ChannelRegistryMutationReport {
            status: "ok".to_string(),
            account_id: None,
            binding_id: Some(request.id),
            action: None,
        })
    }

    pub async fn delete_binding(&self, id: &str) -> Result<ChannelRegistryMutationReport> {
        self.source.delete_binding(id).await?;
        Ok(ChannelRegistryMutationReport {
            status: "ok".to_string(),
            account_id: None,
            binding_id: Some(id.to_string()),
            action: Some("delete".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockChannelRegistryLifecycleSource {
        last_account_upsert: Mutex<Option<ChannelRegistryAccountUpsertRequest>>,
        last_binding_upsert: Mutex<Option<ChannelRegistryBindingUpsertRequest>>,
        last_account_mutation: Mutex<Option<(String, ChannelRegistryAccountMutationAction)>>,
        deleted_account: Mutex<Option<String>>,
        deleted_binding: Mutex<Option<String>>,
    }

    #[async_trait]
    impl ChannelRegistryLifecycleSource for MockChannelRegistryLifecycleSource {
        async fn upsert_account(&self, request: ChannelRegistryAccountUpsertRequest) -> Result<()> {
            *self.last_account_upsert.lock().unwrap() = Some(request);
            Ok(())
        }

        async fn delete_account(&self, id: &str) -> Result<()> {
            *self.deleted_account.lock().unwrap() = Some(id.to_string());
            Ok(())
        }

        async fn mutate_account(
            &self,
            id: &str,
            action: ChannelRegistryAccountMutationAction,
        ) -> Result<()> {
            *self.last_account_mutation.lock().unwrap() = Some((id.to_string(), action));
            Ok(())
        }

        async fn upsert_binding(&self, request: ChannelRegistryBindingUpsertRequest) -> Result<()> {
            *self.last_binding_upsert.lock().unwrap() = Some(request);
            Ok(())
        }

        async fn delete_binding(&self, id: &str) -> Result<()> {
            *self.deleted_binding.lock().unwrap() = Some(id.to_string());
            Ok(())
        }
    }

    fn sample_account_request() -> ChannelRegistryAccountUpsertRequest {
        ChannelRegistryAccountUpsertRequest {
            id: "acct-1".to_string(),
            platform: "slack".to_string(),
            external_user_id: "U123".to_string(),
            display_name: Some("Support".to_string()),
            workspace_id: Some("T123".to_string()),
            channel_scope: None,
            enabled: true,
            approved: false,
            blocked: false,
            workspace_target: Some("workspace-a".to_string()),
            agent_id: None,
            activation_mode: Some("mention".to_string()),
            direct_strategy: None,
            group_strategy: None,
            send_policy: None,
            metadata: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn channel_registry_service_rejects_path_id_mismatch() {
        let service =
            ChannelRegistryLifecycleService::new(MockChannelRegistryLifecycleSource::default());

        let error = service
            .upsert_account(Some("acct-2"), sample_account_request())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("account id does not match path"));
    }

    #[tokio::test]
    async fn channel_registry_service_shapes_account_and_binding_reports() -> Result<()> {
        let service =
            ChannelRegistryLifecycleService::new(MockChannelRegistryLifecycleSource::default());

        let account = service
            .upsert_account(None, sample_account_request())
            .await?;
        assert_eq!(account.status, "ok");
        assert_eq!(account.account_id.as_deref(), Some("acct-1"));

        let binding = service
            .bind_channel(ChannelRegistryBindRequest {
                id: "binding-1".to_string(),
                platform: "slack".to_string(),
                workspace_match: Some("T123".to_string()),
                account_match: Some("acct-1".to_string()),
                channel_match: Some("C123".to_string()),
                workspace_target: Some("workspace-a".to_string()),
                agent_id: None,
                activation_mode: Some("mentioned".to_string()),
            })
            .await?;
        assert_eq!(binding.status, "ok");
        assert_eq!(binding.binding_id.as_deref(), Some("binding-1"));
        Ok(())
    }

    #[tokio::test]
    async fn channel_registry_service_shapes_account_mutations() -> Result<()> {
        let service =
            ChannelRegistryLifecycleService::new(MockChannelRegistryLifecycleSource::default());

        let approve = service.approve_account("acct-1").await?;
        assert_eq!(approve.action.as_deref(), Some("approve"));

        let activate = service
            .activate_account("acct-1", "always".to_string())
            .await?;
        assert_eq!(activate.action.as_deref(), Some("activation"));

        let delete = service.delete_binding("binding-1").await?;
        assert_eq!(delete.action.as_deref(), Some("delete"));
        Ok(())
    }
}
