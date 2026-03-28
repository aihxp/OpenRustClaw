use openrustclaw_core::error::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillAuthPluginBindingRequest {
    pub provider_id: String,
    pub skill_name: String,
    #[serde(default)]
    pub declared_auth_providers: Vec<String>,
    pub blocked: bool,
    #[serde(default)]
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub issuer: Option<String>,
    #[serde(default)]
    pub authorization_endpoint: Option<String>,
    #[serde(default)]
    pub token_endpoint: Option<String>,
    #[serde(default)]
    pub client_id_key: Option<String>,
    #[serde(default)]
    pub client_secret_key: Option<String>,
    #[serde(default)]
    pub raw_scopes: Option<String>,
    #[serde(default)]
    pub vault_key_prefix: Option<String>,
    #[serde(default)]
    pub service: Option<String>,
    #[serde(default)]
    pub component: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillAuthPluginBinding {
    pub provider_id: String,
    pub skill_name: String,
    #[serde(default)]
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub issuer: Option<String>,
    #[serde(default)]
    pub authorization_endpoint: Option<String>,
    #[serde(default)]
    pub token_endpoint: Option<String>,
    pub client_id_key: String,
    pub client_secret_key: String,
    pub scopes: Vec<String>,
    pub vault_key_prefix: String,
    #[serde(default)]
    pub service: Option<String>,
    #[serde(default)]
    pub component: Option<String>,
    pub configured_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillAuthPluginBindingReport {
    pub status: String,
    pub binding: SkillAuthPluginBinding,
}

pub struct SkillAuthPluginBindingService;

impl Default for SkillAuthPluginBindingService {
    fn default() -> Self {
        Self
    }
}

impl SkillAuthPluginBindingService {
    pub fn new() -> Self {
        Self
    }

    pub fn bind(
        &self,
        request: SkillAuthPluginBindingRequest,
        existing_configured_at: Option<String>,
        now: String,
    ) -> Result<SkillAuthPluginBindingReport> {
        if request.blocked {
            return Err(Error::Internal(format!(
                "Compiled skill '{}' is blocked and cannot be bound as an auth plugin",
                request.skill_name
            )));
        }

        if !request.declared_auth_providers.is_empty()
            && !request
                .declared_auth_providers
                .iter()
                .any(|provider| provider == &request.provider_id)
        {
            return Err(Error::Internal(format!(
                "Compiled skill '{}' declares auth providers [{}], not '{}'",
                request.skill_name,
                request.declared_auth_providers.join(", "),
                request.provider_id
            )));
        }

        if request.issuer.is_none()
            && (request.authorization_endpoint.is_none() || request.token_endpoint.is_none())
        {
            return Err(Error::Internal(format!(
                "Bind auth plugin '{}' with either --issuer for OIDC discovery or both --authorization-endpoint and --token-endpoint",
                request.provider_id
            )));
        }

        let vault_key_prefix = request
            .vault_key_prefix
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| default_auth_prefix(&request.provider_id));
        let client_id_key = request
            .client_id_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("{}_CLIENT_ID", vault_key_prefix));
        let client_secret_key = request
            .client_secret_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("{}_CLIENT_SECRET", vault_key_prefix));
        let configured_at = existing_configured_at.unwrap_or_else(|| now.clone());

        Ok(SkillAuthPluginBindingReport {
            status: "ok".to_string(),
            binding: SkillAuthPluginBinding {
                provider_id: request.provider_id,
                skill_name: request.skill_name,
                redirect_uri: request.redirect_uri,
                issuer: request.issuer,
                authorization_endpoint: request.authorization_endpoint,
                token_endpoint: request.token_endpoint,
                client_id_key,
                client_secret_key,
                scopes: auth_scope_list(request.raw_scopes.as_deref()),
                vault_key_prefix,
                service: request.service,
                component: request.component,
                configured_at,
                updated_at: now,
            },
        })
    }
}

fn default_auth_prefix(provider_id: &str) -> String {
    provider_id
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() {
                value.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn auth_scope_list(raw: Option<&str>) -> Vec<String> {
    raw.map(|value| {
        value
            .split(',')
            .map(str::trim)
            .filter(|scope| !scope.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_auth_plugin_rejects_blocked_skill() {
        let error = SkillAuthPluginBindingService::new()
            .bind(
                SkillAuthPluginBindingRequest {
                    provider_id: "google".to_string(),
                    skill_name: "auth-skill".to_string(),
                    declared_auth_providers: vec![],
                    blocked: true,
                    redirect_uri: None,
                    issuer: Some("https://accounts.google.com".to_string()),
                    authorization_endpoint: None,
                    token_endpoint: None,
                    client_id_key: None,
                    client_secret_key: None,
                    raw_scopes: None,
                    vault_key_prefix: None,
                    service: None,
                    component: None,
                },
                None,
                "2026-03-28T00:00:00Z".to_string(),
            )
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("cannot be bound as an auth plugin")
        );
    }

    #[test]
    fn bind_auth_plugin_derives_keys_and_preserves_existing_timestamp() -> Result<()> {
        let report = SkillAuthPluginBindingService::new().bind(
            SkillAuthPluginBindingRequest {
                provider_id: "google-oauth".to_string(),
                skill_name: "auth-skill".to_string(),
                declared_auth_providers: vec!["google-oauth".to_string()],
                blocked: false,
                redirect_uri: Some("http://localhost/callback".to_string()),
                issuer: Some("https://accounts.google.com".to_string()),
                authorization_endpoint: None,
                token_endpoint: None,
                client_id_key: None,
                client_secret_key: None,
                raw_scopes: Some("openid,email".to_string()),
                vault_key_prefix: None,
                service: Some("auth".to_string()),
                component: Some("oauth".to_string()),
            },
            Some("2026-03-27T10:00:00Z".to_string()),
            "2026-03-28T12:00:00Z".to_string(),
        )?;

        assert_eq!(report.status, "ok");
        assert_eq!(report.binding.vault_key_prefix, "GOOGLE_OAUTH");
        assert_eq!(report.binding.client_id_key, "GOOGLE_OAUTH_CLIENT_ID");
        assert_eq!(
            report.binding.client_secret_key,
            "GOOGLE_OAUTH_CLIENT_SECRET"
        );
        assert_eq!(
            report.binding.scopes,
            vec!["openid".to_string(), "email".to_string()]
        );
        assert_eq!(report.binding.configured_at, "2026-03-27T10:00:00Z");
        assert_eq!(report.binding.updated_at, "2026-03-28T12:00:00Z");
        Ok(())
    }
}
