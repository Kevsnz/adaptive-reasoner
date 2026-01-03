FROM rust:1.91-slim-bullseye AS builder

RUN apt-get update \
    && apt-get install -y pkg-config libssl-dev

WORKDIR /app/src

COPY Cargo.toml .
COPY Cargo.lock .
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

COPY . .

RUN cargo build --release

# -----------------------
FROM debian:bullseye-slim

WORKDIR /app

COPY --from=builder /app/src/target/release/adaptive_reasoner .
COPY config.json .

ENV AR_CONFIG_FILE=./config.json

EXPOSE 8080

CMD ["./adaptive_reasoner"]
