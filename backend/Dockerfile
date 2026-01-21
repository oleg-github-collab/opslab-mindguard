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

# Build application with SQLx offline cache (.sqlx/)
ENV SQLX_OFFLINE=true
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

# Create startup script with debugging
RUN echo '#!/bin/sh\n\
set -e\n\
echo "=== OpsLab Mindguard Startup Debug ==="\n\
echo "Timestamp: $(date)"\n\
echo "PORT: ${PORT:-not set}"\n\
echo "BIND_ADDR: ${BIND_ADDR:-not set}"\n\
echo "DATABASE_URL: ${DATABASE_URL:0:40}..."\n\
echo "RAILWAY_ENVIRONMENT: ${RAILWAY_ENVIRONMENT:-not set}"\n\
echo "RUST_LOG: ${RUST_LOG:-not set}"\n\
echo "PWD: $(pwd)"\n\
echo "Checking binary..."\n\
ls -lh /usr/local/bin/opslab-mindguard || echo "Binary not found!"\n\
echo "Checking migrations..."\n\
ls -lh migrations/ | head -5 || echo "Migrations not found!"\n\
echo "========================================"\n\
echo "Starting application now..."\n\
exec /usr/local/bin/opslab-mindguard 2>&1' > /app/start.sh && chmod +x /app/start.sh

CMD ["/app/start.sh"]
