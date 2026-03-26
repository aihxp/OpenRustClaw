# Local Tool Profiles

OpenRustClaw can keep a workspace-owned registry of trusted local CLIs and generate deterministic startup bundles for AI hosts from that registry instead of relying on repeated ad hoc probing.

## What It Stores

- Registry: `.claw/control/tool-profiles.json`
- Generated host bundles: `.claw/control/tool-hosts/<host>/`

Each profile is built from bounded `--version` and `--help` probing, then normalized into:

- executable name and resolved path
- version text
- summary
- detected subcommands
- detected long flags
- executable hash and modified timestamp
- generated host artifact paths

## Commands

Initialize the default tool set:

```bash
openrustclaw tools setup
```

Add one explicit tool:

```bash
openrustclaw tools add gh
openrustclaw tools add docker --path /usr/local/bin/docker --host codex
```

Inspect current state:

```bash
openrustclaw tools status
openrustclaw tools show git
```

Refresh persisted profiles after a local tool changes:

```bash
openrustclaw tools sync
openrustclaw tools sync --name cargo --apply
```

## Drift Detection

`openrustclaw tools status` marks a tool as stale when:

- the executable path is missing
- the executable hash changed since the last inspection
- generated host artifacts are missing

Use `openrustclaw tools sync --apply` to refresh the persisted profile and rewrite host bundles after a local upgrade.

## Host Bundles

The current shipped hosts are:

- `claude-code`
- `cursor`
- `codex`
- `gemini-cli`
- `github-copilot`

Each host gets:

- `<tool>.md` briefing
- `<tool>.json` normalized profile
- `STARTUP.md` aggregate bundle index

## Control API

The same registry is available over the runtime control plane:

- `GET /control/tools`
- `GET /control/tools/{name}`
- `POST /control/tools/add`
- `POST /control/tools/setup`
- `POST /control/tools/sync`

That lets future UIs or external operator clients reuse the same persisted tool model instead of inventing a separate discovery layer.
