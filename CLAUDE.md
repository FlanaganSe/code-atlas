# Tauri Zoom Thing

Tauri desktop app with zoom/pan features — Rust backend, TypeScript + React frontend.

## Commands
```bash
pnpm dev              # Tauri dev (launches app with hot reload)
pnpm build            # Production build
pnpm test             # Frontend unit tests (Vitest)
cargo test            # Rust backend tests
cargo clippy          # Rust lint
pnpm lint             # Frontend lint (ESLint)
pnpm typecheck        # TypeScript type check
```

## Rules
<!-- Auto-discovered from .claude/rules/ — listed here for visibility -->
@.claude/rules/immutable.md
@.claude/rules/conventions.md
@.claude/rules/stack.md

## System
<!-- Uncomment when SYSTEM.md has real content: -->
<!-- @docs/SYSTEM.md -->

## Decisions
See `docs/decisions.md` — append-only ADR log. Read during planning, not loaded every session.

## Personal Overrides
Create `CLAUDE.local.md` (gitignored) for personal, project-specific preferences.

## Workflow
`/prd` → `/research` → `/plan` → `/milestone` (repeat) → `/complete`

## Escalation Policy
- If you discover a new invariant, add it to `.claude/rules/immutable.md`.
