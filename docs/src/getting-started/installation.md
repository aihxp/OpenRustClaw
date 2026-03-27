# Installation

This guide gets a workspace ready for the shipped OpenRustClaw setup flow. The goal is not just to compile the project. The goal is to finish with a workspace that can truthfully pass onboarding, `doctor`, and the first assistant launch.

## Before You Start

### Required

| Dependency | Recommended version | Why |
| --- | --- | --- |
| Rust | 1.85+ | primary runtime and CLI |
| `protoc` | 3.20+ | protobuf compilation |
| SQLite | 3.35+ | default local persistence |

### Optional

| Dependency | Why |
| --- | --- |
| Python 3.11+ | only for the optional compatibility sidecar |
| `mdbook` | build the docs locally |
| `cargo-watch` | local development convenience |

OpenRustClaw is self-hosted and open source. You can start in `solo`, `team`, `company`, or `enterprise` mode. You do not need to decide every detail before building; onboarding captures that choice later.

## 1. Clone and Build

```bash
git clone https://github.com/aihxp/OpenRustClaw.git
cd OpenRustClaw
cargo build --workspace
```

If you package local release artifacts for operator testing:

```bash
scripts/build-release-artifacts.sh
```

## 2. Configure the Environment

Copy the example environment file and add at least one provider key:

```bash
cp .env.example .env
```

Common starting point:

```bash
ANTHROPIC_API_KEY=...
OPENAI_API_KEY=...
OPENROUTER_API_KEY=...
OLLAMA_HOST=http://localhost:11434
```

You do not need every provider configured. The main requirement is that the deployment mode and setup depth you choose can pass the health gate for at least one usable runtime lane.

## 3. Run Onboarding

```bash
cargo run --bin openrustclaw -- onboard
```

The onboarding flow now does the real first-start work:

- asks which product mode you want: `solo`, `team`, `company`, or `enterprise`
- offers `Standard`, `Advanced`, or `Custom` setup depth
- records durable setup state for resume and repair
- validates provider, runtime, and supported channel bootstrap before it claims the workspace is ready
- ends with an explicit setup handoff instead of a vague success message

### Setup Depth

| Depth | Best for |
| --- | --- |
| `Standard` | a straightforward self-hosted install with guided defaults |
| `Advanced` | operators who want deeper configuration choices up front |
| `Custom` | operators who want the most explicit control over what gets configured |

## 4. Verify Readiness

After onboarding, run:

```bash
openrustclaw doctor
```

If the workspace is already partially configured, onboarding and `doctor` now support resume and repair instead of forcing manual file surgery.

Use these checks before you launch the runtime:

```bash
openrustclaw runtime status
openrustclaw session list
```

## 5. Start the Runtime

```bash
openrustclaw start
```

Open `/control/ui` after startup. The shipped dashboard includes:

- `Setup Handoff`
- `Self-Hosted Product Mode`
- session continuity
- runtime status
- enterprise and autonomy surfaces when relevant to the chosen deployment path

## Optional: Compatibility Sidecar

Python is no longer part of the default production path. Install and run the sidecar only if you explicitly need a bounded compatibility workflow or are developing the sidecar itself.

## Next Steps

- [Quickstart](./quickstart.md) for the first persisted assistant loop
- [First Agent](./first-agent.md) for a first useful assistant workflow
- [Production Deployment](../deployment/production.md) for operator-managed environments

