#!/usr/bin/env bash
set -euo pipefail

cargo fmt --check
cargo check --workspace
cargo test --workspace

(
  cd frontend
  pnpm run check
)

if [ "${RUN_MCP_E2E:-false}" = "true" ]; then
  scripts/test-mcp-e2e.sh
fi
