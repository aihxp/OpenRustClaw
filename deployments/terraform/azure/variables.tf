variable "cluster_name" {
  description = "Name of the AKS cluster"
  type        = string
  default     = "openrustclaw"
}

variable "environment" {
  description = "Environment name"
  type        = string
  default     = "production"
}

variable "location" {
  description = "Azure region"
  type        = string
  default     = "West US 2"
}

variable "kubernetes_version" {
  description = "Kubernetes version"
  type        = string
  default     = "1.28"
}

variable "vnet_cidr" {
  description = "CIDR for virtual network"
  type        = string
  default     = "10.0.0.0/16"
}

variable "aks_subnet_cidr" {
  description = "CIDR for AKS subnet"
  type        = string
  default     = "10.0.1.0/24"
}

variable "database_subnet_cidr" {
  description = "CIDR for database subnet"
  type        = string
  default     = "10.0.2.0/24"
}

variable "service_cidr" {
  description = "CIDR for Kubernetes services"
  type        = string
  default     = "10.1.0.0/16"
}

variable "dns_service_ip" {
  description = "DNS service IP"
  type        = string
  default     = "10.1.0.10"
}

variable "zones" {
  description = "Availability zones"
  type        = list(string)
  default     = ["1", "2", "3"]
}

variable "node_vm_size" {
  description = "VM size for nodes"
  type        = string
  default     = "Standard_D4s_v3"
}

variable "node_os_disk_size" {
  description = "OS disk size for nodes in GB"
  type        = number
  default     = 128
}

variable "node_count" {
  description = "Initial node count"
  type        = number
  default     = 3
}

variable "node_min_count" {
  description = "Minimum node count"
  type        = number
  default     = 2
}

variable "node_max_count" {
  description = "Maximum node count"
  type        = number
  default     = 10
}

variable "enable_azure_policy" {
  description = "Enable Azure Policy for Kubernetes"
  type        = bool
  default     = true
}

variable "enable_monitoring" {
  description = "Enable monitoring with Log Analytics"
  type        = bool
  default     = true
}

variable "log_retention_days" {
  description = "Log retention in days"
  type        = number
  default     = 30
}

variable "create_postgresql" {
  description = "Create Azure Database for PostgreSQL"
  type        = bool
  default     = false
}

variable "postgresql_sku" {
  description = "PostgreSQL SKU"
  type        = string
  default     = "B_Standard_B1ms"
}

variable "postgresql_storage_mb" {
  description = "PostgreSQL storage in MB"
  type        = number
  default     = 32768
}

variable "postgresql_admin_password" {
  description = "PostgreSQL admin password"
  type        = string
  default     = ""
  sensitive   = true
}

variable "create_redis" {
  description = "Create Azure Cache for Redis"
  type        = bool
  default     = false
}

variable "redis_capacity" {
  description = "Redis capacity (0-6 for Basic/Standard, 1-5 for Premium)"
  type        = number
  default     = 1
}

variable "redis_family" {
  description = "Redis family (C or P)"
  type        = string
  default     = "C"
}

variable "redis_sku" {
  description = "Redis SKU (Basic, Standard, Premium)"
  type        = string
  default     = "Standard"
}

variable "create_app_gateway" {
  description = "Create Application Gateway"
  type        = bool
  default     = false
}

variable "app_gateway_sku" {
  description = "Application Gateway SKU"
  type        = string
  default     = "Standard_v2"
}

variable "app_gateway_tier" {
  description = "Application Gateway tier"
  type        = string
  default     = "Standard_v2"
}

variable "create_dns_zone" {
  description = "Create DNS zone"
  type        = bool
  default     = false
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

variable "create_acr" {
  description = "Create Azure Container Registry"
  type        = bool
  default     = false
}

variable "tags" {
  description = "Tags to apply to all resources"
  type        = map(string)
  default = {
    ManagedBy = "terraform"
    Project   = "openrustclaw"
  }
}
