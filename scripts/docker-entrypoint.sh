#!/usr/bin/env sh
# Wait for DB then exec the main command. Used as Docker ENTRYPOINT.
set -e
/usr/local/bin/wait-for-db.sh "${DB_HOST:-db}" "${DB_PORT:-5432}"
exec "$@"
