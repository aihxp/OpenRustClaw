# Kubernetes Deployment

This guide covers deploying OpenRustClaw on Kubernetes using Helm charts.

## Prerequisites

- Kubernetes 1.28+
- Helm 3.12+
- kubectl configured
- cert-manager (for TLS)
- nginx-ingress or Traefik (for ingress)

## Quick Start

### 1. Add Helm Repository

```bash
helm repo add openrustclaw https://openrustclaw.github.io/charts
helm repo update
```

### 2. Create Namespace

```bash
kubectl create namespace openrustclaw
```

### 3. Install with Default Values

```bash
helm install openrustclaw openrustclaw/openrustclaw \
  --namespace openrustclaw \
  --set config.providers.anthropic.apiKey="your-key" \
  --set config.providers.anthropic.enabled=true
```

### 4. Verify Installation

```bash
kubectl get pods -n openrustclaw
kubectl logs -f deployment/openrustclaw -n openrustclaw
```

---

## Production Deployment

### 1. Create values-production.yaml

```yaml
# values-production.yaml
replicaCount: 3

image:
  tag: "v0.1.0"

config:
  logLevel: info
  
  gateway:
    workers: 8
  
  security:
    requireAuth: true
    originWhitelist: "https://app.yourdomain.com"
    rateLimitRps: 200
    rateLimitBurst: 300
  
  providers:
    anthropic:
      enabled: true
      apiKey: ""  # Set via secret
      model: claude-3-5-sonnet-20241022
    openai:
      enabled: true
      apiKey: ""  # Set via secret
      model: gpt-4o
  
  observability:
    metricsEnabled: true
    otlpEndpoint: "http://tempo.monitoring:4317"

# Use external secret for API keys
secrets:
  useExternal: true
  externalSecretName: openrustclaw-secrets

ingress:
  enabled: true
  className: nginx
  annotations:
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    nginx.ingress.kubernetes.io/proxy-body-size: "10m"
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
  hosts:
    - host: api.openrustclaw.io
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: openrustclaw-tls
      hosts:
        - api.openrustclaw.io

resources:
  limits:
    cpu: 4000m
    memory: 8Gi
  requests:
    cpu: 1000m
    memory: 2Gi

autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 20
  targetCPUUtilizationPercentage: 60
  targetMemoryUtilizationPercentage: 70

persistence:
  enabled: true
  size: 50Gi
  storageClass: fast-ssd

podDisruptionBudget:
  enabled: true
  minAvailable: 2

monitoring:
  serviceMonitor:
    enabled: true
    namespace: monitoring
```

### 2. Create External Secret

```bash
# Create secret with API keys
kubectl create secret generic openrustclaw-secrets \
  --namespace openrustclaw \
  --from-literal=JWT_SECRET=$(openssl rand -hex 32) \
  --from-literal=ANTHROPIC_API_KEY=sk-ant-... \
  --from-literal=OPENAI_API_KEY=sk-...
```

### 3. Deploy

```bash
helm upgrade --install openrustclaw openrustclaw/openrustclaw \
  --namespace openrustclaw \
  -f values-production.yaml
```

---

## Configuration Reference

### Scaling

```yaml
# Horizontal autoscaling
autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 50
  targetCPUUtilizationPercentage: 70

# Vertical scaling
resources:
  limits:
    cpu: 4000m
    memory: 8Gi
```

### High Availability

```yaml
# Pod disruption budget
podDisruptionBudget:
  enabled: true
  minAvailable: 2

# Anti-affinity
affinity:
  podAntiAffinity:
    requiredDuringSchedulingIgnoredDuringExecution:
      - labelSelector:
          matchLabels:
            app.kubernetes.io/name: openrustclaw
        topologyKey: kubernetes.io/hostname
```

### Database Options

#### SQLite (Default)

```yaml
config:
  database:
    type: sqlite
    sqlite:
      path: /data/openrustclaw.db
      poolSize: 10

persistence:
  enabled: true
  size: 50Gi
```

#### PostgreSQL

```yaml
config:
  database:
    type: postgresql
    postgresql:
      host: postgres-postgresql
      port: 5432
      database: openrustclaw
      user: openrustclaw
      password: "${POSTGRES_PASSWORD}"
      poolSize: 20
      sslMode: require
```

---

## Monitoring

### Prometheus ServiceMonitor

```yaml
monitoring:
  serviceMonitor:
    enabled: true
    namespace: monitoring
    interval: 30s
    labels:
      release: prometheus
```

### Grafana Dashboard

Import dashboard ID `openrustclaw-001` or apply:

```bash
kubectl apply -f https://raw.githubusercontent.com/openrustclaw/openrustclaw/main/deployments/grafana-dashboard.yaml
```

---

## Troubleshooting

### Check Pod Status

```bash
kubectl describe pod -l app.kubernetes.io/name=openrustclaw -n openrustclaw
```

### View Logs

```bash
kubectl logs -f deployment/openrustclaw -n openrustclaw
```

### Debug Configuration

```bash
kubectl get configmap openrustclaw-config -n openrustclaw -o yaml
```

### Port Forward for Local Testing

```bash
kubectl port-forward svc/openrustclaw 8080:8080 -n openrustclaw
curl http://localhost:8080/health
```

---

## Uninstall

```bash
helm uninstall openrustclaw -n openrustclaw
kubectl delete namespace openrustclaw
```
