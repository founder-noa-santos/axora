# AXORA MVP

> Multi-Agent Coding System - Core Infrastructure

## Overview

AXORA is a multi-agent coding system designed for collaborative software development. This repository contains the core infrastructure including the daemon, storage layer, and desktop application.

## Project Structure

```
axora-mvp/
├── proto/           # Protocol Buffer schemas
├── crates/          # Rust workspace crates
│   ├── axora-proto/     # Generated protobuf code
│   ├── axora-storage/   # SQLite storage layer
│   ├── axora-core/      # Core business logic
│   └── axora-daemon/    # Main daemon executable
├── apps/
│   └── desktop/     # Tauri v2 desktop application
└── docs/            # Documentation
```

## Quick Start

### Prerequisites

- Rust 1.75+
- Node.js 20+
- pnpm 8+
- Protocol Buffers compiler

### Development

```bash
# Install dependencies
pnpm install

# Run the daemon
cargo run -p axora-daemon

# Run the desktop app
pnpm dev:desktop
```

## Architecture

See [docs/architecture.md](docs/architecture.md) for detailed architecture documentation.

## License

MIT OR Apache-2.0
