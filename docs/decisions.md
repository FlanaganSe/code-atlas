# Decisions

Append-only log. Never edit past entries.

## Format
```
### ADR-NNN: [Title]
**Date:** YYYY-MM-DD
**Status:** accepted | superseded by ADR-NNN
**Context:** [Why — 1-2 sentences]
**Decision:** [What — 1-2 sentences]
**Consequences:** [What follows]
```

---

<!-- Add new decisions below this line -->

### ADR-001: Manual TypeScript types over tauri-specta
**Date:** 2026-03-18
**Status:** accepted
**Context:** tauri-specta is in RC, has no Channel<T> support, and has compatibility issues with Tauri 2.10.x.
**Decision:** Write TypeScript types manually in `src/types/` mirroring Rust serde output. Add contract tests to verify JSON shape agreement.
**Consequences:** Must maintain TS types manually. Contract tests catch drift. Can revisit when tauri-specta reaches stable.

### ADR-002: Biome 2.x over ESLint + Prettier
**Date:** 2026-03-18
**Status:** accepted
**Context:** Need a linter and formatter. Biome is 35x faster and provides both in a single tool.
**Decision:** Use Biome 2.x for frontend linting and formatting. No ESLint or Prettier.
**Consequences:** Single config file (`biome.json`). Some ESLint-only rules unavailable; Biome's React domain rules cover the critical ones.

### ADR-003: Rust edition 2024 for all crates
**Date:** 2026-03-18
**Status:** accepted
**Context:** Edition 2024 is stable since Rust 1.85, bringing async fn in traits and let chains.
**Decision:** Use `edition = "2024"` in all workspace crates.
**Consequences:** Requires Rust 1.85+. Enables modern patterns without nightly.

### ADR-004: MaterializedKey format — portable from the start
**Date:** 2026-03-18
**Status:** accepted
**Context:** PRD and research conflict on whether workspace root should be part of the key. Including it creates privacy and portability issues.
**Decision:** Key format is `{language}:{entity_kind}:{relative_path}` — no workspace root. Workspace root is session metadata on AnalysisHost.
**Consequences:** Keys are portable and privacy-safe. No MVP rework needed for key format.

### ADR-005: ScanEvent envelope in shell, not core
**Date:** 2026-03-18
**Status:** accepted
**Context:** Core library should be transport-agnostic. ScanEvent is a Tauri Channel<T> transport concern.
**Decision:** `codeatlas-core` exports domain types + `ScanSink` trait. `codeatlas-tauri` wraps sink into `Channel<ScanEvent>` envelope.
**Consequences:** Core is testable without Tauri. Shell adapts to transport. New transports (CLI, WebSocket) just implement ScanSink.

### ADR-006: ELK layout in Web Worker from M3
**Date:** 2026-03-18
**Status:** accepted
**Context:** PRD NF3 requires <500ms ELK layout in Web Worker. NF10 requires UI thread never blocked. API is identical whether bundled or worker mode.
**Decision:** Run ELK.js in a Web Worker from the first graph rendering milestone, not as a later optimization.
**Consequences:** No main-thread blocking for layout. Worker file setup required in Vite. Falls back to main thread if worker fails.

### ADR-007: Zustand store with explicit projection pipeline
**Date:** 2026-03-18
**Status:** accepted
**Context:** The discovered graph and visible graph are different. Need a clear, testable transformation pipeline.
**Decision:** Graph store uses Zustand with a pure `project()` function: discovered → filtered → projected → React Flow nodes/edges.
**Consequences:** Projection is independently testable. State transitions are predictable. Bundled edges are computed projections, not stored entities.

### ADR-008: Two-crate Cargo workspace architecture
**Date:** 2026-03-18
**Status:** accepted
**Context:** Analysis logic must be usable without Tauri (for testing, CLI, future transports). Tauri CLI requires `src-tauri/` directory.
**Decision:** Virtual workspace with `codeatlas-core` (standalone library, zero Tauri deps) at `crates/codeatlas-core/` and `codeatlas-tauri` (thin shell) at `src-tauri/`.
**Consequences:** Core can be tested in CI without Tauri. Shell is thin. Tauri CLI works with standard `src-tauri/` path.
