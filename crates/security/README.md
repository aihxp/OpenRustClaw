# OpenRustClaw Security

Authentication, authorization, and security utilities for OpenRustClaw.

## Features

- **SSO Integration**: SAML 2.0 and OpenID Connect support
- **JWT Handling**: Token validation and generation
- **API Key Management**: Secure key storage and rotation
- **Encryption**: Data encryption utilities
- **Audit Logging**: Security event logging

## SSO Providers

- Azure AD / Microsoft Entra ID
- Okta
- Google Workspace
- Auth0
- Custom SAML/OIDC providers

## Quick Start

### SAML Authentication

```rust
use openrustclaw_security::sso::{SamlClient, SamlConfig};

let config = SamlConfig {
    idp_sso_url: "https://idp.example.com/sso".to_string(),
    idp_cert: std::fs::read_to_string("idp.crt")?,
    sp_entity_id: "openrustclaw".to_string(),
    acs_url: "https://openrustclaw.example.com/auth/saml/callback".to_string(),
};

let client = SamlClient::new(config)?;
```

### OIDC Authentication

```rust
use openrustclaw_security::sso::{OidcClient, OidcConfig};

let config = OidcConfig {
    issuer_url: "https://accounts.google.com".to_string(),
    client_id: std::env::var("GOOGLE_CLIENT_ID")?,
    client_secret: std::env::var("GOOGLE_CLIENT_SECRET")?,
    redirect_uri: "https://openrustclaw.example.com/auth/oidc/callback".to_string(),
};

let client = OidcClient::new(config).await?;
```

## Security Features

### Rate Limiting

Configurable rate limits per user, IP, and endpoint.

### Input Validation

- Prompt injection detection
- XSS prevention
- SQL injection prevention

### Encryption

- AES-256-GCM for data at rest
- TLS 1.3 for data in transit

## License

MIT OR Apache-2.0
