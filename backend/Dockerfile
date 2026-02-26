# Dev-oriented backend image: Rust + pg client (wait-for-db) + cargo-watch.
# Source is mounted at /app; target/ is a volume for fast incremental builds.
# Run: wait for DB, then cargo watch -x run (no container restart on code change).
FROM rust:1-bookworm

# PostgreSQL client for pg_isready; curl for backend healthcheck
RUN apt-get update && apt-get install -y --no-install-recommends \
    postgresql-client \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-watch so code changes trigger recompile/run without container restart
RUN cargo install cargo-watch

WORKDIR /app

# Build context is repo root; copy backend and scripts only.
COPY backend/Cargo.toml ./
COPY backend/src ./src
COPY scripts/wait-for-db.sh scripts/docker-entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/wait-for-db.sh /usr/local/bin/docker-entrypoint.sh

ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
CMD ["cargo", "watch", "-x", "run"]
