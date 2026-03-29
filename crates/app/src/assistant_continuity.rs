use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AssistantContinuitySummary {
    pub assistant_managed: bool,
    pub assistant_identity: Option<String>,
    pub assistant_surface: Option<String>,
    pub assistant_session_model: Option<String>,
    pub route_key: Option<String>,
    pub workspace_root: Option<String>,
    pub route_bound: bool,
    pub history_messages: usize,
    pub likely_resumed: bool,
    pub status_label: String,
    pub detail: String,
}

pub struct AssistantContinuityService;

impl AssistantContinuityService {
    pub fn summarize(
        metadata: &serde_json::Value,
        route_key: Option<&str>,
        history_messages: usize,
    ) -> AssistantContinuitySummary {
        let assistant_identity = Self::metadata_string(metadata, "assistant_identity");
        let assistant_surface = Self::metadata_string(metadata, "assistant_surface");
        let assistant_session_model = Self::metadata_string(metadata, "assistant_session_model");
        let route_key = route_key
            .map(ToString::to_string)
            .or_else(|| Self::metadata_string(metadata, "route_key"));
        let workspace_root = Self::metadata_string(metadata, "workspace_root");
        let route_bound = route_key.is_some();
        let assistant_managed = assistant_identity.is_some()
            || assistant_surface.is_some()
            || assistant_session_model.is_some();
        let likely_resumed = assistant_managed && route_bound && history_messages > 0;
        let status_label = if !assistant_managed {
            "generic"
        } else if likely_resumed {
            "resumed"
        } else if route_bound {
            "route-bound"
        } else if assistant_session_model.as_deref() == Some("persisted") {
            "persisted"
        } else {
            "assistant"
        }
        .to_string();
        let detail = Self::detail(
            assistant_managed,
            assistant_surface.as_deref(),
            route_bound,
            history_messages,
        );

        AssistantContinuitySummary {
            assistant_managed,
            assistant_identity,
            assistant_surface,
            assistant_session_model,
            route_key,
            workspace_root,
            route_bound,
            history_messages,
            likely_resumed,
            status_label,
            detail,
        }
    }

    fn metadata_string(metadata: &serde_json::Value, key: &str) -> Option<String> {
        metadata
            .get(key)
            .and_then(|value| value.as_str())
            .map(ToString::to_string)
    }

    fn detail(
        assistant_managed: bool,
        assistant_surface: Option<&str>,
        route_bound: bool,
        history_messages: usize,
    ) -> String {
        let history_label = if history_messages == 1 {
            "1 persisted message".to_string()
        } else {
            format!("{history_messages} persisted messages")
        };

        if !assistant_managed {
            return format!("Generic session with {history_label}.");
        }

        let surface = assistant_surface.unwrap_or("assistant");
        if route_bound && history_messages > 0 {
            return format!(
                "Primary {surface} assistant session matched by route key with {history_label} restored."
            );
        }
        if route_bound {
            return format!(
                "Primary {surface} assistant session is route-bound and ready to accumulate persisted history."
            );
        }
        format!("Primary {surface} assistant session with {history_label}.")
    }
}

#[cfg(test)]
mod tests {
    use super::{AssistantContinuityService, AssistantContinuitySummary};

    #[test]
    fn summarize_reports_resumed_route_bound_sessions() {
        let metadata = serde_json::json!({
            "assistant_identity": "primary",
            "assistant_surface": "voice",
            "assistant_session_model": "persisted",
        });

        let summary = AssistantContinuityService::summarize(&metadata, Some("voice.default"), 3);
        assert_eq!(
            summary,
            AssistantContinuitySummary {
                assistant_managed: true,
                assistant_identity: Some("primary".to_string()),
                assistant_surface: Some("voice".to_string()),
                assistant_session_model: Some("persisted".to_string()),
                route_key: Some("voice.default".to_string()),
                workspace_root: None,
                route_bound: true,
                history_messages: 3,
                likely_resumed: true,
                status_label: "resumed".to_string(),
                detail: "Primary voice assistant session matched by route key with 3 persisted messages restored.".to_string(),
            }
        );
    }
}
