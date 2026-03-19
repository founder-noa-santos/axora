# ADR-052: Use Next.js App Router for the Renderer

**Date:** 2026-03-17  
**Status:** Accepted

## Context

The renderer required a fresh structure with strong TypeScript ergonomics, good composition, static export support, and a clean path for a premium shell UI.

## Decision

Use Next.js App Router as the desktop renderer framework.

## Why

- file-system structure is clear and scales better than ad-hoc legacy routing
- static export works well for Electron production loading
- strict TypeScript and React ergonomics are strong out of the box
- the framework works cleanly with Tailwind CSS v4 and local shadcn/ui components

## Consequences

- Production renderer assets are generated through `next build`
- The renderer remains static-first unless a concrete desktop need justifies more server complexity
- App composition is centered on `app/`, `components/`, `lib/`, and `shared/`
