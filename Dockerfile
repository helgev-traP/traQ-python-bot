# === Build Rust ===
FROM rust:latest AS builder

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY /app/Cargo.toml /app/Cargo.lock ./
RUN mkdir src && echo "fn main() {println!(\"if you see this, the build broke\")}"> src/main.rs && cargo build --release

COPY /app/src ./src
RUN cargo build --release

# === Run in DinD ===
FROM docker:dind

WORKDIR /app

COPY --from=builder /app/target/release/traq-python-bot /app/traq-python-bot
RUN chmod +x /traq-python-bot

COPY /entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
