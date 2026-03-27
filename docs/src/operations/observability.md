# Observability & Monitoring

OpenRustClaw’s observability story is intentionally operator-facing. The goal is not just to emit traces and metrics. The goal is to give the runtime owner enough signal to understand whether the workspace is healthy, what ran, and what changed.

## Primary Inspection Surfaces

Start with:

- `/control/ui`
- `openrustclaw runtime status`
- recent runtime logs
- `/metrics` when Prometheus scraping is enabled

Use them to answer:

- is the runtime healthy
- is setup still truthful for this workspace
- are sessions, tools, channels, and enterprise surfaces behaving as expected
- is the runtime degraded because of provider or environment issues

## What to Watch

### Runtime health

- startup and liveness state
- provider availability and fallback posture
- restart and recovery guidance

### Operator evidence

- session continuity
- memory timeline
- recent tool and browser activity
- communication and channel receipts
- enterprise audit and autonomy events when those lanes are enabled

### Platform signals

- logs
- metrics
- trace export when configured

## Practical Workflow

When investigating a problem:

1. inspect `/control/ui`
2. run `openrustclaw runtime status`
3. review recent logs
4. check `/metrics` if this is a runtime or throughput issue
5. inspect the relevant evidence surface rather than guessing from symptoms alone

Examples:

- session problem → session list, session show, continuity view
- setup problem → setup handoff and `doctor`
- tool or browser problem → recent execution evidence
- enterprise change problem → enterprise access, policy, governance, audit, and autonomy summaries

## Related Guides

- [Production Deployment](../deployment/production.md)
- [Security](../guides/security.md)
- [Release Checklist](../deployment/release-checklist.md)

