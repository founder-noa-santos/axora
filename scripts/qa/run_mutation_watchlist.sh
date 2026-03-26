#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

# Start with the highest-risk packages instead of mutating the whole workspace.
cargo mutants --package openakta-core --package openakta-agents --timeout 300

