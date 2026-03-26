#!/bin/bash
# Convenience script to run the OPENAKTA CLI
# Usage: ./run-cli.sh <command>

set -e

cd "$(dirname "$0")"

cargo run --bin openakta -- "$@"
