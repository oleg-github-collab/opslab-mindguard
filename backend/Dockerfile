FROM rust:latest AS builder
WORKDIR /app

# Copy manifests; Cargo.lock/sqlx-data.json are optional and can be added later
COPY Cargo.toml ./
COPY Cargo.lock* ./
COPY sqlx-data.json* ./

# Prime dependency cache
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo fetch
RUN rm -rf src

# Force cache invalidation
COPY .cache-bust /tmp/.cache-bust
RUN cat /tmp/.cache-bust && echo " - cache invalidated"

# Copy full source
COPY . .

# Build application (expects DATABASE_URL at build time unless SQLX_OFFLINE + sqlx-data.json are provided)
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates openssl && rm -rf /var/lib/apt/lists/*
WORKDIR /app

COPY --from=builder /app/target/release/opslab_mindguard /usr/local/bin/opslab-mindguard
COPY --from=builder /app/static static
COPY --from=builder /app/index.html index.html
COPY --from=builder /app/migrations migrations

ENV RUST_LOG=info
EXPOSE 3000
CMD ["/usr/local/bin/opslab-mindguard"]
