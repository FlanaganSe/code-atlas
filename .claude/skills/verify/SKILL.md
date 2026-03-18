---
name: verify
description: Run all verification checks (tests, lint, typecheck).
user_invocable: true
---

# Verify Skill

Run the full verification suite:

1. `cargo check` — Rust compilation
2. `cargo test` — Rust tests
3. `cargo clippy` — Rust lint
4. `pnpm test` — Frontend tests (Vitest)
5. `pnpm lint` — Frontend lint (ESLint)
6. `pnpm typecheck` — TypeScript type check

Report results clearly. If anything fails, include the error output and suggest a fix.
