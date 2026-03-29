use chrono::{DateTime, Utc};
use openrustclaw_core::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskTriggerSpec {
    Interval { every_seconds: u64 },
    Absolute { at: String },
    Event { event_name: String },
    Dependency { depends_on: Vec<String> },
}

#[derive(Debug, Clone, Default)]
pub struct SchedulePlanningService;

impl SchedulePlanningService {
    pub fn new() -> Self {
        Self
    }

    pub fn trigger_from_inline(
        &self,
        every_seconds: Option<u64>,
        at: Option<&str>,
    ) -> Result<TaskTriggerSpec> {
        if every_seconds.is_some() && at.is_some() {
            return Err(Error::Config(
                "Use either --every-seconds or --at, not both".to_string(),
            ));
        }

        if let Some(run_at) = at {
            DateTime::parse_from_rfc3339(run_at).map_err(|error| {
                Error::Config(format!(
                    "Invalid RFC3339 timestamp for --at: {run_at}: {error}"
                ))
            })?;
            Ok(TaskTriggerSpec::Absolute {
                at: run_at.to_string(),
            })
        } else {
            Ok(TaskTriggerSpec::Interval {
                every_seconds: every_seconds.unwrap_or(3600),
            })
        }
    }

    pub fn trigger_config_and_next_run(
        &self,
        trigger: &TaskTriggerSpec,
        now: DateTime<Utc>,
    ) -> Result<(String, Value, Option<DateTime<Utc>>)> {
        match trigger {
            TaskTriggerSpec::Interval { every_seconds } => Ok((
                "interval".to_string(),
                serde_json::json!({
                    "type": "interval",
                    "interval_secs": every_seconds,
                }),
                Some(now + chrono::Duration::seconds(*every_seconds as i64)),
            )),
            TaskTriggerSpec::Absolute { at } => {
                let run_at = DateTime::parse_from_rfc3339(at)
                    .map_err(|error| {
                        Error::Config(format!(
                            "Invalid RFC3339 timestamp for absolute trigger: {at}: {error}"
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
            }
            TaskTriggerSpec::Event { event_name } => Ok((
                "event".to_string(),
                serde_json::json!({
                    "type": "event",
                    "event_name": event_name,
                }),
                None,
            )),
            TaskTriggerSpec::Dependency { depends_on } => Ok((
                "dependency".to_string(),
                serde_json::json!({
                    "type": "dependency",
                    "depends_on": depends_on,
                }),
                None,
            )),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build_metadata(
        &self,
        payload: Value,
        priority: i64,
        owner: Option<&str>,
        tags: &[String],
        manifest_path: Option<&Path>,
        source_kind: &str,
        metadata: Value,
        delivery_policy: Option<&Value>,
        hook_policy: Option<&Value>,
        routing: Option<&Value>,
    ) -> Value {
        let mut object = metadata.as_object().cloned().unwrap_or_else(Map::new);
        object.insert("input".to_string(), payload);
        object.insert(
            "task".to_string(),
            serde_json::json!({
                "priority": priority,
                "owner": owner,
                "tags": tags,
                "source_kind": source_kind,
                "manifest_path": manifest_path.map(|value| value.display().to_string()),
            }),
        );
        if let Some(policy) = delivery_policy.cloned() {
            object.insert("delivery_policy".to_string(), policy);
        }
        if let Some(policy) = hook_policy.cloned() {
            object.insert("hook_policy".to_string(), policy);
        }
        if let Some(route) = routing.cloned() {
            object.insert("routing".to_string(), route);
        }
        Value::Object(object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_from_inline_rejects_conflicting_flags() {
        assert!(
            SchedulePlanningService::new()
                .trigger_from_inline(Some(60), Some("2026-01-01T00:00:00Z"))
                .is_err()
        );
    }

    #[test]
    fn trigger_config_and_next_run_uses_interval_window() {
        let now = DateTime::parse_from_rfc3339("2026-03-28T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let (_, _, next_run) = SchedulePlanningService::new()
            .trigger_config_and_next_run(&TaskTriggerSpec::Interval { every_seconds: 60 }, now)
            .unwrap();
        assert_eq!(next_run.unwrap().to_rfc3339(), "2026-03-28T00:01:00+00:00");
    }
}
