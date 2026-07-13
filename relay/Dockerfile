# Build stage
FROM rust:1.82-slim-bookworm AS builder
WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy manifests first for layer caching
COPY Cargo.toml Cargo.lock ./
COPY relay/Cargo.toml ./relay/
# If relay is a workspace member, adjust paths accordingly

# Copy source
COPY . .

# Build release binary
RUN cargo build --release --bin bennett-relay

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/target/release/bennett-relay /usr/local/bin/bennett-relay

# Environment defaults for Render
ENV BENNETT_HTTP_MODE=1
ENV BENNETT_BIND=0.0.0.0:8443
ENV BENNETT_PROXY_API_BIND=0.0.0.0:8443
ENV BENNETT_LOG_LEVEL=info
ENV RUST_LOG=info

EXPOSE 8443

HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=5 \
  CMD curl -fsS http://localhost:8444/health || exit 1

CMD ["bennett-relay"]
