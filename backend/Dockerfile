FROM rust:latest AS builder
WORKDIR /app

# Copy manifests; Cargo.lock is optional and can be added later
COPY Cargo.toml ./
COPY Cargo.lock* ./

# Prime dependency cache
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo fetch
RUN rm -rf src

# Copy full source
COPY . .

# Build application (migrations are embedded, no DB connection needed at build time)
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates openssl && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Copy application and static files
COPY --from=builder /app/target/release/opslab_mindguard /usr/local/bin/opslab-mindguard
COPY --from=builder /app/migrations migrations
COPY --from=builder /app/index.html index.html
COPY --from=builder /app/static static

ENV RUST_LOG=info
EXPOSE 8080

# Use shell form to ensure environment variables are properly expanded
CMD /usr/local/bin/opslab-mindguard
