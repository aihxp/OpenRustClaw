output "cluster_endpoint" {
  description = "AKS cluster endpoint"
  value       = azurerm_kubernetes_cluster.main.kube_config[0].host
}

output "cluster_name" {
  description = "AKS cluster name"
  value       = azurerm_kubernetes_cluster.main.name
}

output "cluster_resource_group" {
  description = "AKS resource group name"
  value       = azurerm_kubernetes_cluster.main.resource_group_name
}

output "resource_group_name" {
  description = "Resource group name"
  value       = azurerm_resource_group.main.name
}

output "vnet_name" {
  description = "Virtual network name"
  value       = azurerm_virtual_network.main.name
}

output "subnet_name" {
  description = "AKS subnet name"
  value       = azurerm_subnet.aks.name
}

output "postgresql_server" {
  description = "PostgreSQL server FQDN"
  value       = var.create_postgresql ? azurerm_postgresql_flexible_server.main[0].fqdn : null
}

output "redis_endpoint" {
  description = "Redis endpoint"
  value       = var.create_redis ? azurerm_redis_cache.main[0].hostname : null
}

output "storage_account" {
  description = "Storage account name"
  value       = azurerm_storage_account.backups.name
}

output "app_gateway_ip" {
  description = "Application Gateway public IP"
  value       = var.create_app_gateway ? azurerm_public_ip.main[0].ip_address : null
}

output "acr_login_server" {
  description = "ACR login server"
  value       = var.create_acr ? azurerm_container_registry.main[0].login_server : null
}

output "configure_kubectl" {
  description = "Command to configure kubectl"
  value       = "az aks get-credentials --resource-group ${azurerm_resource_group.main.name} --name ${azurerm_kubernetes_cluster.main.name}"
}
