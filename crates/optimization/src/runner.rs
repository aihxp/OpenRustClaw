use std::path::{Path, PathBuf};
use std::time::Instant;

use openrustclaw_core::error::{Error, Result};
use openrustclaw_observability::langsmith::{LangSmithClient, RunType};
use tempfile::TempDir;
use tokio::process::Command;
use tracing::warn;
use walkdir::WalkDir;

use crate::models::{
    CandidateEvaluationRecord, CandidateRunSummary, CandidateStatus, EvaluationSpec,
    ExperimentMetrics, OptimizationCandidate, OptimizationTarget,
};
use crate::store::OptimizationStore;

#[derive(Debug, Clone)]
pub struct CandidateRunnerConfig {
    pub copy_excludes: Vec<String>,
}

impl Default for CandidateRunnerConfig {
    fn default() -> Self {
        Self {
            copy_excludes: vec![
                ".git".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
                ".venv".to_string(),
                "venv".to_string(),
                "__pycache__".to_string(),
            ],
        }
    }
}

#[derive(Clone)]
pub struct CandidateRunner {
    store: OptimizationStore,
    langsmith: Option<LangSmithClient>,
    config: CandidateRunnerConfig,
}

impl CandidateRunner {
    pub fn new(
        store: OptimizationStore,
        langsmith: Option<LangSmithClient>,
        config: CandidateRunnerConfig,
    ) -> Self {
        Self {
            store,
            langsmith,
            config,
        }
    }

    pub async fn run_candidate(&self, candidate_id: &str) -> Result<CandidateRunSummary> {
        let candidate = self.store.get_candidate(candidate_id).await?;
        let target = self.store.get_target(&candidate.target_id).await?;
        self.validate_candidate(&target, &candidate)?;

        let mut trace = self.langsmith.as_ref().map(|client| {
            client.new_run(
                "optimization_candidate_run",
                RunType::Chain,
                serde_json::json!({
                    "candidate_id": candidate.id,
                    "target_id": target.id,
                    "target_name": target.name,
                }),
            )
        });

        self.store
            .set_candidate_status(
                &candidate.id,
                CandidateStatus::Running,
                None,
                None,
                Some(serde_json::json!({"state":"running"})),
                trace.as_ref().map(|run| run.id.clone()),
            )
            .await?;

        let temp_workspace = TempDir::new().map_err(|error| Error::Internal(error.to_string()))?;
        let workspace_root = resolve_workspace_root(&target.workspace_root)?;
        copy_workspace(
            &workspace_root,
            temp_workspace.path(),
            &self.config.copy_excludes,
        )
        .map_err(anyhow_to_error)?;

        let diff_summary = apply_changes(
            temp_workspace.path(),
            &candidate,
            target.mutation_policy.max_diff_lines,
        )
        .map_err(anyhow_to_error)?;

        let evaluations = self
            .run_evaluations(
                temp_workspace.path(),
                &target,
                &candidate,
                trace.as_ref().map(|run| run.id.clone()),
            )
            .await?;

        let metrics = build_metrics(&target, &evaluations);
        let result_summary = serde_json::json!({
            "status": serde_json::to_value(&metrics.status).unwrap_or(serde_json::Value::Null),
            "success_count": metrics.success_count,
            "failure_count": metrics.failure_count,
            "required_eval_count": metrics.required_eval_count,
            "required_success_count": metrics.required_success_count,
            "total_duration_ms": metrics.total_duration_ms,
        });

        self.store
            .set_candidate_status(
                &candidate.id,
                metrics.status.clone(),
                Some(diff_summary.clone()),
                Some(result_summary.clone()),
                Some(serde_json::json!({
                    "temp_workspace": temp_workspace.path().display().to_string(),
                    "workspace_root": workspace_root.display().to_string(),
                })),
                trace.as_ref().map(|run| run.id.clone()),
            )
            .await?;

        if let (Some(client), Some(mut run)) = (self.langsmith.clone(), trace.take()) {
            run.outputs = Some(serde_json::json!({
                "candidate_id": candidate.id,
                "metrics": result_summary,
            }));
            run.end_time = Some(chrono::Utc::now());
            if let Err(error) = client.trace_run(&run).await {
                warn!(error = %error, "failed to trace optimization run");
            }
        }

        Ok(CandidateRunSummary {
            candidate: self.store.get_candidate(&candidate.id).await?,
            evaluations,
            metrics,
        })
    }

    fn validate_candidate(
        &self,
        target: &OptimizationTarget,
        candidate: &OptimizationCandidate,
    ) -> Result<()> {
        let policy = &target.mutation_policy;
        if candidate.changes.is_empty() {
            return Err(Error::Config(
                "candidate requires at least one change".to_string(),
            ));
        }
        if candidate.changes.len() > policy.max_changed_files {
            return Err(Error::Config(format!(
                "candidate changes {} files but policy only allows {}",
                candidate.changes.len(),
                policy.max_changed_files
            )));
        }

        let mut total_bytes = 0usize;
        for change in &candidate.changes {
            validate_relative_path(&change.path)?;
            total_bytes += change.new_content.len();
            if total_bytes > policy.max_total_bytes {
                return Err(Error::Config(format!(
                    "candidate exceeds max_total_bytes ({})",
                    policy.max_total_bytes
                )));
            }

            if !policy.allowed_paths.is_empty()
                && !policy
                    .allowed_paths
                    .iter()
                    .any(|prefix| path_matches(&change.path, prefix))
            {
                return Err(Error::Config(format!(
                    "path '{}' is outside the allowed mutation surface",
                    change.path
                )));
            }

            if policy
                .forbidden_paths
                .iter()
                .any(|prefix| path_matches(&change.path, prefix))
            {
                return Err(Error::Config(format!(
                    "path '{}' is forbidden by the mutation policy",
                    change.path
                )));
            }

            if let Some(field_path) = &change.field_path {
                if !policy.allowed_fields.is_empty()
                    && !policy.allowed_fields.iter().any(|allowed| {
                        field_path == allowed || field_path.starts_with(&format!("{allowed}."))
                    })
                {
                    return Err(Error::Config(format!(
                        "field '{}' is outside the allowed field list",
                        field_path
                    )));
                }
            }
        }

        Ok(())
    }

    async fn run_evaluations(
        &self,
        temp_workspace: &Path,
        target: &OptimizationTarget,
        candidate: &OptimizationCandidate,
        parent_trace_id: Option<String>,
    ) -> Result<Vec<CandidateEvaluationRecord>> {
        let evals = if target.eval_suite.is_empty() {
            Vec::new()
        } else {
            target.eval_suite.clone()
        };

        let mut records = Vec::new();
        for eval in evals {
            let mut eval_trace = self.langsmith.as_ref().and_then(|client| {
                parent_trace_id.as_ref().map(|parent| {
                    client.new_child_run(
                        "optimization_eval",
                        RunType::Tool,
                        parent,
                        serde_json::json!({
                            "candidate_id": candidate.id,
                            "eval_name": eval.name,
                            "command": eval.command,
                        }),
                    )
                })
            });
            let record = run_eval(
                &self.store,
                temp_workspace,
                &candidate.id,
                &eval,
                eval_trace.as_ref().map(|run| run.id.clone()),
            )
            .await?;

            if let (Some(client), Some(mut trace)) = (self.langsmith.clone(), eval_trace.take()) {
                trace.outputs = Some(serde_json::json!({
                    "status": record.status,
                    "exit_code": record.exit_code,
                    "duration_ms": record.duration_ms,
                }));
                trace.error = (record.status != "success").then(|| record.stderr.clone());
                trace.end_time = Some(chrono::Utc::now());
                if let Err(error) = client.trace_run(&trace).await {
                    warn!(error = %error, eval = %eval.name, "failed to trace optimization eval");
                }
            }

            records.push(record);
        }
        Ok(records)
    }
}

fn resolve_workspace_root(root: &str) -> Result<PathBuf> {
    let path = PathBuf::from(root);
    if path.is_absolute() {
        Ok(path)
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|error| Error::Internal(format!("failed to resolve workspace root: {error}")))
    }
}

fn validate_relative_path(path: &str) -> Result<()> {
    let path = Path::new(path);
    if path.is_absolute() {
        return Err(Error::Config("absolute paths are not allowed".to_string()));
    }
    if path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(Error::Config(
            "parent directory components are not allowed".to_string(),
        ));
    }
    Ok(())
}

fn path_matches(path: &str, pattern: &str) -> bool {
    let trimmed_pattern = pattern.trim_matches('/');
    let trimmed_path = path.trim_matches('/');
    trimmed_path == trimmed_pattern || trimmed_path.starts_with(&format!("{trimmed_pattern}/"))
}

fn copy_workspace(source: &Path, destination: &Path, excludes: &[String]) -> anyhow::Result<()> {
    for entry in WalkDir::new(source) {
        let entry = entry?;
        let relative = match entry.path().strip_prefix(source) {
            Ok(path) if path.as_os_str().is_empty() => continue,
            Ok(path) => path,
            Err(_) => continue,
        };

        let relative_text = relative.to_string_lossy();
        if excludes
            .iter()
            .any(|exclude| path_matches(&relative_text, exclude))
        {
            if entry.file_type().is_dir() {
                continue;
            }
            continue;
        }

        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(entry.path(), &target)?;
    }

    Ok(())
}

fn apply_changes(
    temp_workspace: &Path,
    candidate: &OptimizationCandidate,
    max_diff_lines: usize,
) -> anyhow::Result<serde_json::Value> {
    let mut files = Vec::new();
    let mut total_diff_lines = 0usize;

    for change in &candidate.changes {
        let target_path = temp_workspace.join(&change.path);
        let original = std::fs::read_to_string(&target_path).unwrap_or_default();
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target_path, &change.new_content)?;

        let diff_lines = estimate_diff_lines(&original, &change.new_content);
        total_diff_lines += diff_lines;
        if total_diff_lines > max_diff_lines {
            anyhow::bail!(
                "candidate diff exceeds max_diff_lines policy ({} > {})",
                total_diff_lines,
                max_diff_lines
            );
        }

        files.push(serde_json::json!({
            "path": change.path,
            "summary": change.summary,
            "field_path": change.field_path,
            "old_bytes": original.len(),
            "new_bytes": change.new_content.len(),
            "diff_lines": diff_lines,
        }));
    }

    Ok(serde_json::json!({
        "file_count": candidate.changes.len(),
        "total_diff_lines": total_diff_lines,
        "files": files,
    }))
}

fn estimate_diff_lines(original: &str, updated: &str) -> usize {
    if original == updated {
        return 0;
    }
    let original_lines = original.lines().count();
    let updated_lines = updated.lines().count();
    original_lines.max(updated_lines).max(1)
}

async fn run_eval(
    store: &OptimizationStore,
    temp_workspace: &Path,
    candidate_id: &str,
    eval: &EvaluationSpec,
    trace_id: Option<String>,
) -> Result<CandidateEvaluationRecord> {
    let start = Instant::now();
    let mut command = Command::new(&eval.command.program);
    command.args(&eval.command.args);

    let workdir = eval
        .working_directory
        .as_ref()
        .map(|dir| temp_workspace.join(dir))
        .unwrap_or_else(|| temp_workspace.to_path_buf());
    command.current_dir(&workdir);

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(eval.timeout_secs.unwrap_or(300)),
        command.output(),
    )
    .await;

    let (status, exit_code, stdout, stderr) = match output {
        Ok(Ok(output)) => (
            if output.status.success() {
                "success"
            } else {
                "failure"
            },
            output.status.code().map(i64::from),
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ),
        Ok(Err(error)) => (
            "failure",
            None,
            String::new(),
            format!("failed to execute command: {error}"),
        ),
        Err(_) => (
            "timeout",
            None,
            String::new(),
            format!("evaluation '{}' timed out", eval.name),
        ),
    };

    let duration_ms = start.elapsed().as_millis() as i64;
    let metrics = serde_json::json!({
        "success": status == "success",
        "exit_code": exit_code,
        "duration_ms": duration_ms,
        "timeout_secs": eval.timeout_secs.unwrap_or(300),
        "working_directory": workdir.display().to_string(),
        "success_metric": eval.success_metric,
        "metadata": eval.metadata,
    });

    store
        .record_evaluation(
            candidate_id,
            &eval.name,
            status,
            exit_code,
            duration_ms,
            &stdout,
            &stderr,
            metrics,
            trace_id,
        )
        .await
}

fn build_metrics(
    target: &OptimizationTarget,
    evaluations: &[CandidateEvaluationRecord],
) -> ExperimentMetrics {
    let success_count = evaluations
        .iter()
        .filter(|record| record.status == "success")
        .count();
    let failure_count = evaluations.len().saturating_sub(success_count);
    let required_names = if target.mutation_policy.mandatory_evals.is_empty() {
        target
            .eval_suite
            .iter()
            .map(|spec| spec.name.clone())
            .collect::<Vec<_>>()
    } else {
        target.mutation_policy.mandatory_evals.clone()
    };
    let required_success_count = evaluations
        .iter()
        .filter(|record| {
            required_names.iter().any(|name| name == &record.eval_name)
                && record.status == "success"
        })
        .count();
    let required_eval_count = required_names.len();
    let total_duration_ms = evaluations
        .iter()
        .map(|record| record.duration_ms)
        .sum::<i64>();
    let status = if failure_count == 0 && required_success_count == required_eval_count {
        CandidateStatus::Passed
    } else {
        CandidateStatus::Failed
    };

    ExperimentMetrics {
        success_count,
        failure_count,
        required_eval_count,
        required_success_count,
        total_duration_ms,
        status,
    }
}

fn anyhow_to_error(error: anyhow::Error) -> Error {
    Error::Internal(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        CandidateChange, EvaluationCommand, EvaluationSpec, ExecutionTier, MutationPolicy,
        PromotionPolicy, RiskClass, ShipStatus, TargetKind, TargetRegistration,
    };
    use crate::store::OptimizationStore;
    use openrustclaw_db::{init_pool, run_migrations};

    #[tokio::test]
    async fn runner_executes_eval_and_records_pass() {
        let workspace = tempfile::tempdir().unwrap();
        std::fs::write(workspace.path().join("prompt.txt"), "original").unwrap();

        let pool = init_pool("sqlite::memory:", 1).await.unwrap();
        run_migrations(&pool).await.unwrap();
        let store = OptimizationStore::new(pool);
        let target = store
            .register_target(TargetRegistration {
                name: "prompt.optimize".to_string(),
                description: None,
                target_kind: TargetKind::PromptPolicy,
                execution_tier: ExecutionTier::RustNative,
                risk_class: RiskClass::SafeConfig,
                ship_status: ShipStatus::Experimental,
                workspace_root: workspace.path().display().to_string(),
                mutation_policy: MutationPolicy {
                    allowed_paths: vec!["prompt.txt".to_string()],
                    mandatory_evals: vec!["verify".to_string()],
                    ..MutationPolicy::default()
                },
                eval_suite: vec![EvaluationSpec {
                    name: "verify".to_string(),
                    command: EvaluationCommand {
                        program: "bash".to_string(),
                        args: vec![
                            "-lc".to_string(),
                            "grep -q optimized prompt.txt".to_string(),
                        ],
                    },
                    working_directory: None,
                    timeout_secs: Some(10),
                    success_metric: None,
                    metadata: serde_json::json!({}),
                }],
                promotion_policy: PromotionPolicy::default(),
                metadata: serde_json::json!({}),
            })
            .await
            .unwrap();

        let candidate = store
            .submit_candidate(
                &target.id,
                "Make prompt clearer",
                "tester",
                vec![CandidateChange {
                    path: "prompt.txt".to_string(),
                    new_content: "optimized".to_string(),
                    summary: Some("replace prompt".to_string()),
                    field_path: None,
                    metadata: serde_json::json!({}),
                }],
                None,
            )
            .await
            .unwrap();

        let runner = CandidateRunner::new(store.clone(), None, CandidateRunnerConfig::default());
        let summary = runner.run_candidate(&candidate.id).await.unwrap();
        assert!(matches!(summary.metrics.status, CandidateStatus::Passed));
        assert_eq!(summary.evaluations.len(), 1);
    }

    #[tokio::test]
    async fn runner_rejects_forbidden_paths() {
        let workspace = tempfile::tempdir().unwrap();
        let pool = init_pool("sqlite::memory:", 1).await.unwrap();
        run_migrations(&pool).await.unwrap();
        let store = OptimizationStore::new(pool);
        let target = store
            .register_target(TargetRegistration {
                name: "bounded.code".to_string(),
                description: None,
                target_kind: TargetKind::BoundedCode,
                execution_tier: ExecutionTier::RustNative,
                risk_class: RiskClass::BoundedCode,
                ship_status: ShipStatus::Experimental,
                workspace_root: workspace.path().display().to_string(),
                mutation_policy: MutationPolicy {
                    allowed_paths: vec!["allowed/".to_string()],
                    ..MutationPolicy::default()
                },
                eval_suite: vec![],
                promotion_policy: PromotionPolicy::default(),
                metadata: serde_json::json!({}),
            })
            .await
            .unwrap();
        let candidate = store
            .submit_candidate(
                &target.id,
                "touch forbidden file",
                "tester",
                vec![CandidateChange {
                    path: "forbidden/file.txt".to_string(),
                    new_content: "nope".to_string(),
                    summary: None,
                    field_path: None,
                    metadata: serde_json::json!({}),
                }],
                None,
            )
            .await
            .unwrap();
        let runner = CandidateRunner::new(store.clone(), None, CandidateRunnerConfig::default());
        let error = runner.run_candidate(&candidate.id).await.unwrap_err();
        assert!(error.to_string().contains("allowed mutation surface"));
    }
}
