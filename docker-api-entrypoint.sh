#!/bin/bash
set -euo pipefail

export ORS_API_HOST="${ORS_API_HOST:-0.0.0.0}"
if [ -n "${PORT:-}" ] && [ -z "${ORS_API_PORT:-}" ]; then
    export ORS_API_PORT="$PORT"
fi

echo "Starting ORSGraph API on ${ORS_API_HOST}:${ORS_API_PORT:-8080}..."
exec /app/orsgraph-api "$@"
