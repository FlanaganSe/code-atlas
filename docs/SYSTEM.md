# System

## What this system does

Code Atlas is a local-first desktop application that builds a profiled, evidence-backed architecture graph from a software repository and renders it as an interactive, zoomable map. It parses Rust and TypeScript source code locally, resolves dependencies within an explicit build context, and produces a hierarchical visualization where developers move from workspace-level structure to file-level detail on a single continuous canvas. Before showing the graph, it provides an upfront compatibility report declaring what it can and cannot analyze.

## Domain model

- **ArchGraph** — Two-layer graph wrapping petgraph's `StableGraph`. The discovered layer (from scanning) is immutable; the overlay layer (from `.codeatlas.yaml` config) adds manual edges and suppressions without mutating discovered data.
- **NodeData** — A node in the graph (package, module, or file) identified by a `MaterializedKey`.
- **EdgeData** — A directed dependency edge with category (value, type-only, dev, build, normal, manual), confidence level, source location, and resolution method.
- **MaterializedKey** — Identity scheme: `{language}:{entity_kind}:{relative_path}`. No workspace root in the key (portable, privacy-safe).
- **GraphOverlay** — Config-driven additions (manual edges) and suppressions. Overlay operations are auditable and reversible.
- **CompatibilityReport** — First-class trust surface declaring Supported/Partial/Unsupported per language, with specific feature-level details. Starts provisional (structural assessment) and becomes final after scan.
- **GraphHealth** — Metrics snapshot: total nodes, resolved edges, unresolved imports, parse failures, unsupported constructs.
- **GraphProfile** — Build context: languages, package manager, resolution mode, cargo features, fingerprint.

## Architecture

```
codeatlas-core (Rust)          codeatlas-tauri (Rust)         React frontend
────────────────────           ──────────────────────         ──────────────
Standalone analysis            Thin Tauri shell               Rendering layer
library. Zero Tauri            Adapts core's ScanSink         React Flow + ELK
dependency.                    to Channel<ScanEvent>.         + Zustand stores.

Detector trait                 Tauri commands:                Graph projection:
├── RustDetector               ├── open_directory             discovered graph
│   (cargo_metadata +          ├── discover_workspace          → overlay merge
│    tree-sitter)              ├── start_scan                  → category filter
└── TypeScriptDetector         └── cancel_scan                 → collapse/bundle
    (tree-sitter +                                             → React Flow nodes
     tsconfig paths)           ScanEvent envelope:
                               Channel<T> streaming
AnalysisHost → Analysis        (progressive phases)
(mutable)      (immutable
                snapshot)
```

Data flows: `Detector.detect()` → `ScanSink.on_phase()` → `Channel<ScanEvent>` → frontend `onmessage` → Zustand stores → `project()` pure function → React Flow → ELK layout (Web Worker).

## Constraints

- **No code execution** — build.rs, proc macros, package scripts, and bundlers are never executed. Constructs requiring execution are detected and badged, not hidden.
- **Local-only** — zero network calls during analysis. All processing happens on the user's machine.
- **Immutable discovered graph** — config overlays (manual edges, suppressions) supplement but never silently mutate what the scanner discovered.
- **Core/shell separation** — `codeatlas-core` has zero Tauri dependency. The `ScanEvent` transport envelope belongs in `codeatlas-tauri`, not core.
- **Parent-before-child ordering** — React Flow requires parent nodes to appear before children in the nodes array. The projection pipeline enforces this via `sortParentsFirst()`.

## Key patterns

- **Detector trait** — Each language analyzer implements `Detector` with `compatibility()` (pre-scan assessment) and `detect()` (three-phase scanning: packages → modules → files). New languages are added by implementing this trait.
- **AnalysisHost / Analysis** — Mutable host accepts scan results; immutable `Analysis` snapshot is safe for concurrent queries. Pattern borrowed from rust-analyzer.
- **Progressive streaming** — Scan results stream via `ScanSink` in three phases. The frontend renders incrementally — package topology appears first, then modules, then file-level edges. Channel<T> delivers ordered events.
- **Graph projection** — Pure function `project(input) → { nodes, edges }` transforms the discovered graph through: overlay merge → category filter → suppression → collapse/bundle. This is the single source of truth for what React Flow renders. All mutations re-run projection.
- **ELK in Web Worker** — Layout computation runs in a dedicated Web Worker with a 5-second timeout and main-thread fallback. Layout is debounced at 300ms to avoid thrash during rapid expand/collapse.
- **Edge taxonomy** — Every edge carries a semantic category (value, type-only, dev, build, normal, manual) enabling filtered impact analysis. Color + dash pattern dual-encoding (Okabe-Ito palette) ensures colorblind accessibility.

## Gotchas

- **tree-sitter anonymous nodes** — The `type` keyword in TypeScript `import type` statements is an anonymous node. You cannot use `child_by_field_name()` — iterate children and check `kind() == "type"` instead.
- **ELK compound layout** — ELK expects hierarchical `children` arrays. React Flow uses flat arrays with `parentId`. The `toElkGraph()` / `fromElkGraph()` conversions in `elk-layout.ts` bridge these representations. Expanded compound nodes must omit `width`/`height` so ELK computes them from children.
- **Parent-before-child ordering** — React Flow silently breaks if children appear before parents in the node array. The projection always sorts with `sortParentsFirst()`.
- **Serde tagged enums** — All cross-boundary enums use `#[serde(tag = "type", content = "data", rename_all = "camelCase")]` to produce TypeScript-compatible discriminated unions. Mismatched serde attributes cause silent runtime deserialization failures.
- **Stale scan events** — Each scan has a UUID `scan_id`. The frontend drops events from stale scans. Starting a new scan cancels any in-progress scan via `CancellationToken`.
- **React Compiler deferred** — Manual `memo()` is used on all custom node/edge components. React Compiler integration was deferred pending React Flow compatibility verification.
