# =======================================================
# BookCore v0.1 Production Dockerfile
# High-performance lightweight personal book & comic reading server
# Port: 3435
# =======================================================

# Stage 1: Build & Dependency Caching
FROM rust:1.82-bookworm AS builder
WORKDIR /app

# Copy dependency manifests
COPY Cargo.toml ./

# Dummy source to cache dependencies
RUN mkdir -p src/ui && echo "fn main() {}" > src/main.rs && echo "dummy" > src/ui/index.html
RUN cargo build --release
RUN rm -rf src target/release/bookcore* target/release/deps/bookcore*

# Copy real source code and static UI assets
COPY src ./src

# Build production release binary
RUN cargo build --release

# Stage 2: Minimal Runtime Image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/bookcore /usr/local/bin/bookcore

# Expose default port 3435
EXPOSE 3435

ENV PORT=3435
ENV BOOKCORE_DB_PATH=/data/bookcore.db
ENV BOOKCORE_BOOKS_DIR=/books
ENV BOOKCORE_CACHE_DIR=/data/cache
ENV RUST_LOG=bookcore=info,tower_http=info

VOLUME ["/data", "/books"]

CMD ["/usr/local/bin/bookcore"]
