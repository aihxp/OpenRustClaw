//! SAML 2.0 SSO implementation

use super::{SsoClient, SsoError, SsoMetadata, SsoTokens, SsoUserInfo};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// SAML configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlConfig {
    /// Provider name
    pub name: String,
    /// Service Provider entity ID
    pub sp_entity_id: String,
    /// Identity Provider entity ID
    pub idp_entity_id: String,
    /// Identity Provider SSO URL
    pub idp_sso_url: String,
    /// Identity Provider certificate (PEM format)
    pub idp_certificate: String,
    /// Service Provider ACS URL
    pub sp_acs_url: String,
    /// Service Provider certificate (PEM format, optional)
    pub sp_certificate: Option<String>,
    /// Service Provider private key (PEM format, optional)
    pub sp_private_key: Option<String>,
    /// Signature algorithm
    pub signature_algorithm: String,
    /// Digest algorithm
    pub digest_algorithm: String,
    /// Request signed
    pub want_requests_signed: bool,
    /// Assertions signed
    pub want_assertions_signed: bool,
    /// Name ID format
    pub name_id_format: String,
    /// Attribute mappings
    pub attribute_mappings: HashMap<String, String>,
}

impl SamlConfig {
    /// Create a new SAML configuration
    pub fn new(
        name: impl Into<String>,
        sp_entity_id: impl Into<String>,
        idp_entity_id: impl Into<String>,
        idp_sso_url: impl Into<String>,
        idp_certificate: impl Into<String>,
        sp_acs_url: impl Into<String>,
    ) -> Self {
        let mut attribute_mappings = HashMap::new();
        attribute_mappings.insert("email".to_string(), "email".to_string());
        attribute_mappings.insert("name".to_string(), "name".to_string());
        attribute_mappings.insert("groups".to_string(), "groups".to_string());

        Self {
            name: name.into(),
            sp_entity_id: sp_entity_id.into(),
            idp_entity_id: idp_entity_id.into(),
            idp_sso_url: idp_sso_url.into(),
            idp_certificate: idp_certificate.into(),
            sp_acs_url: sp_acs_url.into(),
            sp_certificate: None,
            sp_private_key: None,
            signature_algorithm: "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256".to_string(),
            digest_algorithm: "http://www.w3.org/2001/04/xmlenc#sha256".to_string(),
            want_requests_signed: false,
            want_assertions_signed: true,
            name_id_format: "urn:oasis:names:tc:SAML:2.0:nameid-format:persistent".to_string(),
            attribute_mappings,
        }
    }
}

/// SAML AuthnRequest
#[derive(Debug, Clone)]
pub struct AuthnRequest {
    pub id: String,
    pub issue_instant: String,
    pub destination: String,
    pub issuer: String,
    pub acs_url: String,
    pub name_id_format: String,
}

impl AuthnRequest {
    /// Convert to XML
    pub fn to_xml(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<samlp:AuthnRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol"
                    xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion"
                    ID="{}"
                    Version="2.0"
                    IssueInstant="{}"
                    Destination="{}"
                    ProtocolBinding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"
                    AssertionConsumerServiceURL="{}">
    <saml:Issuer>{}</saml:Issuer>
    <samlp:NameIDPolicy Format="{}" AllowCreate="true"/>
</samlp:AuthnRequest>"#,
            self.id,
            self.issue_instant,
            self.destination,
            self.acs_url,
            self.issuer,
            self.name_id_format
        )
    }

    /// Deflate and base64 encode
    pub fn deflate_and_encode(&self) -> Result<String, SsoError> {
        use flate2::write::DeflateEncoder;
        use flate2::Compression;
        use std::io::Write;

        let xml = self.to_xml();
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(xml.as_bytes())
            .map_err(|e| SsoError::SamlError(format!("Compression failed: {}", e)))?;
        let compressed = encoder.finish()
            .map_err(|e| SsoError::SamlError(format!("Compression failed: {}", e)))?;
        
        Ok(BASE64.encode(&compressed))
    }
}

/// SAML Response
#[derive(Debug, Clone, Deserialize)]
pub struct SamlResponse {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "InResponseTo")]
    pub in_response_to: String,
    #[serde(rename = "IssueInstant")]
    pub issue_instant: String,
    pub issuer: String,
    pub status: Status,
    pub assertion: Option<Assertion>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Status {
    #[serde(rename = "StatusCode")]
    pub status_code: StatusCode,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StatusCode {
    #[serde(rename = "Value")]
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Assertion {
    #[serde(rename = "ID")]
    pub id: String,
    pub issuer: String,
    pub subject: Subject,
    pub conditions: Conditions,
    #[serde(rename = "AttributeStatement")]
    pub attribute_statement: Option<AttributeStatement>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Subject {
    #[serde(rename = "NameID")]
    pub name_id: NameID,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NameID {
    #[serde(rename = "$value")]
    pub value: String,
    #[serde(rename = "Format")]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Conditions {
    #[serde(rename = "NotBefore")]
    pub not_before: String,
    #[serde(rename = "NotOnOrAfter")]
    pub not_on_or_after: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AttributeStatement {
    #[serde(rename = "Attribute", default)]
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Attribute {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "AttributeValue", default)]
    pub values: Vec<AttributeValue>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AttributeValue {
    #[serde(rename = "$value")]
    pub value: String,
}

/// SAML client
pub struct SamlClient {
    config: SamlConfig,
    metadata: SsoMetadata,
}

impl SamlClient {
    /// Create a new SAML client
    pub fn new(config: SamlConfig) -> Self {
        let metadata = SsoMetadata {
            provider: super::SsoProvider::Saml,
            issuer: config.idp_entity_id.clone(),
            authorization_endpoint: config.idp_sso_url.clone(),
            token_endpoint: String::new(),
            userinfo_endpoint: None,
            jwks_uri: None,
            end_session_endpoint: None,
            scopes_supported: vec![],
            claims_supported: vec![
                "email".to_string(),
                "name".to_string(),
                "groups".to_string(),
            ],
        };

        Self { config, metadata }
    }

    /// Parse SAML response
    pub fn parse_response(&self, encoded_response: &str) -> Result<SamlResponse, SsoError> {
        let decoded = BASE64.decode(encoded_response)
            .map_err(|e| SsoError::SamlError(format!("Base64 decode failed: {}", e)))?;
        
        let xml = String::from_utf8(decoded)
            .map_err(|e| SsoError::SamlError(format!("Invalid UTF-8: {}", e)))?;

        // Parse XML to SamlResponse
        // This is a simplified implementation
        // In production, use a proper SAML library like `samael`
        
        // For now, return an error indicating this needs a full SAML library
        Err(SsoError::SamlError(
            "SAML response parsing requires the 'samael' feature. Use OIDC for now.".to_string()
        ))
    }

    /// Validate SAML response signature
    fn validate_signature(&self, _response: &str) -> Result<(), SsoError> {
        // Validate XML signature against IdP certificate
        // This requires xmlsec or similar
        Ok(())
    }

    /// Extract user info from assertion
    fn extract_user_info(&self, assertion: &Assertion) -> SsoUserInfo {
        let mut email = None;
        let mut name = None;
        let mut given_name = None;
        let mut family_name = None;
        let mut groups = vec![];
        let mut organization = None;
        let mut department = None;
        let mut extra_claims = HashMap::new();

        if let Some(attr_stmt) = &assertion.attribute_statement {
            for attr in &attr_stmt.attributes {
                let attr_name = attr.name.as_str();
                let values: Vec<String> = attr.values.iter().map(|v| v.value.clone()).collect();
                
                if let Some(first_value) = values.first() {
                    match attr_name {
                        "email" | "mail" | "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress" => {
                            email = Some(first_value.clone());
                        }
                        "name" | "displayName" | "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/name" => {
                            name = Some(first_value.clone());
                        }
                        "givenName" | "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/givenname" => {
                            given_name = Some(first_value.clone());
                        }
                        "surname" | "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/surname" => {
                            family_name = Some(first_value.clone());
                        }
                        "groups" | "memberOf" | "http://schemas.xmlsoap.org/claims/Group" => {
                            groups = values;
                        }
                        "organization" | "org_name" => {
                            organization = Some(first_value.clone());
                        }
                        "department" | "departmentName" => {
                            department = Some(first_value.clone());
                        }
                        _ => {
                            extra_claims.insert(attr_name.to_string(), serde_json::json!(values));
                        }
                    }
                }
            }
        }

        SsoUserInfo {
            sub: assertion.subject.name_id.value.clone(),
            email,
            email_verified: None,
            name,
            given_name,
            family_name,
            preferred_username: None,
            groups,
            organization,
            department,
            extra_claims,
        }
    }

    /// Generate Service Provider metadata
    pub fn generate_metadata(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<EntityDescriptor xmlns="urn:oasis:names:tc:SAML:2.0:metadata"
                  entityID="{}">
    <SPSSODescriptor protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol"
                     WantAssertionsSigned="{}"
                     AuthnRequestsSigned="{}">
        <NameIDFormat>{}</NameIDFormat>
        <AssertionConsumerService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"
                                  Location="{}"
                                  index="1"
                                  isDefault="true"/>
        {}
    </SPSSODescriptor>
</EntityDescriptor>"#,
            self.config.sp_entity_id,
            self.config.want_assertions_signed,
            self.config.want_requests_signed,
            self.config.name_id_format,
            self.config.sp_acs_url,
            if self.config.sp_certificate.is_some() {
                format!(
                    r#"<KeyDescriptor use="signing">
            <KeyInfo xmlns="http://www.w3.org/2000/09/xmldsig#">
                <X509Data>
                    <X509Certificate>{}</X509Certificate>
                </X509Data>
            </KeyInfo>
        </KeyDescriptor>"#,
                    self.config.sp_certificate.as_ref().unwrap()
                )
            } else {
                String::new()
            }
        )
    }
}

impl SamlClient {
    /// Generate SAML AuthnRequest form data
    pub fn generate_authn_request_form(&self, relay_state: &str) -> Result<HashMap<String, String>, SsoError> {
        let request = AuthnRequest {
            id: format!("_{}", uuid::Uuid::new_v4().to_string().replace("-", "")),
            issue_instant: chrono::Utc::now().to_rfc3339(),
            destination: self.config.idp_sso_url.clone(),
            issuer: self.config.sp_entity_id.clone(),
            acs_url: self.config.sp_acs_url.clone(),
            name_id_format: self.config.name_id_format.clone(),
        };

        let saml_request = request.deflate_and_encode()?;

        let mut form = HashMap::new();
        form.insert("SAMLRequest".to_string(), saml_request);
        form.insert("RelayState".to_string(), relay_state.to_string());

        Ok(form)
    }
}

#[async_trait]
impl SsoClient for SamlClient {
    async fn init(&mut self) -> Result<(), SsoError> {
        // Validate configuration
        if self.config.idp_certificate.is_empty() {
            return Err(SsoError::InvalidConfig("IdP certificate is required".to_string()));
        }

        Ok(())
    }

    fn authorization_url(&self, _state: &str, _redirect_uri: &str) -> String {
        // SAML uses POST binding, return the IdP URL
        // The actual AuthnRequest is sent via POST form
        self.config.idp_sso_url.clone()
    }

    async fn exchange_code(&self, _code: &str, _redirect_uri: &str) -> Result<SsoTokens, SsoError> {
        // SAML doesn't use authorization code flow
        // The response is received directly at ACS
        Err(SsoError::SamlError(
            "SAML uses direct POST response, not authorization code flow".to_string()
        ))
    }

    async fn validate_token(&self, token: &str) -> Result<SsoUserInfo, SsoError> {
        // In SAML, the "token" is the SAMLResponse
        let response = self.parse_response(token)?;
        
        if response.status.status_code.value != "urn:oasis:names:tc:SAML:2.0:status:Success" {
            return Err(SsoError::AuthenticationFailed(
                format!("SAML authentication failed: {}", response.status.status_code.value)
            ));
        }

        let assertion = response.assertion
            .ok_or_else(|| SsoError::SamlError("No assertion in response".to_string()))?;

        Ok(self.extract_user_info(&assertion))
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<SsoTokens, SsoError> {
        // SAML doesn't support token refresh
        Err(SsoError::SamlError("SAML doesn't support token refresh".to_string()))
    }

    async fn logout(&self, _token: &str) -> Result<(), SsoError> {
        // SAML single logout would be implemented here
        Ok(())
    }

    fn metadata(&self) -> &SsoMetadata {
        &self.metadata
    }
}
