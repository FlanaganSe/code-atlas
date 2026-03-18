---
description: Code style and established patterns.
---
# Conventions

- Co-located tests: `foo.ts` → `foo.test.ts` (frontend), `mod tests` block (Rust)
- Named exports over default exports (TypeScript)
- Explicit return types on public functions (both TypeScript and Rust)
- Formatter handles formatting — don't bikeshed
- Tauri commands use `snake_case` in Rust, `camelCase` in TypeScript via `@tauri-apps/api`

## Established Patterns
<!-- Add as discovered: **Name**: Description. See `path/to/example`. -->
