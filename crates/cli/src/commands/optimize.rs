//! Autonomous optimization commands.

use anyhow::{Context, Result, anyhow};
use openrustclaw_observability::LangSmithClient;
use openrustclaw_optimization::{
    CandidateChange, CandidateRunner, CandidateRunnerConfig, EvaluationCommand, EvaluationSpec,
    MutationPolicy, OptimizationStore, PromotionPolicy, TargetRegistration,
};

async fn optimization_store() -> Result<OptimizationStore> {
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .context("Failed to connect to database")?;
    openrustclaw_db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;
    Ok(OptimizationStore::new(pool))
}

pub async fn list_targets() -> Result<()> {
    let store = optimization_store().await?;
    let targets = store.list_targets().await?;

    if targets.is_empty() {
        println!("No optimization targets registered.");
        return Ok(());
    }

    for target in targets {
        println!(
            "{} [{} | {:?} | {:?}]",
            target.name, target.id, target.execution_tier, target.target_kind
        );
        if let Some(description) = target.description {
            println!("  {}", description);
        }
        println!("  Workspace: {}", target.workspace_root);
        println!(
            "  Policy: {} allowed paths, {} evals",
            target.mutation_policy.allowed_paths.len(),
            target.eval_suite.len()
        );
        println!();
    }

    Ok(())
}

pub async fn show_target(id: &str) -> Result<()> {
    let store = optimization_store().await?;
    let target = store.get_target(id).await?;
    println!("{}", serde_json::to_string_pretty(&target)?);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn register_target(
    name: &str,
    description: Option<&str>,
    kind: &str,
    tier: &str,
    risk_class: &str,
    ship_status: &str,
    workspace_root: &str,
    allowed_paths: &[String],
    forbidden_paths: &[String],
    allowed_fields: &[String],
    max_changed_files: usize,
    max_total_bytes: usize,
    max_diff_lines: usize,
    required_tests: &[String],
    mandatory_evals: &[String],
    eval_specs: &[String],
    metadata: Option<&str>,
) -> Result<()> {
    let store = optimization_store().await?;
    let target = store
        .register_target(TargetRegistration {
            name: name.to_string(),
            description: description.map(ToString::to_string),
            target_kind: parse_enum(kind, "target kind")?,
            execution_tier: parse_enum(tier, "execution tier")?,
            risk_class: parse_enum(risk_class, "risk class")?,
            ship_status: parse_enum(ship_status, "ship status")?,
            workspace_root: workspace_root.to_string(),
            mutation_policy: MutationPolicy {
                allowed_paths: allowed_paths.to_vec(),
                forbidden_paths: forbidden_paths.to_vec(),
                allowed_fields: allowed_fields.to_vec(),
                max_changed_files,
                max_total_bytes,
                max_diff_lines,
                mandatory_evals: mandatory_evals.to_vec(),
                required_tests: required_tests.to_vec(),
            },
            eval_suite: parse_eval_specs(eval_specs)?,
            promotion_policy: PromotionPolicy::default(),
            metadata: parse_json_arg(metadata)?,
        })
        .await?;

    println!(
        "Registered optimization target '{}' ({})",
        target.name, target.id
    );
    Ok(())
}

pub async fn submit_candidate(
    target: &str,
    hypothesis: &str,
    proposed_by: &str,
    change_set_path: &str,
    trace_id: Option<&str>,
) -> Result<()> {
    let store = optimization_store().await?;
    let changes = tokio::fs::read_to_string(change_set_path)
        .await
        .with_context(|| format!("Failed to read change-set file {}", change_set_path))?;
    let changes: Vec<CandidateChange> =
        serde_json::from_str(&changes).context("Failed to parse change-set JSON")?;
    let target = store.get_target(target).await?;
    let candidate = store
        .submit_candidate(
            &target.id,
            hypothesis,
            proposed_by,
            changes,
            trace_id.map(ToString::to_string),
        )
        .await?;
    println!("Submitted candidate {}", candidate.id);
    Ok(())
}

pub async fn list_candidates(
    target: Option<&str>,
    status: Option<&str>,
    limit: usize,
) -> Result<()> {
    let store = optimization_store().await?;
    let target_id = if let Some(target) = target {
        Some(store.get_target(target).await?.id)
    } else {
        None
    };
    let candidates = store
        .list_candidates(
            target_id.as_deref(),
            status
                .map(|value| parse_enum(value, "candidate status"))
                .transpose()?,
            Some(limit),
        )
        .await?;

    if candidates.is_empty() {
        println!("No optimization candidates found.");
        return Ok(());
    }

    for candidate in candidates {
        println!(
            "{} [{}] target={} proposed_by={}",
            candidate.id,
            serde_json::to_string(&candidate.status)?,
            candidate.target_id,
            candidate.proposed_by
        );
        println!("  {}", candidate.hypothesis);
        println!("  {} file changes", candidate.changes.len());
        println!();
    }
    Ok(())
}

pub async fn show_candidate(id: &str) -> Result<()> {
    let store = optimization_store().await?;
    let candidate = store.get_candidate(id).await?;
    let evaluations = store.list_evaluations(id).await?;
    let promotions = store.list_promotions(id).await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "candidate": candidate,
            "evaluations": evaluations,
            "promotions": promotions,
        }))?
    );
    Ok(())
}

pub async fn run_candidate(id: &str) -> Result<()> {
    let store = optimization_store().await?;
    let langsmith = LangSmithClient::from_env(Some("openrustclaw-optimization".to_string()));
    let langsmith = if langsmith.is_enabled() {
        Some(langsmith)
    } else {
        None
    };
    let runner = CandidateRunner::new(store.clone(), langsmith, CandidateRunnerConfig::default());
    let summary = runner.run_candidate(id).await?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

pub async fn promote_candidate(
    id: &str,
    decision: &str,
    decided_by: &str,
    notes: Option<&str>,
    rollback_reference: Option<&str>,
) -> Result<()> {
    let store = optimization_store().await?;
    let event = store
        .record_promotion(
            id,
            parse_enum(decision, "promotion decision")?,
            decided_by,
            notes,
            rollback_reference,
            None,
        )
        .await?;
    println!("{}", serde_json::to_string_pretty(&event)?);
    Ok(())
}

pub async fn approve_candidate(id: &str, decided_by: &str, notes: Option<&str>) -> Result<()> {
    promote_candidate(id, "approve", decided_by, notes, None).await
}

pub async fn reject_candidate(id: &str, decided_by: &str, notes: Option<&str>) -> Result<()> {
    promote_candidate(id, "reject", decided_by, notes, None).await
}

fn parse_json_arg(raw: Option<&str>) -> Result<serde_json::Value> {
    match raw {
        Some(raw) => Ok(
            serde_json::from_str(raw).with_context(|| format!("invalid JSON value: {}", raw))?
        ),
        None => Ok(serde_json::json!({})),
    }
}

fn parse_eval_specs(raw_specs: &[String]) -> Result<Vec<EvaluationSpec>> {
    raw_specs
        .iter()
        .map(|spec| {
            let (name, command) = spec
                .split_once('=')
                .ok_or_else(|| anyhow!("invalid eval '{}': expected name=command", spec))?;
            let parts = shlex::split(command)
                .ok_or_else(|| anyhow!("invalid shell command '{}'", command))?;
            let (program, args) = parts
                .split_first()
                .ok_or_else(|| anyhow!("empty eval command for '{}'", name))?;
            Ok(EvaluationSpec {
                name: name.to_string(),
                command: EvaluationCommand {
                    program: program.to_string(),
                    args: args.to_vec(),
                },
                working_directory: None,
                timeout_secs: Some(300),
                success_metric: None,
                metadata: serde_json::json!({}),
            })
        })
        .collect()
}

fn parse_enum<T>(raw: &str, label: &str) -> Result<T>
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    raw.parse::<T>()
        .map_err(|error| anyhow!("invalid {} '{}': {}", label, raw, error))
}
