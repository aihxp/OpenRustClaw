use async_trait::async_trait;
use openrustclaw_core::error::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillStartVoiceCallRequest {
    pub plugin_id: String,
    #[serde(default)]
    pub remote: Option<String>,
    #[serde(default)]
    pub greeting_text: Option<String>,
    #[serde(default)]
    pub voice: Option<String>,
    #[serde(default)]
    pub metadata: Option<String>,
    #[serde(default)]
    pub stale_after_secs: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillEndVoiceCallRequest {
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillReconnectVoiceCallRequest {
    #[serde(default)]
    pub remote: Option<String>,
    #[serde(default)]
    pub greeting_text: Option<String>,
    #[serde(default)]
    pub voice: Option<String>,
    #[serde(default)]
    pub metadata: Option<String>,
    #[serde(default)]
    pub stale_after_secs: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillReapVoiceCallsRequest {
    #[serde(default)]
    pub stale_after_secs: Option<u64>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillBindChannelExtensionRequest {
    pub binding_id: String,
    pub skill_name: String,
    #[serde(default)]
    pub service: Option<String>,
    #[serde(default)]
    pub component: Option<String>,
    #[serde(default)]
    pub trigger: Option<String>,
}

#[async_trait]
pub trait SkillVoiceChannelControlSource {
    async fn voice_calls(&self) -> Result<Value>;
    async fn voice_call_health(&self) -> Result<Value>;
    async fn voice_call_metrics(&self) -> Result<Value>;
    async fn voice_call_events(&self, call_id: &str) -> Result<Value>;
    async fn voice_call_artifacts(&self, call_id: &str) -> Result<Value>;
    async fn start_voice_call(&self, request: SkillStartVoiceCallRequest) -> Result<Value>;
    async fn reap_voice_calls(&self, request: SkillReapVoiceCallsRequest) -> Result<Value>;
    async fn end_voice_call(
        &self,
        call_id: &str,
        request: SkillEndVoiceCallRequest,
    ) -> Result<Value>;
    async fn reconnect_voice_call(
        &self,
        call_id: &str,
        request: SkillReconnectVoiceCallRequest,
    ) -> Result<Value>;
    async fn channel_extensions(&self) -> Result<Value>;
    async fn bind_channel_extension(
        &self,
        request: SkillBindChannelExtensionRequest,
    ) -> Result<Value>;
}

pub struct SkillVoiceChannelControlService<S> {
    source: S,
}

impl<S> SkillVoiceChannelControlService<S> {
    pub fn new(source: S) -> Self {
        Self { source }
    }
}

impl<S> SkillVoiceChannelControlService<S>
where
    S: SkillVoiceChannelControlSource,
{
    pub async fn voice_calls(&self) -> Result<Value> {
        self.source.voice_calls().await
    }

    pub async fn voice_call_health(&self) -> Result<Value> {
        self.source.voice_call_health().await
    }

    pub async fn voice_call_metrics(&self) -> Result<Value> {
        self.source.voice_call_metrics().await
    }

    pub async fn voice_call_events(&self, call_id: &str) -> Result<Value> {
        self.source.voice_call_events(call_id).await
    }

    pub async fn voice_call_artifacts(&self, call_id: &str) -> Result<Value> {
        self.source.voice_call_artifacts(call_id).await
    }

    pub async fn start_voice_call(&self, request: SkillStartVoiceCallRequest) -> Result<Value> {
        self.source.start_voice_call(request).await
    }

    pub async fn reap_voice_calls(&self, request: SkillReapVoiceCallsRequest) -> Result<Value> {
        self.source.reap_voice_calls(request).await
    }

    pub async fn end_voice_call(
        &self,
        call_id: &str,
        request: SkillEndVoiceCallRequest,
    ) -> Result<Value> {
        self.source.end_voice_call(call_id, request).await
    }

    pub async fn reconnect_voice_call(
        &self,
        call_id: &str,
        request: SkillReconnectVoiceCallRequest,
    ) -> Result<Value> {
        self.source.reconnect_voice_call(call_id, request).await
    }

    pub async fn channel_extensions(&self) -> Result<Value> {
        self.source.channel_extensions().await
    }

    pub async fn bind_channel_extension(
        &self,
        request: SkillBindChannelExtensionRequest,
    ) -> Result<Value> {
        self.source.bind_channel_extension(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockSkillVoiceChannelControlSource {
        call_ids: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl SkillVoiceChannelControlSource for MockSkillVoiceChannelControlSource {
        async fn voice_calls(&self) -> Result<Value> {
            Ok(json!({ "count": 1, "calls": [{ "call_id": "call-1" }] }))
        }

        async fn voice_call_health(&self) -> Result<Value> {
            Ok(json!({ "health": { "active": 1 } }))
        }

        async fn voice_call_metrics(&self) -> Result<Value> {
            Ok(json!({ "metrics": { "active": 1 } }))
        }

        async fn voice_call_events(&self, call_id: &str) -> Result<Value> {
            self.call_ids
                .lock()
                .expect("call ids lock")
                .push(call_id.to_string());
            Ok(json!({ "call": { "call_id": call_id }, "events": [] }))
        }

        async fn voice_call_artifacts(&self, call_id: &str) -> Result<Value> {
            Ok(json!({ "call": { "call_id": call_id }, "artifacts": [] }))
        }

        async fn start_voice_call(&self, request: SkillStartVoiceCallRequest) -> Result<Value> {
            Ok(json!({
                "status": "ok",
                "call": {
                    "call_id": "call-1",
                    "plugin_id": request.plugin_id,
                    "remote": request.remote
                }
            }))
        }

        async fn reap_voice_calls(&self, request: SkillReapVoiceCallsRequest) -> Result<Value> {
            Ok(json!({ "status": "ok", "checked": request.limit.unwrap_or(0) }))
        }

        async fn end_voice_call(
            &self,
            call_id: &str,
            request: SkillEndVoiceCallRequest,
        ) -> Result<Value> {
            Ok(json!({ "status": "ok", "call": { "call_id": call_id, "reason": request.reason } }))
        }

        async fn reconnect_voice_call(
            &self,
            call_id: &str,
            request: SkillReconnectVoiceCallRequest,
        ) -> Result<Value> {
            Ok(json!({ "status": "ok", "call": { "call_id": call_id, "remote": request.remote } }))
        }

        async fn channel_extensions(&self) -> Result<Value> {
            Ok(json!({ "count": 1, "extensions": [{ "binding_id": "support-inbox" }] }))
        }

        async fn bind_channel_extension(
            &self,
            request: SkillBindChannelExtensionRequest,
        ) -> Result<Value> {
            Ok(json!({
                "status": "ok",
                "binding_id": request.binding_id,
                "skill_name": request.skill_name
            }))
        }
    }

    #[tokio::test]
    async fn skill_voice_channel_control_service_routes_voice_calls_and_extensions() -> Result<()> {
        let service =
            SkillVoiceChannelControlService::new(MockSkillVoiceChannelControlSource::default());

        assert_eq!(service.voice_calls().await?["count"], 1);
        assert_eq!(service.voice_call_health().await?["health"]["active"], 1);
        assert_eq!(service.voice_call_metrics().await?["metrics"]["active"], 1);
        assert_eq!(
            service.channel_extensions().await?["extensions"][0]["binding_id"],
            "support-inbox"
        );

        Ok(())
    }

    #[tokio::test]
    async fn skill_voice_channel_control_service_preserves_call_ids_and_payloads() -> Result<()> {
        let service =
            SkillVoiceChannelControlService::new(MockSkillVoiceChannelControlSource::default());

        let start = service
            .start_voice_call(SkillStartVoiceCallRequest {
                plugin_id: "support-line".to_string(),
                remote: Some("15551234567".to_string()),
                ..Default::default()
            })
            .await?;
        assert_eq!(start["call"]["plugin_id"], "support-line");

        let events = service.voice_call_events("call-1").await?;
        assert_eq!(events["call"]["call_id"], "call-1");

        let reconnect = service
            .reconnect_voice_call(
                "call-1",
                SkillReconnectVoiceCallRequest {
                    remote: Some("15557654321".to_string()),
                    ..Default::default()
                },
            )
            .await?;
        assert_eq!(reconnect["call"]["remote"], "15557654321");

        let end = service
            .end_voice_call(
                "call-1",
                SkillEndVoiceCallRequest {
                    reason: Some("operator_complete".to_string()),
                    ..Default::default()
                },
            )
            .await?;
        assert_eq!(end["call"]["reason"], "operator_complete");

        let bind = service
            .bind_channel_extension(SkillBindChannelExtensionRequest {
                binding_id: "support-inbox".to_string(),
                skill_name: "demo".to_string(),
                ..Default::default()
            })
            .await?;
        assert_eq!(bind["binding_id"], "support-inbox");

        Ok(())
    }
}
