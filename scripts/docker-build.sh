#!/bin/bash
# OpenRustClaw Docker Build Script
# Builds multi-platform Docker images with caching support

set -euo pipefail

# Configuration
IMAGE_NAME="${IMAGE_NAME:-openrustclaw}"
REGISTRY="${REGISTRY:-}"  # e.g., docker.io/username or ghcr.io/username
PLATFORMS="${PLATFORMS:-linux/amd64,linux/arm64}"
DOCKERFILE="${DOCKERFILE:-Dockerfile}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Help message
show_help() {
    cat << EOF
OpenRustClaw Docker Build Script

Usage: $0 [OPTIONS] [TAG]

Arguments:
  TAG                     Image tag (default: latest)

Options:
  -h, --help              Show this help message
  -r, --registry REGISTRY Docker registry prefix
  -p, --platforms         Build platforms (default: linux/amd64,linux/arm64)
  -f, --file FILE         Dockerfile to use (default: Dockerfile)
  -c, --cache             Enable build cache
  --no-cache              Disable build cache
  --push                  Push to registry after build
  --load                  Load image to local Docker daemon
  --target TARGET         Build specific target stage
  --dev                   Build development image

Examples:
  $0 latest                              # Build latest tag
  $0 v0.1.0                             # Build versioned tag
  $0 --push v0.1.0                      # Build and push
  $0 --platforms linux/amd64 latest     # Single platform build
  $0 --dev                              # Build development image

Environment Variables:
  IMAGE_NAME              Image name (default: openrustclaw)
  REGISTRY                Registry prefix (e.g., docker.io/username)
  PLATFORMS               Target platforms
  DOCKER_BUILDKIT         Enable BuildKit (default: 1)
EOF
}

# Parse arguments
TAG="latest"
USE_CACHE=true
PUSH=false
LOAD=false
TARGET=""
DEV_MODE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        -r|--registry)
            REGISTRY="$2"
            shift 2
            ;;
        -p|--platforms)
            PLATFORMS="$2"
            shift 2
            ;;
        -f|--file)
            DOCKERFILE="$2"
            shift 2
            ;;
        -c|--cache)
            USE_CACHE=true
            shift
            ;;
        --no-cache)
            USE_CACHE=false
            shift
            ;;
        --push)
            PUSH=true
            shift
            ;;
        --load)
            LOAD=true
            shift
            ;;
        --target)
            TARGET="$2"
            shift 2
            ;;
        --dev)
            DEV_MODE=true
            DOCKERFILE="Dockerfile.dev"
            shift
            ;;
        -*)
            log_error "Unknown option: $1"
            show_help
            exit 1
            ;;
        *)
            TAG="$1"
            shift
            ;;
    esac
done

# Set full image name
if [[ -n "$REGISTRY" ]]; then
    FULL_IMAGE_NAME="${REGISTRY}/${IMAGE_NAME}"
else
    FULL_IMAGE_NAME="${IMAGE_NAME}"
fi

log_info "Building OpenRustClaw Docker image"
log_info "Image: ${FULL_IMAGE_NAME}:${TAG}"
log_info "Platforms: ${PLATFORMS}"
log_info "Dockerfile: ${DOCKERFILE}"

# Check Docker is running
if ! docker info > /dev/null 2>&1; then
    log_error "Docker is not running or not installed"
    exit 1
fi

# Enable BuildKit
export DOCKER_BUILDKIT=1

# Detect if we need buildx for multi-platform builds
CURRENT_PLATFORM=$(docker system info --format '{{.OSType}}/{{.Architecture}}')
if [[ "$PLATFORMS" == *","* ]] && [[ "$PLATFORMS" != "$CURRENT_PLATFORM" ]]; then
    log_info "Multi-platform build detected, using buildx"
    
    # Create buildx builder if it doesn't exist
    BUILDER_NAME="openrustclaw-builder"
    if ! docker buildx inspect "$BUILDER_NAME" > /dev/null 2>&1; then
        log_info "Creating buildx builder: $BUILDER_NAME"
        docker buildx create --name "$BUILDER_NAME" --driver docker-container --bootstrap
    fi
    docker buildx use "$BUILDER_NAME"
    
    USE_BUILDX=true
else
    USE_BUILDX=false
fi

# Build cache options
CACHE_OPTS=""
if [[ "$USE_CACHE" == true ]]; then
    log_info "Build cache enabled"
    CACHE_OPTS="--cache-from=type=local,src=/tmp/.buildx-cache \
                --cache-to=type=local,dest=/tmp/.buildx-cache-new,mode=max"
else
    log_info "Build cache disabled"
    CACHE_OPTS="--no-cache"
fi

# Target option
TARGET_OPTS=""
if [[ -n "$TARGET" ]]; then
    TARGET_OPTS="--target $TARGET"
    log_info "Building target stage: $TARGET"
fi

# Build command
BUILD_OPTS=""
if [[ "$USE_BUILDX" == true ]]; then
    BUILD_OPTS="--platform $PLATFORMS"
    
    if [[ "$PUSH" == true ]]; then
        BUILD_OPTS="$BUILD_OPTS --push"
        log_info "Will push to registry after build"
    elif [[ "$LOAD" == true ]]; then
        # Multi-platform builds can't be loaded directly
        log_warn "Multi-platform builds cannot be loaded to local daemon"
        log_warn "Building current platform only for local load"
        PLATFORMS="$CURRENT_PLATFORM"
        BUILD_OPTS="--platform $PLATFORMS --load"
    fi
else
    # Single platform build
    if [[ "$LOAD" == true ]] || [[ "$PUSH" == false ]]; then
        BUILD_OPTS="--load"
    fi
    if [[ "$PUSH" == true ]]; then
        BUILD_OPTS="$BUILD_OPTS --push"
        log_info "Will push to registry after build"
    fi
fi

# Build the image
log_info "Starting build..."

if [[ "$USE_BUILDX" == true ]]; then
    # Buildx build
    # shellcheck disable=SC2086
    docker buildx build \
        --file "$DOCKERFILE" \
        --tag "${FULL_IMAGE_NAME}:${TAG}" \
        --tag "${FULL_IMAGE_NAME}:latest" \
        --build-arg BUILDKIT_INLINE_CACHE=1 \
        $BUILD_OPTS \
        $CACHE_OPTS \
        $TARGET_OPTS \
        .
else
    # Standard Docker build
    # shellcheck disable=SC2086
    docker build \
        --file "$DOCKERFILE" \
        --tag "${FULL_IMAGE_NAME}:${TAG}" \
        --tag "${FULL_IMAGE_NAME}:latest" \
        --build-arg BUILDKIT_INLINE_CACHE=1 \
        $CACHE_OPTS \
        $TARGET_OPTS \
        .
    
    if [[ "$PUSH" == true ]]; then
        log_info "Pushing to registry..."
        docker push "${FULL_IMAGE_NAME}:${TAG}"
        docker push "${FULL_IMAGE_NAME}:latest"
    fi
fi

# Update cache
if [[ "$USE_CACHE" == true ]] && [[ "$USE_BUILDX" == true ]]; then
    if [[ -d /tmp/.buildx-cache-new ]]; then
        rm -rf /tmp/.buildx-cache
        mv /tmp/.buildx-cache-new /tmp/.buildx-cache
    fi
fi

# Output image info
log_success "Build completed successfully!"
log_info "Image: ${FULL_IMAGE_NAME}:${TAG}"
log_info "Platforms: ${PLATFORMS}"

# Show image size if built locally
if [[ "$PUSH" == false ]] && [[ "$USE_BUILDX" == false ]]; then
    IMAGE_SIZE=$(docker images "${FULL_IMAGE_NAME}:${TAG}" --format "{{.Size}}")
    log_info "Image size: $IMAGE_SIZE"
fi

# Run security scan if Trivy is available
if command -v trivy &> /dev/null; then
    log_info "Running security scan..."
    trivy image --severity HIGH,CRITICAL "${FULL_IMAGE_NAME}:${TAG}" || true
fi

log_success "All done!"
