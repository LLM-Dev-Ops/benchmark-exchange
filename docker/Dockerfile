# Multi-stage Dockerfile for LLM Benchmark Exchange API
# Optimized for production with minimal image size and security hardening

# ============================================================================
# Stage 1: Cargo Chef - Plan dependencies for caching
# ============================================================================
FROM rust:1.75-slim AS chef

# Install cargo-chef for dependency caching
RUN cargo install cargo-chef --locked

WORKDIR /app

# ============================================================================
# Stage 2: Planner - Generate recipe for dependencies
# ============================================================================
FROM chef AS planner

COPY . .

# Generate dependency recipe
RUN cargo chef prepare --recipe-path recipe.json

# ============================================================================
# Stage 3: Builder - Build dependencies and application
# ============================================================================
FROM chef AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=planner /app/recipe.json recipe.json

# Build dependencies only (cached layer)
RUN cargo chef cook --release --recipe-path recipe.json --bin llm-benchmark-api-rest

# Copy source code
COPY . .

# Build application with optimizations
# SQLx offline mode for compile-time verified queries
ENV SQLX_OFFLINE=true

RUN cargo build --release --bin llm-benchmark-api-rest && \
    strip /app/target/release/llm-benchmark-api-rest

# ============================================================================
# Stage 4: Runtime - Minimal production image
# ============================================================================
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN groupadd -r appuser && useradd -r -g appuser -u 1000 appuser

# Create application directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/llm-benchmark-api-rest /usr/local/bin/llm-benchmark-api-rest

# Ensure binary is executable
RUN chmod +x /usr/local/bin/llm-benchmark-api-rest

# Change ownership to non-root user
RUN chown -R appuser:appuser /app

# Switch to non-root user
USER appuser

# Expose API port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Set environment variables
ENV RUST_LOG=info \
    RUST_BACKTRACE=1 \
    API_HOST=0.0.0.0 \
    API_PORT=8080

# Run the application
ENTRYPOINT ["llm-benchmark-api-rest"]
