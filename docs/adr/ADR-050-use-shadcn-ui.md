# ADR-050: Use shadcn/ui for Desktop Components

**Date:** 2026-03-17  
**Status:** Accepted

## Context

The desktop frontend was reset. We still need an in-repo component foundation that is accessible, editable, and compatible with a strongly themed macOS-first shell.

## Decision

Use shadcn/ui-style local components inside [apps/desktop/components/ui](/Users/noasantos/Fluri/axora/apps/desktop/components/ui).

## Why

- Components stay in-repo and are easy to adapt to the app shell
- Radix primitives give strong accessibility and interaction behavior
- Tailwind CSS v4 tokens can directly drive the design system
- The approach works cleanly with Electron and Next.js

## Consequences

- We keep component generation disciplined and only add primitives we use
- The app owns its component code instead of depending on a heavyweight opaque UI kit
- Theme changes remain local to the desktop package
