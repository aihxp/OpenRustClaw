# OpenRustClaw Makefile
# Simplified commands for Docker operations

.PHONY: help build build-dev run start stop logs shell clean test docker-push

# Default target
.DEFAULT_GOAL := help

# Variables
IMAGE_NAME ?= openrustclaw
REGISTRY ?=
TAG ?= latest

help: ## Show this help message
	@echo "OpenRustClaw Docker Commands"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

# Build commands
build: ## Build production Docker image
	./scripts/docker-build.sh $(TAG)

build-dev: ## Build development Docker image
	./scripts/docker-build.sh --dev latest

build-multi: ## Build multi-platform image
	./scripts/docker-build.sh --platforms linux/amd64,linux/arm64 $(TAG)

# Run commands
run: ## Run OpenRustClaw container (requires .env)
	./scripts/docker-run.sh start -d

run-dev: ## Run development environment with hot reload
	docker-compose -f docker-compose.dev.yml up -d

start: ## Start OpenRustClaw with docker-compose
	docker-compose up -d

stop: ## Stop OpenRustClaw containers
	docker-compose down
	./scripts/docker-run.sh stop 2>/dev/null || true

dev-stop: ## Stop development environment
	docker-compose -f docker-compose.dev.yml down

# Utility commands
logs: ## View container logs
	docker-compose logs -f

logs-dev: ## View development logs
	docker-compose -f docker-compose.dev.yml logs -f

shell: ## Open shell in running container
	docker exec -it openrustclaw /bin/bash

status: ## Check container status
	./scripts/docker-run.sh status

update: ## Pull latest image and restart
	./scripts/docker-run.sh update

# Cleanup
clean: ## Remove containers and volumes (WARNING: data loss!)
	./scripts/docker-run.sh clean

clean-all: ## Remove all containers, volumes, and images
	docker-compose down -v --rmi all
	docker rmi $(IMAGE_NAME):latest 2>/dev/null || true

docker-prune: ## Clean up unused Docker resources
	docker system prune -f
	docker volume prune -f

# Development utilities
dev-db: ## Start development environment with database UI
	docker-compose -f docker-compose.dev.yml --profile db-ui up -d

dev-monitoring: ## Start development environment with monitoring
	docker-compose -f docker-compose.dev.yml --profile monitoring up -d

dev-all: ## Start development environment with all services
	docker-compose -f docker-compose.dev.yml --profile db-ui --profile monitoring up -d

# Testing
test: ## Run tests locally
	cargo test --workspace

docker-test: ## Run tests in Docker container
	docker run --rm -v $(PWD):/app $(IMAGE_NAME):latest cargo test --workspace

# Registry operations
push: ## Push image to registry (set REGISTRY env var)
	./scripts/docker-push.sh --latest $(TAG)

push-multi: ## Build and push multi-platform image
	./scripts/docker-build.sh --platforms linux/amd64,linux/arm64 --push $(TAG)

# Security
scan: ## Scan Docker image for vulnerabilities
	trivy image $(IMAGE_NAME):latest || echo "Install Trivy: https://aquasecurity.github.io/trivy/"

# Database
backup: ## Backup database
	./scripts/docker-run.sh backup

restore: ## Restore database (usage: make restore FILE=backup_xxx.sql)
	./scripts/docker-run.sh restore $(FILE)

# Configuration
env: ## Create environment file from template
	cp .env.docker .env
	@echo "Edit .env with your API keys"

config-check: ## Validate Docker Compose configuration
	docker-compose config
