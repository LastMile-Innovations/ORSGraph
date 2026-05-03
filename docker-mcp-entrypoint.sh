#!/bin/bash
set -euo pipefail

if [ -z "${ORSGRAPH_API_BASE_URL:-}" ]; then
    echo "ORSGRAPH_API_BASE_URL is required for the MCP HTTP deployment profile." >&2
    echo "Point it at the orsgraph-api service, for example https://<api-host>/api/v1." >&2
    exit 64
fi

export ORSGRAPH_MCP_TRANSPORT=http

if [ -z "${ORSGRAPH_MCP_BIND:-}" ]; then
    if [ -n "${PORT:-}" ]; then
        export ORSGRAPH_MCP_BIND="0.0.0.0:${PORT}"
    else
        export ORSGRAPH_MCP_BIND="${ORSGRAPH_MCP_HOST:-0.0.0.0}:${ORSGRAPH_MCP_PORT:-8090}"
    fi
fi

if [ -z "${ORSGRAPH_MCP_ALLOWED_HOSTS:-}" ]; then
    ALLOWED_HOSTS="localhost,127.0.0.1,::1"
    if [ -n "${RAILWAY_PUBLIC_DOMAIN:-}" ]; then
        ALLOWED_HOSTS="${ALLOWED_HOSTS},${RAILWAY_PUBLIC_DOMAIN}"
    fi
    if [ -n "${RAILWAY_PRIVATE_DOMAIN:-}" ]; then
        ALLOWED_HOSTS="${ALLOWED_HOSTS},${RAILWAY_PRIVATE_DOMAIN}"
    fi
    export ORSGRAPH_MCP_ALLOWED_HOSTS="$ALLOWED_HOSTS"
fi

if [ -z "${ORSGRAPH_MCP_ALLOWED_ORIGINS:-}" ] && [ -n "${RAILWAY_PUBLIC_DOMAIN:-}" ]; then
    export ORSGRAPH_MCP_ALLOWED_ORIGINS="https://${RAILWAY_PUBLIC_DOMAIN}"
fi

if [ -z "${ORSGRAPH_MCP_OAUTH_RESOURCE:-}" ] &&
    [ -n "${ORSGRAPH_MCP_JWT_ISSUER:-}" ] &&
    [ -n "${RAILWAY_PUBLIC_DOMAIN:-}" ]; then
    export ORSGRAPH_MCP_OAUTH_RESOURCE="https://${RAILWAY_PUBLIC_DOMAIN}${ORSGRAPH_MCP_PATH:-/mcp}"
fi

echo "Starting ORSGraph MCP over Streamable HTTP on ${ORSGRAPH_MCP_BIND}..."
exec /app/orsgraph-mcp --http "$@"
