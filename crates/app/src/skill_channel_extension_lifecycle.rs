use chrono::{DateTime, Duration, Utc};
use openrustclaw_core::error::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillBackgroundWorkflowScheduleRequest {
    pub job_id: String,
    pub idempotency_key: String,
    pub skill_name: String,
    pub service_name: String,
    #[serde(default)]
    pub component: Option<String>,
    pub compiled_root: String,
    pub workspace_database_url: String,
    pub input: serde_json::Value,
    #[serde(default)]
    pub every_seconds: Option<u64>,
    #[serde(default)]
    pub at: Option<String>,
    pub priority: i64,
    pub now: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillBackgroundWorkflowJobDraft {
    pub id: String,
    pub name: String,
    pub description: String,
    pub workflow_id: String,
    pub trigger_type: String,
    pub trigger_config: serde_json::Value,
    pub idempotency_key: String,
    pub priority: i64,
    pub source_kind: String,
    pub owner: String,
    pub tags: Vec<String>,
    #[serde(default)]
    pub next_run_at: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillBackgroundWorkflowScheduleReport {
    pub status: String,
    pub job_id: String,
    pub workflow_id: String,
    pub skill_name: String,
    pub service: String,
    pub component: String,
    pub compiled_root: String,
    pub trigger_type: String,
    #[serde(default)]
    pub next_run_at: Option<String>,
    pub scheduled_job: SkillBackgroundWorkflowJobDraft,
    pub event_name: String,
    pub event_payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillChannelExtensionBindingRequest {
    pub binding_id: String,
    pub platform: String,
    pub skill_name: String,
    pub service_name: String,
    #[serde(default)]
    pub component: Option<String>,
    #[serde(default)]
    pub trigger: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillChannelExtensionBindingReport {
    pub status: String,
    pub binding_id: String,
    pub skill_name: String,
    pub trigger: String,
    #[serde(default)]
    pub service: Option<String>,
    #[serde(default)]
    pub component: Option<String>,
    pub binding_metadata: serde_json::Value,
    pub event_name: String,
    pub event_payload: serde_json::Value,
}

#[derive(Debug, Clone, Default)]
pub struct SkillChannelExtensionLifecycleService;

impl SkillChannelExtensionLifecycleService {
    pub fn new() -> Self {
        Self
    }

    pub fn schedule_background_workflow(
        &self,
        request: SkillBackgroundWorkflowScheduleRequest,
    ) -> Result<SkillBackgroundWorkflowScheduleReport> {
        let component = request.component.clone().ok_or_else(|| {
            Error::Internal(format!(
                "Background service '{}' for '{}' does not resolve to an executable component",
                request.service_name, request.skill_name
            ))
        })?;
        let (trigger_type, trigger_config, next_run_at) =
            schedule_trigger(request.every_seconds, request.at.as_deref(), &request.now)?;
        let workflow_id = "skill_background".to_string();
        let next_run_at_rfc3339 = next_run_at.map(|value| value.to_rfc3339());
        let scheduled_job = SkillBackgroundWorkflowJobDraft {
            id: request.job_id.clone(),
            name: format!(
                "skill-bg-{}-{}",
                request.skill_name.replace(' ', "-").to_lowercase(),
                request.service_name.replace(' ', "-").to_lowercase()
            ),
            description: format!(
                "Compiled skill background workflow for {}:{}",
                request.skill_name, request.service_name
            ),
            workflow_id: workflow_id.clone(),
            trigger_type: trigger_type.clone(),
            trigger_config,
            idempotency_key: request.idempotency_key,
            priority: request.priority,
            source_kind: "skills".to_string(),
            owner: "skills".to_string(),
            tags: vec![
                "skill".to_string(),
                "background-service".to_string(),
                request.service_name.clone(),
            ],
            next_run_at: next_run_at_rfc3339.clone(),
            metadata: serde_json::json!({
                "input": {
                    "compiled_root": &request.compiled_root,
                    "skill_name": &request.skill_name,
                    "service": &request.service_name,
                    "component": &component,
                    "skill_input": &request.input,
                },
                "workflow_metadata": {
                    "skill_name": &request.skill_name,
                    "background_service": &request.service_name,
                    "component": &component,
                    "source_kind": "plugin_background_workflow",
                    "workspace_database": &request.workspace_database_url,
                },
                "task": {
                    "priority": request.priority,
                    "source_kind": "plugin_background_workflow",
                    "owner": "skills",
                    "tags": ["skill", "background-service", &request.service_name],
                }
            }),
        };

        Ok(SkillBackgroundWorkflowScheduleReport {
            status: "ok".to_string(),
            job_id: request.job_id.clone(),
            workflow_id,
            skill_name: request.skill_name.clone(),
            service: request.service_name.clone(),
            component: component.clone(),
            compiled_root: request.compiled_root,
            trigger_type: trigger_type.clone(),
            next_run_at: next_run_at_rfc3339,
            scheduled_job,
            event_name: "plugin.background_workflow_scheduled".to_string(),
            event_payload: serde_json::json!({
                "job_id": request.job_id,
                "skill_name": request.skill_name,
                "service": request.service_name,
                "component": component,
                "trigger_type": trigger_type,
            }),
        })
    }

    pub fn bind_channel_extension(
        &self,
        request: SkillChannelExtensionBindingRequest,
    ) -> Result<SkillChannelExtensionBindingReport> {
        let component = request.component.clone().ok_or_else(|| {
            Error::Internal(format!(
                "Channel extension service '{}' for '{}' has no executable component mapping",
                request.service_name, request.skill_name
            ))
        })?;
        let trigger = request.trigger.as_deref().unwrap_or("message");
        if !matches!(trigger, "message" | "mentioned") {
            return Err(Error::Internal(format!(
                "Unsupported trigger '{}'; use `message` or `mentioned`",
                trigger
            )));
        }

        Ok(SkillChannelExtensionBindingReport {
            status: "ok".to_string(),
            binding_id: request.binding_id.clone(),
            skill_name: request.skill_name.clone(),
            trigger: trigger.to_string(),
            service: Some(request.service_name.clone()),
            component: Some(component.clone()),
            binding_metadata: serde_json::json!({
                "skill_name": request.skill_name,
                "service": request.service_name,
                "component": component,
                "trigger": trigger,
                "mode": "background_schedule",
            }),
            event_name: "plugin.channel_extension_bound".to_string(),
            event_payload: serde_json::json!({
                "binding_id": request.binding_id,
                "platform": request.platform,
                "skill_name": request.skill_name,
                "service": request.service_name,
                "component": component,
                "trigger": trigger,
            }),
        })
    }
}

fn schedule_trigger(
    every_seconds: Option<u64>,
    at: Option<&str>,
    now: &str,
) -> Result<(String, serde_json::Value, Option<DateTime<Utc>>)> {
    if every_seconds.is_some() && at.is_some() {
        return Err(Error::Internal(
            "Use either --every-seconds or --at, not both".to_string(),
        ));
    }

    if let Some(run_at) = at {
        let run_at = DateTime::parse_from_rfc3339(run_at)
            .map_err(|error| {
                Error::Internal(format!(
                    "Invalid RFC3339 timestamp for --at: {run_at} ({error})"
                ))
            })?
            .with_timezone(&Utc);
        Ok((
            "absolute".to_string(),
            serde_json::json!({
                "type": "absolute",
                "run_at": run_at.to_rfc3339(),
            }),
            Some(run_at),
        ))
    } else {
        let every_seconds = every_seconds.unwrap_or(3600);
        let now = DateTime::parse_from_rfc3339(now)
            .map_err(|error| {
                Error::Internal(format!(
                    "Invalid generated timestamp for background workflow scheduling: {error}"
                ))
            })?
            .with_timezone(&Utc);
        Ok((
            "interval".to_string(),
            serde_json::json!({
                "type": "interval",
                "interval_secs": every_seconds,
            }),
            Some(now + Duration::seconds(every_seconds as i64)),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_background_workflow_shapes_job_and_event() -> Result<()> {
        let report = SkillChannelExtensionLifecycleService::new().schedule_background_workflow(
            SkillBackgroundWorkflowScheduleRequest {
                job_id: "job-1".to_string(),
                idempotency_key: "idem-1".to_string(),
                skill_name: "Channel Demo".to_string(),
                service_name: "sync-loop".to_string(),
                component: Some("sync-loop.wat".to_string()),
                compiled_root: ".claw/skills/compiled".to_string(),
                workspace_database_url: "sqlite:///tmp/test.db".to_string(),
                input: serde_json::json!({"ok": true}),
                every_seconds: Some(600),
                at: None,
                priority: 7,
                now: "2026-03-28T18:30:00Z".to_string(),
            },
        )?;

        assert_eq!(report.status, "ok");
        assert_eq!(report.workflow_id, "skill_background");
        assert_eq!(report.trigger_type, "interval");
        assert_eq!(report.service, "sync-loop");
        assert_eq!(report.component, "sync-loop.wat");
        assert_eq!(
            report.next_run_at.as_deref(),
            Some("2026-03-28T18:40:00+00:00")
        );
        assert_eq!(report.scheduled_job.source_kind, "skills");
        assert_eq!(
            report.event_payload["service"],
            serde_json::json!("sync-loop")
        );
        Ok(())
    }

    #[test]
    fn bind_channel_extension_shapes_metadata_and_validates_trigger() -> Result<()> {
        let report = SkillChannelExtensionLifecycleService::new().bind_channel_extension(
            SkillChannelExtensionBindingRequest {
                binding_id: "support-inbox".to_string(),
                platform: "slack".to_string(),
                skill_name: "Channel Demo".to_string(),
                service_name: "sync-loop".to_string(),
                component: Some("sync-loop.wat".to_string()),
                trigger: Some("mentioned".to_string()),
            },
        )?;

        assert_eq!(report.status, "ok");
        assert_eq!(report.trigger, "mentioned");
        assert_eq!(
            report.binding_metadata["mode"],
            serde_json::json!("background_schedule")
        );
        assert_eq!(
            report.event_payload["binding_id"],
            serde_json::json!("support-inbox")
        );
        Ok(())
    }
}
