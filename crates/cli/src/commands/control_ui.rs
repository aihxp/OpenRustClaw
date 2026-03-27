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
    fn dashboard_includes_browser_workflow_history_panel() {
        assert!(CONTROL_UI_HTML.contains("Recent Browser Workflows"));
        assert!(CONTROL_UI_HTML.contains("function loadBrowserWorkflowHistory"));
    }

    #[test]
    fn dashboard_includes_voice_outcomes_panel() {
        assert!(CONTROL_UI_HTML.contains("voice-outcomes-table"));
        assert!(CONTROL_UI_HTML.contains("function loadVoiceOutcomes"));
    }

    #[test]
    fn dashboard_includes_voice_operator_report_rendering() {
        assert!(CONTROL_UI_HTML.contains("voice-operator-report"));
        assert!(CONTROL_UI_HTML.contains("voice-operator-signals"));
        assert!(CONTROL_UI_HTML.contains("voice-operator-activity"));
        assert!(CONTROL_UI_HTML.contains("function loadVoiceOperatorReport"));
        assert!(CONTROL_UI_HTML.contains("function renderVoiceOperatorReport"));
        assert!(CONTROL_UI_HTML.contains("function renderVoiceStatus"));
        assert!(CONTROL_UI_HTML.contains("function renderTalkStatus"));
    }

    #[test]
    fn dashboard_includes_runtime_operator_ops_panel() {
        assert!(CONTROL_UI_HTML.contains("runtime-operator-ops"));
        assert!(CONTROL_UI_HTML.contains("function loadRuntimeOperatorOps"));
    }

    #[test]
    fn dashboard_includes_security_posture_panel() {
        assert!(CONTROL_UI_HTML.contains("security-posture"));
        assert!(CONTROL_UI_HTML.contains("function loadSecurityPosture"));
    }

    #[test]
    fn dashboard_includes_orchestration_supervision_tables() {
        assert!(CONTROL_UI_HTML.contains("orchestration-workers"));
        assert!(CONTROL_UI_HTML.contains("orchestration-delegations"));
        assert!(CONTROL_UI_HTML.contains("orchestration-active-decisions"));
        assert!(CONTROL_UI_HTML.contains("function renderReceiptSupervision"));
        assert!(CONTROL_UI_HTML.contains("function renderActiveRunSupervision"));
        assert!(CONTROL_UI_HTML.contains("controlActiveOrchestrationRun"));
    }

    #[test]
    fn dashboard_includes_mobile_operator_report_rendering() {
        assert!(CONTROL_UI_HTML.contains("mobile-node-signals"));
        assert!(CONTROL_UI_HTML.contains("mobile-node-activity-table"));
        assert!(CONTROL_UI_HTML.contains("function renderMobileNodeReport"));
    }

    #[test]
    fn dashboard_includes_voice_and_talk_detail_renderers() {
        assert!(CONTROL_UI_HTML.contains("id=\"voice-session-transcript\""));
        assert!(CONTROL_UI_HTML.contains("function renderVoiceSessionDetail"));
        assert!(CONTROL_UI_HTML.contains("function renderVoiceSessionMetrics"));
        assert!(CONTROL_UI_HTML.contains("function renderVoiceSessionTranscript"));
        assert!(CONTROL_UI_HTML.contains("id=\"talk-session-events-detail\""));
        assert!(CONTROL_UI_HTML.contains("function renderTalkSessionDetail"));
        assert!(CONTROL_UI_HTML.contains("function renderTalkSessionMetrics"));
        assert!(CONTROL_UI_HTML.contains("function renderTalkSessionEvents"));
    }

    #[test]
    fn dashboard_includes_skill_and_mobile_detail_renderers() {
        assert!(CONTROL_UI_HTML.contains("id=\"voice-call-events\""));
        assert!(CONTROL_UI_HTML.contains("function renderSkillDetail"));
        assert!(CONTROL_UI_HTML.contains("function renderVoiceCallMetrics"));
        assert!(CONTROL_UI_HTML.contains("id=\"mobile-command-events-detail\""));
        assert!(CONTROL_UI_HTML.contains("function renderMobileAppSessionDetail"));
        assert!(CONTROL_UI_HTML.contains("function renderMobileSyncConflictDetail"));
        assert!(CONTROL_UI_HTML.contains("function renderMobileCommandDetail"));
        assert!(CONTROL_UI_HTML.contains("function renderMobileCommandEvents"));
    }

    #[test]
    fn dashboard_includes_enterprise_foundations_panel() {
        assert!(CONTROL_UI_HTML.contains("enterprise-foundations"));
        assert!(CONTROL_UI_HTML.contains("function loadEnterpriseFoundations"));
    }

    #[test]
    fn dashboard_includes_enterprise_audit_review_panel() {
        assert!(CONTROL_UI_HTML.contains("enterprise-audit-review"));
        assert!(CONTROL_UI_HTML.contains("enterprise-audit-exports"));
        assert!(CONTROL_UI_HTML.contains("function loadEnterpriseAuditReview"));
        assert!(CONTROL_UI_HTML.contains("function renderEnterpriseAuditReview"));
    }

    #[test]
    fn dashboard_includes_enterprise_autonomy_panel() {
        assert!(CONTROL_UI_HTML.contains("enterprise-autonomy"));
        assert!(CONTROL_UI_HTML.contains("enterprise-autonomy-events"));
        assert!(CONTROL_UI_HTML.contains("enterprise-autonomy-runs"));
        assert!(CONTROL_UI_HTML.contains("function loadEnterpriseAutonomy"));
        assert!(CONTROL_UI_HTML.contains("function renderEnterpriseAutonomy"));
    }

    #[test]
    fn dashboard_includes_enterprise_access_panel() {
        assert!(CONTROL_UI_HTML.contains("enterprise-access"));
        assert!(CONTROL_UI_HTML.contains("enterprise-operators-table"));
        assert!(CONTROL_UI_HTML.contains("enterprise-governance-rules"));
        assert!(CONTROL_UI_HTML.contains("enterprise-protected-routes"));
        assert!(CONTROL_UI_HTML.contains("function loadEnterpriseAccess"));
        assert!(CONTROL_UI_HTML.contains("function renderEnterpriseAccess"));
    }

    #[test]
    fn dashboard_includes_enterprise_admin_panel() {
        assert!(CONTROL_UI_HTML.contains("enterprise-admin"));
        assert!(CONTROL_UI_HTML.contains("enterprise-admin-result"));
        assert!(CONTROL_UI_HTML.contains("enterprise-operator-id"));
        assert!(CONTROL_UI_HTML.contains("enterprise-approver-id"));
        assert!(CONTROL_UI_HTML.contains("enterprise-governance-update"));
        assert!(CONTROL_UI_HTML.contains("enterprise-audit-retention-days"));
        assert!(CONTROL_UI_HTML.contains("enterprise-audit-export-history-limit"));
        assert!(CONTROL_UI_HTML.contains("enterprise-autonomy-max-delegations"));
        assert!(CONTROL_UI_HTML.contains("enterprise-autonomy-enable"));
        assert!(CONTROL_UI_HTML.contains("enterprise-autonomy-disable"));
        assert!(CONTROL_UI_HTML.contains("enterprise-autonomy-kill-switch"));
        assert!(CONTROL_UI_HTML.contains("function loadEnterpriseAdmin"));
        assert!(CONTROL_UI_HTML.contains("function renderEnterpriseAdmin"));
        assert!(CONTROL_UI_HTML.contains("function enableEnterpriseAutonomy"));
        assert!(CONTROL_UI_HTML.contains("function disableEnterpriseAutonomy"));
        assert!(CONTROL_UI_HTML.contains("function killSwitchEnterpriseAutonomy"));
        assert!(CONTROL_UI_HTML.contains("function updateEnterprisePolicy"));
        assert!(CONTROL_UI_HTML.contains("function upsertEnterpriseGovernanceRule"));
        assert!(CONTROL_UI_HTML.contains("function exportEnterpriseAuditBundle"));
        assert!(CONTROL_UI_HTML.contains("function bootstrapEnterpriseAccess"));
    }
}
