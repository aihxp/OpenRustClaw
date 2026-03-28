use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use super::{control, enterprise_access, enterprise_autonomy};

const SELF_HOSTED_PRODUCT_VERSION: u32 = 1;

pub const MODE_SOLO: &str = "solo";
pub const MODE_TEAM: &str = "team";
pub const MODE_COMPANY: &str = "company";
pub const MODE_ENTERPRISE: &str = "enterprise";

fn default_version() -> u32 {
    SELF_HOSTED_PRODUCT_VERSION
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfHostedProductManifest {
    #[serde(default = "default_version")]
    pub version: u32,
    pub profile: SelfHostedProductProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfHostedProductProfile {
    pub mode: String,
    pub onboarding_path: String,
    #[serde(default = "default_true")]
    pub self_hosted: bool,
    #[serde(default = "default_true")]
    pub open_source: bool,
    #[serde(default)]
    pub note: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfHostedProductTransitionRequest {
    pub target_mode: String,
    pub actor: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub via: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfHostedProductTransitionEvent {
    pub created_at: String,
    pub from_mode: String,
    pub to_mode: String,
    pub direction: String,
    pub actor: String,
    #[serde(default)]
    pub reason: Option<String>,
    pub via: String,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SelfHostedModeDescriptor {
    pub mode: &'static str,
    pub label: &'static str,
    pub onboarding_path: &'static str,
    pub operator_model: &'static str,
    pub recommended_runtime_mode: &'static str,
    pub multi_user: bool,
}

pub fn self_hosted_product_path(workspace_root: &Path) -> PathBuf {
    control::control_root_for(workspace_root)
        .join("self-hosted")
        .join("product-mode.json")
}

pub fn self_hosted_product_events_path(workspace_root: &Path) -> PathBuf {
    control::control_root_for(workspace_root)
        .join("self-hosted")
        .join("product-mode-events.jsonl")
}

pub fn load_manifest(workspace_root: &Path) -> Result<Option<SelfHostedProductManifest>> {
    let path = self_hosted_product_path(workspace_root);
    if !path.exists() {
        return Ok(None);
    }

    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let manifest = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(manifest))
}

pub fn configure_mode(
    workspace_root: &Path,
    mode: &str,
    onboarding_path: Option<&str>,
    note: Option<&str>,
) -> Result<SelfHostedProductManifest> {
    let descriptor = descriptor_for(mode)?;
    let path = self_hosted_product_path(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let manifest = SelfHostedProductManifest {
        version: SELF_HOSTED_PRODUCT_VERSION,
        profile: SelfHostedProductProfile {
            mode: descriptor.mode.to_string(),
            onboarding_path: onboarding_path
                .unwrap_or(descriptor.onboarding_path)
                .trim()
                .to_string(),
            self_hosted: true,
            open_source: true,
            note: note
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            updated_at: Utc::now().to_rfc3339(),
        },
    };

    fs::write(
        &path,
        serde_json::to_string_pretty(&manifest).context("failed to serialize product mode")?,
    )
    .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(manifest)
}

pub fn transition_mode(
    workspace_root: &Path,
    request: SelfHostedProductTransitionRequest,
) -> Result<SelfHostedProductTransitionEvent> {
    let actor = request.actor.trim();
    if actor.is_empty() {
        anyhow::bail!("transition actor must not be empty");
    }

    let current_manifest = load_manifest(workspace_root)?.unwrap_or_else(|| {
        let descriptor = default_descriptor();
        SelfHostedProductManifest {
            version: SELF_HOSTED_PRODUCT_VERSION,
            profile: SelfHostedProductProfile {
                mode: descriptor.mode.to_string(),
                onboarding_path: descriptor.onboarding_path.to_string(),
                self_hosted: true,
                open_source: true,
                note: None,
                updated_at: String::new(),
            },
        }
    });
    let current_mode = current_manifest.profile.mode.clone();
    let target_descriptor = descriptor_for(&request.target_mode)?;
    if current_mode == target_descriptor.mode {
        anyhow::bail!("product mode is already `{}`", target_descriptor.mode);
    }

    let warnings = transition_warnings(workspace_root, &current_mode, target_descriptor.mode)?;
    configure_mode(
        workspace_root,
        target_descriptor.mode,
        Some(target_descriptor.onboarding_path),
        request.reason.as_deref(),
    )?;

    let event = SelfHostedProductTransitionEvent {
        created_at: Utc::now().to_rfc3339(),
        from_mode: current_mode.clone(),
        to_mode: target_descriptor.mode.to_string(),
        direction: transition_direction(&current_mode, target_descriptor.mode).to_string(),
        actor: actor.to_string(),
        reason: request
            .reason
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        via: request
            .via
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "control_api".to_string()),
        warnings,
    };
    append_transition_event(workspace_root, &event)?;
    Ok(event)
}

pub fn recent_transition_events(
    workspace_root: &Path,
    limit: usize,
) -> Result<Vec<SelfHostedProductTransitionEvent>> {
    let path = self_hosted_product_events_path(workspace_root);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file =
        fs::File::open(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let event = serde_json::from_str::<SelfHostedProductTransitionEvent>(trimmed)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        events.push(event);
    }
    events.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    events.truncate(limit.max(1));
    Ok(events)
}

pub fn current_warnings(workspace_root: &Path, mode: &str) -> Result<Vec<String>> {
    let mut warnings = Vec::new();
    if mode != MODE_ENTERPRISE && enterprise_access::load_manifest(workspace_root)?.is_some() {
        warnings.push(
            "Enterprise access data is still present on disk even though the current product mode is below enterprise.".to_string(),
        );
    }
    if mode != MODE_ENTERPRISE && enterprise_autonomy::load_manifest(workspace_root)?.is_some() {
        warnings.push(
            "Enterprise full-autonomy data is still present on disk and should be reviewed after this downgrade.".to_string(),
        );
    }
    if mode == MODE_ENTERPRISE && enterprise_access::load_manifest(workspace_root)?.is_none() {
        warnings.push(
            "Enterprise mode is selected, but enterprise access is not bootstrapped yet."
                .to_string(),
        );
    }
    if mode == MODE_SOLO
        && let Ok(registry) = control::load_registry(control::control_root_for(workspace_root))
        && let Some(runtime) = registry.runtime
        && runtime.mode != "solo_claw"
    {
        warnings.push(format!(
            "Runtime execution mode is still `{}`; solo product mode recommends `solo_claw`.",
            runtime.mode
        ));
    }
    Ok(warnings)
}

pub fn descriptor_for(mode: &str) -> Result<SelfHostedModeDescriptor> {
    let descriptor = match normalize_mode(mode)?.as_str() {
        MODE_SOLO => SelfHostedModeDescriptor {
            mode: MODE_SOLO,
            label: "Solo",
            onboarding_path: "solo_starter",
            operator_model: "One primary operator on one self-hosted workspace",
            recommended_runtime_mode: "solo_claw",
            multi_user: false,
        },
        MODE_TEAM => SelfHostedModeDescriptor {
            mode: MODE_TEAM,
            label: "Multi-User Team",
            onboarding_path: "shared_team_setup",
            operator_model: "A small team shares one self-hosted deployment",
            recommended_runtime_mode: "task_assigned",
            multi_user: true,
        },
        MODE_COMPANY => SelfHostedModeDescriptor {
            mode: MODE_COMPANY,
            label: "Company",
            onboarding_path: "company_ops_setup",
            operator_model: "An operator-managed company deployment with stronger operational defaults",
            recommended_runtime_mode: "orchestrated",
            multi_user: true,
        },
        MODE_ENTERPRISE => SelfHostedModeDescriptor {
            mode: MODE_ENTERPRISE,
            label: "Enterprise",
            onboarding_path: "enterprise_governed_setup",
            operator_model: "A governed enterprise deployment with explicit access, policy, and audit expectations",
            recommended_runtime_mode: "orchestrated",
            multi_user: true,
        },
        _ => unreachable!("normalize_mode validates all supported modes"),
    };
    Ok(descriptor)
}

pub fn default_descriptor() -> SelfHostedModeDescriptor {
    descriptor_for(MODE_SOLO).expect("solo descriptor should always be valid")
}

fn normalize_mode(mode: &str) -> Result<String> {
    let normalized = mode.trim().to_ascii_lowercase();
    match normalized.as_str() {
        MODE_SOLO | MODE_TEAM | MODE_COMPANY | MODE_ENTERPRISE => Ok(normalized),
        _ => Err(anyhow!(
            "unsupported product mode '{}'; expected one of: solo, team, company, enterprise",
            mode
        )),
    }
}

fn transition_direction(from_mode: &str, to_mode: &str) -> &'static str {
    if mode_rank(to_mode) > mode_rank(from_mode) {
        "upgrade"
    } else {
        "downgrade"
    }
}

fn transition_warnings(
    workspace_root: &Path,
    from_mode: &str,
    to_mode: &str,
) -> Result<Vec<String>> {
    let mut warnings = Vec::new();
    if to_mode == MODE_ENTERPRISE && enterprise_access::load_manifest(workspace_root)?.is_none() {
        warnings.push(
            "Enterprise mode expects explicit access bootstrap; bootstrap enterprise access after the transition.".to_string(),
        );
    }
    if mode_rank(to_mode) < mode_rank(from_mode)
        && enterprise_access::load_manifest(workspace_root)?.is_some()
    {
        warnings.push(
            "Enterprise access and governance data will be retained on disk until you remove or reconfigure it explicitly.".to_string(),
        );
    }
    if mode_rank(to_mode) < mode_rank(from_mode)
        && enterprise_autonomy::load_manifest(workspace_root)?.is_some()
    {
        warnings.push(
            "Enterprise autonomy data will be retained on disk and should be reviewed after the downgrade.".to_string(),
        );
    }
    if to_mode == MODE_SOLO
        && let Ok(registry) = control::load_registry(control::control_root_for(workspace_root))
        && let Some(runtime) = registry.runtime
        && runtime.mode != "solo_claw"
    {
        warnings.push(format!(
            "Runtime execution mode remains `{}` until you change it explicitly; solo mode recommends `solo_claw`.",
            runtime.mode
        ));
    }
    Ok(warnings)
}

fn append_transition_event(
    workspace_root: &Path,
    event: &SelfHostedProductTransitionEvent,
) -> Result<()> {
    let path = self_hosted_product_events_path(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    writeln!(
        file,
        "{}",
        serde_json::to_string(event).context("failed to serialize transition event")?
    )
    .with_context(|| format!("failed to append {}", path.display()))?;
    Ok(())
}

fn mode_rank(mode: &str) -> usize {
    match mode {
        MODE_SOLO => 0,
        MODE_TEAM => 1,
        MODE_COMPANY => 2,
        MODE_ENTERPRISE => 3,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn configure_mode_persists_manifest() -> Result<()> {
        let root = tempdir().expect("tempdir");
        let manifest = configure_mode(
            root.path(),
            MODE_TEAM,
            Some("shared_team_setup"),
            Some("selected in onboarding"),
        )?;

        assert_eq!(manifest.profile.mode, MODE_TEAM);
        assert_eq!(manifest.profile.onboarding_path, "shared_team_setup");
        assert!(manifest.profile.self_hosted);
        assert!(manifest.profile.open_source);
        assert_eq!(
            load_manifest(root.path())?.expect("manifest").profile.mode,
            MODE_TEAM
        );
        Ok(())
    }

    #[test]
    fn descriptor_for_rejects_unknown_mode() {
        let error = descriptor_for("hosted").expect_err("unknown mode should fail");
        assert!(error.to_string().contains("unsupported product mode"));
    }

    #[test]
    fn default_descriptor_points_to_solo_path() {
        let descriptor = default_descriptor();
        assert_eq!(descriptor.mode, MODE_SOLO);
        assert_eq!(descriptor.recommended_runtime_mode, "solo_claw");
        assert!(!descriptor.multi_user);
    }

    #[test]
    fn transition_mode_records_event_and_direction() -> Result<()> {
        let root = tempdir().expect("tempdir");
        configure_mode(root.path(), MODE_TEAM, Some("shared_team_setup"), None)?;
        let event = transition_mode(
            root.path(),
            SelfHostedProductTransitionRequest {
                target_mode: MODE_COMPANY.to_string(),
                actor: "operator-1".to_string(),
                reason: Some("growing team".to_string()),
                via: Some("test".to_string()),
            },
        )?;

        assert_eq!(event.direction, "upgrade");
        assert_eq!(event.from_mode, MODE_TEAM);
        assert_eq!(event.to_mode, MODE_COMPANY);
        assert_eq!(recent_transition_events(root.path(), 5)?.len(), 1);
        assert_eq!(
            load_manifest(root.path())?.expect("manifest").profile.mode,
            MODE_COMPANY
        );
        Ok(())
    }
}
