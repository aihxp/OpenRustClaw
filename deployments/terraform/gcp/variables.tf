variable "project_id" {
  description = "GCP project ID"
  type        = string
}

variable "cluster_name" {
  description = "Name of the GKE cluster"
  type        = string
  default     = "openrustclaw"
}

variable "environment" {
  description = "Environment name"
  type        = string
  default     = "production"
}

variable "region" {
  description = "GCP region"
  type        = string
  default     = "us-central1"
}

variable "zone" {
  description = "GCP zone (leave empty for regional cluster)"
  type        = string
  default     = ""
}

variable "kubernetes_version" {
  description = "Kubernetes version"
  type        = string
  default     = "1.28"
}

variable "subnet_cidr" {
  description = "CIDR for subnet"
  type        = string
  default     = "10.0.0.0/24"
}

variable "pods_cidr" {
  description = "CIDR for pods"
  type        = string
  default     = "10.1.0.0/16"
}

variable "services_cidr" {
  description = "CIDR for services"
  type        = string
  default     = "10.2.0.0/16"
}

variable "master_cidr" {
  description = "CIDR for master nodes"
  type        = string
  default     = "172.16.0.0/28"
}

variable "enable_autopilot" {
  description = "Enable GKE Autopilot mode"
  type        = bool
  default     = false
}

variable "node_machine_type" {
  description = "Machine type for nodes"
  type        = string
  default     = "e2-standard-4"
}

variable "node_disk_size" {
  description = "Disk size for nodes in GB"
  type        = number
  default     = 100
}

variable "node_count" {
  description = "Number of nodes"
  type        = number
  default     = 3
}

variable "node_min_count" {
  description = "Minimum number of nodes"
  type        = number
  default     = 2
}

variable "node_max_count" {
  description = "Maximum number of nodes"
  type        = number
  default     = 10
}

variable "maintenance_start_time" {
  description = "Maintenance window start time"
  type        = string
  default     = "2024-01-01T02:00:00Z"
}

variable "maintenance_end_time" {
  description = "Maintenance window end time"
  type        = string
  default     = "2024-01-01T06:00:00Z"
}

variable "maintenance_recurrence" {
  description = "Maintenance recurrence (RFC 5545)"
  type        = string
  default     = "FREQ=WEEKLY;BYDAY=SA,SU"
}

variable "create_cloudsql" {
  description = "Create Cloud SQL instance"
  type        = bool
  default     = false
}

variable "cloudsql_tier" {
  description = "Cloud SQL machine tier"
  type        = string
  default     = "db-f1-micro"
}

variable "create_redis" {
  description = "Create Cloud Memorystore Redis"
  type        = bool
  default     = false
}

variable "redis_tier" {
  description = "Redis tier (BASIC or STANDARD_HA)"
  type        = string
  default     = "BASIC"
}

variable "redis_memory_size" {
  description = "Redis memory size in GB"
  type        = number
  default     = 1
}

variable "create_load_balancer" {
  description = "Create Cloud Load Balancer"
  type        = bool
  default     = false
}

variable "create_dns_zone" {
  description = "Create Cloud DNS zone"
  type        = bool
  default     = false
}

variable "dns_zone_name" {
  description = "Cloud DNS zone name"
  type        = string
  default     = "openrustclaw-zone"
}

variable "create_dns_record" {
  description = "Create DNS record"
  type        = bool
  default     = false
}

variable "domain_name" {
  description = "Domain name"
  type        = string
  default     = ""
}

variable "subdomain" {
  description = "Subdomain"
  type        = string
  default     = "api"
}
