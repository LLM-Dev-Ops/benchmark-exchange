# Makefile for LLM Benchmark Exchange
# Provides convenient commands for development, testing, and deployment

.PHONY: help build test lint format clean docker-build docker-push k8s-apply k8s-delete helm-install helm-uninstall db-migrate

# Default target
.DEFAULT_GOAL := help

# =============================================================================
# Configuration
# =============================================================================

# Database configuration
DB_NAME := llm_benchmark_exchange
DB_USER := postgres
DB_PASSWORD := postgres
DB_HOST := localhost
DB_PORT := 5432
DATABASE_URL := postgresql://$(DB_USER):$(DB_PASSWORD)@$(DB_HOST):$(DB_PORT)/$(DB_NAME)

# Docker configuration
DOCKER_REGISTRY := ghcr.io
DOCKER_ORG := llm-benchmark-exchange
IMAGE_TAG := latest
API_IMAGE := $(DOCKER_REGISTRY)/$(DOCKER_ORG)/api:$(IMAGE_TAG)
WORKER_IMAGE := $(DOCKER_REGISTRY)/$(DOCKER_ORG)/worker:$(IMAGE_TAG)

# Kubernetes configuration
K8S_NAMESPACE := llm-benchmark
K8S_CONTEXT := $(shell kubectl config current-context)
HELM_RELEASE := llm-benchmark

# Colors for output
BLUE := \033[0;34m
GREEN := \033[0;32m
YELLOW := \033[1;33m
RED := \033[0;31m
NC := \033[0m # No Color

## help: Show this help message
help:
	@echo "$(BLUE)╔════════════════════════════════════════════════════════════╗$(NC)"
	@echo "$(BLUE)║     LLM Benchmark Exchange - Makefile                     ║$(NC)"
	@echo "$(BLUE)╚════════════════════════════════════════════════════════════╝$(NC)"
	@echo ""
	@echo "Available commands:"
	@echo ""
	@grep -E '^## ' Makefile | sed 's/## /  $(GREEN)/g' | sed 's/:/ $(NC)-/g'
	@echo ""

# =============================================================================
# Development Commands
# =============================================================================

## build: Build all Rust projects
build:
	@echo "$(BLUE)Building all projects...$(NC)"
	@cargo build --release --all-features
	@echo "$(GREEN)Build complete$(NC)"

## test: Run all tests
test:
	@echo "$(BLUE)Running tests...$(NC)"
	@cargo test --all-features --workspace
	@echo "$(GREEN)Tests complete$(NC)"

## lint: Run clippy linter
lint:
	@echo "$(BLUE)Running clippy...$(NC)"
	@cargo clippy --all-targets --all-features -- -D warnings
	@echo "$(GREEN)Linting complete$(NC)"

## format: Format code with rustfmt
format:
	@echo "$(BLUE)Formatting code...$(NC)"
	@cargo fmt --all
	@echo "$(GREEN)Formatting complete$(NC)"

## format-check: Check code formatting
format-check:
	@echo "$(BLUE)Checking code formatting...$(NC)"
	@cargo fmt --all -- --check
	@echo "$(GREEN)Format check complete$(NC)"

## clean: Clean build artifacts
clean:
	@echo "$(BLUE)Cleaning build artifacts...$(NC)"
	@cargo clean
	@rm -rf .sqlx/
	@echo "$(GREEN)Clean complete$(NC)"

# =============================================================================
# Docker Commands
# =============================================================================

## docker-build: Build Docker images
docker-build:
	@echo "$(BLUE)Building Docker images...$(NC)"
	@docker build -f docker/Dockerfile -t $(API_IMAGE) .
	@docker build -f docker/Dockerfile.worker -t $(WORKER_IMAGE) .
	@echo "$(GREEN)Docker images built$(NC)"

## docker-push: Push Docker images to registry
docker-push:
	@echo "$(BLUE)Pushing Docker images...$(NC)"
	@docker push $(API_IMAGE)
	@docker push $(WORKER_IMAGE)
	@echo "$(GREEN)Docker images pushed$(NC)"

## docker-build-push: Build and push Docker images
docker-build-push: docker-build docker-push
	@echo "$(GREEN)Docker build and push complete$(NC)"

## docker-run: Run services with Docker Compose
docker-run:
	@echo "$(BLUE)Starting services with Docker Compose...$(NC)"
	@docker-compose up -d
	@echo "$(GREEN)Services started$(NC)"

## docker-stop: Stop Docker Compose services
docker-stop:
	@echo "$(BLUE)Stopping Docker Compose services...$(NC)"
	@docker-compose down
	@echo "$(GREEN)Services stopped$(NC)"

## docker-logs: Show Docker Compose logs
docker-logs:
	@docker-compose logs -f

# =============================================================================
# Kubernetes Commands
# =============================================================================

## k8s-namespace: Create Kubernetes namespace
k8s-namespace:
	@echo "$(BLUE)Creating Kubernetes namespace...$(NC)"
	@kubectl apply -f k8s/namespace.yaml
	@echo "$(GREEN)Namespace created$(NC)"

## k8s-secrets: Create Kubernetes secrets (WARNING: update secrets.yaml first!)
k8s-secrets:
	@echo "$(YELLOW)WARNING: Make sure you've updated k8s/secrets.yaml with real values!$(NC)"
	@read -p "Continue? (yes/no): " confirm && [ "$$confirm" = "yes" ] || exit 1
	@kubectl apply -f k8s/secrets.yaml
	@echo "$(GREEN)Secrets created$(NC)"

## k8s-apply: Apply all Kubernetes manifests
k8s-apply: k8s-namespace
	@echo "$(BLUE)Applying Kubernetes manifests...$(NC)"
	@kubectl apply -f k8s/configmap.yaml
	@kubectl apply -f k8s/deployment-api.yaml
	@kubectl apply -f k8s/deployment-worker.yaml
	@kubectl apply -f k8s/service.yaml
	@kubectl apply -f k8s/ingress.yaml
	@kubectl apply -f k8s/hpa.yaml
	@kubectl apply -f k8s/pdb.yaml
	@echo "$(GREEN)Kubernetes manifests applied$(NC)"

## k8s-delete: Delete all Kubernetes resources
k8s-delete:
	@echo "$(RED)WARNING: This will delete all Kubernetes resources!$(NC)"
	@read -p "Are you sure? (yes/no): " confirm && [ "$$confirm" = "yes" ] || exit 1
	@kubectl delete -f k8s/hpa.yaml --ignore-not-found
	@kubectl delete -f k8s/pdb.yaml --ignore-not-found
	@kubectl delete -f k8s/ingress.yaml --ignore-not-found
	@kubectl delete -f k8s/service.yaml --ignore-not-found
	@kubectl delete -f k8s/deployment-worker.yaml --ignore-not-found
	@kubectl delete -f k8s/deployment-api.yaml --ignore-not-found
	@kubectl delete -f k8s/configmap.yaml --ignore-not-found
	@kubectl delete -f k8s/namespace.yaml --ignore-not-found
	@echo "$(GREEN)Kubernetes resources deleted$(NC)"

## k8s-status: Show Kubernetes resource status
k8s-status:
	@echo "$(BLUE)Kubernetes Status:$(NC)"
	@echo ""
	@echo "$(GREEN)Namespace:$(NC)"
	@kubectl get namespace $(K8S_NAMESPACE) 2>/dev/null || echo "  $(RED)Namespace not found$(NC)"
	@echo ""
	@echo "$(GREEN)Deployments:$(NC)"
	@kubectl get deployments -n $(K8S_NAMESPACE)
	@echo ""
	@echo "$(GREEN)Pods:$(NC)"
	@kubectl get pods -n $(K8S_NAMESPACE)
	@echo ""
	@echo "$(GREEN)Services:$(NC)"
	@kubectl get services -n $(K8S_NAMESPACE)
	@echo ""

## k8s-logs-api: Show API logs
k8s-logs-api:
	@kubectl logs -f -n $(K8S_NAMESPACE) -l app=llm-benchmark-exchange,component=api

## k8s-logs-worker: Show Worker logs
k8s-logs-worker:
	@kubectl logs -f -n $(K8S_NAMESPACE) -l app=llm-benchmark-exchange,component=worker

# =============================================================================
# Helm Commands
# =============================================================================

## helm-lint: Lint Helm chart
helm-lint:
	@echo "$(BLUE)Linting Helm chart...$(NC)"
	@helm lint ./helm
	@echo "$(GREEN)Helm chart linting complete$(NC)"

## helm-template: Template Helm chart
helm-template:
	@echo "$(BLUE)Templating Helm chart...$(NC)"
	@helm template $(HELM_RELEASE) ./helm --values ./helm/values.yaml

## helm-install: Install with Helm
helm-install: helm-lint
	@echo "$(BLUE)Installing with Helm...$(NC)"
	@helm upgrade --install $(HELM_RELEASE) ./helm \
		--namespace $(K8S_NAMESPACE) \
		--create-namespace \
		--values ./helm/values.yaml \
		--wait \
		--timeout 10m
	@echo "$(GREEN)Helm installation complete$(NC)"

## helm-upgrade: Upgrade Helm release
helm-upgrade:
	@echo "$(BLUE)Upgrading Helm release...$(NC)"
	@helm upgrade $(HELM_RELEASE) ./helm \
		--namespace $(K8S_NAMESPACE) \
		--values ./helm/values.yaml \
		--wait \
		--timeout 10m
	@echo "$(GREEN)Helm upgrade complete$(NC)"

## helm-uninstall: Uninstall Helm release
helm-uninstall:
	@echo "$(BLUE)Uninstalling Helm release...$(NC)"
	@helm uninstall $(HELM_RELEASE) --namespace $(K8S_NAMESPACE)
	@echo "$(GREEN)Helm uninstallation complete$(NC)"

## helm-status: Show Helm release status
helm-status:
	@helm status $(HELM_RELEASE) --namespace $(K8S_NAMESPACE)

# =============================================================================
# Database Commands
# =============================================================================

## db-up: Start PostgreSQL database using Docker Compose
db-up:
	@echo "$(BLUE)Starting PostgreSQL database...$(NC)"
	@docker-compose up -d postgres
	@echo "$(GREEN)Waiting for database to be ready...$(NC)"
	@sleep 5
	@docker-compose exec -T postgres pg_isready -U $(DB_USER) || (echo "$(RED)Database not ready$(NC)" && exit 1)
	@echo "$(GREEN)Database is ready!$(NC)"

## db-down: Stop PostgreSQL database
db-down:
	@echo "$(BLUE)Stopping PostgreSQL database...$(NC)"
	@docker-compose down
	@echo "$(GREEN)Database stopped$(NC)"

## db-reset: Reset database (WARNING: destroys all data)
db-reset: db-down
	@echo "$(RED)WARNING: This will delete all data!$(NC)"
	@read -p "Are you sure? (yes/no): " confirm && [ "$$confirm" = "yes" ] || (echo "$(YELLOW)Cancelled$(NC)" && exit 1)
	@echo "$(BLUE)Removing database volumes...$(NC)"
	@docker-compose down -v
	@echo "$(GREEN)Database volumes removed$(NC)"
	@$(MAKE) db-up
	@echo "$(GREEN)Database reset complete$(NC)"

## db-migrate: Run all database migrations
db-migrate: db-up
	@echo "$(BLUE)Running database migrations...$(NC)"
	@export DATABASE_URL=$(DATABASE_URL) && ./migrations/run_migrations.sh
	@echo "$(GREEN)Migrations complete$(NC)"

## db-validate: Validate database schema
db-validate:
	@echo "$(BLUE)Validating database schema...$(NC)"
	@psql $(DATABASE_URL) -f migrations/validate_schema.sql
	@echo "$(GREEN)Validation complete$(NC)"

## db-shell: Open PostgreSQL shell (psql)
db-shell:
	@echo "$(BLUE)Opening PostgreSQL shell...$(NC)"
	@psql $(DATABASE_URL)

## db-backup: Create database backup
db-backup:
	@echo "$(BLUE)Creating database backup...$(NC)"
	@mkdir -p backups
	@pg_dump $(DATABASE_URL) | gzip > backups/backup_$(shell date +%Y%m%d_%H%M%S).sql.gz
	@echo "$(GREEN)Backup created in backups/ directory$(NC)"

## db-restore: Restore from latest backup
db-restore:
	@echo "$(BLUE)Restoring from latest backup...$(NC)"
	@LATEST=$$(ls -t backups/*.sql.gz | head -1); \
	if [ -z "$$LATEST" ]; then \
		echo "$(RED)No backups found$(NC)"; \
		exit 1; \
	fi; \
	echo "$(YELLOW)Restoring from: $$LATEST$(NC)"; \
	gunzip -c $$LATEST | psql $(DATABASE_URL)
	@echo "$(GREEN)Restore complete$(NC)"

## db-logs: Show PostgreSQL logs
db-logs:
	@docker-compose logs -f postgres

## db-status: Show database status and connection info
db-status:
	@echo "$(BLUE)╔════════════════════════════════════════════════════════════╗$(NC)"
	@echo "$(BLUE)║                    Database Status                         ║$(NC)"
	@echo "$(BLUE)╚════════════════════════════════════════════════════════════╝$(NC)"
	@echo ""
	@echo "$(GREEN)Connection Info:$(NC)"
	@echo "  Host:     $(DB_HOST)"
	@echo "  Port:     $(DB_PORT)"
	@echo "  Database: $(DB_NAME)"
	@echo "  User:     $(DB_USER)"
	@echo "  URL:      $(DATABASE_URL)"
	@echo ""
	@echo "$(GREEN)Docker Status:$(NC)"
	@docker-compose ps postgres || echo "  $(RED)Container not running$(NC)"
	@echo ""
	@echo "$(GREEN)Database Info:$(NC)"
	@psql $(DATABASE_URL) -c "SELECT version();" 2>/dev/null || echo "  $(RED)Cannot connect to database$(NC)"
	@echo ""

## db-init: Initialize fresh database with migrations
db-init: db-up db-migrate
	@echo "$(BLUE)Creating initial partitions...$(NC)"
	@psql $(DATABASE_URL) -c "SELECT create_next_month_partitions();"
	@echo "$(BLUE)Refreshing materialized views...$(NC)"
	@psql $(DATABASE_URL) -c "SELECT refresh_all_materialized_views();" || echo "$(YELLOW)No data to refresh yet$(NC)"
	@echo "$(GREEN)Database initialization complete!$(NC)"

## db-refresh-views: Refresh all materialized views
db-refresh-views:
	@echo "$(BLUE)Refreshing materialized views...$(NC)"
	@psql $(DATABASE_URL) -c "SELECT refresh_all_materialized_views();"
	@echo "$(GREEN)Materialized views refreshed$(NC)"

## db-partitions: Create next month's partitions
db-partitions:
	@echo "$(BLUE)Creating next month's partitions...$(NC)"
	@psql $(DATABASE_URL) -c "SELECT create_next_month_partitions();"
	@echo "$(GREEN)Partitions created$(NC)"

## db-vacuum: Run VACUUM ANALYZE on all tables
db-vacuum:
	@echo "$(BLUE)Running VACUUM ANALYZE...$(NC)"
	@psql $(DATABASE_URL) -c "VACUUM ANALYZE;"
	@echo "$(GREEN)VACUUM ANALYZE complete$(NC)"

## db-size: Show database and table sizes
db-size:
	@echo "$(BLUE)Database Size Information:$(NC)"
	@echo ""
	@psql $(DATABASE_URL) -c "SELECT pg_size_pretty(pg_database_size('$(DB_NAME)')) AS database_size;"
	@echo ""
	@echo "$(BLUE)Largest Tables:$(NC)"
	@psql $(DATABASE_URL) -c "\
		SELECT \
			schemaname, \
			tablename, \
			pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS total_size \
		FROM pg_tables \
		WHERE schemaname = 'public' \
		ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC \
		LIMIT 10;"

## db-test: Run database tests (placeholder)
db-test:
	@echo "$(BLUE)Running database tests...$(NC)"
	@echo "$(YELLOW)No tests configured yet$(NC)"

## sqlx-prepare: Prepare SQLx offline data for Rust
sqlx-prepare:
	@echo "$(BLUE)Preparing SQLx offline data...$(NC)"
	@export DATABASE_URL=$(DATABASE_URL) && cargo sqlx prepare
	@echo "$(GREEN)SQLx offline data prepared$(NC)"

## sqlx-migrate: Run migrations using SQLx CLI
sqlx-migrate:
	@echo "$(BLUE)Running migrations with SQLx...$(NC)"
	@export DATABASE_URL=$(DATABASE_URL) && sqlx migrate run
	@echo "$(GREEN)SQLx migrations complete$(NC)"

## clean: Clean up generated files and temporary data
clean:
	@echo "$(BLUE)Cleaning up...$(NC)"
	@rm -rf .sqlx/
	@echo "$(GREEN)Cleanup complete$(NC)"

## pgadmin-up: Start pgAdmin web interface
pgadmin-up:
	@echo "$(BLUE)Starting pgAdmin...$(NC)"
	@docker-compose up -d pgadmin
	@echo "$(GREEN)pgAdmin available at http://localhost:5050$(NC)"
	@echo "$(YELLOW)Email: admin@llm-benchmark.local$(NC)"
	@echo "$(YELLOW)Password: admin$(NC)"

## redis-up: Start Redis cache
redis-up:
	@echo "$(BLUE)Starting Redis...$(NC)"
	@docker-compose up -d redis
	@echo "$(GREEN)Redis started on port 6379$(NC)"

## all-up: Start all services (PostgreSQL, Redis, pgAdmin)
all-up:
	@echo "$(BLUE)Starting all services...$(NC)"
	@docker-compose up -d
	@echo "$(GREEN)All services started!$(NC)"
	@echo ""
	@echo "Services available:"
	@echo "  PostgreSQL: localhost:5432"
	@echo "  Redis:      localhost:6379"
	@echo "  pgAdmin:    http://localhost:5050"

## all-down: Stop all services
all-down:
	@echo "$(BLUE)Stopping all services...$(NC)"
	@docker-compose down
	@echo "$(GREEN)All services stopped$(NC)"
