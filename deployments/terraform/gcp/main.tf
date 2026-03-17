# GCP Terraform Module for OpenRustClaw
# Deploys OpenRustClaw on GKE with supporting infrastructure

terraform {
  required_version = ">= 1.5.0"
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.23"
    }
  }
}

# Enable APIs
resource "google_project_service" "apis" {
  for_each = toset([
    "container.googleapis.com",
    "sqladmin.googleapis.com",
    "redis.googleapis.com",
    "storage.googleapis.com",
    "cloudkms.googleapis.com",
    "dns.googleapis.com",
  ])
  service = each.value
}

# VPC Network
resource "google_compute_network" "vpc" {
  name                    = "${var.cluster_name}-vpc"
  auto_create_subnetworks = false
  depends_on              = [google_project_service.apis]
}

resource "google_compute_subnetwork" "subnet" {
  name          = "${var.cluster_name}-subnet"
  ip_cidr_range = var.subnet_cidr
  network       = google_compute_network.vpc.id
  region        = var.region

  secondary_ip_range {
    range_name    = "pods"
    ip_cidr_range = var.pods_cidr
  }

  secondary_ip_range {
    range_name    = "services"
    ip_cidr_range = var.services_cidr
  }

  private_ip_google_access = true
}

# GKE Cluster
resource "google_container_cluster" "primary" {
  name     = var.cluster_name
  location = var.zone != "" ? var.zone : var.region

  network    = google_compute_network.vpc.name
  subnetwork = google_compute_subnetwork.subnet.name

  release_channel {
    channel = "REGULAR"
  }

  min_master_version = var.kubernetes_version

  # Enable autopilot for simplified management (optional)
  enable_autopilot = var.enable_autopilot

  # Node configuration for standard mode
  dynamic "node_config" {
    for_each = var.enable_autopilot ? [] : [1]
    content {
      machine_type = var.node_machine_type
      disk_size_gb = var.node_disk_size

      oauth_scopes = [
        "https://www.googleapis.com/auth/cloud-platform"
      ]
    }
  }

  # IP allocation for pods and services
  ip_allocation_policy {
    cluster_secondary_range_name  = "pods"
    services_secondary_range_name = "services"
  }

  # Private cluster configuration
  private_cluster_config {
    enable_private_nodes    = true
    enable_private_endpoint = false
    master_ipv4_cidr_block  = var.master_cidr
  }

  # Maintenance window
  maintenance_policy {
    recurring_window {
      start_time = var.maintenance_start_time
      end_time   = var.maintenance_end_time
      recurrence = var.maintenance_recurrence
    }
  }

  depends_on = [google_project_service.apis]
}

# Node pools (for standard mode)
resource "google_container_node_pool" "general" {
  count = var.enable_autopilot ? 0 : 1

  name       = "general"
  location   = var.zone != "" ? var.zone : var.region
  cluster    = google_container_cluster.primary.name
  node_count = var.node_count

  autoscaling {
    min_node_count = var.node_min_count
    max_node_count = var.node_max_count
  }

  node_config {
    machine_type = var.node_machine_type
    disk_size_gb = var.node_disk_size
    disk_type    = "pd-ssd"

    oauth_scopes = [
      "https://www.googleapis.com/auth/cloud-platform"
    ]

    labels = {
      role = "general"
    }

    tags = ["openrustclaw-node"]
  }
}

# Cloud SQL PostgreSQL
resource "google_sql_database_instance" "main" {
  count = var.create_cloudsql ? 1 : 0

  name             = "${var.cluster_name}-db"
  database_version = "POSTGRES_15"
  region           = var.region

  settings {
    tier = var.cloudsql_tier

    ip_configuration {
      ipv4_enabled    = true
      private_network = google_compute_network.vpc.id
    }

    backup_configuration {
      enabled    = true
      start_time = "02:00"
    }

    maintenance_window {
      day          = 7
      hour         = 3
      update_track = "stable"
    }
  }

  depends_on = [google_project_service.apis]
}

resource "google_sql_database" "openrustclaw" {
  count = var.create_cloudsql ? 1 : 0

  name     = "openrustclaw"
  instance = google_sql_database_instance.main[0].name
}

# Cloud Memorystore Redis
resource "google_redis_instance" "main" {
  count = var.create_redis ? 1 : 0

  name               = "${var.cluster_name}-redis"
  tier               = var.redis_tier
  memory_size_gb     = var.redis_memory_size
  region             = var.region
  authorized_network = google_compute_network.vpc.id
  redis_version      = "REDIS_7_0"

  depends_on = [google_project_service.apis]
}

# Cloud Storage for backups
resource "google_storage_bucket" "backups" {
  name          = "${var.cluster_name}-backups-${var.project_id}"
  location      = var.region
  storage_class = "STANDARD"

  versioning {
    enabled = true
  }

  lifecycle_rule {
    action {
      type = "SetStorageClass"
      storage_class = "NEARLINE"
    }
    condition {
      age = 30
    }
  }

  lifecycle_rule {
    action {
      type = "SetStorageClass"
      storage_class = "COLDLINE"
    }
    condition {
      age = 90
    }
  }

  lifecycle_rule {
    action {
      type = "Delete"
    }
    condition {
      age = 365
    }
  }
}

# Cloud Load Balancer
resource "google_compute_global_address" "main" {
  count = var.create_load_balancer ? 1 : 0

  name = "${var.cluster_name}-ip"
}

# Cloud DNS
resource "google_dns_managed_zone" "main" {
  count = var.create_dns_zone ? 1 : 0

  name        = var.dns_zone_name
  dns_name    = "${var.domain_name}."
  description = "DNS zone for OpenRustClaw"
}

resource "google_dns_record_set" "main" {
  count = var.create_dns_record ? 1 : 0

  name         = "${var.subdomain}.${var.domain_name}."
  type         = "A"
  ttl          = 300
  managed_zone = var.dns_zone_name

  rrdatas = [google_compute_global_address.main[0].address]
}
