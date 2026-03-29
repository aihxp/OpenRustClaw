# Docker Deployment

This guide covers deploying OpenRustClaw using Docker and Docker Compose.

## Overview

OpenRustClaw uses a multi-stage Docker build to create optimized production images:

- **Rust Application**: Compiled with LTO optimizations for minimal size
- **Final Image**: Based on `debian:bookworm-slim` with the Rust binary and runtime libraries only
- **Python Sidecar**: Optional compatibility lane that is not part of the default production image
- **Multi-platform**: Supports `linux/amd64` and `linux/arm64`

## Quick Start

### Prerequisites

- Docker 20.10+ with BuildKit enabled
- Docker Compose 2.0+
- At least 2GB RAM for build process

### 1. Clone and Configure

```bash
git clone https://github.com/openrustclaw/openrustclaw.git
cd openrustclaw

# Copy and edit environment configuration
cp .env.example .env
# Edit .env with your API keys
```

### 2. Build the Image

```bash
# Build locally
docker build -t openrustclaw:latest .

# Or use the build script
./scripts/docker-build.sh latest
```

### 3. Run with Docker Compose

```bash
# Start the stack
docker-compose up -d

# View logs
docker-compose logs -f openrustclaw

# Check health
curl http://localhost:18789/health
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ANTHROPIC_API_KEY` | Anthropic API key | Required |
| `OPENAI_API_KEY` | OpenAI API key | Optional |
| `OPENROUTER_API_KEY` | OpenRouter API key | Optional |
| `GATEWAY_PORT` | WebSocket gateway port | `18789` |
| `DATABASE_URL` | SQLite database path | `sqlite:///app/data/openrustclaw.db` |
| `AUTH_SECRET` | JWT signing secret | Required |
| `LANGSMITH_API_KEY` | LangSmith observability | Optional |

### Volume Mounts

```yaml
volumes:
  # Database persistence (required)
  - openrustclaw_data:/app/data
  
  # Custom configuration (optional)
  - ./config:/app/config:ro
  
  # Custom skills (optional)
  - ./skills:/app/skills:ro
  
  # Logs (optional)
  - ./logs:/app/logs
```

## Production Deployment

### Using Docker Compose

```bash
# Create production environment file
cat > .env.production << EOF
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-...
AUTH_SECRET=$(openssl rand -base64 32)
GATEWAY_PORT=18789
GATEWAY_ALLOWED_ORIGINS=https://yourdomain.com
RUST_LOG=info
EOF

# Deploy
docker-compose -f docker-compose.yml --env-file .env.production up -d
```

The production Compose path now runs the Rust runtime only. If you need the optional compatibility sidecar, run it as a separate bounded support service instead of relying on the default production container.

### With Reverse Proxy (Nginx)

```bash
# Start with nginx profile
docker-compose --profile with-proxy up -d
```

Example `nginx/nginx.conf`:

```nginx
server {
    listen 80;
    server_name yourdomain.com;
    
    location / {
        proxy_pass http://openrustclaw:18789;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### With SSL (Let's Encrypt)

```bash
# Start with SSL profile
docker-compose --profile with-ssl --profile with-proxy up -d
```

## Development Setup

### Hot Reload Development

```bash
# Start development environment
docker-compose -f docker-compose.dev.yml up -d

# View Rust logs
docker-compose -f docker-compose.dev.yml logs -f openrustclaw-rust

# View optional Python sidecar logs
docker-compose -f docker-compose.dev.yml logs -f openrustclaw-sidecar
```

### Available Dev Profiles

| Profile | Service | Port | Description |
|---------|---------|------|-------------|
| default | openrustclaw-rust | 18789 | Rust gateway with hot reload |
| default | openrustclaw-sidecar | 50051 | Optional compatibility sidecar with file watching |
| db-ui | sqlite-web | 8080 | Web-based SQLite browser |
| monitoring | prometheus | 9091 | Metrics collection |
| monitoring | grafana | 3000 | Dashboards |

### Start with Database UI

```bash
docker-compose -f docker-compose.dev.yml --profile db-ui up -d
# Access SQLite Web at http://localhost:8080
```

## Multi-Platform Builds

### Build for Multiple Architectures

```bash
# Create buildx builder
docker buildx create --name openrustclaw-builder --use

# Build for amd64 and arm64
./scripts/docker-build.sh \
    --platforms linux/amd64,linux/arm64 \
    --push \
    v0.1.1
```

### Platform-Specific Builds

```bash
# AMD64 only
./scripts/docker-build.sh --platforms linux/amd64 latest

# ARM64 only (for Apple Silicon, Raspberry Pi)
./scripts/docker-build.sh --platforms linux/arm64 latest
```

## Registry Operations

### Push to Docker Hub

```bash
export REGISTRY=docker.io/yourusername
export DOCKER_USERNAME=yourusername
export DOCKER_PASSWORD=yourpassword

./scripts/docker-push.sh --latest v0.1.1
```

### Push to GitHub Container Registry

```bash
export REGISTRY=ghcr.io/yourorg
export DOCKER_USERNAME=yourusername
export DOCKER_PASSWORD=ghp_yourtoken

./scripts/docker-push.sh --build --latest v0.1.1
```

### Push to AWS ECR

```bash
# Login to ECR
aws ecr get-login-password --region us-east-1 | \
    docker login --username AWS --password-stdin 123456789012.dkr.ecr.us-east-1.amazonaws.com

# Build and push
export REGISTRY=123456789012.dkr.ecr.us-east-1.amazonaws.com/yourrepo
./scripts/docker-push.sh --latest v0.1.1
```

## Monitoring and Health

### Health Check Endpoint

```bash
# Gateway health
curl http://localhost:18789/health

# Expected response:
{
  "status": "healthy",
  "version": "0.1.1",
  "timestamp": "2026-03-16T12:00:00Z"
}
```

### Prometheus Metrics

Available at `http://localhost:9090/metrics`:

```
# HELP openrustclaw_requests_total Total requests
# TYPE openrustclaw_requests_total counter
openrustclaw_requests_total{method="GET",status="200"} 1024

# HELP openrustclaw_active_connections Active WebSocket connections
# TYPE openrustclaw_active_connections gauge
openrustclaw_active_connections 42
```

### Docker Health Check

The container includes a built-in health check:

```dockerfile
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
  CMD curl -f http://localhost:18789/health || exit 1
```

Check container health:

```bash
docker ps
# STATUS column shows health: starting/healthy/unhealthy

docker inspect --format='{{.State.Health.Status}}' openrustclaw
```

## Troubleshooting

### Build Issues

**Issue**: Build fails with "protoc not found"
```bash
# Install protobuf compiler locally, or ensure Docker build context is clean
docker builder prune -f
```

**Issue**: Multi-platform build fails
```bash
# Ensure buildx is using docker-container driver
docker buildx create --name multiarch --driver docker-container --use
docker buildx inspect --bootstrap
```

### Runtime Issues

**Issue**: Container exits immediately
```bash
# Check logs
docker-compose logs openrustclaw

# Verify environment variables
docker-compose config
```

**Issue**: Database permission errors
```bash
# Fix data directory permissions
sudo chown -R 1000:1000 ./data

# Or in docker-compose, ensure volume is writable
volumes:
  - openrustclaw_data:/app/data
```

**Issue**: Cannot connect to sidecar

This only applies if you intentionally run a separate compatibility sidecar alongside the production runtime.

```bash
# Verify the sidecar host/port you configured is actually listening
netstat -tlnp | grep 50051

# Then verify your runtime-side sidecar settings
rg -n "sidecar" config/default.toml .env
```

### Performance Issues

**Issue**: High memory usage
```bash
# Set memory limits in docker-compose.yml
deploy:
  resources:
    limits:
      memory: 2G
```

**Issue**: Slow startup
```bash
# Pre-create database directory
mkdir -p data
chmod 777 data

# Use volume for node_modules equivalent (Cargo target)
docker-compose -f docker-compose.dev.yml up -d
```

## Security Considerations

### Non-Root User

The container runs as `appuser` (UID 1000) for security:

```dockerfile
RUN groupadd --gid 1000 appuser && \
    useradd --uid 1000 --gid appuser --shell /bin/bash --create-home appuser
USER appuser
```

### Read-Only Root Filesystem

For additional security, run with read-only root:

```yaml
services:
  openrustclaw:
    read_only: true
    tmpfs:
      - /tmp:noexec,nosuid,size=100m
      - /app/tmp:noexec,nosuid,size=100m
```

Note: SQLite WAL mode requires writable database directory.

### Secrets Management

Use Docker secrets for production:

```yaml
services:
  openrustclaw:
    secrets:
      - anthropic_api_key
      - auth_secret

secrets:
  anthropic_api_key:
    file: ./secrets/anthropic_api_key.txt
  auth_secret:
    file: ./secrets/auth_secret.txt
```

Update entrypoint to read secrets:

```bash
if [[ -f /run/secrets/anthropic_api_key ]]; then
    export ANTHROPIC_API_KEY=$(cat /run/secrets/anthropic_api_key)
fi
```

## Advanced Configuration

### Custom Build Arguments

```bash
docker build \
    --build-arg RUST_LOG=debug \
    --build-arg FEATURES=mcp,scheduler \
    -t openrustclaw:custom .
```

### Using Private Registries

```bash
# Configure registry mirror
cat > /etc/docker/daemon.json << EOF
{
  "registry-mirrors": ["https://private.registry.com"]
}
EOF

# Login and pull base images
docker login private.registry.com
```

### Image Scanning

```bash
# Scan with Trivy
trivy image openrustclaw:latest

# Scan with Snyk
snyk container test openrustclaw:latest

# Scan with Docker Scout
docker scout cves openrustclaw:latest
```

## Cleanup

```bash
# Stop and remove containers
docker-compose down

# Remove with volumes (WARNING: deletes database)
docker-compose down -v

# Remove all images
docker rmi openrustclaw:latest

# Prune build cache
docker builder prune -f
```

## Next Steps

- [Kubernetes Deployment](./kubernetes.md) - Deploy to Kubernetes
- [Systemd Service](./systemd.md) - Run as system service
- [Cloud Deployment](./cloud.md) - Deploy to AWS/GCP/Azure
