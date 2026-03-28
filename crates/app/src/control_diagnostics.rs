use async_trait::async_trait;
use openrustclaw_core::error::Result;
use serde::Serialize;
use serde_json::Value;

#[async_trait]
pub trait ControlDiagnosticsSource {
    async fn collect_report(&self, repair: bool, deep: bool) -> Result<Value>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ControlDiagnosticsEvent {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub struct ControlDiagnosticsService<S> {
    source: S,
}

impl<S> ControlDiagnosticsService<S> {
    pub fn new(source: S) -> Self {
        Self { source }
    }

    pub fn resolved_interval_secs(&self, requested: Option<u64>) -> u64 {
        requested.unwrap_or(5).max(1)
    }
}

impl<S> ControlDiagnosticsService<S>
where
    S: ControlDiagnosticsSource,
{
    pub async fn report(&self, repair: bool, deep: bool) -> Result<Value> {
        self.source.collect_report(repair, deep).await
    }

    pub async fn event_payload(&self, repair: bool, deep: bool) -> ControlDiagnosticsEvent {
        match self.source.collect_report(repair, deep).await {
            Ok(report) => ControlDiagnosticsEvent {
                kind: "diagnostics".to_string(),
                report: Some(report),
                error: None,
            },
            Err(error) => ControlDiagnosticsEvent {
                kind: "error".to_string(),
                report: None,
                error: Some(error.to_string()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openrustclaw_core::error::Error;
    use std::sync::Mutex;

    struct MockControlDiagnosticsSource {
        next_result: Mutex<Option<Result<Value>>>,
    }

    #[async_trait]
    impl ControlDiagnosticsSource for MockControlDiagnosticsSource {
        async fn collect_report(&self, _repair: bool, _deep: bool) -> Result<Value> {
            self.next_result
                .lock()
                .unwrap()
                .take()
                .unwrap_or_else(|| Ok(serde_json::json!({"healthy": true})))
        }
    }

    #[tokio::test]
    async fn control_diagnostics_service_reports_success_payloads() -> Result<()> {
        let service = ControlDiagnosticsService::new(MockControlDiagnosticsSource {
            next_result: Mutex::new(Some(Ok(serde_json::json!({
                "healthy": true,
                "checks": []
            })))),
        });

        let report = service.report(false, true).await?;
        assert_eq!(report["healthy"], true);
        Ok(())
    }

    #[tokio::test]
    async fn control_diagnostics_service_shapes_event_payloads() {
        let ok_service = ControlDiagnosticsService::new(MockControlDiagnosticsSource {
            next_result: Mutex::new(Some(Ok(serde_json::json!({"healthy": true})))),
        });
        let ok_payload = ok_service.event_payload(false, false).await;
        assert_eq!(ok_payload.kind, "diagnostics");
        assert_eq!(ok_payload.report.unwrap()["healthy"], true);

        let err_service = ControlDiagnosticsService::new(MockControlDiagnosticsSource {
            next_result: Mutex::new(Some(Err(Error::Internal("boom".to_string())))),
        });
        let err_payload = err_service.event_payload(false, false).await;
        assert_eq!(err_payload.kind, "error");
        assert!(err_payload.error.unwrap().contains("boom"));
    }

    #[test]
    fn control_diagnostics_service_normalizes_interval() {
        let service = ControlDiagnosticsService::new(MockControlDiagnosticsSource {
            next_result: Mutex::new(None),
        });

        assert_eq!(service.resolved_interval_secs(None), 5);
        assert_eq!(service.resolved_interval_secs(Some(0)), 1);
        assert_eq!(service.resolved_interval_secs(Some(9)), 9);
    }
}
