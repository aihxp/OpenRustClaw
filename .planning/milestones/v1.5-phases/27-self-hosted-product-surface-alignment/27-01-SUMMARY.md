# Plan 27-01 Summary: Align Public Self-Hosted Product Docs

## What Changed

- Updated `README.md` to describe OpenRustClaw explicitly as a self-hosted open-source product and to document the four supported deployment paths: `solo`, `team`, `company`, and `enterprise`.
- Expanded the quick-start guidance so `openrustclaw onboard` now reads as the deployment-path selector and points operators at the shipped self-hosted mode surface for later upgrades or downgrades.
- Updated the installation and first-agent guides so they describe the same deployment-path contract and point to `/control/ui` plus `GET/POST /control/self-hosted/product-mode` instead of implying that product-mode changes happen outside the shipped operator loop.

## Why It Matters

The runtime and Control UI already had a truthful self-hosted product-mode contract. This plan closes the docs gap so the first install story, ongoing operator guidance, and shipped control surface all describe the same product.

## Verification Notes

- Manual review of `README.md`, `docs/src/getting-started/installation.md`, `docs/src/getting-started/quickstart.md`, and `docs/src/getting-started/first-agent.md`
- Keyword coverage check for self-hosted framing, deployment paths, and transition surfaces
