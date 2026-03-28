use async_trait::async_trait;
use openrustclaw_core::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillSearchRequest {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillBindAuthPluginRequest {
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
    #[serde(default)]
    pub client_id_key: Option<String>,
    #[serde(default)]
    pub client_secret_key: Option<String>,
    #[serde(default)]
    pub scopes: Option<String>,
    #[serde(default)]
    pub vault_key_prefix: Option<String>,
    #[serde(default)]
    pub service: Option<String>,
    #[serde(default)]
    pub component: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillAuthCallbackRequest {
    #[serde(default)]
    pub provider_id: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub error_description: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillBindVoicePluginRequest {
    pub plugin_id: String,
    pub skill_name: String,
    #[serde(default)]
    pub service: Option<String>,
    #[serde(default)]
    pub component: Option<String>,
    #[serde(default)]
    pub greeting_text: Option<String>,
    #[serde(default)]
    pub default_voice: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillCompileRequest {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillInvokeRequest {
    #[serde(default)]
    pub args: Option<String>,
    #[serde(default)]
    pub reference: Option<String>,
    #[serde(default)]
    pub max_chars: Option<usize>,
    #[serde(default)]
    pub detail: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SkillExecuteRequest {
    #[serde(default)]
    pub component: Option<String>,
    #[serde(default)]
    pub input: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillAuthAuthorizeRequest {
    #[serde(default)]
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillAuthExchangeRequest {
    pub code: String,
    pub state: String,
    #[serde(default)]
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillPrewarmVoicePluginRequest {
    #[serde(default)]
    pub greeting_text: Option<String>,
    #[serde(default)]
    pub voice: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SkillScheduleBackgroundRequest {
    #[serde(default)]
    pub service: Option<String>,
    #[serde(default)]
    pub component: Option<String>,
    #[serde(default)]
    pub input: Option<String>,
    #[serde(default)]
    pub every_seconds: Option<u64>,
    #[serde(default)]
    pub at: Option<String>,
    #[serde(default = "default_skill_schedule_background_priority")]
    pub priority: i64,
}

fn default_skill_schedule_background_priority() -> i64 {
    100
}

#[async_trait]
pub trait SkillControlSource {
    async fn installed_skills(&self) -> Result<Value>;
    async fn installed_skill_detail(&self, name: &str) -> Result<Value>;
    async fn compiled_skill_detail(&self, name: &str) -> Result<Value>;
    async fn extension_manifest(&self, name: &str) -> Result<Value>;
    async fn background_services(&self, name: &str) -> Result<Value>;
    async fn auth_plugins_for_skill(&self, name: &str) -> Result<Value>;
    async fn voice_plugins_for_skill(&self, name: &str) -> Result<Value>;
    async fn search(&self, query: &str, category: Option<&str>, sort: &str) -> Result<Value>;
    async fn popular(&self, limit: usize) -> Result<Value>;
    async fn trending(&self, limit: usize) -> Result<Value>;
    async fn install(&self, name: &str) -> Result<Value>;
    async fn compile(&self, name: Option<&str>) -> Result<Value>;
    async fn compiled_skills(&self) -> Result<Value>;
    async fn compiled_skill_artifact(&self, name: &str) -> Result<Value>;
    async fn extension_manifests(&self) -> Result<Value>;
    async fn extension_manifest_detail(&self, name: &str) -> Result<Value>;
    async fn auth_plugins(&self) -> Result<Value>;
    async fn bind_auth_plugin(&self, request: SkillBindAuthPluginRequest) -> Result<Value>;
    async fn exchange_auth_plugin_callback(
        &self,
        provider_id: Option<&str>,
        code: &str,
        state: &str,
        redirect_uri: Option<&str>,
    ) -> Result<Value>;
    async fn voice_plugins(&self) -> Result<Value>;
    async fn bind_voice_plugin(&self, request: SkillBindVoicePluginRequest) -> Result<Value>;
    async fn prewarm_voice_plugin(
        &self,
        plugin_id: &str,
        request: SkillPrewarmVoicePluginRequest,
    ) -> Result<Value>;
    async fn invoke(&self, name: &str, request: SkillInvokeRequest) -> Result<Value>;
    async fn execute(&self, name: &str, request: SkillExecuteRequest) -> Result<Value>;
    async fn authorize_auth_plugin(
        &self,
        provider_id: &str,
        request: SkillAuthAuthorizeRequest,
    ) -> Result<Value>;
    async fn exchange_auth_plugin(
        &self,
        provider_id: &str,
        request: SkillAuthExchangeRequest,
    ) -> Result<Value>;
    async fn schedule_background(
        &self,
        name: &str,
        request: SkillScheduleBackgroundRequest,
    ) -> Result<Value>;
    async fn update(&self, name: &str) -> Result<Value>;
    async fn uninstall(&self, name: &str) -> Result<Value>;
    async fn verify(&self, name: &str) -> Result<Value>;
}

pub struct SkillControlService<S> {
    source: S,
}

impl<S> SkillControlService<S> {
    pub fn new(source: S) -> Self {
        Self { source }
    }
}

impl<S> SkillControlService<S>
where
    S: SkillControlSource,
{
    pub async fn skills(&self) -> Result<Value> {
        Ok(json!({ "skills": self.source.installed_skills().await? }))
    }

    pub async fn skill_detail(&self, name: &str) -> Result<Value> {
        let skill = self.source.installed_skill_detail(name).await?;
        let compiled = self.source.compiled_skill_detail(name).await.ok();
        let extension_manifest = self.source.extension_manifest(name).await.ok();
        let background_services = self.source.background_services(name).await.ok();
        let auth_plugins = self.source.auth_plugins_for_skill(name).await.ok();
        let voice_plugins = self.source.voice_plugins_for_skill(name).await.ok();
        Ok(json!({
            "skill": skill,
            "compiled": compiled,
            "extension_manifest": extension_manifest,
            "background_services": background_services,
            "auth_plugins": auth_plugins,
            "voice_plugins": voice_plugins,
        }))
    }

    pub async fn search(&self, request: SkillSearchRequest) -> Result<Value> {
        let Some(search) = request
            .q
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        else {
            return Err(Error::Internal(
                "query parameter `q` is required".to_string(),
            ));
        };
        self.source
            .search(
                search,
                request.category.as_deref(),
                request.sort.as_deref().unwrap_or("relevance"),
            )
            .await
    }

    pub async fn popular(&self, request: SkillSearchRequest) -> Result<Value> {
        self.source
            .popular(request.limit.unwrap_or(12).max(1))
            .await
    }

    pub async fn trending(&self, request: SkillSearchRequest) -> Result<Value> {
        self.source
            .trending(request.limit.unwrap_or(12).max(1))
            .await
    }

    pub async fn install(&self, name: &str) -> Result<Value> {
        self.source.install(name).await
    }

    pub async fn compile(&self, request: SkillCompileRequest) -> Result<Value> {
        self.source.compile(request.name.as_deref()).await
    }

    pub async fn compiled_skills(&self) -> Result<Value> {
        Ok(json!({ "compiled": self.source.compiled_skills().await? }))
    }

    pub async fn compiled_skill_detail(&self, name: &str) -> Result<Value> {
        self.source.compiled_skill_artifact(name).await
    }

    pub async fn extension_manifests(&self) -> Result<Value> {
        Ok(json!({ "extensions": self.source.extension_manifests().await? }))
    }

    pub async fn extension_manifest(&self, name: &str) -> Result<Value> {
        self.source.extension_manifest_detail(name).await
    }

    pub async fn auth_plugins(&self) -> Result<Value> {
        self.source.auth_plugins().await
    }

    pub async fn bind_auth_plugin(&self, request: SkillBindAuthPluginRequest) -> Result<Value> {
        self.source.bind_auth_plugin(request).await
    }

    pub async fn auth_callback(&self, request: SkillAuthCallbackRequest) -> Result<Value> {
        if let Some(error_code) = request.error {
            return Ok(json!({
                "error": error_code,
                "description": request.error_description,
            }));
        }

        let Some(code) = request.code.as_deref() else {
            return Err(Error::Internal(
                "query parameter `code` is required".to_string(),
            ));
        };
        let Some(state) = request.state.as_deref() else {
            return Err(Error::Internal(
                "query parameter `state` is required".to_string(),
            ));
        };

        self.source
            .exchange_auth_plugin_callback(
                request.provider_id.as_deref(),
                code,
                state,
                request.redirect_uri.as_deref(),
            )
            .await
    }

    pub async fn voice_plugins(&self) -> Result<Value> {
        self.source.voice_plugins().await
    }

    pub async fn bind_voice_plugin(&self, request: SkillBindVoicePluginRequest) -> Result<Value> {
        self.source.bind_voice_plugin(request).await
    }

    pub async fn prewarm_voice_plugin(
        &self,
        plugin_id: &str,
        request: SkillPrewarmVoicePluginRequest,
    ) -> Result<Value> {
        self.source.prewarm_voice_plugin(plugin_id, request).await
    }

    pub async fn compile_by_name(&self, name: &str) -> Result<Value> {
        self.source.compile(Some(name)).await
    }

    pub async fn invoke(&self, name: &str, request: SkillInvokeRequest) -> Result<Value> {
        self.source.invoke(name, request).await
    }

    pub async fn execute(&self, name: &str, request: SkillExecuteRequest) -> Result<Value> {
        self.source.execute(name, request).await
    }

    pub async fn auth_authorize(
        &self,
        provider_id: &str,
        request: SkillAuthAuthorizeRequest,
    ) -> Result<Value> {
        self.source
            .authorize_auth_plugin(provider_id, request)
            .await
    }

    pub async fn auth_exchange(
        &self,
        provider_id: &str,
        request: SkillAuthExchangeRequest,
    ) -> Result<Value> {
        self.source.exchange_auth_plugin(provider_id, request).await
    }

    pub async fn background_services(&self, name: &str) -> Result<Value> {
        self.source.background_services(name).await
    }

    pub async fn schedule_background(
        &self,
        name: &str,
        request: SkillScheduleBackgroundRequest,
    ) -> Result<Value> {
        self.source.schedule_background(name, request).await
    }

    pub async fn update(&self, name: &str) -> Result<Value> {
        self.source.update(name).await
    }

    pub async fn uninstall(&self, name: &str) -> Result<Value> {
        self.source.uninstall(name).await
    }

    pub async fn verify(&self, name: &str) -> Result<Value> {
        self.source.verify(name).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockSkillControlSource {
        popular_limit: Mutex<Option<usize>>,
    }

    #[async_trait]
    impl SkillControlSource for MockSkillControlSource {
        async fn installed_skills(&self) -> Result<Value> {
            Ok(json!([{ "name": "Demo Skill" }]))
        }

        async fn installed_skill_detail(&self, _name: &str) -> Result<Value> {
            Ok(json!({ "name": "Demo Skill" }))
        }

        async fn compiled_skill_detail(&self, _name: &str) -> Result<Value> {
            Ok(json!({ "manifest": { "name": "Demo Skill" } }))
        }

        async fn extension_manifest(&self, _name: &str) -> Result<Value> {
            Ok(json!({ "name": "demo-extension" }))
        }

        async fn background_services(&self, _name: &str) -> Result<Value> {
            Ok(json!({ "services": [] }))
        }

        async fn auth_plugins_for_skill(&self, _name: &str) -> Result<Value> {
            Ok(json!({ "auth_plugins": [] }))
        }

        async fn voice_plugins_for_skill(&self, _name: &str) -> Result<Value> {
            Ok(json!({ "voice_plugins": [] }))
        }

        async fn search(&self, query: &str, _category: Option<&str>, sort: &str) -> Result<Value> {
            Ok(json!({ "query": query, "sort": sort }))
        }

        async fn popular(&self, limit: usize) -> Result<Value> {
            *self.popular_limit.lock().expect("popular limit lock") = Some(limit);
            Ok(json!({ "limit": limit }))
        }

        async fn trending(&self, limit: usize) -> Result<Value> {
            Ok(json!({ "limit": limit }))
        }

        async fn install(&self, name: &str) -> Result<Value> {
            Ok(json!({ "installed": name }))
        }

        async fn compile(&self, name: Option<&str>) -> Result<Value> {
            Ok(json!({ "compiled": name }))
        }

        async fn compiled_skills(&self) -> Result<Value> {
            Ok(json!([{ "name": "Demo Skill" }]))
        }

        async fn compiled_skill_artifact(&self, name: &str) -> Result<Value> {
            Ok(json!({ "name": name }))
        }

        async fn extension_manifests(&self) -> Result<Value> {
            Ok(json!([{ "name": "demo-extension" }]))
        }

        async fn extension_manifest_detail(&self, name: &str) -> Result<Value> {
            Ok(json!({ "name": name }))
        }

        async fn auth_plugins(&self) -> Result<Value> {
            Ok(json!({ "auth_plugins": [] }))
        }

        async fn bind_auth_plugin(&self, request: SkillBindAuthPluginRequest) -> Result<Value> {
            Ok(json!({ "provider_id": request.provider_id }))
        }

        async fn exchange_auth_plugin_callback(
            &self,
            provider_id: Option<&str>,
            code: &str,
            state: &str,
            _redirect_uri: Option<&str>,
        ) -> Result<Value> {
            Ok(json!({
                "provider_id": provider_id,
                "code": code,
                "state": state,
            }))
        }

        async fn voice_plugins(&self) -> Result<Value> {
            Ok(json!({ "voice_plugins": [] }))
        }

        async fn bind_voice_plugin(&self, request: SkillBindVoicePluginRequest) -> Result<Value> {
            Ok(json!({ "plugin_id": request.plugin_id }))
        }

        async fn prewarm_voice_plugin(
            &self,
            plugin_id: &str,
            _request: SkillPrewarmVoicePluginRequest,
        ) -> Result<Value> {
            Ok(json!({ "plugin_id": plugin_id }))
        }

        async fn invoke(&self, name: &str, _request: SkillInvokeRequest) -> Result<Value> {
            Ok(json!({ "name": name }))
        }

        async fn execute(&self, name: &str, _request: SkillExecuteRequest) -> Result<Value> {
            Ok(json!({ "name": name }))
        }

        async fn authorize_auth_plugin(
            &self,
            provider_id: &str,
            _request: SkillAuthAuthorizeRequest,
        ) -> Result<Value> {
            Ok(json!({ "provider_id": provider_id }))
        }

        async fn exchange_auth_plugin(
            &self,
            provider_id: &str,
            request: SkillAuthExchangeRequest,
        ) -> Result<Value> {
            Ok(json!({ "provider_id": provider_id, "state": request.state }))
        }

        async fn schedule_background(
            &self,
            name: &str,
            _request: SkillScheduleBackgroundRequest,
        ) -> Result<Value> {
            Ok(json!({ "name": name }))
        }

        async fn update(&self, name: &str) -> Result<Value> {
            Ok(json!({ "updated": name }))
        }

        async fn uninstall(&self, name: &str) -> Result<Value> {
            Ok(json!({ "uninstalled": name }))
        }

        async fn verify(&self, name: &str) -> Result<Value> {
            Ok(json!({ "verified": name }))
        }
    }

    #[tokio::test]
    async fn skill_control_service_validates_search_and_defaults_limits() -> Result<()> {
        let service = SkillControlService::new(MockSkillControlSource::default());
        assert!(service.search(SkillSearchRequest::default()).await.is_err());

        let popular = service.popular(SkillSearchRequest::default()).await?;
        assert_eq!(popular["limit"], 12);
        Ok(())
    }

    #[tokio::test]
    async fn skill_control_service_aggregates_detail_surfaces() -> Result<()> {
        let service = SkillControlService::new(MockSkillControlSource::default());
        let detail = service.skill_detail("Demo Skill").await?;

        assert_eq!(detail["skill"]["name"], "Demo Skill");
        assert_eq!(detail["compiled"]["manifest"]["name"], "Demo Skill");
        assert_eq!(detail["extension_manifest"]["name"], "demo-extension");
        Ok(())
    }

    #[tokio::test]
    async fn skill_control_service_shapes_auth_callback_errors() -> Result<()> {
        let service = SkillControlService::new(MockSkillControlSource::default());
        let callback = service
            .auth_callback(SkillAuthCallbackRequest {
                error: Some("access_denied".to_string()),
                error_description: Some("operator denied".to_string()),
                ..Default::default()
            })
            .await?;

        assert_eq!(callback["error"], "access_denied");
        assert_eq!(callback["description"], "operator denied");
        Ok(())
    }
}
