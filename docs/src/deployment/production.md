# Production Deployment Guide

This guide covers deploying OpenRustClaw in production environments with high availability, security, and observability.

For final MVP sign-off, pair this guide with [Release Checklist](./release-checklist.md) so the deploy-run-recover path and the release gate stay aligned.

For the current enterprise-readiness baseline, also review `/control/enterprise/foundations` in the running control plane. That summary is intentionally narrow: it shows the explicit runtime approval policy, the browser external-backend allowlist and audit log, mobile command approval-state metrics, and recent durable audit evidence for approval-sensitive actions. It is a foundation for future enterprise work, not a claim that RBAC, SSO, or compliance packaging are already complete.

If you want the enterprise access boundary active, bootstrap `/control/enterprise/access/bootstrap` first, then provision additional operators through `/control/enterprise/access/operators`. After bootstrap, the initial sensitive-route contract requires both the regular control-plane auth boundary and scoped enterprise operator headers:

- `x-openrustclaw-operator-id`
- `x-openrustclaw-operator-token`

The shipped `/control/enterprise/access` summary and matching Control UI panel show which routes are protected by that scoped operator boundary. This remains a file-backed enterprise access foundation rather than full SSO, SCIM, or compliance-grade IAM.

Phase 17 adds a second enterprise operator loop on top of that identity boundary:

- `GET /control/enterprise/policy` shows the current approval policy, browser backend policy, mobile command approval defaults, and audit-export settings from one typed surface.
- `PUT /control/enterprise/policy` lets a scoped operator update those controls without manually editing both runtime YAML and runtime TOML files.
- `POST /control/enterprise/audit/export` writes a durable JSON bundle under `.claw/control/enterprise/exports/` with the current policy summary, recent enterprise audit evidence, and recent operator tool history.

Treat this as an operator-managed policy and evidence layer. It improves reviewability and handoff, but it is still not a replacement for full compliance packaging, external GRC systems, or enterprise IAM products.

Phase 20 deepens that loop into explicit enterprise governance:

- `/control/enterprise/access` and `/control/enterprise/admin` now include typed governance rules per protected scope.
- `POST /control/enterprise/governance/rules` updates one governance rule at a time from the shipped operator surface.
- Governed dual-approval scopes require the regular operator headers plus the optional second-approver headers:
  - `x-openrustclaw-approver-id`
  - `x-openrustclaw-approver-token`
- The shipped `/control/ui` `Enterprise Admin` panel now stores both requester and approver headers locally so protected writes can satisfy the stronger governance contract without dropping to raw curl calls.

Treat that as an operator-gated governance baseline. It gives OpenRustClaw explicit approval-chain and separation-of-duties behavior for higher-risk enterprise writes, but it is still not a replacement for external approval systems, enterprise IAM suites, or compliance programs.

Phase 21 strengthens the enterprise audit handoff and review path:

- `PUT /control/enterprise/policy` now also controls enterprise audit retention days and recent-export review limits.
- `POST /control/enterprise/audit/export` now writes richer bundles with governance state, supervision context, enterprise foundations, and recent enterprise or autonomy operator history.
- `GET /control/enterprise/audit/review` gives one typed review summary for recent retained bundles plus current governance and supervision context.
- `/control/ui` now includes an `Enterprise Audit Review` panel so operators can inspect recent retained bundles without manually opening export files under `.claw/control/enterprise/exports/`.

Treat that as a bounded operator review package, not a full compliance archive. It improves retention and handoff for the shipped enterprise lane, but it still stops short of external SIEM, legal hold, or compliance packaging.

Phase 22 adds operator-gated full autonomy as a separate enterprise lane:

- `GET /control/enterprise/autonomy` reports the current full-autonomy manifest, active override budgets, baseline policy, recent lifecycle events, and recent execution evidence.
- `POST /control/enterprise/autonomy/enable` deliberately switches the runtime into the stronger autonomy override without changing the default trust-first baseline for everyone else.
- `POST /control/enterprise/autonomy/disable` restores the captured baseline autonomy policy for future runs.
- `POST /control/enterprise/autonomy/kill-switch` restores the baseline and issues kill requests to matching active full-autonomy runs.
- Those write routes are protected under the dedicated `enterprise.full_autonomy.manage` scope, which defaults to dual owner or admin approval.

Treat that lane as explicit and reversible. It is the shipped answer to "god mode," but it is still bounded by runtime budgets, enterprise approvals, and durable audit evidence rather than being a silent global default.

Phase 18 adds a supervised-autonomy operator loop over active orchestration runs:

- `POST /control/orchestration/active/{run_id}/pause|resume|kill` remains the low-level control surface.
- `POST /control/orchestration/active/{run_id}/escalate` now records an explicit supervised escalation with operator rationale.
- `POST /control/orchestration/active/{run_id}/rollback` records a rollback decision with optional rollback reference and moves the run into an explicit supervised recovery path.
- `/control/orchestration/active/{run_id}/supervision` now reports lifecycle state and structured intervention history in addition to recent events and attention signals.

Use those supervision controls as the truth source for longer-running operator-managed runs. They are intended to make intervention explicit and auditable, not to create an unbounded autonomous executor.

Phase 19 closes the currently shipped enterprise slice with a consolidated admin/operator surface:

- `GET /control/enterprise/admin` combines enterprise access state, enterprise policy state, and active supervised-run attention counts into one typed summary.
- `/control/ui` now includes an `Enterprise Admin` panel that stores `x-openrustclaw-operator-id`, `x-openrustclaw-operator-token`, `x-openrustclaw-approver-id`, and `x-openrustclaw-approver-token` locally in the browser for protected enterprise writes.
- That same panel can bootstrap enterprise access, upsert operators, update enterprise policy, upsert governance rules, and trigger durable audit-export bundles without dropping to raw route calls.
- The admin summary also points operators back to the shipped supervision surface when active orchestration runs still require escalation or rollback review.

Treat that panel as the current enterprise operator loop, not full IAM. It makes the shipped access, policy, audit, and supervision controls usable from one place, but it does not replace SSO, SCIM, or multi-tenant administration.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Architecture Overview](#architecture-overview)
- [Deployment Options](#deployment-options)
  - [Option 1: Docker Compose (Single Node)](#option-1-docker-compose-single-node)
  - [Option 2: Kubernetes (Recommended)](#option-2-kubernetes-recommended)
  - [Option 3: Bare Metal / VM](#option-3-bare-metal--vm)
- [Configuration](#configuration)
- [Security Hardening](#security-hardening)
- [Monitoring & Observability](#monitoring--observability)
- [Backup & Disaster Recovery](#backup--disaster-recovery)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

### System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU | 2 cores | 4+ cores |
| RAM | 4 GB | 8+ GB |
| Disk | 20 GB SSD | 100+ GB NVMe |
| Network | 100 Mbps | 1 Gbps |

### Software Requirements

- **Docker** 24.0+ (for containerized deployment)
- **Kubernetes** 1.28+ (for K8s deployment)
- **PostgreSQL** 15+ (optional, for advanced use cases)
- **Redis** 7+ (optional, for caching/sessions)

### API Keys

Required for LLM providers:

```bash
# At least one provider required
export ANTHROPIC_API_KEY=sk-ant-...
export OPENAI_API_KEY=sk-...
export OPENROUTER_API_KEY=sk-or-...
```

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         LOAD BALANCER                            │
│                    (nginx / AWS ALB / Traefik)                   │
└───────────────────────┬─────────────────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        ▼               ▼               ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│  OpenRust    │ │  OpenRust    │ │  OpenRust    │
│  Claw Node 1 │ │  Claw Node 2 │ │  Claw Node N │
│  (Gateway)   │ │  (Gateway)   │ │  (Gateway)   │
└──────┬───────┘ └──────┬───────┘ └──────┬───────┘
       │                │                │
       └────────────────┼────────────────┘
                        ▼
              ┌──────────────────┐
              │   SQLite (R/W)   │
              │   or PostgreSQL  │
              └──────────────────┘
```

---

## Deployment Options

### Option 1: Docker Compose (Single Node)

Best for: Small deployments, development, testing

#### 1. Create Project Directory

```bash
mkdir -p /opt/openrustclaw
cd /opt/openrustclaw
```

#### 2. Create Environment File

```bash
cat > .env << 'EOF'
# Required: API Keys
ANTHROPIC_API_KEY=your-key-here
OPENAI_API_KEY=your-key-here

# Optional: Additional providers
OPENROUTER_API_KEY=your-key-here
OLLAMA_HOST=http://localhost:11434

# Database
DATABASE_URL=sqlite:///data/openrustclaw.db
DATABASE_POOL_SIZE=10

# Security
JWT_SECRET=$(openssl rand -hex 32)
ORIGIN_WHITELIST=https://yourdomain.com,https://app.yourdomain.com
REQUIRE_AUTH=true

# Gateway
GATEWAY_HOST=0.0.0.0
GATEWAY_PORT=8080
GATEWAY_WORKERS=4

# Logging
RUST_LOG=info,openrustclaw=debug
LOG_FORMAT=json

# Observability
OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4317
METRICS_ENABLED=true
EOF
```

#### 3. Create Docker Compose File

```yaml
version: '3.8'

services:
  openrustclaw:
    image: ghcr.io/openrustclaw/openrustclaw:latest
    container_name: openrustclaw
    restart: unless-stopped
    env_file:
      - .env
    ports:
      - "8080:8080"
    volumes:
      - ./data:/data
      - ./config:/etc/openrustclaw
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 4G
        reservations:
          cpus: '1'
          memory: 2G
    networks:
      - openrustclaw

  # Optional: Prometheus for metrics
  prometheus:
    image: prom/prometheus:latest
    container_name: prometheus
    restart: unless-stopped
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus
    ports:
      - "9090:9090"
    networks:
      - openrustclaw

  # Optional: Grafana for dashboards
  grafana:
    image: grafana/grafana:latest
    container_name: grafana
    restart: unless-stopped
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana-data:/var/lib/grafana
      - ./grafana/dashboards:/etc/grafana/provisioning/dashboards
    ports:
      - "3000:3000"
    networks:
      - openrustclaw

  # Optional: Jaeger for tracing
  jaeger:
    image: jaegertracing/all-in-one:latest
    container_name: jaeger
    restart: unless-stopped
    environment:
      - COLLECTOR_OTLP_ENABLED=true
    ports:
      - "16686:16686"
      - "4317:4317"
    networks:
      - openrustclaw

volumes:
  prometheus-data:
  grafana-data:

networks:
  openrustclaw:
    driver: bridge
```

#### 4. Deploy

```bash
# Pull latest images
docker-compose pull

# Start services
docker-compose up -d

# Check status
docker-compose ps
docker-compose logs -f openrustclaw
```

#### 5. Verify Deployment

```bash
# Health check
curl http://localhost:8080/health

# Check metrics
curl http://localhost:8080/metrics
```

---

### Option 2: Kubernetes (Recommended)

Best for: Production workloads, high availability, auto-scaling

See [Kubernetes Deployment](./kubernetes.md) for detailed Helm chart instructions.

Quick start:

```bash
# Add Helm repository
helm repo add openrustclaw https://openrustclaw.github.io/charts
helm repo update

# Install with custom values
helm install openrustclaw openrustclaw/openrustclaw \
  --namespace openrustclaw \
  --create-namespace \
  --values values-production.yaml
```

---

### Option 3: Bare Metal / VM

Best for: Air-gapped environments, maximum performance

#### 1. Install Dependencies

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y sqlite3 libsqlite3-dev

# RHEL/CentOS/Fedora
sudo dnf install -y sqlite sqlite-devel

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### 2. Create System User

```bash
sudo useradd -r -s /bin/false openrustclaw
sudo mkdir -p /opt/openrustclaw /var/lib/openrustclaw
sudo chown openrustclaw:openrustclaw /opt/openrustclaw /var/lib/openrustclaw
```

#### 3. Build from Source

```bash
cd /opt/openrustclaw
sudo -u openrustclaw git clone https://github.com/openrustclaw/openrustclaw.git .
sudo -u openrustclaw cargo build --release
```

#### 4. Create Systemd Service

```bash
sudo tee /etc/systemd/system/openrustclaw.service > /dev/null << 'EOF'
[Unit]
Description=OpenRustClaw AI Agent Gateway
After=network.target

[Service]
Type=simple
User=openrustclaw
Group=openrustclaw
WorkingDirectory=/opt/openrustclaw
EnvironmentFile=/etc/openrustclaw/environment
ExecStart=/opt/openrustclaw/target/release/openrustclaw-gateway
Restart=on-failure
RestartSec=5

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/openrustclaw
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
EOF
```

#### 5. Create Environment File

```bash
sudo mkdir -p /etc/openrustclaw
sudo tee /etc/openrustclaw/environment > /dev/null << 'EOF'
RUST_LOG=info
DATABASE_URL=sqlite:///var/lib/openrustclaw/openrustclaw.db
GATEWAY_HOST=0.0.0.0
GATEWAY_PORT=8080
ANTHROPIC_API_KEY=your-key-here
JWT_SECRET=your-secret-here
EOF

sudo chmod 600 /etc/openrustclaw/environment
sudo chown openrustclaw:openrustclaw /etc/openrustclaw/environment
```

#### 6. Start Service

```bash
sudo systemctl daemon-reload
sudo systemctl enable openrustclaw
sudo systemctl start openrustclaw
sudo systemctl status openrustclaw
```

For a workspace-owned user-service install in the shipped operator flow, the CLI now also supports:

```bash
openrustclaw runtime services install-status
openrustclaw runtime services install
systemctl --user daemon-reload   # Linux
systemctl --user enable openrustclaw.service
systemctl --user start openrustclaw.service
launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/dev.openrustclaw.openrustclaw.plist   # macOS
```

For ongoing workspace-owned runtime operations, the shipped CLI now also supports:

```bash
openrustclaw runtime services lock-status
openrustclaw runtime services rotate-logs --keep 7 --max-bytes 10485760
openrustclaw runtime migrate-config --config config/default.toml
openrustclaw runtime upgrade-plan --config config/default.toml
openrustclaw runtime self-update-plan --config config/default.toml --artifact ./target/release/openrustclaw
openrustclaw runtime rollback-plan --config config/default.toml --artifact ./.claw/runtime-releases/rollback-YYYYMMDDHHMMSS/openrustclaw
```

The canonical operator loop for the shipped Rust runtime is:

1. Verify the managed runtime path with `openrustclaw runtime services install-status` and install it with `openrustclaw runtime services install` if unattended restarts are required.
2. Review `openrustclaw runtime health`, `openrustclaw runtime reload-plan`, and `GET /control/runtime/operator-ops` before restart windows.
3. Take a workspace snapshot with `openrustclaw runtime backup` and rotate logs with `openrustclaw runtime services rotate-logs --keep 7 --max-bytes 10485760`.
4. Normalize config drift with `openrustclaw runtime migrate-config --config config/default.toml --apply` before production upgrades.
5. Use `openrustclaw runtime upgrade-plan`, `openrustclaw runtime self-update-plan --artifact ...`, and `openrustclaw runtime rollback-plan --artifact ...` as the bounded maintenance and recovery path.
6. After the runtime is back up, confirm the same state from `/control/ui` through `Operator Ops Summary`, runtime health, runtime beacon, recent runtime events, and logs.

- `install-status` reports the detected host user service manager, whether the service is installed, the rendered unit/agent path, and the suggested reload/start commands.
- `install` writes the standalone runtime service definition so the workspace can be managed outside the onboarding wizard, using user-level systemd on Linux and launchd agents on macOS.
- `lock-status` inspects `.claw/control/runtime-lock.json` and reports whether the stored PID is still live or stale.
- `rotate-logs` performs copy-truncate rotation on `.claw/control/runtime.log` and keeps archives under `.claw/control/runtime-log-archives/`.
- `migrate-config` detects legacy config keys, infers missing deployment fields, and can rewrite the canonical config with a timestamped backup when run with `--apply`.
- `upgrade-plan` summarizes the current runtime health, reload guidance, service install state, and runtime-lock status before a restart or binary/config upgrade.
- `self-update-plan` validates a candidate binary artifact, recommends where to snapshot the current executable for rollback, and composes the managed-service restart guidance before an operator swaps the binary.
- `rollback-plan` validates a prior binary artifact and composes the corresponding restore/restart playbook before an operator reverts a bad rollout.
- `/control/runtime/operator-ops` and the matching `Operator Ops Summary` panel in `/control/ui` expose the same deploy-run-recover summary in browser form so operators can confirm service install state, lock state, reload posture, backups, and recovery guidance without reconstructing it manually.
- `scripts/build-release-artifacts.sh --target <triple>` packages a versioned tarball plus `.sha256` for the chosen Rust target, and the release workflow now builds those artifacts for Linux/macOS x86_64 and ARM64.
- `scripts/check-runtime-budgets.sh` enforces bounded release-binary size, CLI startup latency, and idle gateway RSS regressions before release promotion.
- `[channels.runtime]` now controls the shipped channel-health monitor. With `health_monitor_enabled = true`, persisted readiness scans include monitor state; with `auto_restart_on_failure = true` and a supported installed user service, repeated failing scans can trigger a managed restart after `failure_threshold` consecutive failures.

---

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level (error/warn/info/debug/trace) | `info` |
| `DATABASE_URL` | SQLite or PostgreSQL connection string | `sqlite://./openrustclaw.db` |
| `GATEWAY_HOST` | Gateway bind address | `127.0.0.1` |
| `GATEWAY_PORT` | Gateway port | `8080` |

OpenRustClaw now also supports an explicit deployment mode in config:

```toml
[gateway]
network_mode = "remote" # loopback, lan, remote
host = "0.0.0.0"
port = 8080
allowed_origins = ["https://app.example.com"]
```

- `loopback` expects a loopback bind host such as `127.0.0.1`
- `lan` expects a non-loopback bind host for trusted local-network exposure
- `remote` expects a non-loopback bind host plus explicit allowed origins and auth posture
| `GATEWAY_WORKERS` | Number of worker threads | `num_cpus` |
| `JWT_SECRET` | Secret for JWT signing | **Required** |
| `JWT_EXPIRY_HOURS` | JWT token expiry | `24` |
| `ORIGIN_WHITELIST` | Comma-separated allowed origins | **Required** |
| `REQUIRE_AUTH` | Require authentication | `true` |
| `RATE_LIMIT_RPS` | Rate limit (requests per second) | `100` |
| `RATE_LIMIT_BURST` | Rate limit burst size | `150` |

### Provider Configuration

```bash
# Anthropic
ANTHROPIC_API_KEY=sk-ant-...
ANTHROPIC_MODEL=claude-3-5-sonnet-20241022
ANTHROPIC_MAX_TOKENS=4096

# OpenAI
OPENAI_API_KEY=sk-...
OPENAI_MODEL=gpt-4o
OPENAI_MAX_TOKENS=4096

# OpenRouter
OPENROUTER_API_KEY=sk-or-...
OPENROUTER_MODEL=anthropic/claude-3.5-sonnet

# Ollama (local)
OLLAMA_HOST=http://localhost:11434
OLLAMA_MODEL=llama3.2
```

---

## Security Hardening

### 1. Network Security

```bash
# UFW firewall rules (Ubuntu)
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 22/tcp    # SSH
sudo ufw allow 80/tcp    # HTTP (redirect to HTTPS)
sudo ufw allow 443/tcp   # HTTPS
sudo ufw enable
```

### 2. TLS/SSL Configuration

Using Let's Encrypt with certbot:

```bash
# Install certbot
sudo apt-get install -y certbot

# Obtain certificate
sudo certbot certonly --standalone -d yourdomain.com

# Auto-renewal
sudo systemctl enable certbot.timer
```

### 3. JWT Secret Rotation

```bash
# Generate strong secret
openssl rand -base64 64

# Rotate secrets periodically (e.g., monthly)
# Update JWT_SECRET and restart service
```

### 4. Database Encryption

```bash
# Enable SQLite encryption (SQLCipher)
export DATABASE_URL="sqlite:///data/openrustclaw.db?key=mysecretkey"
```

---

## Monitoring & Observability

### Metrics Endpoints

| Endpoint | Description |
|----------|-------------|
| `/health` | Basic health check |
| `/health/ready` | Readiness probe |
| `/health/deep` | Deep health check (includes dependencies) |
| `/metrics` | Prometheus metrics |

### Key Metrics to Monitor

```promql
# Request rate
rate(openrustclaw_requests_total[5m])

# Error rate
rate(openrustclaw_errors_total[5m])

# Response time (p99)
histogram_quantile(0.99, rate(openrustclaw_request_duration_seconds_bucket[5m]))

# Active connections
openrustclaw_active_connections

# Database pool utilization
openrustclaw_db_pool_connections{state="in_use"}
```

### Alerting Rules (Prometheus)

```yaml
groups:
  - name: openrustclaw
    rules:
      - alert: HighErrorRate
        expr: rate(openrustclaw_errors_total[5m]) > 0.05
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High error rate detected"

      - alert: HighLatency
        expr: histogram_quantile(0.99, rate(openrustclaw_request_duration_seconds_bucket[5m])) > 2
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High latency detected"

      - alert: ServiceDown
        expr: up{job="openrustclaw"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "OpenRustClaw service is down"
```

---

## Backup & Disaster Recovery

### Automated Backups

```bash
#!/bin/bash
# /opt/openrustclaw/backup.sh

BACKUP_DIR="/backup/openrustclaw"
DATE=$(date +%Y%m%d_%H%M%S)

# Create backup
sqlite3 /var/lib/openrustclaw/openrustclaw.db ".backup '${BACKUP_DIR}/backup_${DATE}.db'"

# Compress
gzip "${BACKUP_DIR}/backup_${DATE}.db"

# Keep only last 30 days
find ${BACKUP_DIR} -name "backup_*.db.gz" -mtime +30 -delete

# Sync to S3 (optional)
aws s3 sync ${BACKUP_DIR} s3://your-backup-bucket/openrustclaw/
```

Add to crontab:

```bash
# Daily backup at 2 AM
0 2 * * * /opt/openrustclaw/backup.sh
```

### Point-in-Time Recovery

```bash
# Stop service
sudo systemctl stop openrustclaw

# Restore from backup
sudo cp /backup/openrustclaw/backup_20240115_020000.db.gz /tmp/
gunzip /tmp/backup_20240115_020000.db.gz
sudo mv /tmp/backup_20240115_020000.db /var/lib/openrustclaw/openrustclaw.db
sudo chown openrustclaw:openrustclaw /var/lib/openrustclaw/openrustclaw.db

# Start service
sudo systemctl start openrustclaw
```

---

## Troubleshooting

### Common Issues

#### 1. Service Won't Start

```bash
# Check logs
sudo journalctl -u openrustclaw -f

# Verify configuration
sudo -u openrustclaw /opt/openrustclaw/target/release/openrustclaw-gateway --check-config

# Check file permissions
ls -la /var/lib/openrustclaw/
ls -la /etc/openrustclaw/
```

#### 2. Database Connection Issues

```bash
# Test database connectivity
sqlite3 /var/lib/openrustclaw/openrustclaw.db "SELECT 1;"

# Check disk space
df -h /var/lib/openrustclaw/

# Verify WAL mode
sqlite3 /var/lib/openrustclaw/openrustclaw.db "PRAGMA journal_mode;"
```

#### 3. High Memory Usage

```bash
# Check memory usage
ps aux | grep openrustclaw

# Profile memory (if built with debug symbols)
sudo perf top -p $(pgrep openrustclaw-gateway)

# Enable memory profiling
export RUST_LOG=info,openrustclaw=debug
```

#### 4. Rate Limiting Issues

```bash
# Check current rate limits
curl http://localhost:8080/metrics | grep rate_limit

# Adjust in configuration
export RATE_LIMIT_RPS=200
export RATE_LIMIT_BURST=300
```

### Getting Help

- **Documentation**: https://docs.openrustclaw.io
- **GitHub Issues**: https://github.com/openrustclaw/openrustclaw/issues
- **Discord**: https://discord.gg/openrustclaw
- **Email**: support@openrustclaw.io

---

## Next Steps

- [Kubernetes Deployment](./kubernetes.md) - For HA deployments
- [Terraform Modules](./terraform.md) - For cloud infrastructure
- [Enterprise SSO](./sso.md) - For OIDC/SAML authentication
- [Monitoring Setup](./monitoring.md) - Complete observability guide
