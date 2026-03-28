# Remote Connectivity

This guide defines the current OpenRustClaw contract for nodes and remote access. It is intentionally narrow: the goal is to explain what each path means today, which one is preferred, and which fallbacks are acceptable when the preferred path is unavailable.

## Connectivity Order

Use remote connectivity in this order:

1. node-first
2. SSH tunnel fallback
3. reverse proxy fallback

That order matters. OpenRustClaw should not default to "just expose the gateway somehow" when a clearer node boundary is available.

## What "Node" Means Here

OpenRustClaw already has more than one thing called a node, so this milestone narrows the terminology:

- `local runtime` means the main OpenRustClaw process running the gateway, control plane, assistant runtime, and operator APIs on the host you manage directly
- `mobile node` means a paired device-side participant with its own bounded lifecycle, receipts, and operator inspection surface
- `distributed node` means an advanced cluster member from the distributed runtime lane for multi-machine coordination and scaling
- `remote control path` means how an operator reaches the protected `/control/...` and runtime surfaces from another machine

A mobile node is not a generic replacement for the main runtime. A distributed node is not automatically the same thing as a remote access tunnel. The local runtime remains the current production anchor.

## Preferred Path: Node-First

The preferred remote story is node-first:

- keep one explicit runtime owner
- make remote execution or remote access legible through a named node role
- preserve operator-visible state and recovery signals instead of hiding transport decisions behind ad hoc network exposure

Today, this is a product-direction contract more than a fully automated bootstrap flow. OpenRustClaw already ships mobile node surfaces and a distributed crate, but the guided remote-node bootstrap story is still being hardened.

## First Fallback: SSH Tunnel

When the preferred node path is unavailable or not yet supported for a deployment, use an SSH tunnel as the first fallback.

Use this fallback when:

- you control both ends of the host relationship
- you need a durable private path to a local gateway
- you want to avoid broad public exposure while the node path is unavailable

Do not treat the SSH tunnel as the product's primary topology. It is the first recovery and compatibility fallback.

## Last Fallback: Reverse Proxy

Reverse proxy is the third-tier fallback, not the default remote path.

Use it only when:

- you fully control the proxy path end to end
- you understand the auth and origin boundary for the protected control plane
- node-first and SSH tunnel paths are not viable for the current environment

If you expose `/control/...` through a reverse proxy, keep the control auth boundary enabled and only use trusted-proxy behavior when the full proxy chain is under your control.

## Current Shipped Truth

As of `v1.12` planning:

- the default supported production posture is still one local runtime owner
- mobile nodes are shipped and inspectable
- distributed runtime components exist, but the broader distributed lane remains gated from the shipped surface
- onboarding does not yet automate a complete node-first remote bootstrap
- onboarding now records the intended remote-connectivity profile in setup state and the `Setup Handoff` surface
- SSH tunnel and reverse proxy remain operator-managed advanced paths until later phases harden the bootstrap and inspection flow

## Operator Rules

Before exposing OpenRustClaw beyond one local host:

1. decide whether you are using a node boundary or only a remote access path
2. prefer node-first if the deployment supports it
3. use SSH tunnel before reverse proxy
4. keep reverse proxy as a bounded last resort
5. preserve the control auth and origin boundary at every layer

Pair this guide with [Production Deployment](./production.md) and [Security](../guides/security.md).
