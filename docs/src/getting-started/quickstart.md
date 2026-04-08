# Quickstart

This guide assumes the workspace already builds and has at least one usable runtime lane available. That can be a direct provider, a local runtime, or a supported delegated local agent lane. It walks through the shortest truthful path from onboarding to a running assistant and operator-visible control surface.

## What You Will Do

1. finish or resume onboarding
2. verify first-start readiness
3. start the runtime
4. start the persisted assistant session
5. inspect the same workspace through `/control/ui`

## 1. Finish Onboarding

If you have not configured the workspace yet:

```bash
openrustclaw onboard
```

If you already started and need to continue, onboarding can resume or repair the existing setup state instead of starting from scratch.

Choose:

- a deployment mode: `solo`, `team`, `company`, or `enterprise`
- a setup depth: `Standard`, `Advanced`, or `Custom`

If you enter the advanced remote gateway guidance path, onboarding now saves the intended remote-connectivity profile so later operator surfaces can show the chosen primary path and fallback order.

## 2. Verify Readiness

```bash
openrustclaw doctor
```

Do not treat config writes as success. The current shipped setup path is only ready when onboarding and `doctor` both stop reporting blocking first-start issues.

## 3. Start the Runtime

```bash
openrustclaw start
```

The Rust runtime is the default production path. The optional Python sidecar is only for explicit compatibility workflows.

## 4. Start the Assistant

In a second terminal:

```bash
openrustclaw assistant
```

This starts the persisted assistant session for the current workspace. The session continuity contract is shared across CLI inspection and the Control UI.

Try a minimal continuity check:

```text
You: Please remember that I prefer concise status updates.
You: What do you know about my preferences?
```

Then inspect the same state directly:

```bash
openrustclaw session list
openrustclaw session show <session-id>
openrustclaw memory timeline --limit 10
```

## 5. Open the Control Surface

With the runtime still running, open:

```text
http://127.0.0.1:18789/control/ui
```

Use it to confirm:

- `Setup Handoff` reflects the current workspace state
- `Setup Handoff` shows the saved remote-connectivity path and fallback order when remote guidance was selected
- `Self-Hosted Product Mode` matches the deployment mode you selected
- session continuity and recent memory or tool activity are visible
- runtime and enterprise surfaces are available for the current workspace

## 6. Change Modes or Repair Later

You can upgrade or downgrade the deployment mode later from the shipped product-mode surface. If the workspace drifts or loses readiness, use onboarding resume or `doctor` instead of editing files blindly.

## Next Steps

- [First Agent](./first-agent.md)
- [Remote Connectivity](../deployment/remote-connectivity.md)
- [Production Deployment](../deployment/production.md)
- [Security](../guides/security.md)
- [Observability & Monitoring](../operations/observability.md)
