# Code Atlas

Local-first desktop app that builds an architecture graph from a repository and renders it as an interactive, zoomable map. Rust backend (codeatlas-core + Tauri shell), React + TypeScript frontend.

## Commands
```bash
pnpm dev              # Tauri dev (launches app with hot reload)
pnpm build            # Production build
pnpm test             # Frontend unit tests (Vitest)
cargo test            # Rust backend tests
cargo clippy          # Rust lint
pnpm lint             # Frontend lint (Biome)
pnpm typecheck        # TypeScript type check
```

## Rules
<!-- Auto-discovered from .claude/rules/ — listed here for visibility -->
@.claude/rules/immutable.md
@.claude/rules/conventions.md
@.claude/rules/stack.md

## System
@docs/SYSTEM.md

## Decisions
See `docs/decisions.md` — append-only ADR log. Read during planning, not loaded every session.

## Personal Overrides
Create `CLAUDE.local.md` (gitignored) for personal, project-specific preferences.

## Workflow
`/prd` → `/research` → `/plan` → `/milestone` (repeat) → `/complete`

## Escalation Policy
- If you discover a new invariant, add it to `.claude/rules/immutable.md`.
