---
description: Non-negotiable project rules. Violations must be flagged immediately.
---
# Immutable Rules

1. **`codeatlas-core` has zero Tauri dependency** — Core is a standalone analysis library testable without the Tauri shell. Transport types (ScanEvent envelope) belong in `codeatlas-tauri`, not core.
2. **Discovered graph is immutable** — Config overlays (manual edges, suppressions) supplement but never silently mutate what the scanner discovered. Overlay cannot delete edges from the discovered layer.
3. **No code execution during analysis** — build.rs, proc macros, package scripts, and bundlers are never executed. Constructs requiring execution are detected and badged, not hidden.
4. **Parent nodes before children in React Flow arrays** — React Flow silently breaks if children appear before parents. The projection pipeline enforces this via `sortParentsFirst()`.
5. **All cross-boundary enums use adjacently tagged serde** — `#[serde(tag = "type", content = "data", rename_all = "camelCase")]` for TypeScript discriminated union compatibility. Mismatched attributes cause silent runtime failures.

<!-- Add new invariants as discovered, with one-line justification. -->
