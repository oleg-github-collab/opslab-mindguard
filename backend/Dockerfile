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

# Copy full source
COPY . .

# Show what we copied (for debugging)
RUN ls -la index.html && head -7 index.html

# Build application (expects DATABASE_URL at build time unless SQLX_OFFLINE + sqlx-data.json are provided)
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates openssl && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Copy application and static files
COPY --from=builder /app/target/release/opslab_mindguard /usr/local/bin/opslab-mindguard
COPY --from=builder /app/migrations migrations
COPY --from=builder /app/index.html index.html
COPY --from=builder /app/static static

# Show what we have in runtime
RUN ls -la && head -7 index.html

ENV RUST_LOG=info
EXPOSE 3000
CMD ["/usr/local/bin/opslab-mindguard"]
