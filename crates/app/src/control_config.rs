use openrustclaw_core::config::AppConfig;
use openrustclaw_core::error::{Error, Result};
use serde::Serialize;

pub trait ControlConfigSource {
    fn config_path(&self) -> String;
    fn load_effective_config(&self) -> Result<AppConfig>;
    fn write_config_with_backup(&self, config: &AppConfig) -> Result<()>;
}

#[derive(Debug, Clone, Serialize)]
pub struct ControlConfigReport {
    pub path: String,
    pub config: AppConfig,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ControlConfigMutationReport {
    pub status: String,
    pub path: String,
    pub bytes: usize,
}

pub struct ControlConfigService<S> {
    source: S,
}

impl<S> ControlConfigService<S> {
    pub fn new(source: S) -> Self {
        Self { source }
    }

    pub fn render_config(config: &AppConfig) -> Result<String> {
        toml::to_string_pretty(config)
            .map_err(|error| Error::Config(format!("Failed to render config TOML: {error}")))
    }
}

impl<S> ControlConfigService<S>
where
    S: ControlConfigSource,
{
    pub fn status(&self) -> Result<ControlConfigReport> {
        Ok(ControlConfigReport {
            path: self.source.config_path(),
            config: self.source.load_effective_config()?,
        })
    }

    pub fn validate(&self, config: &AppConfig) -> Result<ControlConfigMutationReport> {
        let rendered = Self::render_config(config)?;
        Ok(ControlConfigMutationReport {
            status: "ok".to_string(),
            path: self.source.config_path(),
            bytes: rendered.len(),
        })
    }

    pub fn update(&self, config: &AppConfig) -> Result<ControlConfigMutationReport> {
        let rendered = Self::render_config(config)?;
        self.source.write_config_with_backup(config)?;
        Ok(ControlConfigMutationReport {
            status: "ok".to_string(),
            path: self.source.config_path(),
            bytes: rendered.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct MockControlConfigSource {
        path: String,
        config: RefCell<AppConfig>,
        fail_load: bool,
        fail_write: bool,
    }

    impl ControlConfigSource for MockControlConfigSource {
        fn config_path(&self) -> String {
            self.path.clone()
        }

        fn load_effective_config(&self) -> Result<AppConfig> {
            if self.fail_load {
                return Err(Error::Internal("load failed".to_string()));
            }
            Ok(self.config.borrow().clone())
        }

        fn write_config_with_backup(&self, config: &AppConfig) -> Result<()> {
            if self.fail_write {
                return Err(Error::Internal("write failed".to_string()));
            }
            *self.config.borrow_mut() = config.clone();
            Ok(())
        }
    }

    #[test]
    fn control_config_status_reports_path_and_loaded_config() -> Result<()> {
        let service = ControlConfigService::new(MockControlConfigSource {
            path: "config/default.toml".to_string(),
            config: RefCell::new(AppConfig::default()),
            fail_load: false,
            fail_write: false,
        });

        let report = service.status()?;
        assert_eq!(report.path, "config/default.toml");
        assert_eq!(
            report.config.gateway.port,
            AppConfig::default().gateway.port
        );
        Ok(())
    }

    #[test]
    fn control_config_validate_and_update_return_stable_reports() -> Result<()> {
        let service = ControlConfigService::new(MockControlConfigSource {
            path: "config/default.toml".to_string(),
            config: RefCell::new(AppConfig::default()),
            fail_load: false,
            fail_write: false,
        });
        let mut config = AppConfig::default();
        config.security.require_auth = true;

        let validate = service.validate(&config)?;
        assert_eq!(validate.status, "ok");
        assert_eq!(validate.path, "config/default.toml");
        assert!(validate.bytes > 0);

        let update = service.update(&config)?;
        assert_eq!(update.status, "ok");
        assert_eq!(update.path, "config/default.toml");
        assert!(update.bytes > 0);
        Ok(())
    }

    #[test]
    fn control_config_service_propagates_source_errors() {
        let status_service = ControlConfigService::new(MockControlConfigSource {
            path: "config/default.toml".to_string(),
            config: RefCell::new(AppConfig::default()),
            fail_load: true,
            fail_write: false,
        });
        assert!(status_service.status().is_err());

        let update_service = ControlConfigService::new(MockControlConfigSource {
            path: "config/default.toml".to_string(),
            config: RefCell::new(AppConfig::default()),
            fail_load: false,
            fail_write: true,
        });
        assert!(update_service.update(&AppConfig::default()).is_err());
    }
}
