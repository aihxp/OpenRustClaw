use openrustclaw_core::error::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutonomyLessonRequest {
    pub id: String,
    pub active: bool,
    pub signal: String,
    pub recommendation: String,
    #[serde(default)]
    pub rationale: Option<String>,
    #[serde(default)]
    pub confidence: Option<f32>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub claw_id: Option<String>,
    #[serde(default)]
    pub model_profile_id: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub autonomy_level: Option<String>,
    #[serde(default)]
    pub execution_mode: Option<String>,
}

pub trait AutonomyLessonsControlSource {
    fn autonomy_description(&self) -> Result<Value>;
    fn create_lesson(&self, request: &AutonomyLessonRequest) -> Result<()>;
    fn deactivate_lesson(&self, id: &str) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AutonomySummaryReport {
    pub execution_mode: Value,
    pub default_claw: Value,
    pub orchestrator_claw: Value,
    pub allow_shared_context: Value,
    pub isolation_mode: Value,
    pub autonomy: Value,
    pub decision_lessons: Value,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AutonomyLessonsReport {
    pub decision_lessons: Value,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AutonomyLessonMutationReport {
    pub status: String,
    pub decision_lessons: Value,
}

pub struct AutonomyLessonsControlService<S> {
    source: S,
}

impl<S> AutonomyLessonsControlService<S> {
    pub fn new(source: S) -> Self {
        Self { source }
    }
}

impl<S> AutonomyLessonsControlService<S>
where
    S: AutonomyLessonsControlSource,
{
    pub fn summary(&self) -> Result<AutonomySummaryReport> {
        let description = self.source.autonomy_description()?;
        Ok(AutonomySummaryReport {
            execution_mode: description["execution_mode"].clone(),
            default_claw: description["default_claw"].clone(),
            orchestrator_claw: description["orchestrator_claw"].clone(),
            allow_shared_context: description["allow_shared_context"].clone(),
            isolation_mode: description["isolation_mode"].clone(),
            autonomy: description["autonomy"].clone(),
            decision_lessons: description["decision_lessons"].clone(),
        })
    }

    pub fn lessons(&self) -> Result<AutonomyLessonsReport> {
        let description = self.source.autonomy_description()?;
        Ok(AutonomyLessonsReport {
            decision_lessons: description["decision_lessons"].clone(),
        })
    }

    pub fn create_lesson(
        &self,
        request: &AutonomyLessonRequest,
    ) -> Result<AutonomyLessonMutationReport> {
        self.source.create_lesson(request)?;
        self.mutation_report()
    }

    pub fn deactivate_lesson(&self, id: &str) -> Result<AutonomyLessonMutationReport> {
        self.source.deactivate_lesson(id)?;
        self.mutation_report()
    }

    fn mutation_report(&self) -> Result<AutonomyLessonMutationReport> {
        let description = self.source.autonomy_description()?;
        Ok(AutonomyLessonMutationReport {
            status: "ok".to_string(),
            decision_lessons: description["decision_lessons"].clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openrustclaw_core::error::Error;
    use std::cell::RefCell;

    struct MockAutonomyLessonsControlSource {
        description: RefCell<Value>,
        last_created: RefCell<Option<AutonomyLessonRequest>>,
        last_deactivated: RefCell<Option<String>>,
    }

    impl MockAutonomyLessonsControlSource {
        fn new() -> Self {
            Self {
                description: RefCell::new(serde_json::json!({
                    "execution_mode": "supervised",
                    "default_claw": "default",
                    "orchestrator_claw": "orchestrator",
                    "allow_shared_context": true,
                    "isolation_mode": "workspace",
                    "autonomy": {"enabled": true},
                    "decision_lessons": [{"id": "prefer-local"}],
                })),
                last_created: RefCell::new(None),
                last_deactivated: RefCell::new(None),
            }
        }
    }

    impl AutonomyLessonsControlSource for MockAutonomyLessonsControlSource {
        fn autonomy_description(&self) -> Result<Value> {
            Ok(self.description.borrow().clone())
        }

        fn create_lesson(&self, request: &AutonomyLessonRequest) -> Result<()> {
            self.last_created.replace(Some(request.clone()));
            self.description.borrow_mut()["decision_lessons"] =
                serde_json::json!([{"id": "prefer-local"}, {"id": request.id}]);
            Ok(())
        }

        fn deactivate_lesson(&self, id: &str) -> Result<()> {
            self.last_deactivated.replace(Some(id.to_string()));
            self.description.borrow_mut()["decision_lessons"] =
                serde_json::json!([{"id": id, "active": false}]);
            Ok(())
        }
    }

    #[test]
    fn autonomy_summary_and_lessons_report_registry_fields() -> Result<()> {
        let service = AutonomyLessonsControlService::new(MockAutonomyLessonsControlSource::new());

        let summary = service.summary()?;
        assert_eq!(summary.execution_mode, serde_json::json!("supervised"));
        assert_eq!(
            summary.decision_lessons,
            serde_json::json!([{"id": "prefer-local"}])
        );

        let lessons = service.lessons()?;
        assert_eq!(
            lessons.decision_lessons,
            serde_json::json!([{"id": "prefer-local"}])
        );
        Ok(())
    }

    #[test]
    fn autonomy_mutations_return_stable_reports() -> Result<()> {
        let service = AutonomyLessonsControlService::new(MockAutonomyLessonsControlSource::new());

        let created = service.create_lesson(&AutonomyLessonRequest {
            id: "prefer-verified".to_string(),
            active: true,
            signal: "signal".to_string(),
            recommendation: "recommendation".to_string(),
            rationale: None,
            confidence: Some(0.9),
            source: None,
            task_id: None,
            category: None,
            claw_id: None,
            model_profile_id: None,
            provider: None,
            autonomy_level: None,
            execution_mode: None,
        })?;
        assert_eq!(created.status, "ok");
        assert_eq!(
            created.decision_lessons,
            serde_json::json!([{"id": "prefer-local"}, {"id": "prefer-verified"}])
        );

        let deactivated = service.deactivate_lesson("prefer-local")?;
        assert_eq!(deactivated.status, "ok");
        assert_eq!(
            deactivated.decision_lessons,
            serde_json::json!([{"id": "prefer-local", "active": false}])
        );
        Ok(())
    }

    #[test]
    fn autonomy_service_propagates_source_errors() {
        struct ErrorSource;

        impl AutonomyLessonsControlSource for ErrorSource {
            fn autonomy_description(&self) -> Result<Value> {
                Err(Error::Internal("boom".to_string()))
            }

            fn create_lesson(&self, _request: &AutonomyLessonRequest) -> Result<()> {
                Err(Error::Internal("boom".to_string()))
            }

            fn deactivate_lesson(&self, _id: &str) -> Result<()> {
                Err(Error::Internal("boom".to_string()))
            }
        }

        let service = AutonomyLessonsControlService::new(ErrorSource);
        assert!(service.summary().is_err());
        assert!(service.lessons().is_err());
        assert!(service.deactivate_lesson("id").is_err());
    }
}
