#!/bin/bash
# OpenRustClaw Docker Push Script
# Pushes images to registry with multi-tag support

set -euo pipefail

# Configuration
IMAGE_NAME="${IMAGE_NAME:-openrustclaw}"
REGISTRY="${REGISTRY:-}"
DEFAULT_PLATFORMS="linux/amd64,linux/arm64"

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

# Help message
show_help() {
    cat << EOF
OpenRustClaw Docker Push Script

Usage: $0 [OPTIONS] TAG

Arguments:
  TAG                     Image tag to push (required)

Options:
  -h, --help              Show this help message
  -r, --registry REGISTRY Docker registry (default: from REGISTRY env var)
  --build                 Build before pushing
  --platforms PLATFORMS   Target platforms (default: linux/amd64,linux/arm64)
  --latest                Also tag as latest
  --all-tags              Push all tags for this image
  --dry-run               Show what would be pushed without pushing

Environment Variables:
  IMAGE_NAME              Image name (default: openrustclaw)
  REGISTRY                Registry URL (e.g., docker.io/username, ghcr.io/org)
  DOCKER_USERNAME         Username for registry login
  DOCKER_PASSWORD         Password/token for registry login

Examples:
  $0 v0.1.0                               # Push version tag
  $0 --latest v0.1.0                      # Push v0.1.0 and latest
  $0 --registry ghcr.io/myorg v0.1.0      # Push to specific registry
  $0 --build --latest v0.1.0              # Build then push
  REGISTRY=docker.io/user $0 v0.1.0       # Using env var
EOF
}

# Parse arguments
TAG=""
BUILD=false
PLATFORMS=""
TAG_LATEST=false
ALL_TAGS=false
DRY_RUN=false

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
        --build)
            BUILD=true
            shift
            ;;
        --platforms)
            PLATFORMS="$2"
            shift 2
            ;;
        --latest)
            TAG_LATEST=true
            shift
            ;;
        --all-tags)
            ALL_TAGS=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
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

# Validate required arguments
if [[ -z "$TAG" ]]; then
    log_error "TAG is required"
    show_help
    exit 1
fi

if [[ -z "$REGISTRY" ]]; then
    log_error "Registry not specified. Use -r/--registry or set REGISTRY env var"
    show_help
    exit 1
fi

# Set platforms default
PLATFORMS="${PLATFORMS:-$DEFAULT_PLATFORMS}"

# Construct full image names
FULL_IMAGE_NAME="${REGISTRY}/${IMAGE_NAME}"
VERSION_TAG="${FULL_IMAGE_NAME}:${TAG}"
LATEST_TAG="${FULL_IMAGE_NAME}:latest"

log_info "Docker Push Configuration"
echo "  Image: $FULL_IMAGE_NAME"
echo "  Tag: $TAG"
echo "  Registry: $REGISTRY"
echo "  Platforms: $PLATFORMS"

if [[ "$DRY_RUN" == true ]]; then
    log_warn "DRY RUN MODE - No actual push will occur"
fi

# Check if we're using buildx (multi-platform)
NEEDS_BUILDX=false
if [[ "$PLATFORMS" == *","* ]]; then
    NEEDS_BUILDX=true
fi

# Build if requested
if [[ "$BUILD" == true ]]; then
    log_info "Building image before push..."
    BUILD_OPTS="--push"
    if [[ "$NEEDS_BUILDX" == true ]]; then
        BUILD_OPTS="--push --platform $PLATFORMS"
    fi
    
    if [[ "$DRY_RUN" == false ]]; then
        # shellcheck disable=SC2086
        ./scripts/docker-build.sh $BUILD_OPTS "$TAG"
    else
        log_info "[DRY RUN] Would build with: docker-build.sh $BUILD_OPTS $TAG"
    fi
fi

# Login to registry if credentials provided
if [[ -n "${DOCKER_USERNAME:-}" ]] && [[ -n "${DOCKER_PASSWORD:-}" ]]; then
    log_info "Logging in to registry..."
    if [[ "$DRY_RUN" == false ]]; then
        echo "$DOCKER_PASSWORD" | docker login "$REGISTRY" -u "$DOCKER_USERNAME" --password-stdin
    else
        log_info "[DRY RUN] Would login to $REGISTRY as $DOCKER_USERNAME"
    fi
fi

# Push logic
if [[ "$NEEDS_BUILDX" == true ]]; then
    # Multi-platform push requires buildx
    log_info "Multi-platform push using buildx"
    
    if [[ "$BUILD" == false ]]; then
        # Need to build and push in one command for multi-platform
        log_info "Building and pushing multi-platform image..."
        
        if [[ "$DRY_RUN" == false ]]; then
            docker buildx build \
                --platform "$PLATFORMS" \
                --push \
                --tag "$VERSION_TAG" \
                $(if [[ "$TAG_LATEST" == true ]]; then echo "--tag $LATEST_TAG"; fi) \
                .
        else
            log_info "[DRY RUN] Would run:"
            echo "  docker buildx build --platform $PLATFORMS --push --tag $VERSION_TAG \\"
            if [[ "$TAG_LATEST" == true ]]; then
                echo "    --tag $LATEST_TAG \\"
            fi
            echo "    ."
        fi
    fi
else
    # Single platform push
    log_info "Single platform push"
    
    # Tag with latest if requested
    if [[ "$TAG_LATEST" == true ]]; then
        log_info "Tagging as latest..."
        if [[ "$DRY_RUN" == false ]]; then
            docker tag "$VERSION_TAG" "$LATEST_TAG"
        else
            log_info "[DRY RUN] Would run: docker tag $VERSION_TAG $LATEST_TAG"
        fi
    fi
    
    # Push tags
    if [[ "$ALL_TAGS" == true ]]; then
        log_info "Pushing all tags..."
        if [[ "$DRY_RUN" == false ]]; then
            docker push "$FULL_IMAGE_NAME" --all-tags
        else
            log_info "[DRY RUN] Would run: docker push $FULL_IMAGE_NAME --all-tags"
        fi
    else
        log_info "Pushing tag: $TAG"
        if [[ "$DRY_RUN" == false ]]; then
            docker push "$VERSION_TAG"
        else
            log_info "[DRY RUN] Would run: docker push $VERSION_TAG"
        fi
        
        if [[ "$TAG_LATEST" == true ]]; then
            log_info "Pushing tag: latest"
            if [[ "$DRY_RUN" == false ]]; then
                docker push "$LATEST_TAG"
            else
                log_info "[DRY RUN] Would run: docker push $LATEST_TAG"
            fi
        fi
    fi
fi

# Verify push
if [[ "$DRY_RUN" == false ]]; then
    log_success "Push completed successfully!"
    log_info "Pushed: $VERSION_TAG"
    [[ "$TAG_LATEST" == true ]] && log_info "Pushed: $LATEST_TAG"
    
    # Output pull commands
    echo ""
    log_info "To pull this image:"
    echo "  docker pull $VERSION_TAG"
    if [[ "$TAG_LATEST" == true ]]; then
        echo "  docker pull $LATEST_TAG"
    fi
else
    log_info "[DRY RUN] Push simulation completed"
fi

# Sign image with Cosign if available (supply chain security)
if command -v cosign &> /dev/null && [[ "$DRY_RUN" == false ]]; then
    log_info "Signing image with Cosign..."
    cosign sign --yes "$VERSION_TAG" || log_warn "Image signing failed"
fi

log_success "All done!"
