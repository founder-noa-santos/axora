#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

mkdir -p target/coverage
cargo llvm-cov --workspace --all-features --locked --lcov --output-path target/coverage/lcov.info

