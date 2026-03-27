# Security

OpenRustClaw’s security model is built around a self-hosted operator boundary. The runtime should not silently trust browser clients, reverse proxies, extension code, or high-risk autonomy changes.

## Core Security Posture

- the Rust runtime is the default production path
- control surfaces should sit behind explicit operator auth
- secrets belong in the runtime environment or vault, not in committed files
- higher-risk enterprise and autonomy writes should remain explicit and auditable

## Control Plane Protection

Protect `/control/...` with the normal control auth boundary. In deployments that need stronger isolation:

- enable the bearer-token gate
- use trusted-proxy mode only when you control the proxy and forwarded-origin behavior
- keep browser origins explicit rather than permissive

## Enterprise Operator Controls

The current enterprise baseline is intentionally narrow but real:

- enterprise access can require scoped operator headers
- governed writes can require dual approval through approver headers
- enterprise policy and audit export are operator-visible
- full autonomy is a separate gated lane with budgets and a kill switch

That is a production control boundary, not a claim that SSO, SCIM, multi-tenancy, or compliance packaging are already complete.

## Secrets and Runtime Discipline

- keep provider keys and secrets in `.env` or the runtime vault
- take a backup before major config changes
- prefer shipped runtime or control-surface mutations over direct file surgery

Useful commands:

```bash
openrustclaw doctor
openrustclaw runtime status
openrustclaw runtime backup
openrustclaw runtime rollback-plan
```

## What Security Review Should Check

- control-plane auth is enabled for the deployment posture
- setup handoff and `doctor` show no known blocking issues
- enterprise headers and governance are only enabled where needed
- full autonomy is disabled by default and reviewed before use
- observability is sufficient to explain high-risk changes after the fact

## Related Guides

- [Production Deployment](../deployment/production.md)
- [Observability & Monitoring](../operations/observability.md)
- [Release Checklist](../deployment/release-checklist.md)
- [Security Policy](../../SECURITY.md)

