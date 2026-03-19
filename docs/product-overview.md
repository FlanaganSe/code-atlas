# Product Overview

## What this is

Code Atlas is a local-first desktop app that turns a software repository into an interactive, zoomable architecture map. You point it at a directory containing a Rust Cargo workspace or TypeScript monorepo, and it produces a hierarchical dependency graph — packages, modules, files, and their relationships — rendered as a navigable compound visualization. All analysis runs locally; no code leaves the machine, no build steps execute. Before showing the graph, it provides an upfront compatibility report declaring what it can and cannot analyze, so users never mistake partial coverage for complete coverage.

This is a proof-of-concept (POC, milestones M1–M9 complete). It validates the core thesis: that a static, evidence-backed architecture graph with honest limits is more useful than either a fully-automated-but-opaque tool or a manually-drawn-but-stale diagram.

## Stack

| Layer | Technology | Notes |
|-------|-----------|-------|
| Desktop runtime | **Tauri v2** | Rust backend + webview frontend |
| Core library | **Rust** (edition 2024) | `codeatlas-core` — standalone, zero Tauri dependency |
| Parsing | **tree-sitter** 0.26 | Rust + TypeScript grammars |
| Graph engine | **petgraph** 0.8 | `StableGraph` with stable indices |
| Workspace introspection | **cargo_metadata** 0.19 | Cargo workspace discovery |
| Frontend framework | **React 19** + TypeScript 5.8 | Strict mode, no default exports |
| Graph visualization | **React Flow** v12 (`@xyflow/react`) | Compound nodes, custom edges |
| Layout | **ELK.js** 0.11 | Hierarchical layout in Web Worker |
| State management | **Zustand** 5 | Immutable stores, pure projection |
| Styling | **Tailwind CSS** v4 | Vite plugin, OKLCH color space, dark-mode-only |
| UI components | **shadcn/ui** + Base UI | Tabs, command palette, badges, sheets |
| Linter/formatter | **Biome** 2.x | Replaces ESLint + Prettier (35x faster) |
| Testing | **Vitest** (frontend) + `cargo test` (Rust) | Co-located tests, snapshot/contract tests |
| Build | **Vite** 7 | Fast refresh, path aliases (`@/`) |
| Git hooks | **lefthook** | Parallel: biome check + clippy + rustfmt |

## Architecture

The system has three layers with strict dependency direction:

```
┌─────────────────────────────────────────────────────────────────────┐
│  React Frontend (src/)                                              │
│  React Flow + ELK layout + Zustand stores + projection pipeline     │
│  Receives ScanEvent stream, renders interactive compound graph      │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ Tauri Channel<ScanEvent>
┌──────────────────────────────┴──────────────────────────────────────┐
│  codeatlas-tauri (src-tauri/)                                       │
│  Thin Tauri shell: 4 IPC commands, ScanEvent envelope, ChannelSink  │
│  Adapts core's domain-level ScanSink trait to Tauri Channel<T>      │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ ScanSink trait (domain types only)
┌──────────────────────────────┴──────────────────────────────────────┐
│  codeatlas-core (crates/codeatlas-core/)                            │
│  Standalone library: workspace discovery, detectors, graph model,   │
│  scan pipeline, config, profile, health. Zero Tauri dependency.     │
└─────────────────────────────────────────────────────────────────────┘
```

**How a scan flows end-to-end:**

1. User clicks "Open Directory" → Tauri `open_directory` command opens native file picker
2. `discover_workspace` command runs in a blocking thread → `cargo_metadata` and/or pnpm workspace discovery → returns `DiscoveryResult` (workspace info, config, profile, provisional compatibility report)
3. User clicks "Scan" → `start_scan` command creates `ChannelSink` and runs `codeatlas_core::run_scan()` in a blocking thread
4. Detectors (`RustDetector`, `TypeScriptDetector`) run in three phases — packages → modules → files — streaming results via `ScanSink`
5. `ChannelSink` wraps domain events in `ScanEvent` envelope and sends via `Channel<T>` to the frontend
6. Frontend `useScan` hook receives events, dispatches to graph store (nodes/edges) and scan store (health/compatibility/details)
7. Graph store runs the projection pipeline: merge → filter → suppress → collapse/bundle → sorted React Flow nodes/edges
8. `useLayout` hook triggers ELK layout in a Web Worker (debounced 300ms) → positioned nodes rendered by React Flow

## Directory structure

```
tauri-poc-zoom-thing/
├── crates/
│   └── codeatlas-core/           # Standalone Rust library (all analysis logic)
│       └── src/
│           ├── lib.rs            # AnalysisHost/Analysis, public API
│           ├── graph/            # ArchGraph, types, identity, overlay, query
│           ├── detector/         # Detector trait + Rust/TS implementations
│           ├── scan/             # Scan pipeline, ScanSink trait, ScanResults
│           ├── workspace/        # Cargo + JS workspace discovery
│           ├── config/           # .codeatlas.yaml schema + parsing
│           ├── profile/          # GraphProfile auto-detection
│           ├── health/           # CompatibilityReport + GraphHealth
│           └── error.rs          # Error types (thiserror)
├── src-tauri/                    # Tauri shell (thin adapter)
│   └── src/
│       ├── lib.rs                # AppState, Tauri plugin setup
│       ├── main.rs               # Entry point
│       └── commands.rs           # 4 IPC commands + ScanEvent + ChannelSink
├── src/                          # React frontend
│   ├── App.tsx                   # App shell, discovery/scan workflow, keyboard shortcuts
│   ├── store/                    # Zustand stores
│   │   ├── graph-store.ts        # Graph state, projection trigger, React Flow handlers
│   │   ├── scan-store.ts         # Scan lifecycle, health, compatibility
│   │   └── graph-projection.ts   # Pure projection pipeline (the core frontend contract)
│   ├── hooks/                    # Custom React hooks
│   │   ├── use-scan.ts           # Tauri Channel<ScanEvent> management
│   │   ├── use-layout.ts         # ELK layout trigger on projection changes
│   │   └── viewport-ref.ts       # Module-level React Flow viewport functions
│   ├── components/
│   │   ├── graph/
│   │   │   ├── GraphCanvas.tsx   # React Flow wrapper, event handlers, provenance
│   │   │   ├── nodes/            # PackageNode, ModuleNode, FileNode (memoized)
│   │   │   ├── edges/            # DependencyEdge (color + dash per category)
│   │   │   └── layout/           # elk-layout.ts + elk.worker.ts (Web Worker)
│   │   ├── panels/               # DetailPanel, HealthIndicator, CompatibilityPanel,
│   │   │                         # ProfileBadge, EdgeProvenance
│   │   ├── search/               # CommandPalette (Cmd+K fuzzy search)
│   │   └── ui/                   # shadcn/ui generated components
│   ├── types/                    # TS types mirroring Rust serde output
│   │   ├── graph.ts              # NodeData, EdgeData, MaterializedKey, enums
│   │   ├── scan.ts               # ScanEvent discriminated union, ScanPhase
│   │   └── config.ts             # CompatibilityReport, GraphHealth, WorkspaceInfo
│   ├── constants/                # Shared styling constants (edge colors, labels)
│   └── fixtures/                 # Demo graph fixture (~65 nodes)
├── tests/                        # Rust integration tests
│   ├── fixtures/                 # 4 golden corpus repos (2 Rust, 2 TS)
│   ├── contract_tests.rs         # Rust↔TypeScript serde shape verification
│   ├── golden_corpus.rs          # Assertion-based corpus validation
│   └── snapshot_tests.rs         # insta snapshot tests
└── docs/                         # PRD, research, architecture, decisions
```

## Core concepts

### Identity: MaterializedKey and EdgeId

Every node has a **MaterializedKey**: `{language}:{entity_kind}:{relative_path}` (e.g., `rust:package:crates/codeatlas-core`). No workspace root in the key — keys are portable and privacy-safe by design. Relative paths are normalized: forward slashes, no trailing slash, no `./` prefix.

Every edge has an **EdgeId**: a deterministic hash of `(source_key, target_key, edge_kind, edge_category)`. This supports parallel edges between the same nodes (e.g., both a `value` and `type_only` import from the same source to the same target).

### Two-layer graph: ArchGraph

The graph has an immutable **discovered layer** (populated by detectors) and a mutable **overlay layer** (from `.codeatlas.yaml` config). The overlay can add manual edges and suppress discovered edges, but it can never silently mutate or delete discovered data. This preserves trust — users always know what the scanner found vs. what was manually declared.

### Edge taxonomy

Every edge carries:
- **Kind**: imports, re-exports, contains, depends-on, manual
- **Category**: value, type-only, dev, build, test, peer, normal, manual — enables filtered impact analysis ("show me only runtime dependencies")
- **Confidence**: structural < syntactic < resolver-aware < semantic < runtime — indicates evidence quality
- **Source location**: file path + line range where the import/dependency was found
- **Resolution method**: how the edge was resolved (e.g., "cargo_metadata", "tsconfig paths alias")

### Compatibility report

A first-class trust surface. Before showing the graph, the system declares what it can and cannot analyze per language. Starts **provisional** (manifest-level findings from workspace discovery), becomes **final** after scanning (enriched with source-level findings like dynamic imports, cfg gates). Each feature gets a status: Supported, Partial, or Unsupported with an explanation.

### Graph projection pipeline

The **discovered graph** and the **visible graph** are different things. A pure function `project()` transforms one into the other through four steps:

1. **Overlay merge** — combine discovered edges + manual edges, mark suppressed edges
2. **Category filter** — remove categories the user has hidden
3. **Suppression filter** — hide suppressed edges unless "show suppressed" is on
4. **Collapse projection** — hide descendants of collapsed compound nodes, bundle edges across collapsed boundaries into synthetic bundled edges with counts

This pipeline runs on every store mutation. It is the single source of truth for what React Flow renders.

### Scan lifecycle

Every scan has a UUID `scan_id`. The frontend tracks the active scan ID and silently drops events from stale scans. Only one scan can run at a time — starting a new scan cancels any in-progress scan via `CancellationToken`. Results stream in three phases: package topology → module structure → file-level edges. Cancel preserves partial results.

### AnalysisHost / Analysis pattern

Borrowed from rust-analyzer. `AnalysisHost` is the mutable handle that accepts workspace discovery results and scan data. `Analysis` is an immutable snapshot (Arc-wrapped fields) safe for concurrent queries. In the Tauri shell, the host is held in `Mutex<AnalysisHost>` managed state.

## Key patterns and conventions

### Naming
- **Rust**: snake_case everywhere, types in CamelCase
- **TypeScript**: camelCase fields (matches Rust serde `rename_all = "camelCase"`), types in PascalCase
- **Tauri commands**: snake_case in Rust, camelCase on the TypeScript invoke side (Tauri handles translation)

### File organization
- Co-located tests: `foo.ts` → `foo.test.ts` (frontend), `mod tests` block (Rust)
- Named exports over default exports (TypeScript)
- Custom React Flow node/edge types defined outside components, wrapped with `memo()`
- `nodeTypes`/`edgeTypes` objects are module-level constants (not recreated on render)

### Error handling
- **Rust**: `thiserror` derive for all error types, domain-specific enums aggregated into `CoreError`
- **Frontend**: Tauri command errors surface as `Result<T, String>` — the shell formats error messages
- **Channel sends**: `if let Err(e)` with `eprintln!` logging (no silent swallowing, no panics)

### Serde conventions (cross-boundary types)
- Flat structs: `#[serde(rename_all = "camelCase")]`
- Enums with data: `#[serde(tag = "type", content = "data", rename_all = "camelCase")]` — produces TypeScript-compatible discriminated unions
- Multi-word fields in enum variants need explicit `#[serde(rename = "camelCase")]` because the enum-level `rename_all` only renames variant tags, not inner fields

### State management
- Zustand stores with functional updaters — no direct mutation
- Every graph store mutation triggers the projection pipeline
- Layout coordination uses a `layoutVersion` counter (not direct node watching) to avoid cycles

### Styling
- Dark-mode-only (HTML `class="dark"` hardcoded)
- OKLCH color space for perceptual uniformity
- Edge category colors use the Okabe-Ito palette (colorblind-safe)
- Dual encoding: color + dash pattern for edge categories

## Data layer

No persistent database in the POC. All data is in-memory:
- **Rust side**: `AnalysisHost` holds the `ArchGraph` (petgraph `StableGraph`), workspace info, config, profile, and compatibility report in memory
- **Frontend side**: Zustand stores hold discovered nodes/edges, projection results, scan metadata, and health data
- **Config**: `.codeatlas.yaml` is read from disk on workspace discovery (parsed via `serde_yaml`)
- **No SQLite, no IndexedDB** — MVP phase will add SQLite for persistence and `LineageKey` (UUID-based identity for tracking renames)

## API surface

### Tauri IPC commands (the only API)

| Command | Params | Returns | Purpose |
|---------|--------|---------|---------|
| `open_directory` | — | `Option<String>` | Native file picker, returns directory path |
| `discover_workspace` | `path: String` | `DiscoveryResult` | Discover workspace structure, load config, detect profile |
| `start_scan` | `scan_id: String, on_event: Channel<ScanEvent>` | `()` | Run detectors, stream results via channel |
| `cancel_scan` | — | `()` | Signal cancellation token to stop in-progress scan |

### ScanEvent variants (streamed via Channel<T>)

| Event | Payload | When |
|-------|---------|------|
| `compatibilityReport` | `{ scanId, report }` | After workspace discovery (provisional) and after scan (final) |
| `phase` | `{ scanId, phase, nodes, edges }` | Three times: packages → modules → files |
| `health` | `{ scanId, health }` | After graph health computation |
| `progress` | `{ scanId, scanned, total }` | During file parsing |
| `details` | `{ scanId, unsupportedConstructs, parseFailures, unresolvedImports }` | After scanning |
| `overlay` | `{ scanId, manualEdges, suppressedEdgeIds }` | After overlay application |
| `complete` | `{ scanId }` | Scan finished successfully |
| `error` | `{ scanId, message }` | Scan failed or cancelled |

### Detector trait (internal, for adding languages)

```rust
pub trait Detector: Send + Sync {
    fn name(&self) -> &str;
    fn language(&self) -> Language;
    fn applies_to(&self, workspace: &WorkspaceInfo) -> bool;
    fn compatibility(&self, workspace: &WorkspaceInfo) -> CompatibilityAssessment;
    fn detect(&self, workspace: &WorkspaceInfo, profile: &GraphProfile,
              config: &RepoConfig, sink: &dyn DetectorSink) -> Result<DetectorReport>;
}
```

## Environment and config

### Running locally

```bash
# Prerequisites
rustup install stable          # Rust 1.85+ (edition 2024)
node --version                 # Node.js 22+
corepack enable && corepack prepare pnpm@latest --activate

# Install and run
pnpm install
pnpm dev                       # Launches Tauri dev window with hot reload
```

### Project config file: `.codeatlas.yaml`

Optional file in workspace root. Parsed on discovery, applied during scan.

```yaml
version: 1
ignore:
  - "vendor/**"
  - "generated/**"
entrypoints:
  - path: src/main.ts
    kind: app
dependencies:
  add:
    - from: crates/core
      to: crates/cli
      reason: "Runtime plugin loading not visible to static analysis"
  suppress:
    - from: crates/legacy
      to: crates/deprecated
      reason: "Migration in progress, hiding noise"
```

**Functional sections** (POC): `ignore`, `entrypoints`, `dependencies` (add/suppress).
**Parsed but non-functional** (deferred to MVP): `packages`, `frameworks`, `declarations`.

### Environment variables

| Variable | Purpose |
|----------|---------|
| `TAURI_DEV_HOST` | Optional: remote HMR host for Vite dev server |
| `RUST_LOG` | Optional: tracing filter for Rust backend logging |

No secrets, no API keys, no network configuration — by design.

## Testing

### How to run

```bash
pnpm test                      # Frontend (Vitest): 65 tests across 6 files
cargo test --workspace         # Rust: 126 tests (97 core + 18 contract + 5 golden corpus + 4 snapshot + 2 tauri shell)
cargo clippy --workspace       # Rust lint
pnpm typecheck                 # TypeScript type check
pnpm lint                      # Biome lint
```

### Test structure

**Rust unit tests** — co-located `#[cfg(test)] mod tests` blocks in every module. Cover: MaterializedKey generation/hashing/serde, ArchGraph invariants (duplicate rejection, overlay immutability), config parsing/validation, detector compatibility assessments, tree-sitter query patterns, workspace discovery, scan pipeline phases, cancellation.

**Contract tests** (`tests/contract_tests.rs`) — Serialize each cross-boundary type in Rust, assert JSON field names/structure match TypeScript type definitions. 18 tests covering all types that cross the Rust↔TypeScript boundary. Critical because TypeScript types are manually maintained (no codegen).

**Golden corpus tests** (`tests/golden_corpus.rs`) — 5 assertion-based integration tests scanning real fixture repos:
- `rust-workspace`: multi-crate Cargo workspace with features/targets
- `rust-unsupported`: crate with build.rs, proc-macro, cfg gates
- `ts-monorepo`: pnpm workspace with tsconfig paths
- `ts-unsupported`: workspace with dynamic imports, re-exports, exports conditions
- `self-scan`: this project scans itself

**Snapshot tests** (`tests/snapshot_tests.rs`) — 4 insta snapshot tests capturing full graph output for fixture repos. Catches regressions in scan results.

**Frontend tests** (Vitest) — co-located `*.test.ts` files. Cover: graph store mutations, projection pipeline (collapse/bundle/filter/suppress combinations), scan store event handling (stale rejection, phase appending), ELK layout conversion, initial expansion logic.

### What's well-covered vs. not

- **Strong coverage**: graph model, identity, projection pipeline, detector outputs, cross-boundary contracts
- **Moderate coverage**: UI components (DetailPanel, CommandPalette tested but not exhaustively)
- **Not covered in automated tests**: full Tauri integration (requires running app), ELK Web Worker (mocked in tests), visual regression

## Important decisions and tradeoffs

### Manual TypeScript types over tauri-specta (ADR-001)
TypeScript types in `src/types/` are written by hand to mirror Rust serde output. This adds maintenance burden but avoids tauri-specta (RC status, no Channel<T> support, Tauri 2.10.x compatibility issues). Contract tests verify JSON shape agreement.

### Two-crate architecture (ADR-008)
`codeatlas-core` has zero Tauri dependency. This means the entire analysis engine is testable from `cargo test` without Tauri, reusable for a future CLI tool, and the core/shell boundary is enforced at the crate level. The tradeoff is a `ScanSink` trait adapter pattern — the shell must implement `ChannelSink` to bridge domain events to Tauri's `Channel<T>`.

### Projection as pure function (ADR-007)
The graph projection pipeline is a pure function: `project(input) → { nodes, edges }`. It runs on every store mutation (typically < 5ms). This makes the rendering pipeline independently testable and deterministic, at the cost of recomputing on every change rather than incremental updates. For the POC graph sizes (< 500 nodes), this is fast enough.

### ELK in Web Worker from the start (ADR-006)
Layout computation runs in a Web Worker even for small graphs. This prevents any UI thread blocking (PRD NF3/NF10) and means layout performance is proven from M3 rather than retrofitted. Falls back to main-thread ELK if the worker fails to load.

### MaterializedKey without workspace root (ADR-004)
Keys are portable between machines — `rust:file:crates/core/src/lib.rs` works regardless of where the repo is cloned. Workspace root is session metadata on `AnalysisHost`, not baked into identity. This avoids a rework that research-strategy warned about for MVP.

### Dark-mode-only
The HTML entry point hardcodes `class="dark"`. All CSS variables are dark-themed. This is a deliberate POC simplification — light mode would require a second set of OKLCH color tokens and contrast testing.

### No React Compiler
React Compiler (v1.0, stable) was deferred. All custom node/edge components use manual `memo()`. The decision was to prove React Flow compound rendering works before adding the compiler, which has unknown interactions with React Flow's internal rendering.

## Gotchas

**tree-sitter anonymous nodes** — The `type` keyword in TypeScript `import type` is an anonymous node. `child_by_field_name()` won't find it. The detector iterates children and checks `kind() == "type"` instead. This is a recurring source of bugs in tree-sitter TypeScript parsing.

**ELK ↔ React Flow conversion** — ELK expects hierarchical `children` arrays; React Flow uses flat arrays with `parentId`. The `toElkGraph()` / `fromElkGraph()` functions in `elk-layout.ts` bridge these. Expanded compound nodes must omit `width`/`height` so ELK computes them from children. Getting this wrong produces overlapping or invisible nodes.

**Parent-before-child ordering** — React Flow silently fails to render children that appear before their parents in the nodes array. The projection pipeline enforces this via `sortParentsFirst()`. Breaking this invariant produces a blank canvas with no error message.

**Serde variant vs. field renaming** — `#[serde(rename_all = "camelCase")]` on an enum renames variant tags (e.g., `CompatibilityReport` → `"compatibilityReport"`), but does NOT rename fields inside struct variants. Multi-word fields need explicit `#[serde(rename = "fieldName")]`. Mismatches cause silent deserialization failures on the frontend.

**`cargo_metadata` first-call latency** — The first `cargo metadata` call can take 2–10 seconds (builds dependency tree). Runs in `tokio::task::spawn_blocking()`. The compatibility report streams first while metadata resolves.

**`DefaultHasher` for EdgeId** — EdgeId uses `std::hash::DefaultHasher`, which is not guaranteed stable across Rust versions. This is acceptable for the POC (IDs are session-scoped, not persisted), but must be replaced with a stable hasher (e.g., `siphasher`) before persistence is added in MVP.

**Edge bundling after category filtering** — Category filtering happens before collapse projection. If the user filters to "runtime only," bundled edge counts between collapsed packages reflect only the filtered edges. A bundle might show "3 imports" with dev filtered out vs. "7 imports" with all categories. This is intentional but can surprise users.

**Channel send failures** — If the frontend disconnects during a scan, `ChannelSink` logs the failure via `eprintln!` but continues scanning. The scan completes and results are applied to the host regardless of whether the frontend received them.
