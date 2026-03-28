use async_trait::async_trait;
use openrustclaw_core::error::Result;
use serde_json::Value;

#[async_trait]
pub trait OperatorStatusControlSource {
    async fn runtime_status(&self) -> Result<Value>;
    async fn runtime_operator_ops(&self) -> Result<Value>;
    async fn voice_status(&self) -> Result<Value>;
    async fn voice_providers(&self) -> Result<Value>;
    async fn voice_metrics(&self) -> Result<Value>;
    async fn voice_operator_summary(
        &self,
        limit: usize,
        stale_after_secs: Option<u64>,
    ) -> Result<Value>;
    async fn voice_outcomes(
        &self,
        stale_after_secs: Option<u64>,
        limit: Option<usize>,
    ) -> Result<Value>;
    async fn voice_sessions(&self) -> Result<Value>;
    async fn voice_session_health(&self, stale_after_secs: Option<u64>) -> Result<Value>;
    async fn talk_status(&self, limit: usize) -> Result<Value>;
    async fn talk_metrics(&self) -> Result<Value>;
    async fn talk_sessions(&self, limit: usize) -> Result<Value>;
    async fn mobile_nodes(&self) -> Result<Value>;
    async fn mobile_node_summary(&self, id: &str, limit: Option<usize>) -> Result<Value>;
}

pub struct OperatorStatusControlService<S> {
    source: S,
}

impl<S> OperatorStatusControlService<S> {
    pub fn new(source: S) -> Self {
        Self { source }
    }

    pub fn resolved_voice_operator_limit(&self, limit: Option<usize>) -> usize {
        limit.unwrap_or(12).max(1)
    }

    pub fn resolved_talk_limit(&self, limit: Option<usize>) -> usize {
        limit.unwrap_or(20).max(1)
    }

    pub fn resolved_mobile_summary_limit(&self, limit: Option<usize>) -> Option<usize> {
        Some(limit.unwrap_or(12).max(1))
    }
}

impl<S> OperatorStatusControlService<S>
where
    S: OperatorStatusControlSource,
{
    pub async fn runtime_status(&self) -> Result<Value> {
        self.source.runtime_status().await
    }

    pub async fn runtime_operator_ops(&self) -> Result<Value> {
        self.source.runtime_operator_ops().await
    }

    pub async fn voice_status(&self) -> Result<Value> {
        self.source.voice_status().await
    }

    pub async fn voice_providers(&self) -> Result<Value> {
        self.source.voice_providers().await
    }

    pub async fn voice_metrics(&self) -> Result<Value> {
        self.source.voice_metrics().await
    }

    pub async fn voice_operator_summary(
        &self,
        limit: Option<usize>,
        stale_after_secs: Option<u64>,
    ) -> Result<Value> {
        self.source
            .voice_operator_summary(self.resolved_voice_operator_limit(limit), stale_after_secs)
            .await
    }

    pub async fn voice_outcomes(
        &self,
        stale_after_secs: Option<u64>,
        limit: Option<usize>,
    ) -> Result<Value> {
        self.source.voice_outcomes(stale_after_secs, limit).await
    }

    pub async fn voice_sessions(&self) -> Result<Value> {
        self.source.voice_sessions().await
    }

    pub async fn voice_session_health(&self, stale_after_secs: Option<u64>) -> Result<Value> {
        self.source.voice_session_health(stale_after_secs).await
    }

    pub async fn talk_status(&self, limit: Option<usize>) -> Result<Value> {
        self.source
            .talk_status(self.resolved_talk_limit(limit))
            .await
    }

    pub async fn talk_metrics(&self) -> Result<Value> {
        self.source.talk_metrics().await
    }

    pub async fn talk_sessions(&self, limit: Option<usize>) -> Result<Value> {
        self.source
            .talk_sessions(self.resolved_talk_limit(limit))
            .await
    }

    pub async fn mobile_nodes(&self) -> Result<Value> {
        Ok(serde_json::json!({
            "nodes": self.source.mobile_nodes().await?,
        }))
    }

    pub async fn mobile_node_summary(&self, id: &str, limit: Option<usize>) -> Result<Value> {
        self.source
            .mobile_node_summary(id, self.resolved_mobile_summary_limit(limit))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockOperatorStatusControlSource {
        talk_limit: Mutex<Option<usize>>,
        voice_operator_limit: Mutex<Option<usize>>,
        mobile_summary_limit: Mutex<Option<Option<usize>>>,
    }

    #[async_trait]
    impl OperatorStatusControlSource for MockOperatorStatusControlSource {
        async fn runtime_status(&self) -> Result<Value> {
            Ok(serde_json::json!({"network_mode": "loopback"}))
        }

        async fn runtime_operator_ops(&self) -> Result<Value> {
            Ok(serde_json::json!({"status": "ok"}))
        }

        async fn voice_status(&self) -> Result<Value> {
            Ok(serde_json::json!({"enabled": false}))
        }

        async fn voice_providers(&self) -> Result<Value> {
            Ok(serde_json::json!([]))
        }

        async fn voice_metrics(&self) -> Result<Value> {
            Ok(serde_json::json!({"sessions": []}))
        }

        async fn voice_operator_summary(
            &self,
            limit: usize,
            _stale_after_secs: Option<u64>,
        ) -> Result<Value> {
            *self.voice_operator_limit.lock().unwrap() = Some(limit);
            Ok(serde_json::json!({"limit": limit}))
        }

        async fn voice_outcomes(
            &self,
            _stale_after_secs: Option<u64>,
            limit: Option<usize>,
        ) -> Result<Value> {
            Ok(serde_json::json!({"limit": limit}))
        }

        async fn voice_sessions(&self) -> Result<Value> {
            Ok(serde_json::json!({"sessions": []}))
        }

        async fn voice_session_health(&self, stale_after_secs: Option<u64>) -> Result<Value> {
            Ok(serde_json::json!({"stale_after_secs": stale_after_secs}))
        }

        async fn talk_status(&self, limit: usize) -> Result<Value> {
            *self.talk_limit.lock().unwrap() = Some(limit);
            Ok(serde_json::json!({"total_sessions": 0}))
        }

        async fn talk_metrics(&self) -> Result<Value> {
            Ok(serde_json::json!({"total_sessions": 0}))
        }

        async fn talk_sessions(&self, limit: usize) -> Result<Value> {
            *self.talk_limit.lock().unwrap() = Some(limit);
            Ok(serde_json::json!({"sessions": [], "limit": limit}))
        }

        async fn mobile_nodes(&self) -> Result<Value> {
            Ok(serde_json::json!([]))
        }

        async fn mobile_node_summary(&self, _id: &str, limit: Option<usize>) -> Result<Value> {
            *self.mobile_summary_limit.lock().unwrap() = Some(limit);
            Ok(serde_json::json!({"status": "ok", "limit": limit}))
        }
    }

    #[tokio::test]
    async fn operator_status_service_applies_default_limits() -> Result<()> {
        let source = MockOperatorStatusControlSource::default();
        let service = OperatorStatusControlService::new(source);

        let voice = service.voice_operator_summary(None, None).await?;
        assert_eq!(voice["limit"], 12);

        let talk = service.talk_status(None).await?;
        assert_eq!(talk["total_sessions"], 0);

        let mobile = service.mobile_node_summary("node-1", None).await?;
        assert_eq!(mobile["limit"], 12);
        Ok(())
    }

    #[tokio::test]
    async fn operator_status_service_wraps_mobile_nodes() -> Result<()> {
        let service = OperatorStatusControlService::new(MockOperatorStatusControlSource::default());
        let nodes = service.mobile_nodes().await?;
        assert_eq!(nodes, serde_json::json!({"nodes": []}));
        Ok(())
    }

    #[tokio::test]
    async fn operator_status_service_passes_through_status_reports() -> Result<()> {
        let service = OperatorStatusControlService::new(MockOperatorStatusControlSource::default());
        let runtime = service.runtime_status().await?;
        assert_eq!(runtime["network_mode"], "loopback");
        let voice = service.voice_status().await?;
        assert_eq!(voice["enabled"], false);
        Ok(())
    }
}
