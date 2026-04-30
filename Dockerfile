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
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

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

# Command to run
ENTRYPOINT ["/app/ors-crawler-v0"]
