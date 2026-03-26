use axum::response::Html;

const CONTROL_UI_HTML: &str = include_str!("control_ui.html");

pub fn dashboard() -> Html<&'static str> {
    Html(CONTROL_UI_HTML)
}

#[cfg(test)]
mod tests {
    use super::CONTROL_UI_HTML;

    #[test]
    fn dashboard_includes_session_continuity_panel() {
        assert!(CONTROL_UI_HTML.contains("id=\"session-continuity\""));
        assert!(CONTROL_UI_HTML.contains("Assistant Continuity"));
        assert!(CONTROL_UI_HTML.contains("function renderSessionContinuity"));
    }

    #[test]
    fn dashboard_includes_memory_policy_timeline_rendering() {
        assert!(CONTROL_UI_HTML.contains("Content / Policy"));
        assert!(CONTROL_UI_HTML.contains("function memoryPolicySummary"));
    }

    #[test]
    fn dashboard_includes_tool_execution_history_panel() {
        assert!(CONTROL_UI_HTML.contains("Recent Tool Executions"));
        assert!(CONTROL_UI_HTML.contains("function loadToolExecutions"));
    }

    #[test]
    fn dashboard_includes_coding_artifact_history_panel() {
        assert!(CONTROL_UI_HTML.contains("Recent Coding Artifacts"));
        assert!(CONTROL_UI_HTML.contains("function loadCodingArtifacts"));
    }

    #[test]
    fn dashboard_includes_email_activity_panel() {
        assert!(CONTROL_UI_HTML.contains("Recent Email Activity"));
        assert!(CONTROL_UI_HTML.contains("function loadEmailActivity"));
    }

    #[test]
    fn dashboard_includes_voice_outcomes_panel() {
        assert!(CONTROL_UI_HTML.contains("voice-outcomes-table"));
        assert!(CONTROL_UI_HTML.contains("function loadVoiceOutcomes"));
    }

    #[test]
    fn dashboard_includes_runtime_operator_ops_panel() {
        assert!(CONTROL_UI_HTML.contains("runtime-operator-ops"));
        assert!(CONTROL_UI_HTML.contains("function loadRuntimeOperatorOps"));
    }
}
