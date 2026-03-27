use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::{Query, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use openrustclaw_core::config::AppConfig;
use openrustclaw_security::OriginValidator;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::commands::enterprise_access;

#[derive(Clone)]
pub(super) struct EnterpriseAccessState {
    pub(super) workspace_root: PathBuf,
}

#[derive(Clone)]
pub(super) struct ControlAuthState {
    pub(super) bearer_token: Option<Arc<String>>,
    pub(super) trusted_proxy_token: Option<Arc<String>>,
    pub(super) origin_validation: bool,
    pub(super) origin_validator: Arc<OriginValidator>,
}

pub(super) fn protect_control_router(router: Router, state: ControlAuthState) -> Router {
    router.layer(middleware::from_fn_with_state(
        state,
        control_auth_middleware,
    ))
}

pub(super) fn protect_enterprise_router(router: Router, state: EnterpriseAccessState) -> Router {
    router.layer(middleware::from_fn_with_state(
        state,
        enterprise_access_middleware,
    ))
}

pub(super) fn load_control_api_token(config: &AppConfig) -> Result<Option<Arc<String>>> {
    let Some(env_name) = config.security.control_api_token_env.as_deref() else {
        return Ok(None);
    };

    let value = std::env::var(env_name).with_context(|| {
        format!(
            "security.control_api_token_env is set to '{}' but that environment variable is missing",
            env_name
        )
    })?;
    let token = value.trim();
    if token.is_empty() {
        anyhow::bail!(
            "security.control_api_token_env resolved from '{}' but the token value is empty",
            env_name
        );
    }

    Ok(Some(Arc::new(token.to_string())))
}

pub(super) fn load_trusted_proxy_token(config: &AppConfig) -> Result<Option<Arc<String>>> {
    let Some(env_name) = config.security.trusted_proxy_token_env.as_deref() else {
        return Ok(None);
    };

    let value = std::env::var(env_name).with_context(|| {
        format!(
            "security.trusted_proxy_token_env is set to '{}' but that environment variable is missing",
            env_name
        )
    })?;
    let token = value.trim();
    if token.is_empty() {
        anyhow::bail!(
            "security.trusted_proxy_token_env resolved from '{}' but the token value is empty",
            env_name
        );
    }

    Ok(Some(Arc::new(token.to_string())))
}

fn control_request_token(req: &Request) -> Option<String> {
    let bearer = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(|value| value.to_string());
    if bearer.is_some() {
        return bearer;
    }

    Query::<HashMap<String, String>>::try_from_uri(req.uri())
        .ok()
        .and_then(|query| query.0.get("token").cloned())
}

fn control_request_proxy_token(req: &Request) -> Option<String> {
    req.headers()
        .get("x-openrustclaw-trusted-proxy-token")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
}

fn control_request_origin(req: &Request, trusted_proxy: bool) -> Option<String> {
    req.headers()
        .get(axum::http::header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
        .or_else(|| {
            trusted_proxy.then(|| {
                req.headers()
                    .get(axum::http::header::HeaderName::from_static(
                        "x-forwarded-origin",
                    ))
                    .and_then(|value| value.to_str().ok())
                    .map(|value| value.to_string())
            })?
        })
}

fn control_request_authorization_method(
    state: &ControlAuthState,
    req: &Request,
) -> Option<&'static str> {
    let bearer_expected = state.bearer_token.as_deref();
    let proxy_expected = state.trusted_proxy_token.as_deref();

    if bearer_expected.is_none() && proxy_expected.is_none() {
        return Some("open");
    }

    let provided = control_request_token(req).unwrap_or_default();
    if let Some(expected_token) = bearer_expected
        && provided == *expected_token
    {
        return Some("control_token");
    }

    let proxy_provided = control_request_proxy_token(req).unwrap_or_default();
    if let Some(expected_token) = proxy_expected
        && proxy_provided == *expected_token
    {
        return Some("trusted_proxy");
    }

    None
}

fn validate_control_request_origin(
    state: &ControlAuthState,
    req: &Request,
    auth_method: &str,
) -> Result<()> {
    if !state.origin_validation || !state.origin_validator.has_origins() {
        return Ok(());
    }

    let trusted_proxy = auth_method == "trusted_proxy";
    let Some(origin) = control_request_origin(req, trusted_proxy) else {
        if trusted_proxy {
            openrustclaw_observability::metrics::record_origin_check("denied");
            anyhow::bail!("missing trusted proxy forwarded origin");
        }
        return Ok(());
    };

    if let Err(error) = state.origin_validator.validate(&origin) {
        openrustclaw_observability::metrics::record_origin_check("denied");
        return Err(anyhow::anyhow!(error.to_string()));
    }

    openrustclaw_observability::metrics::record_origin_check("allowed");
    Ok(())
}

async fn control_auth_middleware(
    State(state): State<ControlAuthState>,
    req: Request,
    next: Next,
) -> Response {
    if let Some(method) = control_request_authorization_method(&state, &req) {
        if let Err(error) = validate_control_request_origin(&state, &req, method) {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "error": error.to_string()
                })),
            )
                .into_response();
        }
        if method != "open" {
            openrustclaw_observability::metrics::record_auth_attempt(method, "success");
        }
        return next.run(req).await;
    }

    let bearer_expected = state.bearer_token.as_deref();
    let proxy_expected = state.trusted_proxy_token.as_deref();
    let proxy_provided = control_request_proxy_token(&req).unwrap_or_default();
    if bearer_expected.is_some() {
        openrustclaw_observability::metrics::record_auth_attempt("control_token", "failure");
    }
    if proxy_expected.is_some() && !proxy_provided.is_empty() {
        openrustclaw_observability::metrics::record_auth_attempt("trusted_proxy", "failure");
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "error": "missing or invalid control api token"
        })),
    )
        .into_response()
}

async fn enterprise_access_middleware(
    State(state): State<EnterpriseAccessState>,
    req: Request,
    next: Next,
) -> Response {
    let Some(required_scope) =
        enterprise_access::protected_scope_for_request(req.method(), req.uri().path())
    else {
        return next.run(req).await;
    };

    let access_enabled =
        enterprise_access::access_is_configured(&state.workspace_root).unwrap_or(false);
    if !access_enabled {
        return next.run(req).await;
    }

    match enterprise_access::authenticate_request(
        &state.workspace_root,
        req.headers(),
        required_scope,
    ) {
        Ok(operator) => {
            let mut req = req;
            req.extensions_mut().insert(operator);
            next.run(req).await
        }
        Err(error) => (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": error.to_string(),
                "required_scope": required_scope,
            })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ControlAuthState, EnterpriseAccessState, control_request_authorization_method,
        protect_enterprise_router, validate_control_request_origin,
    };
    use crate::commands::enterprise_access;
    use axum::{
        Router,
        body::Body,
        http::StatusCode,
        routing::{post, put},
    };
    use openrustclaw_security::OriginValidator;
    use std::sync::Arc;
    use tempfile::tempdir;
    use tower::ServiceExt;

    #[test]
    fn control_auth_accepts_trusted_proxy_token() {
        let request = axum::extract::Request::builder()
            .uri("/protected")
            .header("x-openrustclaw-trusted-proxy-token", "proxy-secret")
            .body(axum::body::Body::empty())
            .unwrap();

        let method = control_request_authorization_method(
            &ControlAuthState {
                bearer_token: None,
                trusted_proxy_token: Some(Arc::new("proxy-secret".to_string())),
                origin_validation: false,
                origin_validator: Arc::new(OriginValidator::new(Vec::new())),
            },
            &request,
        );

        assert_eq!(method, Some("trusted_proxy"));
    }

    #[test]
    fn control_origin_validation_allows_configured_origin() {
        let request = axum::extract::Request::builder()
            .uri("/control/runtime")
            .header("origin", "https://console.example.com")
            .body(axum::body::Body::empty())
            .unwrap();

        let state = ControlAuthState {
            bearer_token: None,
            trusted_proxy_token: None,
            origin_validation: true,
            origin_validator: Arc::new(OriginValidator::new(vec![
                "https://console.example.com".to_string(),
            ])),
        };

        assert!(validate_control_request_origin(&state, &request, "open").is_ok());
    }

    #[test]
    fn control_origin_validation_rejects_unknown_origin() {
        let request = axum::extract::Request::builder()
            .uri("/control/runtime")
            .header("origin", "https://evil.example.com")
            .body(axum::body::Body::empty())
            .unwrap();

        let state = ControlAuthState {
            bearer_token: None,
            trusted_proxy_token: None,
            origin_validation: true,
            origin_validator: Arc::new(OriginValidator::new(vec![
                "https://console.example.com".to_string(),
            ])),
        };

        assert!(validate_control_request_origin(&state, &request, "open").is_err());
    }

    #[test]
    fn control_origin_validation_requires_forwarded_origin_for_trusted_proxy() {
        let request = axum::extract::Request::builder()
            .uri("/control/runtime")
            .header("x-openrustclaw-trusted-proxy-token", "proxy-secret")
            .body(axum::body::Body::empty())
            .unwrap();

        let state = ControlAuthState {
            bearer_token: None,
            trusted_proxy_token: Some(Arc::new("proxy-secret".to_string())),
            origin_validation: true,
            origin_validator: Arc::new(OriginValidator::new(vec![
                "https://console.example.com".to_string(),
            ])),
        };

        assert!(validate_control_request_origin(&state, &request, "trusted_proxy").is_err());
    }

    #[tokio::test]
    async fn enterprise_access_middleware_blocks_protected_route_without_operator_headers() {
        let temp = tempdir().expect("tempdir");
        enterprise_access::bootstrap_manifest(
            temp.path(),
            enterprise_access::EnterpriseAccessBootstrapRequest {
                organization_id: "acme".to_string(),
                organization_name: "Acme Ops".to_string(),
                owner_id: "owner-1".to_string(),
                owner_name: None,
                owner_email: None,
                owner_token: "owner-secret-123".to_string(),
            },
        )
        .expect("bootstrap enterprise access");

        let app = protect_enterprise_router(
            Router::new().route(
                "/control/mobile/commands/cmd-1/approve",
                post(|| async { StatusCode::OK }),
            ),
            EnterpriseAccessState {
                workspace_root: temp.path().to_path_buf(),
            },
        );

        let denied = app
            .clone()
            .oneshot(
                axum::extract::Request::builder()
                    .method("POST")
                    .uri("/control/mobile/commands/cmd-1/approve")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);

        let allowed = app
            .oneshot(
                axum::extract::Request::builder()
                    .method("POST")
                    .uri("/control/mobile/commands/cmd-1/approve")
                    .header(enterprise_access::OPERATOR_ID_HEADER, "owner-1")
                    .header(enterprise_access::OPERATOR_TOKEN_HEADER, "owner-secret-123")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(allowed.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn enterprise_access_middleware_blocks_enterprise_policy_write_without_operator_headers()
    {
        let temp = tempdir().expect("tempdir");
        enterprise_access::bootstrap_manifest(
            temp.path(),
            enterprise_access::EnterpriseAccessBootstrapRequest {
                organization_id: "acme".to_string(),
                organization_name: "Acme Ops".to_string(),
                owner_id: "owner-1".to_string(),
                owner_name: None,
                owner_email: None,
                owner_token: "owner-secret-123".to_string(),
            },
        )
        .expect("bootstrap enterprise access");
        enterprise_access::upsert_operator(
            temp.path(),
            enterprise_access::EnterpriseAccessOperatorRequest {
                id: "admin-1".to_string(),
                name: None,
                email: None,
                role: "admin".to_string(),
                token: "admin-secret-123".to_string(),
                scopes: Vec::new(),
                active: true,
            },
        )
        .expect("upsert enterprise admin");

        let app = protect_enterprise_router(
            Router::new().route("/control/enterprise/policy", put(|| async { StatusCode::OK })),
            EnterpriseAccessState {
                workspace_root: temp.path().to_path_buf(),
            },
        );

        let denied = app
            .clone()
            .oneshot(
                axum::extract::Request::builder()
                    .method("PUT")
                    .uri("/control/enterprise/policy")
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);

        let allowed = app
            .oneshot(
                axum::extract::Request::builder()
                    .method("PUT")
                    .uri("/control/enterprise/policy")
                    .header(enterprise_access::OPERATOR_ID_HEADER, "owner-1")
                    .header(enterprise_access::OPERATOR_TOKEN_HEADER, "owner-secret-123")
                    .header(enterprise_access::APPROVER_ID_HEADER, "admin-1")
                    .header(enterprise_access::APPROVER_TOKEN_HEADER, "admin-secret-123")
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(allowed.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn enterprise_access_middleware_blocks_full_autonomy_write_without_dual_approval() {
        let temp = tempdir().expect("tempdir");
        enterprise_access::bootstrap_manifest(
            temp.path(),
            enterprise_access::EnterpriseAccessBootstrapRequest {
                organization_id: "acme".to_string(),
                organization_name: "Acme Ops".to_string(),
                owner_id: "owner-1".to_string(),
                owner_name: None,
                owner_email: None,
                owner_token: "owner-secret-123".to_string(),
            },
        )
        .expect("bootstrap enterprise access");
        enterprise_access::upsert_operator(
            temp.path(),
            enterprise_access::EnterpriseAccessOperatorRequest {
                id: "admin-1".to_string(),
                name: None,
                email: None,
                role: "admin".to_string(),
                token: "admin-secret-123".to_string(),
                scopes: Vec::new(),
                active: true,
            },
        )
        .expect("upsert enterprise admin");

        let app = protect_enterprise_router(
            Router::new().route(
                "/control/enterprise/autonomy/enable",
                post(|| async { StatusCode::OK }),
            ),
            EnterpriseAccessState {
                workspace_root: temp.path().to_path_buf(),
            },
        );

        let denied = app
            .clone()
            .oneshot(
                axum::extract::Request::builder()
                    .method("POST")
                    .uri("/control/enterprise/autonomy/enable")
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);

        let operator_only = app
            .clone()
            .oneshot(
                axum::extract::Request::builder()
                    .method("POST")
                    .uri("/control/enterprise/autonomy/enable")
                    .header(enterprise_access::OPERATOR_ID_HEADER, "owner-1")
                    .header(enterprise_access::OPERATOR_TOKEN_HEADER, "owner-secret-123")
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(operator_only.status(), StatusCode::FORBIDDEN);

        let allowed = app
            .oneshot(
                axum::extract::Request::builder()
                    .method("POST")
                    .uri("/control/enterprise/autonomy/enable")
                    .header(enterprise_access::OPERATOR_ID_HEADER, "owner-1")
                    .header(enterprise_access::OPERATOR_TOKEN_HEADER, "owner-secret-123")
                    .header(enterprise_access::APPROVER_ID_HEADER, "admin-1")
                    .header(enterprise_access::APPROVER_TOKEN_HEADER, "admin-secret-123")
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(allowed.status(), StatusCode::OK);
    }
}
