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
# Stage 4: Python Sidecar Builder - Install Python dependencies
# =============================================================================
FROM python:3.11-slim AS python-builder

WORKDIR /app

# Install build dependencies for Python packages
RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    && rm -rf /var/lib/apt/lists/*

# Copy Python sidecar requirements
COPY sidecar/pyproject.toml ./
COPY sidecar/src ./src

# Create virtual environment and install dependencies
RUN python -m venv /opt/venv && \
    /opt/venv/bin/pip install --no-cache-dir --upgrade pip && \
    /opt/venv/bin/pip install --no-cache-dir \
        langgraph>=0.4 \
        langchain>=0.3 \
        langsmith>=0.3 \
        grpcio>=1.60 \
        grpcio-tools>=1.60 \
        protobuf>=5.0 \
        openai>=1.0 \
        anthropic>=0.40 \
        langchain-openai>=0.3 \
        langchain-anthropic>=0.3 \
        pydantic>=2.0 \
        typing-extensions>=4.0 \
        python-dateutil>=2.8 \
        pytz>=2024.1 \
        numpy>=1.24

# Copy Python source code
COPY sidecar/src /app/sidecar/src

# =============================================================================
# Stage 5: Runtime - Minimal production image
# =============================================================================
FROM python:3.11-slim AS runtime

# Security: Create non-root user
RUN groupadd --gid 1000 appuser && \
    useradd --uid 1000 --gid appuser --shell /bin/bash --create-home appuser

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Copy Rust binary from builder
COPY --from=builder /app/target/release/openrustclaw /usr/local/bin/openrustclaw

# Copy Python virtual environment from python-builder
COPY --from=python-builder /opt/venv /opt/venv
COPY --from=python-builder /app/sidecar/src /app/sidecar/src

# Copy configuration files
COPY config/default.toml /app/config/default.toml
COPY --chmod=644 proto/*.proto /app/proto/ 2>/dev/null || true

# Set up directories with proper permissions
RUN mkdir -p /app/data /app/logs /app/skills && \
    chown -R appuser:appuser /app

# Set environment variables
ENV PATH="/opt/venv/bin:$PATH" \
    PYTHONPATH="/app/sidecar/src:$PYTHONPATH" \
    RUST_LOG=info \
    APP_ENV=production \
    GATEWAY_HOST=0.0.0.0 \
    GATEWAY_PORT=18789 \
    DATABASE_URL=sqlite:///app/data/openrustclaw.db \
    DATABASE_WAL_MODE=true \
    SIDECAR_GRPC_PORT=50051 \
    SIDECAR_PYTHON_PATH=/opt/venv/bin/python \
    SIDECAR_AUTO_START=false

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:${GATEWAY_PORT}/health || exit 1

# Expose ports
EXPOSE 18789 50051 9090

# Switch to non-root user
USER appuser

# Data volume for persistence
VOLUME ["/app/data"]

# Graceful shutdown handler
STOPSIGNAL SIGTERM

# Default command - start both Rust gateway and Python sidecar
# Uses a process manager approach
CMD ["openrustclaw", "start"]

# Labels for image metadata
LABEL org.opencontainers.image.title="OpenRustClaw" \
      org.opencontainers.image.description="Hybrid Rust + Python AI agent framework" \
      org.opencontainers.image.version="0.1.0" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.source="https://github.com/openrustclaw/openrustclaw"
