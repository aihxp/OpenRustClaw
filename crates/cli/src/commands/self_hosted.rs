use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use super::control;

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

#[derive(Debug, Clone)]
pub struct SelfHostedModeDescriptor {
    pub mode: &'static str,
    pub label: &'static str,
    pub onboarding_path: &'static str,
    pub operator_model: &'static str,
    pub recommended_runtime_mode: &'static str,
    pub detail: &'static str,
    pub multi_user: bool,
    pub enterprise_controls_expected: bool,
    pub transition_targets: &'static [&'static str],
}

pub fn self_hosted_product_path(workspace_root: &Path) -> PathBuf {
    control::control_root_for(workspace_root)
        .join("self-hosted")
        .join("product-mode.json")
}

pub fn load_manifest(workspace_root: &Path) -> Result<Option<SelfHostedProductManifest>> {
    let path = self_hosted_product_path(workspace_root);
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
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
            note: note.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()),
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

pub fn descriptor_for(mode: &str) -> Result<SelfHostedModeDescriptor> {
    let descriptor = match normalize_mode(mode)?.as_str() {
        MODE_SOLO => SelfHostedModeDescriptor {
            mode: MODE_SOLO,
            label: "Solo",
            onboarding_path: "solo_starter",
            operator_model: "One primary operator on one self-hosted workspace",
            recommended_runtime_mode: "solo_claw",
            detail: "Solo mode keeps OpenRustClaw lightweight for one operator while preserving the full self-hosted open-source runtime.",
            multi_user: false,
            enterprise_controls_expected: false,
            transition_targets: &[MODE_TEAM, MODE_COMPANY, MODE_ENTERPRISE],
        },
        MODE_TEAM => SelfHostedModeDescriptor {
            mode: MODE_TEAM,
            label: "Multi-User Team",
            onboarding_path: "shared_team_setup",
            operator_model: "A small team shares one self-hosted deployment",
            recommended_runtime_mode: "task_assigned",
            detail: "Team mode keeps the product self-hosted and open-source while preparing the workspace for multiple operators and shared channels.",
            multi_user: true,
            enterprise_controls_expected: false,
            transition_targets: &[MODE_SOLO, MODE_COMPANY, MODE_ENTERPRISE],
        },
        MODE_COMPANY => SelfHostedModeDescriptor {
            mode: MODE_COMPANY,
            label: "Company",
            onboarding_path: "company_ops_setup",
            operator_model: "An operator-managed company deployment with stronger operational defaults",
            recommended_runtime_mode: "orchestrated",
            detail: "Company mode assumes a broader internal deployment, stronger operator practices, and a clearer path into governance without claiming full enterprise controls by default.",
            multi_user: true,
            enterprise_controls_expected: true,
            transition_targets: &[MODE_TEAM, MODE_ENTERPRISE],
        },
        MODE_ENTERPRISE => SelfHostedModeDescriptor {
            mode: MODE_ENTERPRISE,
            label: "Enterprise",
            onboarding_path: "enterprise_governed_setup",
            operator_model: "A governed enterprise deployment with explicit access, policy, and audit expectations",
            recommended_runtime_mode: "orchestrated",
            detail: "Enterprise mode keeps the product self-hosted and open-source while signaling the strongest operator, policy, audit, and governance expectations in the current shipped runtime.",
            multi_user: true,
            enterprise_controls_expected: true,
            transition_targets: &[MODE_COMPANY, MODE_TEAM],
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
        assert_eq!(load_manifest(root.path())?.expect("manifest").profile.mode, MODE_TEAM);
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
}
