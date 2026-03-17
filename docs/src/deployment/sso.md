# Enterprise SSO (OIDC/SAML)

OpenRustClaw supports enterprise Single Sign-On (SSO) via **OpenID Connect (OIDC)** and **SAML 2.0**, enabling seamless integration with identity providers like Okta, Azure AD, Auth0, and more.

---

## Supported Providers

| Provider | Protocol | Features |
|----------|----------|----------|
| **Okta** | OIDC, SAML | Full support with group mapping |
| **Azure AD / Entra ID** | OIDC, SAML | Microsoft ecosystem integration |
| **Auth0** | OIDC, SAML | Flexible configuration |
| **Google Workspace** | OIDC | Google SSO |
| **Keycloak** | OIDC, SAML | Self-hosted option |
| **OneLogin** | OIDC, SAML | Enterprise focus |
| **Ping Identity** | OIDC, SAML | Enterprise focus |

---

## Quick Start

### 1. Configure OIDC

Add to your configuration:

```toml
[sso]
enabled = true
primary_provider = "oidc"
auto_provision = true
default_role = "user"
allowed_domains = ["company.com"]

[[sso.oidc]]
name = "okta"
client_id = "your-client-id"
client_secret = "your-client-secret"
issuer = "https://company.okta.com"
scopes = ["openid", "email", "profile", "groups"]
```

### 2. Configure SAML

```toml
[[sso.saml]]
name = "azure-ad"
sp_entity_id = "https://openrustclaw.company.com"
idp_entity_id = "https://sts.windows.net/{tenant-id}/"
idp_sso_url = "https://login.microsoftonline.com/{tenant-id}/saml2"
idp_certificate = """-----BEGIN CERTIFICATE-----
MIIDXTCCAkWgAwIBAgIJAJC1HiIAZAiUMA0GCSqGSIb3Q...
-----END CERTIFICATE-----"""
sp_acs_url = "https://openrustclaw.company.com/auth/saml/acs"
```

### 3. Environment Variables

```bash
export SSO_ENABLED=true
export SSO_OIDC_OKTA_CLIENT_ID=xxx
export SSO_OIDC_OKTA_CLIENT_SECRET=xxx
export SSO_OIDC_OKTA_ISSUER=https://company.okta.com
```

---

## OIDC Configuration

### Okta

1. Create an OIDC app in Okta Admin Console
2. Set redirect URI: `https://yourdomain.com/auth/callback`
3. Grant scopes: `openid`, `email`, `profile`, `groups`
4. Copy Client ID and Secret

```toml
[[sso.oidc]]
name = "okta"
client_id = "0oabc123..."
client_secret = "super-secret"
issuer = "https://company.okta.com"
scopes = ["openid", "email", "profile", "groups"]
```

### Azure AD

1. Register application in Azure Portal
2. Add redirect URI: `https://yourdomain.com/auth/callback`
3. Enable ID tokens
4. Add Microsoft Graph permissions: `openid`, `email`, `profile`

```toml
[[sso.oidc]]
name = "azure-ad"
client_id = "12345678-1234-1234-1234-123456789012"
client_secret = "client-secret"
issuer = "https://login.microsoftonline.com/{tenant-id}/v2.0"
scopes = ["openid", "email", "profile"]
```

### Auth0

```toml
[[sso.oidc]]
name = "auth0"
client_id = "..."
client_secret = "..."
issuer = "https://company.auth0.com"
scopes = ["openid", "email", "profile"]
```

---

## SAML Configuration

### Azure AD SAML

1. Create Enterprise Application in Azure AD
2. Configure SAML SSO:
   - **Identifier (Entity ID)**: `https://openrustclaw.company.com`
   - **Reply URL (ACS)**: `https://openrustclaw.company.com/auth/saml/acs`
   - **Sign on URL**: `https://openrustclaw.company.com/login`

3. Download SAML signing certificate
4. Copy Login URL and Azure AD Identifier

```toml
[[sso.saml]]
name = "azure-ad"
sp_entity_id = "https://openrustclaw.company.com"
idp_entity_id = "https://sts.windows.net/{tenant-id}/"
idp_sso_url = "https://login.microsoftonline.com/{tenant-id}/saml2"
idp_certificate = """-----BEGIN CERTIFICATE-----
...
-----END CERTIFICATE-----"""
sp_acs_url = "https://openrustclaw.company.com/auth/saml/acs"
```

### Okta SAML

```toml
[[sso.saml]]
name = "okta-saml"
sp_entity_id = "openrustclaw"
idp_entity_id = "http://www.okta.com/{id}"
idp_sso_url = "https://company.okta.com/app/.../sso/saml"
idp_certificate = """-----BEGIN CERTIFICATE-----
...
-----END CERTIFICATE-----"""
sp_acs_url = "https://openrustclaw.company.com/auth/saml/acs"
```

---

## Attribute Mapping

Map SSO attributes to OpenRustClaw user fields:

```toml
[sso.attribute_mapping]
email = "email"
name = "name"
groups = "groups"
organization = "org_name"
department = "department"
```

### Common Attribute Names

| Provider | Email | Name | Groups |
|----------|-------|------|--------|
| **Okta** | `email` | `name` | `groups` |
| **Azure AD** | `http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress` | `http://schemas.xmlsoap.org/ws/2005/05/identity/claims/name` | `http://schemas.xmlsoap.org/claims/Group` |
| **Auth0** | `email` | `name` | `groups` |

---

## Group/Roles Mapping

Map SSO groups to OpenRustClaw roles:

```toml
[sso.role_mapping]
"openrustclaw-admins" = "admin"
"openrustclaw-users" = "user"
"openrustclaw-readonly" = "readonly"
```

---

## Authentication Flow

### 1. User Initiates Login

```bash
# Redirect to SSO provider
GET /auth/login?provider=okta
```

### 2. Provider Authenticates User

User authenticates with their identity provider.

### 3. Callback Handling

**OIDC:**
```bash
GET /auth/callback?code=xxx&state=yyy
```

**SAML:**
```bash
POST /auth/saml/acs
SAMLResponse=xxx&RelayState=yyy
```

### 4. Session Creation

OpenRustClaw creates a session and issues JWT.

---

## API Integration

### Get SSO Login URL

```bash
GET /api/v1/auth/sso/login?provider=okta

Response:
{
  "login_url": "https://company.okta.com/oauth2/...",
  "state": "random-state-string"
}
```

### Handle Callback (Server-side)

```rust
use openrustclaw_security::sso::{OidcClient, OidcConfig};

let config = OidcConfig::new(
    "okta",
    client_id,
    client_secret,
    "https://company.okta.com",
);

let mut client = OidcClient::new(config);
client.init().await?;

// Exchange code for tokens
let tokens = client.exchange_code(code, redirect_uri).await?;

// Get user info
let user_info = client.validate_token(&tokens.access_token).await?;
```

---

## Security Considerations

### 1. HTTPS Only

Always use HTTPS in production. SSO will fail on HTTP.

### 2. State Parameter

Always validate the `state` parameter to prevent CSRF attacks.

### 3. Token Validation

OIDC tokens are validated against JWKS. SAML assertions are signed and validated.

### 4. Session Management

```toml
[sso.session]
jwt_expiry = 3600  # 1 hour
refresh_token_expiry = 86400  # 24 hours
idle_timeout = 1800  # 30 minutes
```

### 5. Domain Restrictions

Restrict SSO to allowed domains:

```toml
[sso]
allowed_domains = ["company.com", "subsidiary.com"]
```

---

## Troubleshooting

### OIDC Issues

**"Invalid client"**
- Check client_id and client_secret
- Verify redirect URI matches exactly

**"Invalid issuer"**
- Ensure issuer URL has trailing slash if required
- Check for `/oauth2` vs `/oauth2/default`

**"Token validation failed"**
- Verify system clock is synchronized (NTP)
- Check JWKS endpoint is accessible

### SAML Issues

**"SAML response parsing failed"**
- Ensure certificate is in PEM format
- Check for proper line breaks in certificate

**"Signature validation failed"**
- Verify IdP certificate hasn't expired
- Check certificate chain if applicable

**"Audience restriction invalid"**
- Ensure SP entity ID matches exactly
- Check for http vs https

---

## Advanced Configuration

### Multiple Providers

```toml
[[sso.oidc]]
name = "okta"
# ... Okta config

[[sso.oidc]]
name = "google"
# ... Google config

[[sso.saml]]
name = "azure-ad"
# ... Azure AD config
```

Users can choose provider at login:

```bash
GET /auth/login?provider=okta
GET /auth/login?provider=google
GET /auth/login?provider=azure-ad
```

### Just-In-Time Provisioning

```toml
[sso]
auto_provision = true
default_role = "user"
```

New users are automatically created on first login.

### Manual User Linking

```bash
# Link existing user to SSO identity
POST /api/v1/users/{id}/link-sso
{
  "provider": "okta",
  "external_id": "00u123..."
}
```

---

## Migration from Password Auth

1. Enable SSO alongside password auth
2. Users can link accounts on first SSO login
3. Gradually disable password auth:

```toml
[auth]
password_auth = false  # Disable after migration
sso_required = true    # Require SSO
```

---

## Testing

### Test OIDC Locally

Use ngrok for local development:

```bash
ngrok http 8080
# Use https://xxx.ngrok.io/auth/callback as redirect URI
```

### Test SAML

Use SAML test tools:
- [SAMLTool](https://www.samltool.com/)
- OneLogin SAML Toolkit

---

## References

- [OpenID Connect Core 1.0](https://openid.net/specs/openid-connect-core-1_0.html)
- [SAML 2.0 Technical Overview](http://docs.oasis-open.org/security/saml/Post2.0/sstc-saml-tech-overview-2.0.html)
- [Okta OIDC Guide](https://developer.okta.com/docs/concepts/oauth-openid/)
- [Azure AD OIDC](https://docs.microsoft.com/en-us/azure/active-directory/develop/v2-protocols-oidc)
