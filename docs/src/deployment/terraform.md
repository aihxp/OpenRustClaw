# Terraform Modules

Production-ready Terraform modules for deploying OpenRustClaw infrastructure on AWS, GCP, and Azure.

## Overview

| Cloud | Module | Features |
|-------|--------|----------|
| **AWS** | `deployments/terraform/aws` | EKS, RDS, ElastiCache, ALB, S3 |
| **GCP** | `deployments/terraform/gcp` | GKE, Cloud SQL, Memorystore, GCS |
| **Azure** | `deployments/terraform/azure` | AKS, PostgreSQL, Redis, Storage |

---

## Quick Start

### AWS

```bash
cd deployments/terraform/aws

# Initialize
terraform init

# Plan
terraform plan -var="cluster_name=openrustclaw-prod" -var="environment=production"

# Apply
terraform apply

# Configure kubectl
aws eks update-kubeconfig --region us-west-2 --name openrustclaw-prod
```

### GCP

```bash
cd deployments/terraform/gcp

# Set project
gcloud config set project YOUR_PROJECT_ID

# Initialize
terraform init

# Apply
terraform apply -var="project_id=YOUR_PROJECT_ID" -var="create_cloudsql=true"

# Configure kubectl
gcloud container clusters get-credentials openrustclaw --region us-central1
```

### Azure

```bash
cd deployments/terraform/azure

# Login
az login

# Initialize
terraform init

# Apply
terraform apply -var="create_postgresql=true" -var="create_redis=true"

# Configure kubectl
az aks get-credentials --resource-group openrustclaw-rg --name openrustclaw
```

---

## Module Features

### AWS Module

```hcl
module "openrustclaw" {
  source = "github.com/openrustclaw/openrustclaw//deployments/terraform/aws"

  cluster_name = "openrustclaw-prod"
  environment  = "production"
  region       = "us-west-2"

  # EKS Configuration
  kubernetes_version    = "1.28"
  node_instance_types   = ["m6i.xlarge"]
  node_desired_size     = 3
  node_min_size         = 2
  node_max_size         = 10

  # Database
  create_rds            = true
  rds_instance_class    = "db.t3.medium"
  rds_allocated_storage = 100

  # Cache
  create_redis          = true
  redis_node_type       = "cache.t3.micro"

  # DNS
  create_route53_record = true
  route53_zone_id       = "Z123456789"
  domain_name           = "api.openrustclaw.io"

  tags = {
    Environment = "production"
    Team        = "platform"
  }
}
```

### GCP Module

```hcl
module "openrustclaw" {
  source = "github.com/openrustclaw/openrustclaw//deployments/terraform/gcp"

  project_id   = "my-project"
  cluster_name = "openrustclaw-prod"
  region       = "us-central1"

  # GKE Configuration
  enable_autopilot      = false
  node_machine_type     = "e2-standard-4"
  node_count            = 3
  node_min_count        = 2
  node_max_count        = 10

  # Database
  create_cloudsql       = true
  cloudsql_tier         = "db-custom-2-4096"

  # Cache
  create_redis          = true
  redis_tier            = "STANDARD_HA"
  redis_memory_size     = 5

  # DNS
  create_dns_record     = true
  domain_name           = "openrustclaw.io"
  subdomain             = "api"
}
```

### Azure Module

```hcl
module "openrustclaw" {
  source = "github.com/openrustclaw/openrustclaw//deployments/terraform/azure"

  cluster_name  = "openrustclaw-prod"
  location      = "West US 2"
  environment   = "production"

  # AKS Configuration
  kubernetes_version = "1.28"
  node_vm_size       = "Standard_D4s_v3"
  node_count         = 3
  node_min_count     = 2
  node_max_count     = 10

  # Database
  create_postgresql          = true
  postgresql_sku             = "GP_Standard_D2s_v3"
  postgresql_admin_password  = var.db_password

  # Cache
  create_redis      = true
  redis_sku         = "Standard"
  redis_capacity    = 1

  # DNS
  create_dns_record = true
  domain_name       = "openrustclaw.io"
  subdomain         = "api"
}
```

---

## State Management

### S3 Backend (AWS)

```hcl
terraform {
  backend "s3" {
    bucket         = "openrustclaw-terraform-state"
    key            = "production/terraform.tfstate"
    region         = "us-west-2"
    encrypt        = true
    dynamodb_table = "terraform-locks"
  }
}
```

### GCS Backend (GCP)

```hcl
terraform {
  backend "gcs" {
    bucket = "openrustclaw-terraform-state"
    prefix = "production"
  }
}
```

### Azure Backend

```hcl
terraform {
  backend "azurerm" {
    resource_group_name  = "terraform-state"
    storage_account_name = "openrustclawtfstate"
    container_name       = "tfstate"
    key                  = "production.terraform.tfstate"
  }
}
```

---

## Multi-Environment Setup

### Directory Structure

```
terraform/
├── environments/
│   ├── dev/
│   │   ├── main.tf
│   │   ├── variables.tf
│   │   └── terraform.tfvars
│   ├── staging/
│   │   └── ...
│   └── production/
│       └── ...
└── modules/
    └── openrustclaw/
```

### Example: Development Environment

```hcl
# environments/dev/main.tf
module "openrustclaw" {
  source = "../../modules/aws"

  cluster_name = "openrustclaw-dev"
  environment  = "development"

  node_desired_size = 2
  node_min_size     = 1
  node_max_size     = 3

  create_rds   = false
  create_redis = false
}
```

---

## Best Practices

1. **Use workspaces for environments**
   ```bash
   terraform workspace new production
   terraform workspace select production
   ```

2. **Separate state per environment**
   - Different S3 buckets or prefixes
   - Different state locks

3. **Use variables for sensitive data**
   ```bash
   terraform apply -var-file="secrets.tfvars"
   ```

4. **Enable versioning on state buckets**

5. **Use locking mechanisms**
   - DynamoDB (AWS)
   - Cloud Storage (GCP)
   - Azure Storage

---

## Outputs

All modules provide these outputs:

| Output | Description |
|--------|-------------|
| `cluster_endpoint` | Kubernetes API endpoint |
| `cluster_name` | Cluster name |
| `configure_kubectl` | Command to configure kubectl |
| `database_endpoint` | Database connection endpoint |
| `cache_endpoint` | Cache connection endpoint |
| `storage_bucket` | Backup storage location |

---

## Troubleshooting

### AWS

```bash
# Check EKS node status
aws eks describe-nodegroup --cluster-name openrustclaw-prod --nodegroup-name general

# View CloudTrail logs
aws cloudtrail lookup-events --lookup-attributes AttributeKey=EventSource,AttributeValue=eks.amazonaws.com
```

### GCP

```bash
# Check GKE node status
gcloud container clusters describe openrustclaw --region us-central1

# View audit logs
gcloud logging read "resource.type=k8s_cluster" --limit=50
```

### Azure

```bash
# Check AKS node status
az aks nodepool list --resource-group openrustclaw-rg --cluster-name openrustclaw

# View activity logs
az monitor activity-log list --resource-group openrustclaw-rg
```
