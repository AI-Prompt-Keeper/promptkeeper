#!/usr/bin/env sh
# Wait for PostgreSQL to be ready (retry with backoff). Use in backend entrypoint.
# Usage: wait-for-db.sh [host] [port] [max_attempts]
# Requires: pg_isready (postgresql-client)

set -e
HOST="${1:-db}"
PORT="${2:-5432}"
MAX_ATTEMPTS="${3:-30}"
INTERVAL="${4:-2}"

attempt=1
until pg_isready -h "$HOST" -p "$PORT" -q 2>/dev/null; do
  if [ "$attempt" -ge "$MAX_ATTEMPTS" ]; then
    echo "wait-for-db: gave up after $MAX_ATTEMPTS attempts" >&2
    exit 1
  fi
  echo "wait-for-db: attempt $attempt/$MAX_ATTEMPTS — $HOST:$PORT not ready, retrying in ${INTERVAL}s..."
  sleep "$INTERVAL"
  attempt=$((attempt + 1))
done
echo "wait-for-db: $HOST:$PORT is ready"
