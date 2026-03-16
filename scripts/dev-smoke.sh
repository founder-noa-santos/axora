#!/bin/bash
# AXORA Development Smoke Test Script
# Quick validation that the project builds and basic tests pass

set -e

echo "=========================================="
echo "AXORA Development Smoke Test"
echo "=========================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print status
print_status() {
    echo -e "${GREEN}[✓]${NC} $1"
}

print_error() {
    echo -e "${RED}[✗]${NC} $1"
}

print_info() {
    echo -e "${YELLOW}[i]${NC} $1"
}

# Check Rust toolchain
print_info "Checking Rust toolchain..."
if ! command -v rustc &> /dev/null; then
    print_error "Rust not found. Please install Rust."
    exit 1
fi
RUST_VERSION=$(rustc --version)
print_status "Rust version: $RUST_VERSION"

# Check Node.js
print_info "Checking Node.js..."
if ! command -v node &> /dev/null; then
    print_error "Node.js not found. Please install Node.js 20+."
    exit 1
fi
NODE_VERSION=$(node --version)
print_status "Node.js version: $NODE_VERSION"

# Check pnpm
print_info "Checking pnpm..."
if ! command -v pnpm &> /dev/null; then
    print_error "pnpm not found. Please install pnpm."
    exit 1
fi
PNPM_VERSION=$(pnpm --version)
print_status "pnpm version: $PNPM_VERSION"

# Build Rust workspace
print_info "Building Rust workspace..."
if cargo build --workspace 2>&1 | tee /tmp/rust-build.log; then
    print_status "Rust workspace built successfully"
else
    print_error "Rust workspace build failed"
    cat /tmp/rust-build.log
    exit 1
fi

# Run Rust tests
print_info "Running Rust tests..."
if cargo test --workspace 2>&1 | tee /tmp/rust-test.log; then
    print_status "Rust tests passed"
else
    print_error "Rust tests failed"
    cat /tmp/rust-test.log
    exit 1
fi

# Install Node dependencies
print_info "Installing Node dependencies..."
cd apps/desktop
if pnpm install 2>&1 | tee /tmp/pnpm-install.log; then
    print_status "Node dependencies installed"
else
    print_error "Node dependency installation failed"
    cat /tmp/pnpm-install.log
    exit 1
fi

# Type check TypeScript
print_info "Type-checking TypeScript..."
if pnpm typecheck 2>&1 | tee /tmp/tsc.log; then
    print_status "TypeScript type check passed"
else
    print_error "TypeScript type check failed"
    cat /tmp/tsc.log
    exit 1
fi

cd ../..

# Summary
echo ""
echo "=========================================="
echo -e "${GREEN}All smoke tests passed!${NC}"
echo "=========================================="
echo ""
echo "Next steps:"
echo "  1. Run the daemon: cargo run -p axora-daemon"
echo "  2. Run the desktop app: pnpm --filter @axora/desktop dev"
echo ""
