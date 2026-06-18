FROM rust:1.91-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN useradd --system --uid 10001 --create-home appuser \
    && apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/miner_btc /usr/local/bin/miner-btc
USER appuser
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -fsS http://127.0.0.1:8080/health >/dev/null || exit 1
CMD ["miner-btc", "dashboard", "--bind", "0.0.0.0:8080"]
