#!/usr/bin/env bash
set -euo pipefail

cargo fmt --check
cargo check --workspace
cargo test --workspace

(
  cd frontend
  pnpm run check
)
