# Requirements: OpenRustClaw

## Active Milestone: v1.12 Secure Node Connectivity and SSH Tunnel Revisit

**Goal:** Revisit OpenRustClaw's node model around a primary node-first remote path, an SSH tunnel fallback, and a reverse-proxy last-resort fallback for self-hosted deployments, with operator-visible health, failover, recovery, and documentation.

## Requirements

### NODE-01 Node and topology contract

OpenRustClaw must define one bounded node and topology model that explains how the local runtime, mobile nodes, distributed cluster nodes, SSH-tunneled remote access, and reverse-proxy fallback relate to each other, where each mode is supported, and when fallback paths act as backup rather than the primary transport.

**Acceptance signals:**
- the product has one canonical explanation of node roles instead of scattered or conflicting terminology
- local control, remote-node execution, and mobile node behavior are clearly separated
- unsupported or future node modes remain explicit rather than implied

### NODE-02 SSH tunnel bootstrap path

Operators must have one explicit supported node-first remote bootstrap path for advanced self-hosted deployments plus an SSH tunnel fallback and a bounded reverse-proxy fallback, including required config, trust boundaries, failover expectations, and the relationship between the local gateway and the remote endpoint.

**Acceptance signals:**
- onboarding or setup can describe or configure the supported node-first path intentionally
- the SSH tunnel fallback is documented or configurable as a recovery path
- the reverse-proxy fallback is documented as a clearly bounded last-resort path
- the security boundary for the node path, SSH tunnel fallback, and reverse-proxy fallback is documented and inspectable
- remote exposure no longer depends on vague "bring your own tunnel" wording alone

### NODE-03 Remote node inspection and recovery

Shipped operator surfaces must expose the health, enrollment state, failover state, tunnel state, reverse-proxy fallback state, and recovery clues for remote nodes or remote connectivity paths so operators can debug failures without stitching raw endpoints manually.

**Acceptance signals:**
- node connectivity state is available through one shipped inspection path
- operator-visible evidence distinguishes between configuration, connectivity, auth, node-path, SSH-tunnel-fallback, and reverse-proxy-fallback failures
- recovery guidance exists for reconnecting or repairing a remote node path

### NODE-04 Onboarding and docs alignment

Onboarding and documentation must explain the supported local-only, remote-node, SSH-tunneled, and reverse-proxy-fallback deployment paths truthfully, including when operators should choose standard local setup versus advanced remote connectivity and when each fallback is used instead of the preferred node path.

**Acceptance signals:**
- setup guidance clearly differentiates standard local deployment from advanced remote connectivity
- docs describe upgrade or transition paths between local and remote setups where supported
- milestone verification preserves the supported remote-connectivity contract in both docs and control surfaces

## Most Recent Archive

- Last shipped milestone: `v1.11 Crates.io and Docs.rs Publication Foundation`
- Archived requirements: `.planning/milestones/v1.11-REQUIREMENTS.md`
- Archived verification bundle: `.planning/milestones/v1.11-VERIFICATIONS.md`

## Next Step

Start execution with `$gsd-plan-phase 53` or `$gsd-autonomous`.
