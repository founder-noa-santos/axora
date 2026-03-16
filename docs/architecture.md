# AXORA Architecture

## Overview

AXORA is a multi-agent coding system built with a modular architecture that separates concerns across multiple layers.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Desktop Application                       │
│                    (Tauri v2 + React + TypeScript)              │
└───────────────────────────────┬─────────────────────────────────┘
                                │ IPC (gRPC/WebSocket)
┌───────────────────────────────▼─────────────────────────────────┐
│                         AXORA Daemon                             │
│                    (Tokio + Tonic gRPC Server)                  │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   Config    │  │    Frame    │  │   Collective Server     │  │
│  │   Module    │  │   Executor  │  │   (gRPC Service)        │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              Protocol Buffer Definitions                 │    │
│  │           (Agent, Task, Message schemas)                │    │
│  └─────────────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              SQLite Storage Layer                        │    │
│  │         (Agents, Tasks, Messages, Sessions)             │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

## Crate Structure

### axora-proto
- Protocol Buffer definitions
- Generated Rust code for gRPC
- Shared types between client and server

### axora-storage
- SQLite database management
- Migration system
- Data access layer (AgentStore, TaskStore, MessageStore)

### axora-core
- Frame-based execution model
- Configuration management
- gRPC server implementation
- Business logic

### axora-daemon
- Main executable
- CLI argument parsing
- Service orchestration

## Data Flow

1. **Agent Registration**: Desktop → Daemon → Storage
2. **Task Submission**: Desktop → Daemon → Frame Executor → Storage
3. **Message Streaming**: Daemon → Desktop (bidirectional)
4. **State Updates**: Storage → Daemon → Desktop

## Frame System

The frame system provides deterministic execution:

- Target: 60 FPS (16ms frames)
- Each frame processes pending operations
- State updates are batched per frame
- Enables reproducible behavior

## Storage Schema

See `crates/axora-storage/migrations/0001_init.sql` for the complete schema.

Key tables:
- `agents`: Registered agents
- `tasks`: Task definitions and status
- `messages`: Inter-agent communication
- `sessions`: Active agent sessions
