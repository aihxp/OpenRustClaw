# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability, please report it responsibly:

1. **Do not** open a public issue
2. Email security@openrustclaw.dev with:
   - A description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

We will acknowledge receipt within 48 hours and provide a timeline for a fix within 7 days.

## Security Features

### Authentication

- **JWT Token Validation**: All incoming webhook requests from Microsoft Teams are validated using Microsoft's OpenID Connect public keys
- **API Key Management**: Secure API key storage with support for environment variables and secret management systems
- **Session Isolation**: Per-session authentication with configurable timeouts
- **SSO Support**: SAML 2.0 and OpenID Connect integration for enterprise authentication

### Authorization

- **Role-Based Access Control (RBAC)**: Fine-grained permissions for users and workspaces
- **Workspace Isolation**: Complete separation between different workspaces
- **Channel-Level Permissions**: Control which users can access specific channels

### Input Validation

- **Prompt Injection Detection**: Automated scanning for known prompt injection patterns
- **Input Sanitization**: All user inputs are sanitized before processing
- **Selective Rate Limiting**: Webhook and session rate limit components exist, but not every endpoint is rate-limited uniformly
- **Origin Validation**: WebSocket origins are validated before upgrade when enabled

### Data Protection

- **Deployment-Provided Transport Security**: TLS should be terminated by your reverse proxy or service mesh
- **Session Isolation**: Workspace and session boundaries are enforced in application logic
- **Audit Logging**: Comprehensive audit logs for security events

### Skill Security

- **Ed25519 Signature Verification**: Signed external skills can be verified when a public key is configured
- **WASM Sandbox Executor**: Planned, but not yet implemented
- **Capability Metadata**: Skills declare required capabilities for policy checks
- **Resource Limit Targets**: CPU, memory, and execution limits are design targets for the future executor

### Network Security

- **Origin Validation**: Configurable origin validation for WebSocket endpoints
- **CORS Configuration**: CORS handling for the current HTTP surface
- **SSRF Defenses**: SSO-related networking paths reject private IP targets

## Security Hardening

### Deployment Recommendations

1. **Run as Non-Root**: Always run OpenRustClaw as a non-root user
2. **Network Segmentation**: Place the gateway in a DMZ
3. **Database Security**: Use connection encryption and strong authentication
4. **Secrets Management**: Use a dedicated secrets manager (e.g., HashiCorp Vault, AWS Secrets Manager)
5. **Regular Updates**: Keep all dependencies up to date

### Configuration Security

```yaml
# Example secure configuration
security:
  # Require authentication for all endpoints
  require_auth: true
  
  # Strict rate limiting
  rate_limit:
    requests_per_minute: 60
    burst_size: 10
  
  # Content security policy
  csp:
    default_src: "'self'"
    script_src: "'self'"
    style_src: "'self'"
  
  # Audit logging
  audit_log:
    enabled: true
    events:
      - authentication
      - authorization
      - data_access
      - skill_execution
```

## Known Security Considerations

### Current Limitations

1. **Development Mode**: Some features have relaxed security in development mode
2. **Self-Signed Certificates**: Not recommended for production use
3. **Skill Marketplace**: Skills are community-contributed; review before installation

### Security Roadmap

- [ ] Automated security scanning in CI/CD
- [ ] Penetration testing (quarterly)
- [ ] Bug bounty program
- [ ] SOC 2 compliance
- [ ] FIPS 140-2 compliance option

## Security Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Client Layer                          │
│           (Web, Mobile, CLI, Third-party bots)              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼ Deployment TLS boundary
┌─────────────────────────────────────────────────────────────┐
│                      Gateway Layer                           │
│    ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│    │ Rate Limiter │  │ Auth Middleware│ │ Input Filter │    │
│    └──────────────┘  └──────────────┘  └──────────────┘    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼ In-process / internal transport
┌─────────────────────────────────────────────────────────────┐
│                      Core Services                           │
│    ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│    │   Agent      │  │   Memory     │  │   Skills     │    │
│    │   Runtime    │  │   Engine     │  │   Sandbox    │    │
│    └──────────────┘  └──────────────┘  └──────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## Vulnerability Disclosure Timeline

1. **Day 0**: Vulnerability reported
2. **Day 2**: Acknowledgment sent to reporter
3. **Day 7**: Fix developed and tested
4. **Day 14**: Security patch released
5. **Day 21**: Public disclosure (if applicable)

## Security Credits

We thank the following security researchers for their contributions:

- *None yet - be the first!*

## Additional Resources

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Rust Security Guidelines](https://rust-lang.github.io/rust-clippy/master/index.html#correctness)
- [Bot Framework Security](https://docs.microsoft.com/en-us/azure/bot-service/bot-builder-security)

---

**Last Updated**: 2026-03-17

**Security Contact**: security@openrustclaw.dev

**PGP Key**: [Available upon request]
