#!/usr/bin/env bash
set -euo pipefail

mkdir -p reports/coverage
cargo llvm-cov --workspace --summary-only "$@"
