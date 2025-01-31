# === Build Rust ===
FROM rust:latest AS builder

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY /app/Cargo.toml /app/Cargo.lock ./
RUN mkdir src \
    && echo "fn main() {println!(\"if you see this, the build broke\")}"> src/main.rs \
    && cargo build --release

COPY /app/src ./src
RUN cargo build --release

# === Run in DinD ===
FROM ubuntu:24.04

# install build dependencies
RUN apt update \
    && apt install -y \
    build-essential

# install docker
RUN apt update \
    && apt install ca-certificates curl gnupg lsb-release -y \
    && mkdir -p /etc/apt/keyrings/ \
    && curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg \
    && chmod a+r /etc/apt/keyrings/docker.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null \
    && apt update \
    && apt install docker-ce docker-ce-cli containerd.io -y

COPY --from=builder /app/target/release/traq-python-bot /app/traq-python-bot
RUN chmod +x /app/traq-python-bot

COPY /entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

CMD ["mkdir", "-p", "/app/sandbox"]

ENTRYPOINT ["sh", "/app/entrypoint.sh"]
