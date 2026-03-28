# Production Deployment

This guide is the operator-facing runbook for a real self-hosted OpenRustClaw deployment. It assumes the product story from the README and getting-started docs is already understood and focuses on what matters when the runtime needs to stay stable, inspectable, and recoverable.

## Deployment Posture

Choose a posture that matches the product mode you selected during onboarding:

| Mode | Typical deployment posture |
| --- | --- |
| `solo` | one local host or private VM |
| `team` | a shared internal host with explicit operator ownership |
| `company` | a managed shared runtime with backups, logs, and change discipline |
| `enterprise` | a managed runtime with enterprise access, policy, governance, audit export, and operator-gated autonomy controls enabled where needed |

OpenRustClaw is self-hosted. There is no hosted control plane that substitutes for your runtime, storage, or operator discipline.

For advanced remote deployments, use the current connectivity order from [Remote Connectivity](./remote-connectivity.md): node-first, then SSH tunnel fallback, then reverse proxy fallback as a bounded last resort.

## Baseline Production Loop

The normal production loop is:

1. prepare the workspace and secrets
2. run onboarding for the chosen deployment mode
3. verify readiness with `doctor`
4. start the runtime
5. inspect `/control/ui`, runtime status, logs, and metrics
6. use backups, release checklist, and rollback planning before risky changes

Core commands:

```bash
openrustclaw onboard
openrustclaw doctor
openrustclaw start
openrustclaw runtime status
openrustclaw runtime backup
openrustclaw runtime restore
```

## Control Plane and Enterprise Boundaries

Production deployments should treat the control plane as a protected operator surface.

### Control auth

- use the normal control auth boundary for `/control/...`
- enable the bearer-token gate when the deployment needs explicit operator auth
- use the trusted-proxy mode only when you control the reverse proxy path end to end
- do not use a reverse proxy as the default remote-access story when a node-first or SSH tunnel path is viable

### Enterprise operator boundary

When you enable the enterprise lane, sensitive writes can require:

- `x-openrustclaw-operator-id`
- `x-openrustclaw-operator-token`

Higher-risk governed writes can also require:

- `x-openrustclaw-approver-id`
- `x-openrustclaw-approver-token`

### Full autonomy

Full autonomy is a separate operator-gated lane. It is not the default runtime posture. If you enable it, treat it as a reviewed operational override with budgets, kill switch, and audit evidence.

## What to Inspect in Production

Use `/control/ui` and the matching CLI surfaces to inspect:

- setup handoff and product mode
- runtime status and health
- sessions and assistant continuity
- tool, browser, coding, channel, and communication evidence
- enterprise access, policy, governance, audit, and autonomy surfaces when enabled

## Backups and Release Discipline

Use these before major config, version, or deployment-mode changes:

```bash
openrustclaw runtime backup
openrustclaw runtime upgrade-plan
openrustclaw runtime rollback-plan
```

Pair this guide with:

- [Release Checklist](./release-checklist.md)
- [Observability & Monitoring](../operations/observability.md)
- [Security](../guides/security.md)

## Recovery Rules

When something drifts:

1. run `openrustclaw doctor`
2. inspect runtime status and recent logs
3. use onboarding resume or repair if setup truth drifted
4. restore from backup or use rollback planning before manual surgery

The product should recover through shipped operator paths before resorting to hand-edited workspace state.
