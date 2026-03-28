use openrustclaw_core::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnterpriseAccessBootstrapRequest {
    pub organization_id: String,
    pub organization_name: String,
    pub owner_id: String,
    #[serde(default)]
    pub owner_name: Option<String>,
    #[serde(default)]
    pub owner_email: Option<String>,
    pub owner_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnterpriseAccessOperatorUpsertRequest {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    pub role: String,
    pub token: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnterpriseGovernanceRuleUpsertRequest {
    pub scope: String,
    pub approval_mode: String,
    #[serde(default)]
    pub requester_roles: Vec<String>,
    #[serde(default)]
    pub approver_roles: Vec<String>,
    pub forbid_self_approval: bool,
    pub active: bool,
    #[serde(default)]
    pub detail: Option<String>,
}

pub trait EnterpriseAccessControlSource {
    type Report;

    fn bootstrap_enterprise_access(&self, request: EnterpriseAccessBootstrapRequest) -> Result<()>;

    fn upsert_enterprise_access_operator(
        &self,
        request: EnterpriseAccessOperatorUpsertRequest,
    ) -> Result<()>;

    fn upsert_enterprise_governance_rule(
        &self,
        request: EnterpriseGovernanceRuleUpsertRequest,
    ) -> Result<()>;

    fn load_enterprise_access_report(&self) -> Result<Self::Report>;
}

pub struct EnterpriseAccessControlService<S> {
    source: S,
}

impl<S> EnterpriseAccessControlService<S> {
    pub fn new(source: S) -> Self {
        Self { source }
    }
}

impl<S> EnterpriseAccessControlService<S>
where
    S: EnterpriseAccessControlSource,
{
    pub fn bootstrap_and_report(
        &self,
        request: EnterpriseAccessBootstrapRequest,
    ) -> Result<S::Report> {
        self.source.bootstrap_enterprise_access(request)?;
        self.source.load_enterprise_access_report()
    }

    pub fn upsert_operator_and_report(
        &self,
        request: EnterpriseAccessOperatorUpsertRequest,
    ) -> Result<S::Report> {
        self.source.upsert_enterprise_access_operator(request)?;
        self.source.load_enterprise_access_report()
    }

    pub fn upsert_governance_rule_and_report(
        &self,
        request: EnterpriseGovernanceRuleUpsertRequest,
    ) -> Result<S::Report> {
        self.source.upsert_enterprise_governance_rule(request)?;
        self.source.load_enterprise_access_report()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openrustclaw_core::error::Error;
    use std::cell::RefCell;

    #[derive(Default)]
    struct TestSource {
        operations: RefCell<Vec<String>>,
    }

    impl EnterpriseAccessControlSource for TestSource {
        type Report = String;

        fn bootstrap_enterprise_access(
            &self,
            request: EnterpriseAccessBootstrapRequest,
        ) -> Result<()> {
            self.operations.borrow_mut().push(format!(
                "bootstrap:{}:{}",
                request.organization_id, request.owner_id
            ));
            Ok(())
        }

        fn upsert_enterprise_access_operator(
            &self,
            request: EnterpriseAccessOperatorUpsertRequest,
        ) -> Result<()> {
            self.operations
                .borrow_mut()
                .push(format!("operator:{}:{}", request.id, request.role));
            Ok(())
        }

        fn upsert_enterprise_governance_rule(
            &self,
            request: EnterpriseGovernanceRuleUpsertRequest,
        ) -> Result<()> {
            self.operations.borrow_mut().push(format!(
                "governance:{}:{}",
                request.scope, request.approval_mode
            ));
            Ok(())
        }

        fn load_enterprise_access_report(&self) -> Result<Self::Report> {
            Ok(self.operations.borrow().join("|"))
        }
    }

    #[test]
    fn enterprise_access_control_bootstraps_and_returns_report() -> Result<()> {
        let service = EnterpriseAccessControlService::new(TestSource::default());
        let report = service.bootstrap_and_report(EnterpriseAccessBootstrapRequest {
            organization_id: "acme".to_string(),
            organization_name: "Acme".to_string(),
            owner_id: "owner-1".to_string(),
            owner_name: Some("Owner".to_string()),
            owner_email: None,
            owner_token: "secret".to_string(),
        })?;

        assert_eq!(report, "bootstrap:acme:owner-1");
        Ok(())
    }

    #[test]
    fn enterprise_access_control_upserts_operator_and_governance_rule() -> Result<()> {
        let service = EnterpriseAccessControlService::new(TestSource::default());
        let report = service.upsert_operator_and_report(EnterpriseAccessOperatorUpsertRequest {
            id: "admin-1".to_string(),
            name: Some("Admin".to_string()),
            email: None,
            role: "admin".to_string(),
            token: "secret".to_string(),
            scopes: vec!["enterprise.config.write".to_string()],
            active: true,
        })?;
        assert_eq!(report, "operator:admin-1:admin");

        let report =
            service.upsert_governance_rule_and_report(EnterpriseGovernanceRuleUpsertRequest {
                scope: "enterprise.config.write".to_string(),
                approval_mode: "dual".to_string(),
                requester_roles: vec!["admin".to_string()],
                approver_roles: vec!["owner".to_string()],
                forbid_self_approval: true,
                active: true,
                detail: Some("Needs separation of duties".to_string()),
            })?;
        assert_eq!(
            report,
            "operator:admin-1:admin|governance:enterprise.config.write:dual"
        );
        Ok(())
    }

    #[test]
    fn enterprise_access_control_propagates_source_errors() {
        struct FailingSource;

        impl EnterpriseAccessControlSource for FailingSource {
            type Report = String;

            fn bootstrap_enterprise_access(
                &self,
                _request: EnterpriseAccessBootstrapRequest,
            ) -> Result<()> {
                Err(Error::Internal("bootstrap failed".to_string()))
            }

            fn upsert_enterprise_access_operator(
                &self,
                _request: EnterpriseAccessOperatorUpsertRequest,
            ) -> Result<()> {
                unreachable!()
            }

            fn upsert_enterprise_governance_rule(
                &self,
                _request: EnterpriseGovernanceRuleUpsertRequest,
            ) -> Result<()> {
                unreachable!()
            }

            fn load_enterprise_access_report(&self) -> Result<Self::Report> {
                unreachable!()
            }
        }

        let service = EnterpriseAccessControlService::new(FailingSource);
        let error = service
            .bootstrap_and_report(EnterpriseAccessBootstrapRequest {
                organization_id: "acme".to_string(),
                organization_name: "Acme".to_string(),
                owner_id: "owner-1".to_string(),
                owner_name: None,
                owner_email: None,
                owner_token: "secret".to_string(),
            })
            .expect_err("bootstrap should fail");

        assert!(error.to_string().contains("bootstrap failed"));
    }
}
