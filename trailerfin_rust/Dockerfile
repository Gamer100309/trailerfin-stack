FROM rust:1.87-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/trailerfin_rust /usr/local/bin/trailerfin_rust
ENTRYPOINT ["/usr/local/bin/trailerfin_rust"]
