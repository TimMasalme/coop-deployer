# --- Build stage ---
FROM rust:1.82-slim AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY src/ src/

RUN cargo build --release

# --- Runtime stage ---
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/faf-coop-deployer /usr/local/bin/faf-coop-deployer

ENV PORT=8080
EXPOSE 8080

CMD ["faf-coop-deployer"]
