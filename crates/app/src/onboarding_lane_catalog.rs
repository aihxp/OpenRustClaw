use crate::agent_backend_control::{
    DelegatedAgentBackendContract, DelegatedExecutionKind, DelegatedModelCatalogMode,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingLaneKind {
    DirectApi,
    LocalRuntime,
    DelegatedAgent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirectProviderLaneStatus {
    pub provider_id: String,
    pub available: bool,
    pub status_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OnboardingLaneDescriptor {
    pub lane_id: String,
    pub provider_id: String,
    pub backend_id: Option<String>,
    pub label: String,
    pub kind: OnboardingLaneKind,
    pub supported_access_modes: Vec<String>,
    pub api_key_prompt: Option<String>,
    pub recommended: bool,
    pub status_label: String,
    pub detail: String,
    pub compatibility_note: Option<String>,
    pub model_catalog_label: Option<String>,
}

impl OnboardingLaneDescriptor {
    pub fn selection_label(&self) -> String {
        let mut parts = vec![self.label.clone()];
        if self.recommended {
            parts.push("Recommended".to_string());
        }
        if !self.status_label.is_empty() {
            parts.push(self.status_label.clone());
        }
        parts.join(" - ")
    }
}

#[derive(Debug, Clone, Default)]
pub struct OnboardingLaneCatalogService;

impl OnboardingLaneCatalogService {
    pub fn new() -> Self {
        Self
    }

    pub fn catalog(
        &self,
        direct_statuses: &[DirectProviderLaneStatus],
        delegated_contracts: &[DelegatedAgentBackendContract],
    ) -> Vec<OnboardingLaneDescriptor> {
        let direct_by_provider = direct_statuses
            .iter()
            .map(|status| (status.provider_id.as_str(), status))
            .collect::<BTreeMap<_, _>>();

        let mut lanes = DIRECT_LANE_TEMPLATES
            .iter()
            .map(|template| {
                let status = direct_by_provider.get(template.provider_id).copied();
                OnboardingLaneDescriptor {
                    lane_id: template.provider_id.to_string(),
                    provider_id: template.provider_id.to_string(),
                    backend_id: None,
                    label: template.label.to_string(),
                    kind: template.kind.clone(),
                    supported_access_modes: template
                        .supported_access_modes
                        .iter()
                        .map(|value| value.to_string())
                        .collect(),
                    api_key_prompt: template.api_key_prompt.map(ToString::to_string),
                    recommended: template.recommended,
                    status_label: status
                        .map(|value| value.status_label.clone())
                        .unwrap_or_else(|| template.default_status_label.to_string()),
                    detail: template.detail.to_string(),
                    compatibility_note: template.compatibility_note.map(ToString::to_string),
                    model_catalog_label: None,
                }
            })
            .collect::<Vec<_>>();

        let mut delegated = delegated_contracts
            .iter()
            .filter(|contract| {
                contract.readiness
                    != crate::agent_backend_catalog::AgentBackendReadiness::Unavailable
            })
            .map(|contract| {
                let mut supported_access_modes = vec!["subscription_managed".to_string()];
                if let Some(provider_id) = contract.provider_id.as_deref() {
                    for mode in supported_access_modes_for_provider(provider_id) {
                        if !supported_access_modes.iter().any(|existing| existing == &mode) {
                            supported_access_modes.push(mode);
                        }
                    }
                }

                OnboardingLaneDescriptor {
                    lane_id: contract.backend_id.clone(),
                    provider_id: contract
                        .provider_id
                        .clone()
                        .unwrap_or_else(|| contract.backend_id.clone()),
                    backend_id: Some(contract.backend_id.clone()),
                    label: format!("{} (Delegated local agent)", contract.display_name),
                    kind: OnboardingLaneKind::DelegatedAgent,
                    supported_access_modes,
                    api_key_prompt: contract
                        .provider_id
                        .as_deref()
                        .and_then(api_key_prompt_for_provider)
                        .map(ToString::to_string),
                    recommended: false,
                    status_label: delegated_status_label(contract),
                    detail: delegated_detail(contract),
                    compatibility_note: Some(delegated_compatibility_note(contract)),
                    model_catalog_label: Some(model_catalog_label(contract).to_string()),
                }
            })
            .collect::<Vec<_>>();
        delegated.sort_by(|left, right| left.label.cmp(&right.label));
        lanes.extend(delegated);
        lanes
    }
}

#[derive(Debug, Clone)]
struct DirectLaneTemplate {
    provider_id: &'static str,
    label: &'static str,
    kind: OnboardingLaneKind,
    supported_access_modes: &'static [&'static str],
    api_key_prompt: Option<&'static str>,
    recommended: bool,
    default_status_label: &'static str,
    detail: &'static str,
    compatibility_note: Option<&'static str>,
}

const API_KEY_MODE: [&str; 1] = ["api_key"];
const LOCAL_RUNTIME_MODE: [&str; 1] = ["local_runtime"];

const DIRECT_LANE_TEMPLATES: [DirectLaneTemplate; 5] = [
    DirectLaneTemplate {
        provider_id: "anthropic",
        label: "Anthropic (Claude)",
        kind: OnboardingLaneKind::DirectApi,
        supported_access_modes: &API_KEY_MODE,
        api_key_prompt: Some("Anthropic API key (starts with sk-ant-...)"),
        recommended: true,
        default_status_label: "not configured",
        detail: "Use a provider API key stored in `.env`.",
        compatibility_note: None,
    },
    DirectLaneTemplate {
        provider_id: "openai",
        label: "OpenAI (GPT)",
        kind: OnboardingLaneKind::DirectApi,
        supported_access_modes: &API_KEY_MODE,
        api_key_prompt: Some("OpenAI API key (starts with sk-...)"),
        recommended: false,
        default_status_label: "not configured",
        detail: "Use a provider API key stored in `.env`.",
        compatibility_note: None,
    },
    DirectLaneTemplate {
        provider_id: "openrouter",
        label: "OpenRouter (Multiple models)",
        kind: OnboardingLaneKind::DirectApi,
        supported_access_modes: &API_KEY_MODE,
        api_key_prompt: Some("OpenRouter API key"),
        recommended: false,
        default_status_label: "not configured",
        detail: "Use a provider API key stored in `.env`.",
        compatibility_note: None,
    },
    DirectLaneTemplate {
        provider_id: "gemini",
        label: "Google Gemini",
        kind: OnboardingLaneKind::DirectApi,
        supported_access_modes: &API_KEY_MODE,
        api_key_prompt: Some("Gemini API key"),
        recommended: false,
        default_status_label: "not configured",
        detail: "Use a provider API key stored in `.env` or the documented Google API credential path.",
        compatibility_note: None,
    },
    DirectLaneTemplate {
        provider_id: "ollama",
        label: "Ollama (Local models)",
        kind: OnboardingLaneKind::LocalRuntime,
        supported_access_modes: &LOCAL_RUNTIME_MODE,
        api_key_prompt: None,
        recommended: false,
        default_status_label: "not detected",
        detail: "Use a local runtime already running on this machine.",
        compatibility_note: None,
    },
];

fn supported_access_modes_for_provider(provider_id: &str) -> Vec<String> {
    DIRECT_LANE_TEMPLATES
        .iter()
        .find(|template| template.provider_id == provider_id)
        .map(|template| {
            template
                .supported_access_modes
                .iter()
                .map(|value| value.to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn api_key_prompt_for_provider(provider_id: &str) -> Option<&'static str> {
    DIRECT_LANE_TEMPLATES
        .iter()
        .find(|template| template.provider_id == provider_id)
        .and_then(|template| template.api_key_prompt)
}

fn delegated_status_label(contract: &DelegatedAgentBackendContract) -> String {
    match contract.readiness {
        crate::agent_backend_catalog::AgentBackendReadiness::Ready => "ready locally".to_string(),
        crate::agent_backend_catalog::AgentBackendReadiness::Candidate => {
            "available locally".to_string()
        }
        crate::agent_backend_catalog::AgentBackendReadiness::DetectionOnly => {
            "detected only".to_string()
        }
        crate::agent_backend_catalog::AgentBackendReadiness::Unavailable => {
            "not detected".to_string()
        }
    }
}

fn delegated_detail(contract: &DelegatedAgentBackendContract) -> String {
    contract.readiness_reason.clone().unwrap_or_else(|| {
        "Detected local agent contract with vendor-managed account semantics.".to_string()
    })
}

fn delegated_compatibility_note(contract: &DelegatedAgentBackendContract) -> String {
    let catalog_note = match contract.model_catalog_mode {
        DelegatedModelCatalogMode::Supported => {
            "models are discoverable through a documented surface"
        }
        DelegatedModelCatalogMode::VendorManaged => {
            "model selection remains vendor-managed and should not be guessed"
        }
        DelegatedModelCatalogMode::Unknown => "model discovery is not yet trustworthy",
        DelegatedModelCatalogMode::Unsupported => {
            "this backend does not expose a supported delegated execution contract"
        }
    };
    let execution_note = match contract.delegated_execution_kind {
        DelegatedExecutionKind::LocalCli if contract.execution_eligible => {
            "delegated local execution is eligible under policy-aware routing work"
        }
        DelegatedExecutionKind::LocalCli => {
            "the local agent is visible, but delegated execution is not fully ready yet"
        }
        DelegatedExecutionKind::Unsupported => {
            "this surface remains visible for compatibility only"
        }
    };
    format!("{catalog_note}; {execution_note}.")
}

fn model_catalog_label(contract: &DelegatedAgentBackendContract) -> &'static str {
    match contract.model_catalog_mode {
        DelegatedModelCatalogMode::Supported => "supported",
        DelegatedModelCatalogMode::VendorManaged => "vendor-managed",
        DelegatedModelCatalogMode::Unknown => "unknown",
        DelegatedModelCatalogMode::Unsupported => "unsupported",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_backend_catalog::AgentBackendReadiness;

    fn delegated_contract() -> DelegatedAgentBackendContract {
        DelegatedAgentBackendContract {
            backend_id: "claude_code".to_string(),
            display_name: "Claude Code".to_string(),
            provider_id: Some("anthropic".to_string()),
            transport: "local_cli_wrapper".to_string(),
            readiness: AgentBackendReadiness::Ready,
            readiness_reason: None,
            model_catalog_mode: DelegatedModelCatalogMode::VendorManaged,
            delegated_execution_kind: DelegatedExecutionKind::LocalCli,
            execution_eligible: true,
            policy_classification: "delegated_cli_candidate".to_string(),
            auth_method: Some("claude_ai".to_string()),
            notes: vec!["note".to_string()],
        }
    }

    #[test]
    fn catalog_includes_direct_and_delegated_lanes() {
        let catalog = OnboardingLaneCatalogService::new().catalog(
            &[
                DirectProviderLaneStatus {
                    provider_id: "anthropic".to_string(),
                    available: true,
                    status_label: "configured".to_string(),
                },
                DirectProviderLaneStatus {
                    provider_id: "ollama".to_string(),
                    available: false,
                    status_label: "not detected".to_string(),
                },
            ],
            &[delegated_contract()],
        );

        assert!(catalog.iter().any(|lane| lane.lane_id == "anthropic"));
        assert!(catalog.iter().any(|lane| lane.lane_id == "claude_code"));
        assert_eq!(
            catalog
                .iter()
                .find(|lane| lane.lane_id == "claude_code")
                .unwrap()
                .model_catalog_label
                .as_deref(),
            Some("vendor-managed")
        );
        assert_eq!(
            catalog
                .iter()
                .find(|lane| lane.lane_id == "claude_code")
                .unwrap()
                .supported_access_modes,
            vec!["subscription_managed".to_string(), "api_key".to_string()]
        );
    }

    #[test]
    fn selection_label_includes_recommendation_and_status() {
        let lane = OnboardingLaneDescriptor {
            lane_id: "anthropic".to_string(),
            provider_id: "anthropic".to_string(),
            backend_id: None,
            label: "Anthropic (Claude)".to_string(),
            kind: OnboardingLaneKind::DirectApi,
            supported_access_modes: vec!["api_key".to_string()],
            api_key_prompt: Some("Anthropic API key".to_string()),
            recommended: true,
            status_label: "configured".to_string(),
            detail: "detail".to_string(),
            compatibility_note: None,
            model_catalog_label: None,
        };

        assert_eq!(
            lane.selection_label(),
            "Anthropic (Claude) - Recommended - configured"
        );
    }
}
