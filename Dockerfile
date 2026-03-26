FROM rust:1.94-bookworm AS builder

WORKDIR /app

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY common/Cargo.toml common/Cargo.toml
COPY host/Cargo.toml host/Cargo.toml
COPY orchestrator/Cargo.toml orchestrator/Cargo.toml

# Create dummy source files so cargo can resolve dependencies
RUN mkdir -p common/src host/src orchestrator/src && \
    echo "pub mod helpers; pub mod key; pub mod protocols;" > common/src/lib.rs && \
    touch common/src/helpers.rs common/src/key.rs common/src/protocols.rs && \
    echo "fn main() {}" > host/src/main.rs && \
    echo "fn main() {}" > orchestrator/src/main.rs

# Build dependencies only (cached layer)
RUN cargo build --release -p cocompute_orchestrator 2>/dev/null || true

# Remove dummy source, copy real source
RUN rm -rf common/src host/src orchestrator/src
COPY common/ common/
COPY host/ host/
COPY orchestrator/ orchestrator/

# Build the real binary
RUN cargo build --release -p cocompute_orchestrator

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/cocompute_orchestrator /usr/local/bin/cocompute-orchestrator

# Data directory for SQLite and keys
RUN mkdir -p /data /root/.cocompute
VOLUME /data

ENV COCOMPUTE_DB_PATH=/data/cocompute.db
ENV COCOMPUTE_KEY_PATH=/data/orchestrator.key
ENV COCOMPUTE_PORT=3000

EXPOSE 3000

ENTRYPOINT ["cocompute-orchestrator", "serve"]
