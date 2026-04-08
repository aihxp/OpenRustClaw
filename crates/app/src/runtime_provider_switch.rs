use openrustclaw_core::config::AppConfig;
use openrustclaw_core::error::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeProviderSwitchRequest {
    pub provider: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub fallback_chain: Option<Vec<String>>,
}

pub trait RuntimeConfigMutationSource {
    fn load_effective_runtime_config(&self) -> Result<AppConfig>;
    fn validate_runtime_provider(&self, config: &AppConfig, provider: &str) -> Result<()>;
    fn write_runtime_config_with_backup(&self, config: &AppConfig) -> Result<()>;
}

pub struct RuntimeProviderSwitchService<S> {
    source: S,
}

impl<S> RuntimeProviderSwitchService<S> {
    pub fn new(source: S) -> Self {
        Self { source }
    }
}

impl<S> RuntimeProviderSwitchService<S>
where
    S: RuntimeConfigMutationSource,
{
    pub fn switch_provider(&self, request: RuntimeProviderSwitchRequest) -> Result<AppConfig> {
        let mut config = self.source.load_effective_runtime_config()?;
        apply_provider_switch(&mut config, request)?;
        self.source
            .validate_runtime_provider(&config, &config.providers.default_provider)?;
        self.source.write_runtime_config_with_backup(&config)?;
        Ok(config)
    }

    pub fn switch_model(&self, provider: &str, model: &str) -> Result<AppConfig> {
        self.switch_provider(RuntimeProviderSwitchRequest {
            provider: provider.to_string(),
            model: Some(model.to_string()),
            api_key_env: None,
            fallback_chain: None,
        })
    }
}

fn apply_provider_switch(
    config: &mut AppConfig,
    request: RuntimeProviderSwitchRequest,
) -> Result<()> {
    let provider = request.provider.trim().to_ascii_lowercase();
    match provider.as_str() {
        "anthropic" => {
            if let Some(model) = request.model {
                config.providers.anthropic.model = model;
            }
            if let Some(api_key_env) = request.api_key_env {
                config.providers.anthropic.api_key_env = Some(api_key_env);
            }
        }
        "openai" => {
            if let Some(model) = request.model {
                config.providers.openai.model = model;
            }
            if let Some(api_key_env) = request.api_key_env {
                config.providers.openai.api_key_env = Some(api_key_env);
            }
        }
        "openrouter" => {
            if let Some(model) = request.model {
                config.providers.openrouter.model = model;
            }
            if let Some(api_key_env) = request.api_key_env {
                config.providers.openrouter.api_key_env = Some(api_key_env);
            }
        }
        "gemini" => {
            if let Some(model) = request.model {
                config.providers.gemini.model = model;
            }
            if let Some(api_key_env) = request.api_key_env {
                config.providers.gemini.api_key_env = Some(api_key_env);
            }
        }
        "ollama" => {
            if let Some(model) = request.model {
                config.providers.ollama.model = model;
            }
        }
        _ => {
            return Err(Error::Internal(format!(
                "Unknown provider '{}'",
                request.provider
            )));
        }
    }

    config.providers.default_provider = provider;
    if let Some(fallback_chain) = request.fallback_chain {
        config.providers.fallback_chain = fallback_chain;
    }
    ensure_control_plane_defaults(config);
    Ok(())
}

fn provider_supports_control_plane(config: &AppConfig, provider: &str) -> bool {
    match provider {
        "ollama" => true,
        "anthropic" => config.providers.anthropic.api_key_env.is_some(),
        "openai" => config.providers.openai.api_key_env.is_some(),
        "openrouter" => config.providers.openrouter.api_key_env.is_some(),
        "gemini" => config.providers.gemini.api_key_env.is_some(),
        _ => false,
    }
}

fn preferred_control_plane_candidates(config: &AppConfig) -> Vec<String> {
    let mut ordered = Vec::new();
    let default = config.providers.default_provider.as_str();

    for candidate in ["ollama", "openrouter", "anthropic", "openai", "gemini"] {
        if candidate == default {
            continue;
        }
        if provider_supports_control_plane(config, candidate)
            && !ordered.iter().any(|entry| entry == candidate)
        {
            ordered.push(candidate.to_string());
        }
    }

    if ordered.is_empty() {
        ordered.push(config.providers.default_provider.clone());
    }

    ordered
}

fn ensure_control_plane_defaults(config: &mut AppConfig) {
    let preferred = preferred_control_plane_candidates(config);
    let default = config.providers.default_provider.clone();
    let current = config.providers.control_plane_provider.clone();

    let should_replace = current
        .as_deref()
        .map(|provider| provider == default || !provider_supports_control_plane(config, provider))
        .unwrap_or(true);

    if should_replace {
        config.providers.control_plane_provider = preferred.first().cloned();
    }

    let primary = config.providers.control_plane_provider.clone();
    let mut next_chain = Vec::new();
    for candidate in preferred.into_iter().chain(
        config
            .providers
            .control_plane_fallback_chain
            .clone()
            .into_iter(),
    ) {
        if Some(candidate.as_str()) == primary.as_deref() || candidate == default {
            continue;
        }
        if !next_chain.iter().any(|existing| existing == &candidate) {
            next_chain.push(candidate);
        }
    }
    config.providers.control_plane_fallback_chain = next_chain;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestSource {
        config: AppConfig,
    }

    impl RuntimeConfigMutationSource for TestSource {
        fn load_effective_runtime_config(&self) -> Result<AppConfig> {
            Ok(self.config.clone())
        }

        fn validate_runtime_provider(&self, _config: &AppConfig, _provider: &str) -> Result<()> {
            Ok(())
        }

        fn write_runtime_config_with_backup(&self, _config: &AppConfig) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn runtime_provider_switch_updates_provider_fields_and_defaults() -> Result<()> {
        let service = RuntimeProviderSwitchService::new(TestSource {
            config: AppConfig::default(),
        });
        let config = service.switch_provider(RuntimeProviderSwitchRequest {
            provider: "openai".to_string(),
            model: Some("gpt-4.1-mini".to_string()),
            api_key_env: Some("OPENAI_API_KEY".to_string()),
            fallback_chain: Some(vec!["anthropic".to_string()]),
        })?;

        assert_eq!(config.providers.default_provider, "openai");
        assert_eq!(config.providers.openai.model, "gpt-4.1-mini");
        assert_eq!(
            config.providers.openai.api_key_env.as_deref(),
            Some("OPENAI_API_KEY")
        );
        assert_eq!(
            config.providers.fallback_chain,
            vec!["anthropic".to_string()]
        );
        assert!(config.providers.control_plane_provider.is_some());
        Ok(())
    }

    #[test]
    fn runtime_provider_switch_model_reuses_provider_switch_lane() -> Result<()> {
        let service = RuntimeProviderSwitchService::new(TestSource {
            config: AppConfig::default(),
        });
        let config = service.switch_model("anthropic", "claude-sonnet-4-20250514")?;

        assert_eq!(config.providers.default_provider, "anthropic");
        assert_eq!(config.providers.anthropic.model, "claude-sonnet-4-20250514");
        assert!(config.providers.control_plane_provider.is_some());
        Ok(())
    }

    #[test]
    fn runtime_provider_switch_supports_gemini() -> Result<()> {
        let service = RuntimeProviderSwitchService::new(TestSource {
            config: AppConfig::default(),
        });
        let config = service.switch_provider(RuntimeProviderSwitchRequest {
            provider: "gemini".to_string(),
            model: Some("gemini-2.5-pro".to_string()),
            api_key_env: Some("GEMINI_API_KEY".to_string()),
            fallback_chain: None,
        })?;

        assert_eq!(config.providers.default_provider, "gemini");
        assert_eq!(config.providers.gemini.model, "gemini-2.5-pro");
        assert_eq!(
            config.providers.gemini.api_key_env.as_deref(),
            Some("GEMINI_API_KEY")
        );
        assert!(config.providers.control_plane_provider.is_some());
        Ok(())
    }
}
