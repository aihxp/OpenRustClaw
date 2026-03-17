#!/bin/bash
# OpenRustClaw Docker Run Helper Script
# Simplifies running the OpenRustClaw container

set -euo pipefail

# Configuration
IMAGE_NAME="${IMAGE_NAME:-openrustclaw}"
CONTAINER_NAME="${CONTAINER_NAME:-openrustclaw}"
DATA_DIR="${DATA_DIR:-./data}"
CONFIG_DIR="${CONFIG_DIR:-./config}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

show_help() {
    cat << EOF
OpenRustClaw Docker Run Script

Usage: $0 [COMMAND] [OPTIONS]

Commands:
  start           Start OpenRustClaw container
  stop            Stop OpenRustClaw container
  restart         Restart OpenRustClaw container
  logs            View container logs
  shell           Open shell in running container
  status          Check container status
  update          Pull and restart with latest image
  backup          Backup database
  restore         Restore database from backup
  clean           Remove container and volumes (WARNING: data loss!)

Options:
  -d, --detach    Run in detached mode
  -p, --port      Gateway port (default: 18789)
  --tag           Image tag (default: latest)
  -h, --help      Show this help message

Examples:
  $0 start                    # Start with default settings
  $0 start -d -p 8080         # Start detached on port 8080
  $0 logs -f                  # Follow logs
  $0 shell                    # Open container shell
  $0 backup                   # Create database backup
EOF
}

# Ensure data directory exists
mkdir -p "$DATA_DIR"

# Parse command
COMMAND="${1:-start}"
shift || true

# Parse options
DETACH=false
PORT=18789
TAG="latest"

while [[ $# -gt 0 ]]; do
    case $1 in
        -d|--detach)
            DETACH=true
            shift
            ;;
        -p|--port)
            PORT="$2"
            shift 2
            ;;
        --tag)
            TAG="$2"
            shift 2
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        -f)
            # Used with logs command
            shift
            ;;
        *)
            log_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Load environment variables from .env if exists
if [[ -f .env ]]; then
    # shellcheck source=/dev/null
    set -a && source .env && set +a
fi

case $COMMAND in
    start)
        log_info "Starting OpenRustClaw..."
        
        # Check if already running
        if docker ps -q -f name="$CONTAINER_NAME" | grep -q .; then
            log_warn "Container $CONTAINER_NAME is already running"
            exit 0
        fi
        
        # Remove stopped container if exists
        if docker ps -aq -f name="$CONTAINER_NAME" | grep -q .; then
            log_info "Removing stopped container..."
            docker rm "$CONTAINER_NAME" > /dev/null
        fi
        
        # Build run options
        RUN_OPTS=""
        if [[ "$DETACH" == true ]]; then
            RUN_OPTS="$RUN_OPTS -d"
        fi
        
        # Port mapping
        RUN_OPTS="$RUN_OPTS -p $PORT:18789"
        
        # Environment variables
        ENV_VARS=""
        [[ -n "${ANTHROPIC_API_KEY:-}" ]] && ENV_VARS="$ENV_VARS -e ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY"
        [[ -n "${OPENAI_API_KEY:-}" ]] && ENV_VARS="$ENV_VARS -e OPENAI_API_KEY=$OPENAI_API_KEY"
        [[ -n "${OPENROUTER_API_KEY:-}" ]] && ENV_VARS="$ENV_VARS -e OPENROUTER_API_KEY=$OPENROUTER_API_KEY"
        [[ -n "${LANGSMITH_API_KEY:-}" ]] && ENV_VARS="$ENV_VARS -e LANGSMITH_API_KEY=$LANGSMITH_API_KEY"
        [[ -n "${AUTH_SECRET:-}" ]] && ENV_VARS="$ENV_VARS -e AUTH_SECRET=$AUTH_SECRET"
        ENV_VARS="$ENV_VARS -e RUST_LOG=${RUST_LOG:-info}"
        ENV_VARS="$ENV_VARS -e GATEWAY_HOST=0.0.0.0"
        ENV_VARS="$ENV_VARS -e GATEWAY_PORT=18789"
        ENV_VARS="$ENV_VARS -e DATABASE_URL=sqlite:///app/data/openrustclaw.db"
        
        # Run container
        # shellcheck disable=SC2086
        docker run $RUN_OPTS \
            --name "$CONTAINER_NAME" \
            --restart unless-stopped \
            -v "$(realpath $DATA_DIR):/app/data" \
            -v "$(realpath $CONFIG_DIR):/app/config:ro" \
            $ENV_VARS \
            "${IMAGE_NAME}:${TAG}"
        
        if [[ "$DETACH" == true ]]; then
            log_success "OpenRustClaw started!"
            log_info "Gateway: http://localhost:$PORT"
            log_info "Logs: $0 logs"
        fi
        ;;
    
    stop)
        log_info "Stopping OpenRustClaw..."
        docker stop "$CONTAINER_NAME" 2>/dev/null || log_warn "Container not running"
        log_success "Stopped"
        ;;
    
    restart)
        log_info "Restarting OpenRustClaw..."
        docker restart "$CONTAINER_NAME"
        log_success "Restarted"
        ;;
    
    logs)
        log_info "Viewing logs..."
        docker logs "$CONTAINER_NAME" -f
        ;;
    
    shell)
        log_info "Opening shell in container..."
        docker exec -it "$CONTAINER_NAME" /bin/bash
        ;;
    
    status)
        if docker ps -q -f name="$CONTAINER_NAME" | grep -q .; then
            log_success "OpenRustClaw is running"
            docker ps -f name="$CONTAINER_NAME"
            echo ""
            log_info "Health check:"
            curl -s http://localhost:$PORT/health || echo "Health endpoint not responding"
        else
            log_warn "OpenRustClaw is not running"
        fi
        ;;
    
    update)
        log_info "Updating OpenRustClaw..."
        docker pull "${IMAGE_NAME}:${TAG}"
        $0 stop
        $0 start -d -p "$PORT"
        log_success "Updated to ${IMAGE_NAME}:${TAG}"
        ;;
    
    backup)
        BACKUP_FILE="backup_$(date +%Y%m%d_%H%M%S).sql"
        log_info "Creating database backup: $BACKUP_FILE"
        docker exec "$CONTAINER_NAME" sqlite3 /app/data/openrustclaw.db ".dump" > "$BACKUP_FILE"
        log_success "Backup created: $BACKUP_FILE"
        ;;
    
    restore)
        if [[ $# -eq 0 ]]; then
            log_error "Backup file required"
            echo "Usage: $0 restore <backup_file>"
            exit 1
        fi
        BACKUP_FILE="$1"
        if [[ ! -f "$BACKUP_FILE" ]]; then
            log_error "Backup file not found: $BACKUP_FILE"
            exit 1
        fi
        log_info "Restoring database from: $BACKUP_FILE"
        docker exec -i "$CONTAINER_NAME" sqlite3 /app/data/openrustclaw.db < "$BACKUP_FILE"
        log_success "Database restored"
        ;;
    
    clean)
        log_warn "This will remove the container and all data!"
        read -p "Are you sure? [y/N] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            docker stop "$CONTAINER_NAME" 2>/dev/null || true
            docker rm "$CONTAINER_NAME" 2>/dev/null || true
            docker volume rm "${CONTAINER_NAME}_data" 2>/dev/null || true
            log_success "Cleaned up"
        else
            log_info "Cancelled"
        fi
        ;;
    
    *)
        log_error "Unknown command: $COMMAND"
        show_help
        exit 1
        ;;
esac
