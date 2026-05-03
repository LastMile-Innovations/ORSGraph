#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TIMEOUT_MS="${ORSGRAPH_MCP_E2E_TIMEOUT_MS:-45000}"
API_BASE_URL="${ORSGRAPH_API_BASE_URL:-}"
MCP_URL="${ORSGRAPH_MCP_URL:-}"
BEARER_TOKEN="${ORSGRAPH_MCP_BEARER_TOKEN:-}"
RUN_DOCKER="${ORSGRAPH_MCP_E2E_DOCKER:-false}"
DOCKER_IMAGE="${ORSGRAPH_MCP_DOCKER_IMAGE:-orsgraph-mcp:e2e}"

usage() {
    cat <<'EOF'
ORSGraph MCP build + end-to-end test

Usage:
  scripts/test-mcp-e2e.sh
  scripts/test-mcp-e2e.sh --api-base-url http://127.0.0.1:8080/api/v1
  scripts/test-mcp-e2e.sh --mcp-url https://mcp.example.com/mcp --bearer-token "$TOKEN"
  scripts/test-mcp-e2e.sh --docker

Default behavior:
  - checks MCP script/entrypoint syntax
  - runs cargo fmt/check/test for orsgraph-mcp
  - builds target/release/orsgraph-mcp
  - runs the Streamable HTTP smoke against the built release binary
  - uses a stub ORSGraph API unless --api-base-url is provided

Options:
  --api-base-url <url>   Use a real ORSGraph API for the local release-binary smoke
  --mcp-url <url>        Smoke an already-running or deployed MCP endpoint
  --bearer-token <tok>   Send Authorization: Bearer <tok>
  --timeout-ms <ms>      Smoke timeout; default 45000
  --docker               Also build Dockerfile.mcp; requires a running Docker daemon
  -h, --help             Show this help
EOF
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --api-base-url)
            API_BASE_URL="${2:?--api-base-url requires a value}"
            shift 2
            ;;
        --mcp-url)
            MCP_URL="${2:?--mcp-url requires a value}"
            shift 2
            ;;
        --bearer-token)
            BEARER_TOKEN="${2:?--bearer-token requires a value}"
            shift 2
            ;;
        --timeout-ms)
            TIMEOUT_MS="${2:?--timeout-ms requires a value}"
            shift 2
            ;;
        --docker)
            RUN_DOCKER=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "unknown argument: $1" >&2
            usage >&2
            exit 64
            ;;
    esac
done

echo "==> MCP smoke script syntax"
node --check scripts/smoke-mcp-http.mjs

echo "==> MCP Docker entrypoint syntax"
bash -n docker-mcp-entrypoint.sh

echo "==> MCP Rust formatting"
cargo fmt -p orsgraph-mcp -- --check

echo "==> MCP Rust check"
cargo check -p orsgraph-mcp

echo "==> MCP Rust tests"
cargo test -p orsgraph-mcp

echo "==> MCP release build"
cargo build --release -p orsgraph-mcp

SMOKE_ARGS=(--timeout-ms "$TIMEOUT_MS")
if [ -n "$BEARER_TOKEN" ]; then
    SMOKE_ARGS+=(--bearer-token "$BEARER_TOKEN")
fi

if [ -n "$MCP_URL" ]; then
    echo "==> MCP deployed/remote HTTP smoke"
    SMOKE_ARGS+=(--mcp-url "$MCP_URL")
else
    echo "==> MCP release-binary HTTP smoke"
    SMOKE_ARGS+=(--mcp-bin "$ROOT_DIR/target/release/orsgraph-mcp")
    if [ -n "$API_BASE_URL" ]; then
        SMOKE_ARGS+=(--api-base-url "$API_BASE_URL")
    fi
fi

node scripts/smoke-mcp-http.mjs "${SMOKE_ARGS[@]}"

case "$(printf '%s' "$RUN_DOCKER" | tr '[:upper:]' '[:lower:]')" in
    1|true|yes|on)
        echo "==> MCP Docker image build"
        if ! docker info >/dev/null 2>&1; then
            echo "Docker daemon is not reachable; cannot build Dockerfile.mcp" >&2
            exit 1
        fi
        docker build -f Dockerfile.mcp -t "$DOCKER_IMAGE" .
        ;;
    *)
        echo "==> Docker build skipped; pass --docker or set ORSGRAPH_MCP_E2E_DOCKER=true to require it"
        ;;
esac

echo "ORSGraph MCP build + end-to-end test passed"
