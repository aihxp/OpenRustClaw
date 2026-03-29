use openrustclaw_core::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentProfileSpec {
    pub id: String,
    pub extends: Vec<String>,
    pub model_profile_id: Option<String>,
    pub memory_scope: String,
    pub output_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelProfileSpec {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub role_tags: Vec<String>,
    pub artifact_preferences: Vec<String>,
    pub fallback_order: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClawSpec {
    pub id: String,
    pub name: Option<String>,
    pub role: String,
    pub agent_profile_id: String,
    pub model_profile_id: String,
    pub memory_scope: String,
    pub task_categories: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutonomyPolicy {
    pub autonomy_level: String,
    pub yolo_mode: bool,
    pub steering_enabled: bool,
    pub decision_learning_enabled: bool,
    pub critic_enabled: bool,
    pub max_delegations: usize,
    pub max_iterations: usize,
    pub max_runtime_secs: u64,
    pub max_lesson_hints: usize,
    pub approval_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeModeSpec {
    pub mode: String,
    pub default_claw_id: Option<String>,
    pub orchestrator_claw_id: Option<String>,
    pub task_assignments: Vec<(String, String)>,
    pub category_assignments: Vec<(String, String)>,
    pub allow_shared_context: bool,
    pub isolation_mode: String,
    pub autonomy: AutonomyPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DecisionLessonScope {
    pub category: Option<String>,
    pub claw_id: Option<String>,
    pub model_profile_id: Option<String>,
    pub provider: Option<String>,
    pub autonomy_level: Option<String>,
    pub execution_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionLessonSpec {
    pub id: String,
    pub active: bool,
    pub signal: String,
    pub recommendation: String,
    pub confidence: f32,
    pub source: String,
    pub scope: DecisionLessonScope,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ControlRegistrySnapshot {
    pub agent_profiles: Vec<AgentProfileSpec>,
    pub model_profiles: Vec<ModelProfileSpec>,
    pub claws: Vec<ClawSpec>,
    pub lessons: Vec<DecisionLessonSpec>,
    pub runtime: Option<RuntimeModeSpec>,
}

#[derive(Debug, Clone, Default)]
pub struct ControlRegistryService;

impl ControlRegistryService {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_registry(&self, registry: &ControlRegistrySnapshot) -> Result<()> {
        if let Some(runtime) = &registry.runtime {
            self.validate_runtime(registry, runtime)?;
        }

        for profile in &registry.agent_profiles {
            for parent in &profile.extends {
                if !registry
                    .agent_profiles
                    .iter()
                    .any(|candidate| candidate.id == *parent)
                {
                    return Err(Error::Config(format!(
                        "agent profile '{}' extends missing profile '{}'",
                        profile.id, parent
                    )));
                }
            }
            if let Some(model_profile_id) = &profile.model_profile_id
                && !registry
                    .model_profiles
                    .iter()
                    .any(|candidate| candidate.id == *model_profile_id)
            {
                return Err(Error::Config(format!(
                    "agent profile '{}' references missing model profile '{}'",
                    profile.id, model_profile_id
                )));
            }
        }

        for profile in &registry.model_profiles {
            for fallback in &profile.fallback_order {
                if !registry
                    .model_profiles
                    .iter()
                    .any(|candidate| candidate.id == *fallback)
                {
                    return Err(Error::Config(format!(
                        "model profile '{}' fallback '{}' does not exist",
                        profile.id, fallback
                    )));
                }
            }
        }

        for claw in &registry.claws {
            if !registry
                .agent_profiles
                .iter()
                .any(|candidate| candidate.id == claw.agent_profile_id)
            {
                return Err(Error::Config(format!(
                    "claw '{}' references missing agent profile '{}'",
                    claw.id, claw.agent_profile_id
                )));
            }
            if !registry
                .model_profiles
                .iter()
                .any(|candidate| candidate.id == claw.model_profile_id)
            {
                return Err(Error::Config(format!(
                    "claw '{}' references missing model profile '{}'",
                    claw.id, claw.model_profile_id
                )));
            }
        }

        for lesson in &registry.lessons {
            if lesson.signal.trim().is_empty() {
                return Err(Error::Config(format!(
                    "lesson '{}' is missing signal",
                    lesson.id
                )));
            }
            if lesson.recommendation.trim().is_empty() {
                return Err(Error::Config(format!(
                    "lesson '{}' is missing recommendation",
                    lesson.id
                )));
            }
            if let Some(claw_id) = &lesson.scope.claw_id
                && !registry
                    .claws
                    .iter()
                    .any(|candidate| candidate.id == *claw_id)
            {
                return Err(Error::Config(format!(
                    "lesson '{}' references missing claw '{}'",
                    lesson.id, claw_id
                )));
            }
            if let Some(model_profile_id) = &lesson.scope.model_profile_id
                && !registry
                    .model_profiles
                    .iter()
                    .any(|candidate| candidate.id == *model_profile_id)
            {
                return Err(Error::Config(format!(
                    "lesson '{}' references missing model profile '{}'",
                    lesson.id, model_profile_id
                )));
            }
            if let Some(autonomy_level) = &lesson.scope.autonomy_level {
                self.validate_autonomy_level(autonomy_level, &format!("lesson '{}'", lesson.id))?;
            }
            if let Some(execution_mode) = &lesson.scope.execution_mode {
                self.validate_execution_mode(execution_mode, &format!("lesson '{}'", lesson.id))?;
            }
        }

        Ok(())
    }

    pub fn describe_registry(&self, registry: &ControlRegistrySnapshot) -> Result<Value> {
        self.validate_registry(registry)?;

        let runtime = registry.runtime.clone().unwrap_or(RuntimeModeSpec {
            mode: "solo_claw".to_string(),
            default_claw_id: None,
            orchestrator_claw_id: None,
            task_assignments: Vec::new(),
            category_assignments: Vec::new(),
            allow_shared_context: false,
            isolation_mode: "strict".to_string(),
            autonomy: AutonomyPolicy {
                autonomy_level: "managed".to_string(),
                yolo_mode: false,
                steering_enabled: true,
                decision_learning_enabled: true,
                critic_enabled: true,
                max_delegations: 4,
                max_iterations: 8,
                max_runtime_secs: 600,
                max_lesson_hints: 5,
                approval_policy: "side_effects".to_string(),
            },
        });

        Ok(json!({
            "execution_mode": runtime.mode,
            "default_claw": runtime.default_claw_id,
            "orchestrator_claw": runtime.orchestrator_claw_id,
            "allow_shared_context": runtime.allow_shared_context,
            "isolation_mode": runtime.isolation_mode,
            "autonomy": runtime.autonomy,
            "task_assignments": runtime.task_assignments,
            "category_assignments": runtime.category_assignments,
            "available_claws": registry.claws.iter().map(|claw| json!({
                "id": claw.id,
                "name": claw.name,
                "role": claw.role,
                "agent_profile_id": claw.agent_profile_id,
                "model_profile_id": claw.model_profile_id,
                "memory_scope": claw.memory_scope,
                "task_categories": claw.task_categories,
                "enabled": claw.enabled,
            })).collect::<Vec<_>>(),
            "agent_profiles": registry.agent_profiles.iter().map(|profile| json!({
                "id": profile.id,
                "model_profile_id": profile.model_profile_id,
                "memory_scope": profile.memory_scope,
                "output_policy": profile.output_policy,
                "extends": profile.extends,
            })).collect::<Vec<_>>(),
            "model_profiles": registry.model_profiles.iter().map(|profile| json!({
                "id": profile.id,
                "provider": profile.provider,
                "model": profile.model,
                "role_tags": profile.role_tags,
                "artifact_preferences": profile.artifact_preferences,
                "fallback_order": profile.fallback_order,
            })).collect::<Vec<_>>(),
            "decision_lessons": registry.lessons.iter().map(|lesson| json!({
                "id": lesson.id,
                "active": lesson.active,
                "signal": lesson.signal,
                "recommendation": lesson.recommendation,
                "confidence": lesson.confidence,
                "source": lesson.source,
                "scope": lesson.scope,
            })).collect::<Vec<_>>(),
        }))
    }

    fn validate_runtime(
        &self,
        registry: &ControlRegistrySnapshot,
        runtime: &RuntimeModeSpec,
    ) -> Result<()> {
        self.validate_execution_mode(&runtime.mode, "runtime")?;
        if let Some(default_claw) = &runtime.default_claw_id
            && !registry
                .claws
                .iter()
                .any(|candidate| candidate.id == *default_claw)
        {
            return Err(Error::Config(format!(
                "runtime.default_claw_id '{}' does not exist",
                default_claw
            )));
        }
        if let Some(orchestrator) = &runtime.orchestrator_claw_id
            && !registry
                .claws
                .iter()
                .any(|candidate| candidate.id == *orchestrator)
        {
            return Err(Error::Config(format!(
                "runtime.orchestrator_claw_id '{}' does not exist",
                orchestrator
            )));
        }
        for (task, claw_id) in &runtime.task_assignments {
            if !registry
                .claws
                .iter()
                .any(|candidate| candidate.id == *claw_id)
            {
                return Err(Error::Config(format!(
                    "task assignment '{}' references unknown claw '{}'",
                    task, claw_id
                )));
            }
        }
        for (category, claw_id) in &runtime.category_assignments {
            if !registry
                .claws
                .iter()
                .any(|candidate| candidate.id == *claw_id)
            {
                return Err(Error::Config(format!(
                    "category assignment '{}' references unknown claw '{}'",
                    category, claw_id
                )));
            }
        }

        self.validate_autonomy_level(&runtime.autonomy.autonomy_level, "runtime autonomy")?;
        match runtime.autonomy.approval_policy.as_str() {
            "none" | "side_effects" | "always" => {}
            other => {
                return Err(Error::Config(format!(
                    "invalid approval policy '{}'",
                    other
                )));
            }
        }
        if runtime.autonomy.yolo_mode && runtime.autonomy.autonomy_level != "yolo" {
            return Err(Error::Config(
                "yolo_mode requires autonomy_level 'yolo'".to_string(),
            ));
        }
        if runtime.autonomy.max_delegations == 0 {
            return Err(Error::Config(
                "autonomy.max_delegations must be at least 1".to_string(),
            ));
        }
        if runtime.autonomy.max_iterations == 0 {
            return Err(Error::Config(
                "autonomy.max_iterations must be at least 1".to_string(),
            ));
        }
        if runtime.autonomy.max_runtime_secs == 0 {
            return Err(Error::Config(
                "autonomy.max_runtime_secs must be at least 1".to_string(),
            ));
        }
        if runtime.autonomy.max_lesson_hints == 0 {
            return Err(Error::Config(
                "autonomy.max_lesson_hints must be at least 1".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_execution_mode(&self, value: &str, scope: &str) -> Result<()> {
        match value {
            "solo_claw" | "task_assigned" | "category_assigned" | "orchestrated" => Ok(()),
            other => Err(Error::Config(format!(
                "{} uses invalid execution mode '{}'",
                scope, other
            ))),
        }
    }

    fn validate_autonomy_level(&self, value: &str, scope: &str) -> Result<()> {
        match value {
            "assisted" | "supervised" | "managed" | "autonomous" | "yolo" => Ok(()),
            other => Err(Error::Config(format!(
                "{} uses invalid autonomy level '{}'",
                scope, other
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_registry() -> ControlRegistrySnapshot {
        ControlRegistrySnapshot {
            agent_profiles: vec![AgentProfileSpec {
                id: "default".to_string(),
                extends: Vec::new(),
                model_profile_id: Some("core-groq".to_string()),
                memory_scope: "workspace_shared".to_string(),
                output_policy: "standard".to_string(),
            }],
            model_profiles: vec![ModelProfileSpec {
                id: "core-groq".to_string(),
                provider: "groq".to_string(),
                model: "llama".to_string(),
                role_tags: vec!["assistant".to_string()],
                artifact_preferences: Vec::new(),
                fallback_order: Vec::new(),
            }],
            claws: vec![ClawSpec {
                id: "main".to_string(),
                name: Some("Main".to_string()),
                role: "worker".to_string(),
                agent_profile_id: "default".to_string(),
                model_profile_id: "core-groq".to_string(),
                memory_scope: "workspace_shared".to_string(),
                task_categories: vec!["ops".to_string()],
                enabled: true,
            }],
            lessons: Vec::new(),
            runtime: Some(RuntimeModeSpec {
                mode: "solo_claw".to_string(),
                default_claw_id: Some("main".to_string()),
                orchestrator_claw_id: None,
                task_assignments: Vec::new(),
                category_assignments: Vec::new(),
                allow_shared_context: false,
                isolation_mode: "strict".to_string(),
                autonomy: AutonomyPolicy {
                    autonomy_level: "managed".to_string(),
                    yolo_mode: false,
                    steering_enabled: true,
                    decision_learning_enabled: true,
                    critic_enabled: true,
                    max_delegations: 4,
                    max_iterations: 8,
                    max_runtime_secs: 600,
                    max_lesson_hints: 5,
                    approval_policy: "side_effects".to_string(),
                },
            }),
        }
    }

    #[test]
    fn describe_registry_returns_runtime_summary() {
        let description = ControlRegistryService::new()
            .describe_registry(&sample_registry())
            .unwrap();
        assert_eq!(description["execution_mode"].as_str(), Some("solo_claw"));
        assert_eq!(
            description["available_claws"][0]["id"].as_str(),
            Some("main")
        );
    }

    #[test]
    fn validate_registry_rejects_unknown_claw_assignment() {
        let mut registry = sample_registry();
        registry.runtime.as_mut().unwrap().task_assignments =
            vec![("daily-review".to_string(), "missing".to_string())];
        assert!(
            ControlRegistryService::new()
                .validate_registry(&registry)
                .is_err()
        );
    }
}
