---
name: Tauri dev environment research completed
description: Rust/Tauri v2 macOS dev environment prerequisites researched March 2026 — key versions, edition choice, tooling list
type: project
---

Rust+Tauri v2 macOS dev environment research completed 2026-03-17. Key facts:

- Rust stable: 1.84.0; minimum for Tauri v2: 1.77.2
- Tauri: 2.10.3 (2026-03-04); tauri-cli: 2.10.1
- Node.js: stack locks to 22+; current active LTS is 24 ("Krypton")
- pnpm: 10.32.1
- aarch64-apple-darwin is Tier 1 (since 2024-10-17); x86_64 demoted to Tier 2 in Rust 1.90 (Aug 2025)
- Rust 2024 edition had a tauri-build parse bug (issues #10412, #11829); fix merged (PR #12270) but exact 2.x release not confirmed — use edition 2021 to be safe
- cargo-watch is archived (Jan 2025) — use bacon instead
- Verification script drafted in .claude/plans/research.md

**Why:** Developer is new to Rust/Tauri; choices favor lowest-friction, proven baseline.
**How to apply:** Use this as the baseline for any setup plan or scaffolding step.
