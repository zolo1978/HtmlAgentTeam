# ---- Build Stage ----
FROM rust:1.79-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY backend ./backend

RUN cargo build --release --package harp-backend

# ---- Runtime Stage ----
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/backend/target/release/harp-backend /usr/local/bin/harp-backend
COPY --from=builder /app/backend/migrations /app/migrations

WORKDIR /app
EXPOSE 8080

CMD ["harp-backend"]
