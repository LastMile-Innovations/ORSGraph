# Stage 1: Build
FROM rust:1.95-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    g++ \
    cmake \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build the application
RUN cargo build --release

# Stage 2: Runtime
FROM debian:trixie-slim AS runtime

# Install runtime dependencies + rclone for S3 sync
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# Install rclone for S3 data sync
RUN curl -sSL https://downloads.rclone.org/rclone-current-linux-amd64.zip -o /tmp/rclone.zip \
    && unzip /tmp/rclone.zip -d /tmp \
    && mv /tmp/rclone-*-linux-amd64/rclone /usr/local/bin/rclone \
    && chmod +x /usr/local/bin/rclone \
    && rm -rf /tmp/rclone.zip /tmp/rclone-*-linux-amd64

WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/ors-crawler-v0 /app/ors-crawler-v0

# Copy the Cypher queries (the loader looks for them in 'cypher/queries')
COPY cypher ./cypher

# Create a data directory for the volume mount
RUN mkdir -p /app/data

# Set environment variables (defaults)
ENV RUST_LOG=info
ENV NEO4J_USER=neo4j

# Copy startup script
COPY docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

# Command to run
ENTRYPOINT ["/app/docker-entrypoint.sh"]
