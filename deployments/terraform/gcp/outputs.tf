output "cluster_endpoint" {
  description = "GKE cluster endpoint"
  value       = "https://${google_container_cluster.primary.endpoint}"
}

output "cluster_name" {
  description = "GKE cluster name"
  value       = google_container_cluster.primary.name
}

output "cluster_location" {
  description = "GKE cluster location"
  value       = google_container_cluster.primary.location
}

output "vpc_network" {
  description = "VPC network name"
  value       = google_compute_network.vpc.name
}

output "subnet_name" {
  description = "Subnet name"
  value       = google_compute_subnetwork.subnet.name
}

output "cloudsql_instance" {
  description = "Cloud SQL instance connection name"
  value       = var.create_cloudsql ? google_sql_database_instance.main[0].connection_name : null
}

output "cloudsql_private_ip" {
  description = "Cloud SQL private IP"
  value       = var.create_cloudsql ? google_sql_database_instance.main[0].private_ip_address : null
}

output "redis_endpoint" {
  description = "Redis endpoint"
  value       = var.create_redis ? "${google_redis_instance.main[0].host}:${google_redis_instance.main[0].port}" : null
}

output "storage_bucket" {
  description = "GCS bucket for backups"
  value       = google_storage_bucket.backups.name
}

output "load_balancer_ip" {
  description = "Load balancer IP address"
  value       = var.create_load_balancer ? google_compute_global_address.main[0].address : null
}

output "configure_kubectl" {
  description = "Command to configure kubectl"
  value       = "gcloud container clusters get-credentials ${google_container_cluster.primary.name} --region ${var.region} --project ${var.project_id}"
}
