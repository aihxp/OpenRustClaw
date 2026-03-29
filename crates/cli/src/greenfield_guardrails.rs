#[cfg(test)]
mod tests {
    const INSPECT_SOURCE: &str = include_str!("commands/inspect.rs");
    const SKILLS_SOURCE: &str = include_str!("commands/skills.rs");
    const START_SOURCE: &str = include_str!("commands/start.rs");

    fn assert_contains(source: &str, needle: &str, label: &str) {
        assert!(
            source.contains(needle),
            "expected {label} to contain `{needle}`"
        );
    }

    fn assert_not_contains(source: &str, needle: &str, label: &str) {
        assert!(
            !source.contains(needle),
            "expected {label} to stop owning `{needle}`"
        );
    }

    #[test]
    fn inspect_uses_greenfield_services_for_continuity_and_audit() {
        assert_contains(INSPECT_SOURCE, "AssistantContinuityService", "inspect.rs");
        assert_contains(INSPECT_SOURCE, "ToolExecutionAuditService", "inspect.rs");
        assert_not_contains(INSPECT_SOURCE, "fn metadata_string(", "inspect.rs");
        assert_not_contains(INSPECT_SOURCE, "fn continuity_detail(", "inspect.rs");
    }

    #[test]
    fn skills_uses_greenfield_services_for_voice_call_reporting() {
        assert_contains(SKILLS_SOURCE, "VoiceCallReportingService", "skills.rs");
        assert_contains(SKILLS_SOURCE, "CompiledSkillMcpService", "skills.rs");
        assert_not_contains(SKILLS_SOURCE, "fn refresh_voice_call_health(", "skills.rs");
        assert_not_contains(SKILLS_SOURCE, "fn voice_call_health_summary(", "skills.rs");
        assert_not_contains(SKILLS_SOURCE, "fn voice_call_metrics_summary(", "skills.rs");
        assert_not_contains(
            SKILLS_SOURCE,
            "fn voice_call_artifacts_for_record(",
            "skills.rs",
        );
        assert_not_contains(
            SKILLS_SOURCE,
            "fn voice_call_events_for_record(",
            "skills.rs",
        );
    }

    #[test]
    fn start_uses_named_adapter_boundary_for_compiled_skill_mcp() {
        assert_contains(START_SOURCE, "CompiledSkillWorkspaceCatalog", "start.rs");
        assert_contains(START_SOURCE, "CompiledSkillMcpService", "start.rs");
        assert_not_contains(START_SOURCE, "fn sanitize_compiled_skill_name(", "start.rs");
        assert_not_contains(START_SOURCE, "fn compiled_skill_tool_prefix(", "start.rs");
    }
}
