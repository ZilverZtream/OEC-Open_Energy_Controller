# Multi-stage Dockerfile for Open Energy Controller
# Optimized for small image size, security, and build caching

# ============================================================================
# Stage 1: Build dependencies cache layer
# ============================================================================
FROM rust:1.78-bookworm AS chef

# Install cargo-chef for dependency caching
RUN cargo install cargo-chef --version 0.1.62

WORKDIR /app

# ============================================================================
# Stage 2: Compute recipe file for dependencies
# ============================================================================
FROM chef AS planner

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo chef prepare --recipe-path recipe.json

# ============================================================================
# Stage 3: Build dependencies (this layer will be cached)
# ============================================================================
FROM chef AS builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy recipe and build dependencies
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --features "swagger,metrics,db,modbus,ocpp" --recipe-path recipe.json

# ============================================================================
# Stage 4: Build the application
# ============================================================================
# Copy source code and build the binary
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY config ./config
COPY migrations ./migrations

# Build with full optimizations
RUN cargo build --release --features "swagger,metrics,db,modbus,ocpp" --bin open-energy-controller

# Strip debug symbols to reduce binary size
RUN strip target/release/open-energy-controller

# ============================================================================
# Stage 5: Runtime image
# ============================================================================
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash energy && \
    mkdir -p /app /app/config /app/migrations && \
    chown -R energy:energy /app

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/open-energy-controller /usr/local/bin/open-energy-controller

# Copy configuration files
COPY --chown=energy:energy config ./config
COPY --chown=energy:energy migrations ./migrations

# Switch to non-root user
USER energy

# Expose ports
# 8080: HTTP API
# 9090: Prometheus metrics
EXPOSE 8080 9090

# Health check (uses /health endpoint)
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Set environment variables
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV OEC__SERVER__HOST=0.0.0.0
ENV OEC__SERVER__PORT=8080

# Metadata
LABEL org.opencontainers.image.title="Open Energy Controller" \
      org.opencontainers.image.description="Edge-based energy management system with battery optimization" \
      org.opencontainers.image.version="0.2.0" \
      org.opencontainers.image.vendor="Open Energy Controller" \
      org.opencontainers.image.licenses="MIT"

# Run the application
ENTRYPOINT ["/usr/local/bin/open-energy-controller"]
CMD []
