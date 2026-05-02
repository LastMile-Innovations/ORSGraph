#!/bin/bash
set -euo pipefail

ROLE="${ORS_CONTAINER_ROLE:-api}"
if [ "${RUN_CRAWLER_ONLY:-false}" = "true" ]; then
    ROLE="crawler"
fi

case "$ROLE" in
    api)
        exec /app/docker-api-entrypoint.sh "$@"
        ;;
    crawler|worker|job)
        exec /app/docker-crawler-entrypoint.sh "$@"
        ;;
    *)
        echo "Unsupported ORS_CONTAINER_ROLE=$ROLE. Use api or crawler." >&2
        exit 64
        ;;
esac
