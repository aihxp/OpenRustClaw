# First Agent

This guide is about your first useful assistant workflow, not a speculative from-scratch framework extension. By the time you start here, the workspace should already pass onboarding and `doctor`.

## Goal

Get one assistant workflow running end to end with:

- a configured provider or delegated lane
- a persisted assistant session
- memory continuity
- operator visibility in `/control/ui`

## 1. Confirm the Workspace State

Make sure these already work:

```bash
openrustclaw doctor
openrustclaw start
openrustclaw assistant
```

If setup is still blocked or degraded, return to [Installation](./installation.md) or [Quickstart](./quickstart.md) first.

## 2. Establish the First Working Pattern

Use the assistant for one narrow job first. Good starting patterns are:

- coding support in a local repo
- operator triage and note capture
- inbox or communication follow-up with visible audit trails
- browser-assisted research with bounded operator review

Keep the first workflow small enough that you can verify it through the shipped inspection surfaces.

## 3. Exercise Continuity

In the assistant session, set one explicit durable preference or fact:

```text
You: Please remember that I want concise review notes and explicit risk callouts.
```

Then verify the continuity path:

```bash
openrustclaw session list
openrustclaw session show <session-id>
openrustclaw memory timeline --limit 10
```

You should be able to confirm:

- the session is persisted
- memory writes include policy-aware evidence
- the same state is inspectable outside the live chat

## 4. Use the Control Surface

Open `/control/ui` and review:

- `Sessions`
- `Setup Handoff`
- `Self-Hosted Product Mode`
- recent tool, browser, or communication activity if your workflow touched those lanes

The point of the first-agent path is not just to get a response from a model. It is to confirm that the assistant workflow is visible, durable, and operator-reviewable.

## 5. Expand Carefully

Once the first workflow works, deepen it through the shipped guides:

- [Configuring Providers](../guides/providers.md)
- [Memory System](../guides/memory.md)
- [Local Tool Profiles](../guides/tools.md)
- [Creating Skills](../guides/skills.md)
- [Connecting MCP Servers](../guides/mcp-servers.md)
- [Security](../guides/security.md)

If you need production rollout next, continue to [Production Deployment](../deployment/production.md).
