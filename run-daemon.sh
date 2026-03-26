#!/bin/bash
# Convenience script to run the OPENAKTA Daemon
# Usage: ./run-daemon.sh

set -e

cd "$(dirname "$0")"

cargo run --bin openakta-daemon -- "$@"
