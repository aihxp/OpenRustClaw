//! AWS authentication and credential handling.
//!
//! This module provides AWS SigV4 signing and credential chain support
//! for authenticating requests to AWS Bedrock.

use std::fmt;
use std::time::SystemTime;

use aws_sigv4::http_request::{sign, SignableBody, SignableRequest, SigningSettings};
use aws_sigv4::sign::v4;
use aws_credential_types::Credentials;
use reqwest::header::{HeaderMap, HeaderValue};
use tracing::{debug, trace};

use crate::constants::{
    BEDROCK_AGENT_RUNTIME_SERVICE_NAME, BEDROCK_AGENT_SERVICE_NAME, BEDROCK_RUNTIME_SERVICE_NAME,
    BEDROCK_SERVICE_NAME,
};
use crate::error::{BedrockError, Result};

mod credentials;

pub use credentials::{
    AwsCredentials, ContainerCredentialProvider, CredentialChain, CredentialProvider,
    EnvironmentCredentialProvider, InstanceMetadataCredentialProvider, ProfileCredentialProvider,
    StaticCredentialProvider,
};

/// AWS region.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Region {
    /// The region code (e.g., "us-east-1").
    name: String,
}

impl Region {
    /// Create a new region.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Get the region name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the partition for this region.
    pub fn partition(&self) -> &str {
        if self.name.starts_with("cn-") {
            "aws-cn"
        } else if self.name.starts_with("us-gov-") {
            "aws-us-gov"
        } else if self.name.starts_with("us-isob-") || self.name.starts_with("us-iso-") {
            "aws-iso"
        } else {
            "aws"
        }
    }
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl From<&str> for Region {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Region {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// Service type for signing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Service {
    /// Bedrock control plane.
    Bedrock,
    /// Bedrock runtime.
    BedrockRuntime,
    /// Bedrock agent.
    BedrockAgent,
    /// Bedrock agent runtime.
    BedrockAgentRuntime,
}

impl Service {
    /// Get the service name for SigV4 signing.
    pub fn as_str(&self) -> &'static str {
        match self {
            Service::Bedrock => BEDROCK_SERVICE_NAME,
            Service::BedrockRuntime => BEDROCK_RUNTIME_SERVICE_NAME,
            Service::BedrockAgent => BEDROCK_AGENT_SERVICE_NAME,
            Service::BedrockAgentRuntime => BEDROCK_AGENT_RUNTIME_SERVICE_NAME,
        }
    }
}

/// Request signer using AWS SigV4.
#[derive(Debug, Clone)]
pub struct SigV4Signer {
    region: Region,
    credentials: AwsCredentials,
}

impl SigV4Signer {
    /// Create a new SigV4 signer.
    pub fn new(region: Region, credentials: AwsCredentials) -> Self {
        Self {
            region,
            credentials,
        }
    }

    /// Sign an HTTP request.
    pub fn sign_request(
        &self,
        method: &str,
        uri: &str,
        headers: &mut HeaderMap,
        body: &[u8],
        service: Service,
    ) -> Result<()> {
        let signing_time = SystemTime::now();

        trace!(
            method = %method,
            uri = %uri,
            service = %service.as_str(),
            region = %self.region,
            "Signing request"
        );

        // Build signing parameters
        let credentials = Credentials::new(
            self.credentials.access_key_id(),
            self.credentials.secret_access_key(),
            self.credentials.session_token().map(|s| s.to_string()),
            None,
            "aws-bedrock",
        );
        let identity: aws_smithy_runtime_api::client::identity::Identity = credentials.into();
        
        let signing_params = v4::SigningParams::builder()
            .identity(&identity)
            .region(self.region.name())
            .name(service.as_str())
            .time(signing_time)
            .settings(SigningSettings::default())
            .build()
            .map_err(|e| BedrockError::SigV4 {
                message: e.to_string(),
            })?;

        // Create signable request
        let header_vec: Vec<(String, String)> = headers
            .iter()
            .filter_map(|(k, v)| {
                let key = k.as_str().to_string();
                let value = v.to_str().ok()?.to_string();
                Some((key, value))
            })
            .collect();
        
        let signable_request = SignableRequest::new(
            method,
            uri,
            header_vec.iter().map(|(k, v)| (k.as_str(), v.as_str())),
            SignableBody::Bytes(body),
        )
        .map_err(|e| BedrockError::SigV4 {
            message: e.to_string(),
        })?;

        // Sign the request
        let signing_params: aws_sigv4::http_request::SigningParams<'_> = signing_params.into();
        let signing_output = sign(signable_request, &signing_params)
            .map_err(|e| BedrockError::SigV4 {
                message: e.to_string(),
            })?;
        let signing_instructions = signing_output.output();

        // Apply signing instructions to headers
        for (name, value) in signing_instructions.headers() {
            let header_name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
                .map_err(|e| BedrockError::SigV4 {
                    message: format!("Invalid header name: {e}"),
                })?;
            let header_value = HeaderValue::from_str(value)
                .map_err(|e| BedrockError::SigV4 {
                    message: format!("Invalid header value: {e}"),
                })?;
            headers.insert(header_name, header_value);
        }

        debug!("Request signed successfully");

        Ok(())
    }
}

/// Authentication middleware for reqwest.
#[derive(Debug, Clone)]
pub struct AuthMiddleware {
    signer: SigV4Signer,
    service: Service,
}

impl AuthMiddleware {
    /// Create new authentication middleware.
    pub fn new(signer: SigV4Signer, service: Service) -> Self {
        Self { signer, service }
    }

    /// Sign a request builder.
    pub fn sign_request(
        &self,
        method: &str,
        uri: &str,
        headers: &mut HeaderMap,
        body: &[u8],
    ) -> Result<()> {
        self.signer.sign_request(method, uri, headers, body, self.service)
    }
}

/// Create the base URL for Bedrock in a region.
pub fn bedrock_endpoint(region: &Region) -> String {
    format!("https://bedrock.{}.amazonaws.com", region.name())
}

/// Create the base URL for Bedrock Runtime in a region.
pub fn bedrock_runtime_endpoint(region: &Region) -> String {
    format!("https://bedrock-runtime.{}.amazonaws.com", region.name())
}

/// Create the base URL for Bedrock Agent in a region.
pub fn bedrock_agent_endpoint(region: &Region) -> String {
    format!("https://bedrock-agent.{}.amazonaws.com", region.name())
}

/// Create the base URL for Bedrock Agent Runtime in a region.
pub fn bedrock_agent_runtime_endpoint(region: &Region) -> String {
    format!("https://bedrock-agent-runtime.{}.amazonaws.com", region.name())
}

/// Get the appropriate endpoint for a service.
pub fn service_endpoint(service: Service, region: &Region) -> String {
    match service {
        Service::Bedrock => bedrock_endpoint(region),
        Service::BedrockRuntime => bedrock_runtime_endpoint(region),
        Service::BedrockAgent => bedrock_agent_endpoint(region),
        Service::BedrockAgentRuntime => bedrock_agent_runtime_endpoint(region),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_creation() {
        let region = Region::new("us-east-1");
        assert_eq!(region.name(), "us-east-1");
        assert_eq!(region.partition(), "aws");

        let cn_region = Region::new("cn-north-1");
        assert_eq!(cn_region.partition(), "aws-cn");

        let gov_region = Region::new("us-gov-west-1");
        assert_eq!(gov_region.partition(), "aws-us-gov");
    }

    #[test]
    fn test_service_names() {
        assert_eq!(Service::Bedrock.as_str(), "bedrock");
        assert_eq!(Service::BedrockRuntime.as_str(), "bedrock-runtime");
        assert_eq!(Service::BedrockAgent.as_str(), "bedrock-agent");
        assert_eq!(Service::BedrockAgentRuntime.as_str(), "bedrock-agent-runtime");
    }

    #[test]
    fn test_endpoints() {
        let region = Region::new("us-east-1");

        assert_eq!(
            bedrock_endpoint(&region),
            "https://bedrock.us-east-1.amazonaws.com"
        );
        assert_eq!(
            bedrock_runtime_endpoint(&region),
            "https://bedrock-runtime.us-east-1.amazonaws.com"
        );
        assert_eq!(
            bedrock_agent_endpoint(&region),
            "https://bedrock-agent.us-east-1.amazonaws.com"
        );
        assert_eq!(
            bedrock_agent_runtime_endpoint(&region),
            "https://bedrock-agent-runtime.us-east-1.amazonaws.com"
        );
    }
}
