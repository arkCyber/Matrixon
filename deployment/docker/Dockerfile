# =============================================================================
# Matrixon Matrix NextServer - Production Docker Build
# =============================================================================
#
# Project: Matrixon - Ultra High Performance Matrix NextServer (Synapse Alternative)
# Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
# Date: 2024-12-17
# Version: 2.0.0-production
# License: Apache 2.0 / MIT
#
# Description:
#   Production-grade multi-stage Docker build for Matrixon Matrix server
#   designed for enterprise deployment with optimized performance and security.
#
# Performance Targets:
#   • 200k+ concurrent connections
#   • <50ms response latency
#   • >99% success rate
#   • Memory-efficient operation
#   • Horizontal scalability
#
# Build Arguments:
#   RUST_VERSION: Rust toolchain version (default: 1.85.0)
#   TARGET_ARCH: Target architecture (default: x86_64-unknown-linux-musl)
#   OPTIMIZATION_LEVEL: Rust optimization level (default: 3)
#
# Usage:
#   docker build -t matrixon:latest .
#   docker run -p 6167:6167 -v ./data:/app/data matrixon:latest
#
# =============================================================================

# Build stage - Optimized Rust compilation
FROM rust:1.85.0-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    libpq-dev \
    musl-tools \
    gcc-musl-dev \
    curl \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Add musl target for static linking
RUN rustup target add x86_64-unknown-linux-musl

# Set working directory
WORKDIR /app

# Copy dependency files first for better Docker layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Create src directory with dummy main.rs for dependency compilation
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only (cached layer)
RUN cargo build --release --target x86_64-unknown-linux-musl

# Remove dummy source
RUN rm -rf src/

# Copy actual source code
COPY src/ ./src/
COPY .cursorrules ./

# Build the actual application with maximum optimization
ENV RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C lto=fat -C codegen-units=1"
RUN cargo build --release --target x86_64-unknown-linux-musl

# Runtime stage - Minimal Alpine Linux
FROM alpine:3.19 AS runtime

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    sqlite \
    libgcc \
    && addgroup -g 1000 matrixon \
    && adduser -D -s /bin/sh -u 1000 -G matrixon matrixon

# Create application directories
RUN mkdir -p /app/data /app/config /app/logs \
    && chown -R matrixon:matrixon /app

# Copy binary from builder stage
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/matrixon /app/matrixon
COPY --chown=matrixon:matrixon docker/production-config.toml /app/config/

# Set permissions
RUN chmod +x /app/matrixon

# Switch to non-root user
USER matrixon

# Set working directory
WORKDIR /app

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:6167/_matrix/client/versions || exit 1

# Expose port
EXPOSE 6167

# Default command
CMD ["./matrixon", "--config", "/app/config/production-config.toml", "start"]

# Labels for production deployment
LABEL maintainer="arkSong <arksong2018@gmail.com>"
LABEL org.opencontainers.image.title="Matrixon Matrix NextServer"
LABEL org.opencontainers.image.description="Ultra High Performance Matrix Server"
LABEL org.opencontainers.image.version="2.0.0-production"
LABEL org.opencontainers.image.vendor="Matrixon Innovation Project"
LABEL org.opencontainers.image.licenses="Apache-2.0 OR MIT" 
