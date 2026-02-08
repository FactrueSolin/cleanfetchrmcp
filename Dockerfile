# syntax=docker/dockerfile:1

FROM rust:1.88-bookworm AS builder

WORKDIR /app

# Build dependencies commonly required by Rust crates.
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Cache optimization: copy manifests first.
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked \
    && cp /app/target/release/cleanfetchrmcp /app/cleanfetchrmcp


FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home appuser

COPY --from=builder /app/cleanfetchrmcp /usr/local/bin/cleanfetchrmcp

ENV PORT=3000 \
    SELENIUM_URL=http://selenium:4444 \
    RUST_LOG=info,cleanfetchrmcp=debug

EXPOSE 3000

USER appuser

CMD ["cleanfetchrmcp"]
