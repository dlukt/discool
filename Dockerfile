# syntax=docker/dockerfile:1

# Floating tag: always the current stable Rust on trixie, matching
# `channel = "stable"` in server/rust-toolchain.toml.
FROM rust:trixie AS chef

ARG DEBIAN_FRONTEND=noninteractive

# The image ships its toolchain named for the version (`1.98.1-<host>`), but
# server/rust-toolchain.toml asks for `stable`, which rustup treats as a
# different toolchain and would auto-install on the first cargo call in the
# builder stage -- a fresh download on every build. Installing it here instead
# puts it in this cached base layer. Same compiler, so the cargo-chef
# dependency cache is unaffected.
RUN rustup toolchain install stable --profile minimal --component rustfmt --component clippy

# Install Node.js 24 LTS + cargo-chef for fast dependency caching.
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    gnupg \
  && curl -fsSL https://deb.nodesource.com/setup_24.x | bash - \
  && apt-get install -y --no-install-recommends nodejs \
  && cargo install cargo-chef --locked \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

FROM chef AS planner
COPY server/ ./server/
WORKDIR /app/server
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
WORKDIR /app/server
COPY --from=planner /app/server/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Client build (embedded into the Rust binary via rust-embed at compile time).
WORKDIR /app/client
COPY client/package.json client/package-lock.json ./
RUN npm ci
COPY client/ ./
RUN npm run build

# Server build (release requires client/dist/index.html to exist).
WORKDIR /app/server
COPY server/ ./
RUN cargo build --release

FROM debian:trixie-slim AS runtime

ARG DEBIAN_FRONTEND=noninteractive

# ca-certificates provides the root CAs that rustls-native-certs reads at runtime; pg_dump (v18,
# matching the postgres:18 server) is required for backup support. No libssl needed: the binary is
# rustls-only (no openssl-sys), and openssl-probe (via rustls-native-certs) only locates the cert bundle.
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    gnupg \
  && install -d -m 0755 /etc/apt/keyrings \
  && curl -fsSL https://www.postgresql.org/media/keys/ACCC4CF8.asc \
    | gpg --dearmor -o /etc/apt/keyrings/postgresql.gpg \
  && echo "deb [signed-by=/etc/apt/keyrings/postgresql.gpg] https://apt.postgresql.org/pub/repos/apt trixie-pgdg main" \
    > /etc/apt/sources.list.d/postgresql-pgdg.list \
  && apt-get update \
  && apt-get install -y --no-install-recommends postgresql-client-18 \
  && apt-get purge -y --auto-remove curl gnupg \
  && rm -rf /var/lib/apt/lists/*

RUN useradd -u 1000 -m -s /bin/bash discool

WORKDIR /app
COPY --from=builder /app/server/target/release/discool-server ./discool-server

USER discool

EXPOSE 3000
CMD ["./discool-server"]
