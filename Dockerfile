# OpenRustClaw Multi-Stage Docker Build
# Supports: linux/amd64, linux/arm64
# Final image size target: < 200MB

# =============================================================================
# Stage 1: Cargo Chef - Dependency caching layer
# =============================================================================
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# =============================================================================
# Stage 2: Planner - Generate dependency recipe
# =============================================================================
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# =============================================================================
# Stage 3: Rust Builder - Build the application
# =============================================================================
FROM chef AS builder

# Copy dependency recipe
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies (cached layer)
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code
COPY . .

# Build the application (release mode with LTO for size optimization)
ENV CARGO_PROFILE_RELEASE_LTO=true
ENV CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
ENV CARGO_PROFILE_RELEASE_OPT_LEVEL=3
ENV CARGO_PROFILE_RELEASE_STRIP=true
RUN cargo build --release --bin openrustclaw

# =============================================================================
# Stage 4: Runtime - Minimal production image
# =============================================================================
FROM debian:bookworm-slim AS runtime

# Security: Create non-root user
RUN groupadd --gid 1000 appuser && \
    useradd --uid 1000 --gid appuser --shell /bin/bash --create-home appuser

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    libgcc-s1 \
    libsqlite3-0 \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Copy Rust binary from builder
COPY --from=builder /app/target/release/openrustclaw /usr/local/bin/openrustclaw

# Copy configuration files
COPY config/default.toml /app/config/default.toml
COPY --chmod=644 proto/*.proto /app/proto/ 2>/dev/null || true

# Set up directories with proper permissions
RUN mkdir -p /app/data /app/logs /app/skills && \
    chown -R appuser:appuser /app

# Set environment variables
ENV RUST_LOG=info \
    APP_ENV=production \
    GATEWAY_HOST=0.0.0.0 \
    GATEWAY_PORT=18789 \
    DATABASE_URL=sqlite:///app/data/openrustclaw.db \
    DATABASE_WAL_MODE=true

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:${GATEWAY_PORT}/health || exit 1

# Expose ports
EXPOSE 18789 9090

# Switch to non-root user
USER appuser

# Data volume for persistence
VOLUME ["/app/data"]

# Graceful shutdown handler
STOPSIGNAL SIGTERM

# Default command - start the Rust runtime
CMD ["openrustclaw", "start"]

# Labels for image metadata
LABEL org.opencontainers.image.title="OpenRustClaw" \
      org.opencontainers.image.description="Rust-native AI agent platform" \
      org.opencontainers.image.version="0.1.0" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.source="https://github.com/openrustclaw/openrustclaw"
