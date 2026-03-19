# Code Atlas — Implementation Plan

**Date:** 2026-03-18
**Status:** Draft v2 — incorporates architecture review feedback
**Review basis:** External architecture review identifying 14 issues (7 fixes, 3 clarifications, 2 structural splits, 2 no-change)
**Scope:** POC phase (M1–M9), with forward-looking notes for MVP
**Basis:** PRD v5, implementation research, strategy research, targeted library verification

---

## 1. Summary

Code Atlas is a local-first desktop app that builds a **profiled, evidence-backed architecture graph** from a repository and renders it as an interactive, hierarchically-zoomable map. The core architectural decision is a **two-crate Cargo workspace**: `codeatlas-core` (standalone analysis library with zero Tauri dependency) and `codeatlas-tauri` (thin Tauri shell). The frontend uses React 19 + React Flow v12 + ELK.js for compound graph layout + Zustand for state management + Tailwind CSS v4 + shadcn/ui for non-graph UI.

The build sequence follows the PRD's "trust-first" principle: lock the identity model and data types → workspace discovery + compatibility report → prove React Flow + ELK compound rendering → connect real scanning with streaming → layer health/provenance/config → add interactivity → validate against golden corpus. Every milestone is independently verifiable and committable. The plan prioritizes correctness, provenance integrity, and honest limits over polish.

---

## 2. Current State

The project is an **empty scaffold** — documentation only, no source code. The repo contains:

- `docs/prd.md` — Product Requirements Document v5 (comprehensive, ~1,400 lines)
- `docs/research-implementation.md` — Library APIs, architecture patterns, version references
- `docs/research-strategy.md` — Product direction, competitive landscape, UX, risks
- `docs/decisions.md` — Empty ADR log (template only)
- `docs/SYSTEM.md` — Empty system doc (template only)
- `CLAUDE.md` — Project instructions for Claude
- `.claude/rules/` — Conventions, stack, immutable rules (templates)
- `.gitignore` — Standard Tauri/Node/Rust ignores
- No `Cargo.toml`, no `package.json`, no `src/`, no `src-tauri/`

**The entire project needs to be scaffolded from scratch.**

---

## 3. Repository Layout (Target)

```
tauri-poc-zoom-thing/
├── Cargo.toml                    # Virtual workspace manifest
├── package.json                  # Frontend: React + Vite + Tauri CLI
├── pnpm-lock.yaml
├── biome.json                    # Biome 2.x config
├── lefthook.yml                  # Git hooks
├── vite.config.ts                # Vite + Tailwind + React Compiler
├── tsconfig.json                 # TypeScript config
├── tsconfig.node.json            # Node config for vite.config.ts
├── index.html                    # Vite entry
├── src/                          # React frontend
│   ├── main.tsx                  # React root
│   ├── App.tsx                   # Main app shell
│   ├── index.css                 # Tailwind entry (@import "tailwindcss")
│   ├── lib/                      # Shared utilities
│   │   └── utils.ts              # cn() helper for shadcn/ui
│   ├── components/
│   │   ├── ui/                   # shadcn/ui components (generated)
│   │   ├── graph/                # React Flow graph components
│   │   │   ├── GraphCanvas.tsx   # Main React Flow wrapper
│   │   │   ├── nodes/            # Custom node components
│   │   │   │   ├── PackageNode.tsx
│   │   │   │   ├── ModuleNode.tsx
│   │   │   │   └── FileNode.tsx
│   │   │   ├── edges/            # Custom edge components
│   │   │   │   └── DependencyEdge.tsx
│   │   │   └── layout/           # ELK layout logic
│   │   │       ├── elk-layout.ts # ELK graph conversion + layout
│   │   │       └── elk.worker.ts # Web Worker (required — PRD NF3/NF10)
│   │   ├── panels/               # UI panels
│   │   │   ├── DetailPanel.tsx
│   │   │   ├── CompatibilityPanel.tsx
│   │   │   ├── HealthIndicator.tsx
│   │   │   └── ProfileBadge.tsx
│   │   └── search/               # Command palette
│   │       └── CommandPalette.tsx
│   ├── store/                    # Zustand stores
│   │   ├── graph-store.ts        # Graph nodes/edges/expanded state
│   │   ├── scan-store.ts         # Scan status/progress
│   │   └── ui-store.ts           # Panel state, selection, search
│   ├── hooks/                    # Custom React hooks
│   │   ├── use-scan.ts           # Tauri invoke/channel for scanning
│   │   └── use-layout.ts         # ELK layout trigger
│   └── types/                    # Shared TypeScript types
│       ├── graph.ts              # Node/edge/evidence types (mirrors Rust)
│       ├── scan.ts               # Scan event types
│       └── config.ts             # Profile/compatibility types
├── crates/
│   ├── codeatlas-core/           # Standalone analysis library
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # Public API: AnalysisHost, Analysis
│   │       ├── workspace/        # Workspace discovery
│   │       │   ├── mod.rs
│   │       │   ├── cargo.rs      # cargo_metadata integration
│   │       │   └── javascript.rs # JS/TS workspace detection
│   │       ├── detector/         # Detector trait + implementations
│   │       │   ├── mod.rs        # Detector trait, DetectorRegistry
│   │       │   ├── rust.rs       # Rust detector (cargo + tree-sitter)
│   │       │   └── typescript.rs # TypeScript detector
│   │       ├── graph/            # Graph model
│   │       │   ├── mod.rs
│   │       │   ├── types.rs      # NodeData, EdgeData, enums
│   │       │   ├── arch_graph.rs # ArchGraph wrapper around StableGraph
│   │       │   ├── overlay.rs    # GraphOverlay (manual edges, suppressions)
│   │       │   ├── identity.rs   # MaterializedKey, LineageKey
│   │       │   └── query.rs      # Query methods on Analysis snapshot
│   │       ├── config/           # .codeatlas.yaml
│   │       │   ├── mod.rs
│   │       │   └── schema.rs     # Config types + serde + validation
│   │       ├── profile/          # Graph profile
│   │       │   └── mod.rs        # GraphProfile, ProfileFingerprint
│   │       ├── health/           # Compatibility + graph health
│   │       │   ├── mod.rs
│   │       │   ├── compatibility.rs  # CompatibilityReport
│   │       │   └── graph_health.rs   # GraphHealth metrics
│   │       ├── scan/             # Scan orchestration
│   │       │   ├── mod.rs
│   │       │   └── pipeline.rs   # Streaming scan pipeline
│   │       └── error.rs          # Error types (thiserror)
│   └── codeatlas-tauri/          # Tauri desktop app (thin shell)
│       ├── Cargo.toml
│       ├── build.rs              # Tauri build script
│       ├── tauri.conf.json       # Tauri configuration
│       ├── capabilities/
│       │   └── default.json      # Security capabilities
│       ├── icons/                # App icons
│       └── src/
│           ├── main.rs           # Tauri entry point
│           ├── lib.rs            # Tauri command registration
│           └── commands.rs       # Tauri commands (thin wrappers)
├── tests/                        # Integration tests
│   └── fixtures/                 # Golden corpus test fixtures
│       ├── rust-workspace/       # Minimal Rust workspace
│       └── ts-monorepo/          # Minimal TS monorepo
├── docs/                         # Documentation (existing)
└── .claude/                      # Claude instructions (existing)
```

---

## 4. Files to Create

### Phase: Scaffold (M1)

| File | Purpose | Pattern/Notes |
|------|---------|---------------|
| `Cargo.toml` | Virtual workspace manifest | `[workspace] members = ["crates/*"]`, `resolver = "2"`, shared `[workspace.dependencies]` |
| `crates/codeatlas-core/Cargo.toml` | Core library crate | `edition = "2024"`, deps: tree-sitter, petgraph, cargo_metadata, serde, thiserror, tracing, rayon, tokio, serde_yaml, globset, camino, ignore |
| `crates/codeatlas-core/src/lib.rs` | Core public API surface | Exports `AnalysisHost`, `Analysis`, all domain types |
| `crates/codeatlas-core/src/error.rs` | Error types | `thiserror` derive, per-module error enums |
| `crates/codeatlas-core/src/graph/mod.rs` | Graph module root | Re-exports types, ArchGraph, overlay |
| `crates/codeatlas-core/src/graph/types.rs` | All domain types | NodeData, EdgeData, NodeKind, EdgeKind, EdgeCategory, Confidence, Language, EntityKind, etc. |
| `crates/codeatlas-core/src/graph/identity.rs` | Identity scheme | MaterializedKey, LineageKey (stub), key generation functions |
| `crates/codeatlas-core/src/graph/arch_graph.rs` | Graph wrapper | `ArchGraph` wrapping `StableGraph` with invariant enforcement |
| `crates/codeatlas-core/src/graph/overlay.rs` | Config overlay | `GraphOverlay` struct with manual edges, suppressions |
| `crates/codeatlas-core/src/graph/query.rs` | Query API | Methods on `Analysis` snapshot: neighbors, transitive deps, health |
| `crates/codeatlas-core/src/detector/mod.rs` | Detector trait | `Detector` trait, `DetectorSink` trait, `DetectorRegistry`, `DetectorReport`, `CompatibilityAssessment` |
| `crates/codeatlas-core/src/config/mod.rs` | Config module root | Re-exports |
| `crates/codeatlas-core/src/config/schema.rs` | `.codeatlas.yaml` types | `RepoConfig` with serde, validation, version field |
| `crates/codeatlas-core/src/profile/mod.rs` | Graph profile | `GraphProfile`, `ProfileFingerprint`, profile detection |
| `crates/codeatlas-core/src/health/mod.rs` | Health module root | Re-exports |
| `crates/codeatlas-core/src/health/compatibility.rs` | Compatibility report | `CompatibilityReport`, `SupportStatus`, `UnsupportedConstruct` |
| `crates/codeatlas-core/src/health/graph_health.rs` | Graph health | `GraphHealth` metrics struct |
| `crates/codeatlas-core/src/workspace/mod.rs` | Workspace discovery | `WorkspaceInfo`, discovery orchestration |
| `crates/codeatlas-core/src/scan/mod.rs` | Scan module | Scan orchestration, domain result types (`ScanResults`, `ScanPhase`). **No transport/streaming types** — `ScanEvent` envelope lives in `codeatlas-tauri`. |
| `crates/codeatlas-core/src/scan/pipeline.rs` | Scan pipeline | Phased scanning with channel output |
| `crates/codeatlas-tauri/Cargo.toml` | Tauri app crate | Depends on `codeatlas-core`, `tauri`, `tauri-plugin-dialog`, `tauri-plugin-shell`, `serde_json` |
| `crates/codeatlas-tauri/build.rs` | Tauri build | `tauri_build::build()` |
| `crates/codeatlas-tauri/tauri.conf.json` | Tauri config | App identifier, window config, bundle config |
| `crates/codeatlas-tauri/capabilities/default.json` | Capabilities | `dialog:allow-open`, scoped fs |
| `crates/codeatlas-tauri/src/main.rs` | Entry point | `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` + main |
| `crates/codeatlas-tauri/src/lib.rs` | Tauri setup | Plugin registration, command handler setup, state management |
| `crates/codeatlas-tauri/src/commands.rs` | Tauri commands | `open_directory`, `start_scan`, `cancel_scan` — thin wrappers over core |
| `package.json` | Frontend deps | React 19, @xyflow/react, elkjs, zustand, @tauri-apps/api, @tauri-apps/plugin-dialog, tailwindcss, motion, cmdk |
| `vite.config.ts` | Build config | React plugin + Tailwind plugin + path aliases (React Compiler deferred to M9) |
| `tsconfig.json` | TS config | `"moduleResolution": "bundler"`, path aliases for `@/` |
| `biome.json` | Linter/formatter | Biome 2.x, React domain rules, import sorting |
| `lefthook.yml` | Git hooks | `biome check`, `cargo clippy`, `cargo fmt --check` in parallel |
| `index.html` | Vite entry | Minimal HTML with `<div id="root">` |
| `src/main.tsx` | React entry | `createRoot`, render `<App />` |
| `src/index.css` | Tailwind entry | `@import "tailwindcss"` + theme vars |
| `src/App.tsx` | App shell | Layout skeleton, ReactFlowProvider |
| `src/lib/utils.ts` | Utilities | `cn()` for class merging (shadcn/ui pattern) |
| `src/types/graph.ts` | TS graph types | Mirrors Rust domain types: NodeData, EdgeData, MaterializedKey, etc. |
| `src/types/scan.ts` | TS scan types | ScanEvent discriminated union, ScanPhase, ScanStatus |
| `src/types/config.ts` | TS config types | CompatibilityReport, GraphProfile, GraphHealth |

### Phase: Graph Rendering (M3)

| File | Purpose |
|------|---------|
| `src/components/graph/GraphCanvas.tsx` | Main React Flow canvas wrapper with controls, MiniMap |
| `src/components/graph/nodes/PackageNode.tsx` | Custom compound node for packages/crates |
| `src/components/graph/nodes/ModuleNode.tsx` | Custom compound node for modules/folders |
| `src/components/graph/nodes/FileNode.tsx` | Custom leaf node for files |
| `src/components/graph/edges/DependencyEdge.tsx` | Custom edge with category styling (color + dash pattern) |
| `src/components/graph/layout/elk-layout.ts` | ELK graph conversion: RF nodes → ELK hierarchical → positioned RF nodes |
| `src/components/graph/layout/elk.worker.ts` | Web Worker for ELK layout (required — PRD NF3/NF10) |
| `src/store/graph-store.ts` | Zustand store for graph state: nodes, edges, expandedIds, projection pipeline |
| `src/store/ui-store.ts` | Zustand store for UI state: selectedNode, panelOpen, searchQuery |
| `src/hooks/use-layout.ts` | Hook to trigger ELK layout on expand/collapse/data changes |

### Phase: Scanning (M4–M5)

| File | Purpose |
|------|---------|
| `crates/codeatlas-core/src/workspace/cargo.rs` | cargo_metadata integration |
| `crates/codeatlas-core/src/workspace/javascript.rs` | JS/TS workspace detection (pnpm-workspace.yaml, package.json workspaces) |
| `crates/codeatlas-core/src/detector/rust.rs` | Rust detector: cargo_metadata + tree-sitter mod/use/pub use parsing |
| `crates/codeatlas-core/src/detector/typescript.rs` | TypeScript detector: tree-sitter imports + tsconfig paths resolution |
| `src/store/scan-store.ts` | Zustand store for scan status/progress |
| `src/hooks/use-scan.ts` | Hook wrapping Tauri Channel<T> for scan streaming |
| `tests/fixtures/rust-workspace/` | Rust fixture 1: multi-crate workspace with features/targets |
| `tests/fixtures/rust-unsupported/` | Rust fixture 2: crate with build.rs, proc-macro, cfg gates |
| `tests/fixtures/ts-monorepo/` | TS fixture 1: pnpm workspace with tsconfig paths |
| `tests/fixtures/ts-unsupported/` | TS fixture 2: workspace with dynamic imports, re-exports, exports conditions |

### Phase: Health/Provenance/Config (M6)

| File | Purpose |
|------|---------|
| `src/components/panels/CompatibilityPanel.tsx` | Compatibility report display |
| `src/components/panels/HealthIndicator.tsx` | Graph health badge/summary |
| `src/components/panels/ProfileBadge.tsx` | Active profile display |

### Phase: Interactive (M7)

| File | Purpose |
|------|---------|
| `src/components/panels/DetailPanel.tsx` | Node detail panel with tabs: Overview, Dependencies, Exports, Health |
| `src/components/search/CommandPalette.tsx` | Cmd+K search using shadcn/ui Command (cmdk) |

---

## 5. Verified Dependency Versions

### Rust (Cargo.toml workspace dependencies)

| Crate | Version | Notes |
|-------|---------|-------|
| `tree-sitter` | `0.26` | Stable. `StreamingIterator` API for query results. |
| `tree-sitter-rust` | `0.24` | |
| `tree-sitter-typescript` | `0.23` | Contains both `LANGUAGE_TYPESCRIPT` and `LANGUAGE_TSX` |
| `petgraph` | `0.8` | `StableGraph` with stable indices. `tarjan_scc()` works. |
| `cargo_metadata` | `0.23` | Wraps `cargo metadata --format-version 1` |
| `serde` | `1` | Features: `derive` |
| `serde_json` | `1` | For Tauri IPC serialization |
| `serde_yaml` | `0.9` | For `.codeatlas.yaml` |
| `thiserror` | `2` | Breaking from 1.x: needs direct dependency |
| `tracing` | `0.1` | |
| `tracing-subscriber` | `0.3` | With `env-filter` feature |
| `rayon` | `1.10` | Parallel file parsing |
| `tokio` | `1` | Features: `full` (Tauri uses tokio internally) |
| `camino` | `1` | UTF-8 path types |
| `ignore` | `0.4` | `.gitignore`-aware file walking |
| `globset` | `0.4` | Glob pattern matching for config ignore paths |
| `tauri` | `2` | Only in `codeatlas-tauri` |
| `tauri-plugin-dialog` | `2` | Native file dialog |
| `tauri-plugin-shell` | `2` | For editor open (stretch) |

**Explicitly NOT using:** `tauri-specta` (RC, no Channel<T> support, breakage with Tauri 2.10.x). Manual TS types instead.

### Frontend (package.json)

| Package | Version | Notes |
|---------|---------|-------|
| `react` | `^19` | |
| `react-dom` | `^19` | |
| `@xyflow/react` | `^12` | React Flow v12. Uses `parentId` not `parentNode`. |
| `elkjs` | `^0.9` | EPL-2.0 license — legal review needed |
| `zustand` | `^5` | |
| `@tauri-apps/api` | `^2` | Core Tauri IPC |
| `@tauri-apps/plugin-dialog` | `^2` | |
| `tailwindcss` | `^4` | CSS-first config |
| `@tailwindcss/vite` | `^4` | Vite plugin |
| `motion` | `^12` | (Formerly `framer-motion`). Import from `motion/react`. |
| `lucide-react` | latest | Icons (shadcn/ui default) |
| `class-variance-authority` | latest | shadcn/ui dependency |
| `clsx` | latest | shadcn/ui dependency |
| `tailwind-merge` | latest | shadcn/ui dependency |

**Dev dependencies:**

| Package | Version | Notes |
|---------|---------|-------|
| `@tauri-apps/cli` | `^2` | Tauri CLI |
| `@biomejs/biome` | `2.4` | Pin exact (`-E` flag) |
| `typescript` | `^5.7` | |
| `vite` | `^6` | Or latest stable. Check React Compiler path. |
| `@vitejs/plugin-react` | `^4` | With Babel for React Compiler |
| `babel-plugin-react-compiler` | `^1` | React Compiler — **deferred to M9**, not installed in M1 |
| `vitest` | latest | Frontend testing |
| `@testing-library/react` | latest | Component testing |
| `lefthook` | `^2` | Git hooks |

---

## 6. Milestone Outline

### Phase A — Trustworthy POC Core

#### M1: Scaffold + Architecture
**Goal:** Tauri v2 + React + Vite builds and launches. `codeatlas-core` exists with all domain types, traits, and the identity scheme. No functionality yet — just the skeleton.

- [x] M1: Scaffold + Architecture — buildable Tauri app with core crate + domain types + detector trait
  - [x] Step 1 — Scaffold Tauri v2 + React app with `pnpm create tauri-app` → verify: `pnpm tauri dev` launches
  - [x] Step 2 — Cargo workspace with src-tauri/ + crates/codeatlas-core/ (spike: crates/codeatlas-tauri/ skipped, Tauri CLI requires src-tauri/) → verify: `cargo check --workspace`
  - [x] Step 3 — Create codeatlas-core with all domain types, traits, identity scheme, graph model, config schema, overlay model → verify: `cargo check -p codeatlas-core`
  - [x] Step 4 — Set up Biome 2.x, lefthook, Tailwind CSS v4 → verify: `pnpm biome check`
  - [x] Step 5 — Write TypeScript types in src/types/ mirroring Rust serde output → verify: `pnpm typecheck`
  - [x] Step 6 — Write Rust unit tests + serde contract tests (38 tests) → verify: `cargo test --workspace`
  - [x] Step 7 — Final verification: `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, `pnpm typecheck`, `pnpm biome check`
  Commit: "feat: scaffold Tauri v2 app with codeatlas-core domain types and architecture"

**What to verify:**
- `pnpm dev` launches the Tauri window with a React "hello world"
- `cargo test -p codeatlas-core` passes (empty tests)
- `cargo clippy --workspace` clean
- `pnpm typecheck` passes
- `codeatlas-core` has **zero** Tauri imports and **zero** transport/streaming types (no `ScanEvent` envelope — that belongs in `codeatlas-tauri`)
- All domain types compile with serde Serialize/Deserialize
- Detector trait compiles (including `compatibility()` and `detect()` signatures)
- MaterializedKey: generation from components, hashing, equality, Display
- EdgeId: generation from (source_key, target_key, kind, category) tuple, uniqueness within graph
- `ArchGraph` enforces: no duplicate MaterializedKeys, overlay cannot mutate discovered layer
- Serde round-trip test: Rust serialize → JSON → verify camelCase keys match TypeScript type definitions

**Critical implementation notes:**
1. **Scaffold spike:** Scaffold with `pnpm create tauri-app` first, then restructure into workspace. Move `src-tauri/` → `crates/codeatlas-tauri/`. Add `crates/codeatlas-core/`. **This is a spike with a go/no-go checkpoint** — if the custom directory layout breaks Tauri CLI, fall back to `src-tauri/` naming with a path dependency. Verify `pnpm tauri dev` and `pnpm tauri build` both work before proceeding.
2. Root `Cargo.toml` is a virtual workspace. Use `[workspace.dependencies]` for shared dep versions.
3. All domain types from PRD §§9–11 must be defined. **Critically, this includes edge identity** — see "Identity Decisions" below.
4. **Core/shell boundary rule:** `codeatlas-core` exports domain types (`ScanResults`, `ScanPhase`, `CompatibilityReport`, `GraphHealth`, etc.) and a `ScanSink` trait for streaming output. The `ScanEvent` transport envelope (with `Progress`, `Complete`, `Error` variants) belongs in `codeatlas-tauri/src/commands.rs` and adapts core's sink into Tauri's `Channel<T>`. This preserves the PRD §16 requirement that the core API uses domain terminology, not transport terminology.
5. Use adjacently tagged serde enums for all discriminated unions: `#[serde(tag = "type", content = "data")]` — matches TypeScript discriminated union pattern.
6. Write corresponding TypeScript types in `src/types/` manually (no tauri-specta). **Add a serde round-trip test** that serializes each cross-boundary type in Rust and asserts the JSON shape matches the TS type (field names, tag values).
7. Set up Biome, lefthook, shadcn/ui, Tailwind CSS v4 in this milestone.
8. `edition = "2024"` for both crates.

**Identity Decisions (must be locked in M1):**

The PRD (§10) and research-strategy (§2.1) conflict on `MaterializedKey` format. Resolution:

- **POC format:** `{language}:{entity_kind}:{relative_path}` — NO `workspace_root` in the key. Relative path is from workspace root, normalized (forward slashes, no trailing slash, no `./` prefix). This is portable and privacy-safe from the start, avoiding the rework research-strategy warns about.
- **Workspace root** is stored as session metadata on `AnalysisHost`, not baked into keys.
- **Edge identity:** `EdgeId` = hash of `(source_key, target_key, edge_kind, edge_category)`. This supports parallel edges (value + type-only between same nodes) and per-edge overlay suppression. `EdgeIndex` from petgraph is internal-only; external references use `EdgeId`.
- **LineageKey:** Data model field exists (`Option<LineageKey>` on `NodeData`), always `None` in POC. UUID generation and persistence activate in MVP with SQLite. No stub implementation needed — just the type and the field.
- **Path normalization policy:** All paths entering the system are normalized through a single function: `camino::Utf8Path`, forward slashes, no trailing slash, no `./` prefix, symlinks resolved. This function is defined in M1 and used everywhere. Case-sensitivity: preserve original case, compare case-sensitively (Linux behavior). macOS case-insensitivity handled in MVP with a platform-aware comparator.

---

#### M2: Workspace Discovery + Compatibility Report
**Goal:** Point at a directory, discover workspace structure, generate a compatibility report. No graph rendering yet — just core logic + display.

- [x] M2: Workspace Discovery + Compatibility — discover Cargo/JS workspaces, generate compatibility report, display in UI
  - [x] Step 1 — Add deps + workspace discovery types + cargo/JS discovery implementation → verify: `cargo test -p codeatlas-core`
  - [x] Step 2 — Config loading from filesystem + profile detection + detector compatibility + AnalysisHost/Analysis → verify: `cargo test --workspace`
  - [x] Step 3 — Tauri commands (open_directory, discover_workspace) with spawn_blocking → verify: `cargo check -p codeatlas-tauri`
  - [x] Step 4 — Frontend UI (TS types, Open Directory, compatibility report, profile badge) → verify: `pnpm typecheck && pnpm biome check`
  - [x] Step 5 — Full verification + test fixtures → verify: `cargo clippy --workspace -- -D warnings && cargo test --workspace && pnpm typecheck && pnpm biome check`
  Commit: "feat: workspace discovery, compatibility report, and profile detection (M2)"

**What to verify:**
- `cargo_metadata` correctly discovers this project's Cargo workspace
- JS/TS workspace detection finds `pnpm-workspace.yaml` or `package.json` workspaces
- Compatibility report accurately classifies Supported/Partial/Unsupported per language
- Profile is auto-detected (resolution mode, features, package manager)
- `.codeatlas.yaml` parser validates and loads config (even if mostly non-functional)
- `open_directory` Tauri command works with native file dialog
- Compatibility report renders in the UI
- Profile badge shows in the UI

**Critical implementation notes:**
1. `cargo_metadata::MetadataCommand::new()` — the selected directory may not *be* a `Cargo.toml`. Workspace discovery must first locate the manifest: walk upward from the selected directory looking for `Cargo.toml` with `[workspace]`, then call `.manifest_path(found_cargo_toml).exec()`. First call may be slow (~2-10s). Cache result.
2. JS workspace detection order: check for `pnpm-workspace.yaml` first (pnpm doesn't use `package.json` `workspaces`), then fall back to `package.json` `workspaces`.
3. Each detector contributes its `CompatibilityAssessment` to the report — this means the `Detector` trait `compatibility()` method must work before `detect()`.
4. **Compatibility report lifecycle:** The M2 report is a **structural/manifest-level** assessment (workspace shape, package manager, detected configs, presence of build.rs/proc-macro crates, tsconfig resolution mode). It is explicitly **provisional** — labeled as such in the UI. When M4/M5 detectors run, they **enrich** the report with source-level findings (actual cfg gates encountered, dynamic imports found, inline type imports). The report transitions from provisional → final at scan completion. This is not a contradiction — it's the designed progressive behavior per PRD §6.1 ("runs during workspace discovery, before or alongside the first scan").
5. The Tauri command `open_directory` uses `tauri-plugin-dialog` to get a directory path, then invokes workspace discovery.
6. Use `tokio::spawn_blocking` for the `cargo_metadata` call (it's synchronous and may take seconds).
7. The `.codeatlas.yaml` parser should be permissive — validate schema version, parse all sections. **`ignore` paths must be functional from M2 forward** so that the file walker in M4/M5 respects them from the first scan (PRD F25). `entrypoints` are parsed and stored for display. Other sections (dependencies, packages, frameworks, declarations) are parsed and acknowledged as "recognized, not yet functional in POC."
8. **Profile model:** The POC profile is **workspace-level**, not per-package. A mixed ESM/CJS monorepo gets a single auto-detected profile based on the root tsconfig. If multiple resolution modes are detected, the compatibility report flags this as "mixed resolution modes detected — per-package profiles available in MVP." This is an explicit POC limitation per PRD §8.

---

#### M3: Static Graph Rendering
**Goal:** React Flow renders compound nodes with ELK layout. Hardcoded fixture data — no real scanning yet. Expand/collapse works. Graph adaptation defaults work.

- [x] M3: Static Graph Rendering — React Flow + ELK compound layout with fixture data, expand/collapse, adaptive defaults
  - [x] Step 1 — Install frontend deps (@xyflow/react, elkjs, zustand, class-variance-authority) + vitest + testing-library → verify: `pnpm install && pnpm typecheck`
  - [x] Step 2 — AnalysisHost as Tauri managed state (Mutex<AnalysisHost> in tauri::State) → verify: `cargo clippy --workspace -- -D warnings && cargo test --workspace`
  - [x] Step 3 — Graph projection pipeline (src/store/graph-projection.ts), Zustand store (src/store/graph-store.ts), fixture data (src/fixtures/demo-graph.ts), + unit tests → verify: `pnpm test && pnpm typecheck`
  - [x] Step 4 — ELK Web Worker (src/components/graph/layout/elk.worker.ts), layout orchestration (elk-layout.ts), + unit tests → verify: `pnpm test && pnpm typecheck`
  - [x] Step 5 — Custom nodes (PackageNode, ModuleNode, FileNode), custom edge (DependencyEdge), GraphCanvas, App integration with "Load Demo Graph" button → verify: `pnpm typecheck && pnpm lint`
  - [x] Step 6 — Final verification: all checks pass → verify: `cargo clippy --workspace -- -D warnings && cargo test --workspace && pnpm typecheck && pnpm lint && pnpm test`
  Commit: "feat: static graph rendering with React Flow + ELK compound layout (M3)"

**What to verify:**
- Hardcoded graph fixture renders as nested packages → modules → files
- ELK `layered` + `INCLUDE_CHILDREN` produces correct compound layout
- Expand/collapse toggles work: children get `hidden: true/false`, edges update
- After toggle, ELK re-layout runs and positions update smoothly
- MiniMap shows viewport position
- Graph adaptation: small graphs start expanded, large start collapsed
- Custom nodes render with correct styling per node kind
- Edge styling: color + dash pattern per EdgeCategory (Okabe-Ito palette)
- 60fps pan/zoom with <200 visible nodes
- `onlyRenderVisibleElements` enabled

**Critical implementation notes:**
1. **ELK must run in a Web Worker from the start.** PRD NF3 requires "ELK layout computation — <500ms in Web Worker" and NF10 requires "UI thread never blocked by layout or parsing — Always responsive." Research-implementation §3.2 says "never run on main thread." Use `elkjs` with its built-in worker mode:
   ```typescript
   // elk.worker.ts — the worker file
   // Uses Vite's worker import: import ElkWorker from './elk.worker?worker'
   // OR: new ELK({ workerUrl: '/elk-worker.min.js' })
   ```
   The API is identical (returns Promise) whether using bundled or worker mode, so this does not add complexity — it just requires the worker file setup in Vite. Copy `elkjs/lib/elk-worker.min.js` to `public/` or use Vite's `?worker` import pattern.
2. **ELK ↔ React Flow conversion is the hardest part of this milestone.** ELK expects hierarchical `children` arrays. React Flow uses a flat array with `parentId`. You must:
   - Convert React Flow nodes → ELK hierarchical format (group by parent)
   - Run `elk.layout(graph)` with `INCLUDE_CHILDREN`
   - Flatten ELK output back to React Flow format (each node gets `position: { x, y }` relative to parent)
   - Set parent node `style: { width, height }` from ELK's computed dimensions
3. **Parent nodes MUST appear before children** in the React Flow nodes array. Sort after flattening.
4. **Node sizing:** ELK needs `width` and `height` before layout. Use estimated sizes (e.g., 150x50 for files, 200x40 for collapsed packages). After first render, you could measure DOM and re-layout, but for POC, estimates are fine.
5. **Zustand store pattern:** Use `useFlowStore` with `applyNodeChanges`/`applyEdgeChanges` from `@xyflow/react`. Define `nodeTypes` **outside** the component. Wrap custom nodes with `memo()`.
6. **Graph projection model — see §12 "Graph Projection Model" below.** This milestone implements the basic projection (collapse/expand + edge bundling). Suppression and category filtering are added in M6/M7.
7. **Memoization (no React Compiler yet):** Manually `memo()` all custom node/edge components. Define `nodeTypes`/`edgeTypes` outside components. React Compiler is deferred to M9 — use explicit memoization patterns until then.

**Fixture data structure:**
Create a JSON fixture representing a small multi-package monorepo (~30 nodes) that exercises:
- 3-4 top-level packages with inter-package edges
- Modules within packages
- Files within modules
- Edge categories: value, type_only, dev
- A few unsupported construct markers
- One manual edge, one suppressed edge

---

#### M4: Rust Detector + Streaming
**Goal:** Rust detector parses a real directory. Graph builds in petgraph with edge evidence and categories. Data streams via Channel<T> with progressive rendering.

- [x] M4: Rust Detector + Streaming — real Rust scanning with tree-sitter + cargo_metadata, Channel<T> streaming, cancel support
  - [x] Step 1 — Add dependencies (tree-sitter, tree-sitter-rust, rayon, tokio-util, uuid, ignore)
  - [x] Step 2 — Implement Rust detector (detector/rust.rs) with 3 phases
  - [x] Step 3 — Implement scan pipeline (scan/pipeline.rs)
  - [x] Step 4 — Update AnalysisHost + Tauri commands (start_scan, cancel_scan, ChannelSink)
  - [x] Step 5 — Frontend scan-store, use-scan hook, NodeData→AppNode conversion
  - [x] Step 6 — Update App.tsx with scan UI (Scan/Cancel buttons, progress)
  - [x] Step 7 — Tests (13 detector, 2 pipeline, 8 scan-store, 4 graph-store scan tests)
  - [x] Step 8 — All checks pass (clippy, cargo test 81, pnpm test 43, typecheck, biome)
  Commit: "feat: Rust detector + streaming scan pipeline (M4)"

**What to verify:**
- Rust detector discovers all crates in a Cargo workspace
- `cargo_metadata` provides inter-crate dependency edges with kinds (Normal/Dev/Build)
- tree-sitter parses `.rs` files and extracts `mod`, `use`, `pub use` declarations
- Edge categories correctly set: Normal/Dev/Build from cargo_metadata
- Unsupported constructs detected: `cfg` gates, `build.rs` presence, `proc_macro` crate type
- Data streams via Channel<T> in 3 phases: packages → modules → files
- Frontend receives stream events and progressively renders
- Cancel via `CancellationToken` works
- Graph builds correctly in petgraph `StableGraph`
- MaterializedKey generated for every node

**Critical implementation notes:**
1. **File walking must respect `.codeatlas.yaml` `ignore` paths.** The `ignore` crate handles `.gitignore`; the scan pipeline additionally applies glob patterns from `.codeatlas.yaml` `ignore` section (parsed in M2). This means scanning produces correct results from the first real scan without M6 retrofitting.
2. **tree-sitter Query patterns for Rust:**
   - `(mod_item name: (identifier) @mod_name)` — module declarations
   - `(use_declaration argument: (_) @use_path)` — use statements
   - `(visibility_modifier) @vis` — pub visibility
   - Use `.scm` query files per language, loaded at detector startup
3. **tree-sitter `StreamingIterator`:** Must `use tree_sitter::StreamingIterator` and use `while let Some(m) = matches.next()` — NOT `for m in matches`.
4. **Parallel file parsing with rayon:** `files.par_iter().map(|f| parse_file(f)).collect()`. Bridge to tokio via `spawn_blocking`.
5. **Core/shell streaming boundary:**
   - **In `codeatlas-core`:** The scan pipeline accepts a `ScanSink` trait implementor:
     ```rust
     // In codeatlas-core — domain-level, transport-agnostic
     pub trait ScanSink: Send + Sync {
         fn on_compatibility(&self, report: CompatibilityReport);
         fn on_phase(&self, phase: ScanPhase, nodes: Vec<NodeData>, edges: Vec<EdgeData>);
         fn on_health(&self, health: GraphHealth);
         fn on_progress(&self, scanned: usize, total: usize);
     }
     ```
   - **In `codeatlas-tauri`:** The Tauri command creates a `ChannelSink` that adapts `ScanSink` to `Channel<ScanEvent>`:
     ```rust
     // In codeatlas-tauri — transport-specific envelope
     #[serde(tag = "event", content = "data", rename_all = "camelCase")]
     enum ScanEvent {
         CompatibilityReport(CompatibilityReport),
         Phase { scan_id: String, phase: ScanPhase, nodes: Vec<NodeData>, edges: Vec<EdgeData> },
         Health(GraphHealth),
         Progress { scanned: usize, total: usize },
         Complete { scan_id: String },
         Error { message: String },
     }
     ```
   Frontend: `const channel = new Channel<ScanEvent>(); channel.onmessage = handleEvent; await invoke('start_scan', { path, onEvent: channel });`
6. **Scan lifecycle — see §12 "Scan Lifecycle" below.** Every scan has a `scan_id`. The frontend rejects events from stale scans. Cancel triggers `CancellationToken`. Detectors check `token.is_cancelled()` periodically. The compatibility report enriches progressively (provisional → final).

---

#### M5: TypeScript Detector
**Goal:** TypeScript detector parses a real TS workspace. Import resolution with tsconfig paths. Edge categories captured.

- [x] M5: TypeScript Detector — tree-sitter TS parsing, workspace package discovery, tsconfig paths resolution, value/type-only edge categories
  - [x] Step 1 — Add tree-sitter-typescript dep, new UnsupportedConstructType variants, extend TS fixtures with source files, create ts-unsupported fixture → verify: `cargo check -p codeatlas-core`
  - [x] Step 2 — Write tree-sitter query validation tests (import, import type, inline type, re-exports, dynamic import, require, TSX) → verify: `cargo test -p codeatlas-core -- typescript`
  - [x] Step 3 — Implement TsConfigResolver (paths, baseUrl, workspace packages, relative path probing) + resolution tests → verify: `cargo test -p codeatlas-core -- typescript`
  - [x] Step 4 — Implement TypeScriptDetector::detect() (Phase 1–3: packages, modules, file edges) → verify: `cargo test -p codeatlas-core -- typescript`
  - [x] Step 5 — Register TS detector in pipeline, update enrichment, mixed-repo integration test, frontend type updates → verify: `cargo clippy --workspace -- -D warnings && cargo test --workspace && pnpm typecheck && pnpm lint && pnpm test`
  Commit: "feat: TypeScript detector + streaming scan pipeline (M5)"

**What to verify:**
- Workspace packages discovered from `pnpm-workspace.yaml` or `package.json` `workspaces`
- tree-sitter parses `.ts`/`.tsx` files and extracts imports/exports
- `import type { Foo }` classified as `type_only` edge category
- `import { Foo }` classified as `value` edge category
- Inline type imports (`import { type Foo }`) detected via `type` child on `import_specifier`
- Basic tsconfig `paths`/`baseUrl` resolution works
- Dynamic imports detected and badged as unsupported construct
- `package.json` `exports` conditions detected but badged as "not evaluated in POC"
- Inter-package edges resolved from bare specifiers to workspace packages

**Critical implementation notes:**
1. **tree-sitter Query patterns for TypeScript:**
   - Import type detection: The `type` keyword in `import_statement` is an **anonymous node**. You cannot use `child_by_field_name()`. Use `(import_statement "type" @type_keyword) @import` or iterate children checking `kind() == "type"`.
   - Value imports: `(import_statement source: (string) @source)` without `type` child
   - Named imports: `(import_specifier name: (identifier) @name)` — each specifier may have its own `type` child for inline type imports
   - Re-exports: `(export_statement source: (string) @source)` — distinction between `export type` and `export`
2. **tsconfig resolution (POC scope):**
   - Read `tsconfig.json` → `compilerOptions.paths` and `compilerOptions.baseUrl`
   - Map path aliases (e.g., `@/*` → `./src/*`) to filesystem paths
   - This is NOT full resolution — `exports`/`imports` conditions, project references, and PnP are badged as unsupported
3. **Workspace package resolution:**
   - Read `pnpm-workspace.yaml` `packages` globs → resolve to directories
   - Each directory with `package.json` is a workspace package
   - Bare import specifiers matching workspace package names resolve to that package
4. **Use `tree_sitter_typescript::LANGUAGE_TYPESCRIPT` for `.ts` files and `LANGUAGE_TSX` for `.tsx` files.** They are different parsers.

---

#### M6: Graph Health + Provenance + Config UI
**Goal:** Surface the health/provenance/overlay data that M1–M5 already produce. The data model and enforcement were built in M1 (types, overlay immutability), config parsing in M2, and edge evidence in M4/M5. This milestone adds the **UI surfaces** and makes overlay operations functional end-to-end.

- [x] M6: Graph Health + Provenance + Config UI — health display, edge evidence, unsupported badges, overlay operations
  - [x] Step 1 — Backend: Apply overlay from config, stream details (unsupported constructs, parse failures, overlay edges, suppressed IDs) to frontend → verify: `cargo test --workspace && cargo clippy --workspace -- -D warnings`
  - [x] Step 2 — Frontend: Install shadcn/ui components, extend stores + types for new events → verify: `pnpm typecheck`
  - [x] Step 3 — Frontend: Build HealthIndicator, EdgeProvenance popover, CompatibilityPanel, ProfileBadge components → verify: `pnpm typecheck && pnpm biome check`
  - [x] Step 4 — Frontend: Wire panels into App.tsx layout + GraphCanvas edge click → verify: `pnpm typecheck && pnpm biome check`
  - [x] Step 5 — Tests: Vitest (health, provenance, overlay projection, scan store) + Rust (overlay application, immutability) → verify: `cargo test --workspace && pnpm test && cargo clippy --workspace -- -D warnings && pnpm biome check`
  Commit: "feat: Graph Health + Provenance + Config UI (M6)"

**What to verify:**
- Health indicator shows: total nodes, resolved edges, unresolved imports, parse failures, unsupported constructs
- Unresolved imports listed on click, with **reason** for each (path alias outside config, missing package, dynamic import, etc.)
- Unsupported constructs badged per type with explanations
- Edge hover/click shows: evidence class, category, source location, resolution method
- `.codeatlas.yaml` `entrypoints` displayed in profile panel
- Overlay operations end-to-end: `add` in config creates manual edges visible in graph with "manual" badge, `suppress` hides discovered edges in default view (visible with "show suppressed" toggle, dimmed/dashed with reason)
- Discovered graph immutability enforced — config cannot delete edges from discovered layer
- Compatibility report accessible from profile panel (final report, not provisional)
- Profile badge shows detected packages, resolution mode, unsupported construct summary

---

#### M7: Interactive Features
**Goal:** Node detail panel with category-aware edge display, search, edge filtering, rescan.

- [x] M7: Interactive Features — detail panel, Cmd+K search, edge category filtering, manual rescan
  - [x] Step 1 — Install shadcn/ui components (command, dialog, input, tabs) → verify: `pnpm typecheck`
  - [x] Step 2 — Detail panel: sliding right panel with Overview/Dependencies/Health tabs, node selection → verify: `pnpm typecheck && pnpm biome check`
  - [x] Step 3 — Command palette: Cmd+K fuzzy search over all nodes, expand-to-reveal, center viewport → verify: `pnpm typecheck && pnpm biome check`
  - [x] Step 4 — Edge category filter bar + move suppressed toggle into unified toolbar → verify: `pnpm typecheck && pnpm biome check`
  - [x] Step 5 — Manual rescan with viewport preservation + keyboard shortcuts → verify: `pnpm typecheck && pnpm biome check`
  - [x] Step 6 — Tests: DetailPanel, CommandPalette, category filtering, rescan, keyboard shortcuts → verify: `pnpm test && cargo test --workspace`
  - [x] Step 7 — Final verification → verify: `cargo clippy --workspace -- -D warnings && cargo test --workspace && pnpm typecheck && pnpm biome check && pnpm test`
  Commit: "feat: interactive features — detail panel, search, filtering, rescan (M7)"

**What to verify:**
- Click node opens detail panel with tabs: Overview, Dependencies, Exports, Health
- Dependencies tab shows incoming/outgoing edges with evidence class and category
- Edge list is filterable by category (value/type-only/dev/build)
- Cmd+K opens command palette with fuzzy search on node labels
- Selecting search result: centers viewport, expands parent packages, highlights node
- "Rescan" button triggers fresh scan preserving viewport position
- Dark theme applied
- Keyboard shortcuts: Tab through nodes, Enter expand/collapse, Escape deselect, Cmd+0 fit-to-view

---

#### M8: Golden Corpus Validation
**Goal:** All POC golden corpus repos produce correct graphs. This is the headline acceptance metric (PRD §22). Correctness is proven before polish.

- [x] M8: Golden Corpus Validation — corpus correctness, compatibility accuracy, edge category verification
  - [x] Step 1 — Backend: Add UnresolvedImport types + update DetectorReport/ScanResults/ScanSink → verify: `cargo check -p codeatlas-core`
  - [x] Step 2 — Backend: Update Rust + TypeScript detectors to track unresolved imports → verify: `cargo test -p codeatlas-core`
  - [x] Step 3 — Backend: Update pipeline to merge unresolved imports + compute real GraphHealth count → verify: `cargo test --workspace`
  - [x] Step 4 — Tauri shell: Update ScanEvent::Details + ChannelSink for unresolved imports → verify: `cargo check --workspace`
  - [x] Step 5 — Frontend: Update types, stores, and UI for unresolved imports → verify: `pnpm typecheck && pnpm test`
  - [x] Step 6 — Golden corpus: assertion-based integration tests for all 5 fixtures → verify: `cargo test --workspace`
  - [x] Step 7 — Cross-boundary contract tests for all Rust↔TypeScript types → verify: `cargo test --workspace`
  - [x] Step 8 — Add insta + snapshot tests for golden corpus fixtures → verify: `cargo test --workspace`
  - [x] Step 9 — Final verification: all checks pass → verify: `cargo clippy --workspace -- -D warnings && cargo test --workspace && pnpm typecheck && pnpm biome check && pnpm test`
  Commit: "feat: golden corpus validation — unresolved imports, contract tests, 5-fixture corpus (M8)"

**What to verify (PRD §22 — "at least 2 reference repos per supported language"):**
- **This project's own repo** scans correctly: all workspace packages discovered, inter-package deps correct, compatibility report accurate
- **Rust fixture 1** (multi-crate workspace with features/targets): correct crate graph, dependency kinds (normal/dev/build), mod/use hierarchy
- **Rust fixture 2** (crate with build.rs, proc-macro, cfg gates): unsupported constructs detected and badged, compatibility report flags them as Partial
- **TS fixture 1** (pnpm workspace with tsconfig paths): correct package graph, import resolution, value/type-only classification
- **TS fixture 2** (workspace with dynamic imports, re-exports, exports conditions): unsupported constructs detected, compatibility report flags them
- Edge categories correct across all fixtures (verified via snapshot tests)
- Unresolved imports accurately reported (no false completeness claims)
- `codeatlas-core` exercisable from test harness without Tauri
- >80% test coverage on non-UI Rust code
- >80% test coverage on frontend pure logic

---

#### M9: Performance + Demo + Polish
**Goal:** Performance budgets met. Demo fixture works. Dark theme. Zero network calls verified.

- [ ] M9: Performance + Demo + Polish — perf tuning, demo fixture, dark theme, final QA

**What to verify:**
- First meaningful frame <2s for ≤2,000 files (test with this repo)
- 60fps pan/zoom with <200 visible nodes
- Memory <200MB for 500-node graph
- ELK layout <500ms in Web Worker for 200 nodes
- Demo graph fixture loads from built-in JSON and exercises all interaction patterns
- Dark theme applied
- Zero network calls (audit with browser devtools network tab)
- All tests green across `cargo test`, `pnpm test`, `cargo clippy`, `pnpm typecheck`

---

## 7. Testing Strategy

### Rust Tests (codeatlas-core)

| Test Type | What | Where | Milestone |
|-----------|------|-------|-----------|
| Unit | MaterializedKey generation, hashing, equality | `graph/identity.rs` `#[cfg(test)]` | M1 |
| Unit | NodeData/EdgeData serde round-trip | `graph/types.rs` `#[cfg(test)]` | M1 |
| Unit | ArchGraph invariants: add/remove nodes, overlay immutability | `graph/arch_graph.rs` `#[cfg(test)]` | M1 |
| Unit | RepoConfig parsing, validation, version check | `config/schema.rs` `#[cfg(test)]` | M1 |
| Unit | GraphProfile detection from workspace info | `profile/mod.rs` `#[cfg(test)]` | M2 |
| Unit | CompatibilityReport generation from detector assessments | `health/compatibility.rs` `#[cfg(test)]` | M2 |
| Integration | Workspace discovery on fixture repos | `tests/` directory | M2 |
| Unit | tree-sitter Rust query patterns (mod, use, pub use) | `detector/rust.rs` `#[cfg(test)]` | M4 |
| Unit | tree-sitter TS query patterns (imports, type imports, exports) | `detector/typescript.rs` `#[cfg(test)]` | M5 |
| Integration | Full scan of fixture repos → golden corpus verification | `tests/` directory | M4, M5, M8 |
| Property | Graph invariants: no duplicate keys, overlay immutability, parent consistency | `proptest` | M4 |
| Snapshot | Scan output against known-good results | `insta` | M8 |

### Frontend Tests (Vitest + Testing Library)

| Test Type | What | Where | Milestone |
|-----------|------|-------|-----------|
| Unit | Zustand store actions: node toggle, edge filtering, state transitions | `store/*.test.ts` | M3 |
| Unit | ELK conversion: RF nodes ↔ ELK hierarchical format | `layout/elk-layout.test.ts` | M3 |
| Unit | Edge category filtering logic | `store/graph-store.test.ts` | M7 |
| Unit | Search/filter matching logic | `store/ui-store.test.ts` | M7 |
| Component | GraphCanvas renders fixture data | `graph/GraphCanvas.test.tsx` | M3 |
| Component | DetailPanel shows correct data for selected node | `panels/DetailPanel.test.tsx` | M7 |
| IPC Mock | Tauri invoke/channel mocking with `@tauri-apps/api/mocks` | test setup | M4 |

### Cross-Boundary Contract Tests

| Test Type | What | Where | Milestone |
|-----------|------|-------|-----------|
| Contract | Rust serialize → JSON shape matches TS type definitions | `tests/contract/` | M1 |
| Contract | ScanEvent variants: Rust serialize → TS discriminated union parse | `tests/contract/` | M4 |

These tests serialize each cross-boundary type in Rust, assert the JSON field names/structure, and verify against the TypeScript type definitions. This catches serde attribute mismatches (missing `rename_all`, wrong tag format) that would otherwise surface as runtime errors. **Critical because we intentionally skipped tauri-specta.**

### Frontend Projection + Lifecycle Tests

| Test Type | What | Where | Milestone |
|-----------|------|-------|-----------|
| Unit | `project()` pure function: collapse + bundle + filter + suppress combinations | `store/graph-store.test.ts` | M3, M6, M7 |
| Unit | Bundled edge → underlying edge ID mapping for detail panel | `store/graph-store.test.ts` | M3 |
| Unit | Scan lifecycle: stale event rejection, cancel + partial results, rescan clears state | `store/scan-store.test.ts` | M4 |
| Unit | Phase delivery + debounced re-layout scheduling | `hooks/use-layout.test.ts` | M4 |

### Testing Conventions

- **Rust:** Co-located `#[cfg(test)] mod tests` blocks. Integration tests in `tests/` directory.
- **Frontend:** Co-located `foo.test.ts` next to `foo.ts`. Mock Tauri IPC with `mockIPC()` / `clearMocks()`.
- **Fixtures:** Minimal representative repos in `tests/fixtures/`. Each tests specific features. **Minimum 2 per language per PRD §22.**
- **Coverage:** `cargo-llvm-cov` for Rust (macOS compatible), Vitest `--coverage` for frontend.

---

## 8. Migration & Rollback

Not applicable for POC — no existing data, users, or APIs to migrate. The POC is greenfield.

**Forward-looking:** The identity scheme (MaterializedKey) and config schema (`.codeatlas.yaml` version: 1) are designed for future versioning. If schema changes are needed before MVP, bump the version number and add migration logic.

---

## 9. Manual Setup Tasks

| Task | Description | Required Before |
|------|-------------|-----------------|
| **Install Rust toolchain** | `rustup` with stable channel (≥1.85 for edition 2024) | M1 |
| **Install Node.js 22+** | Required for Vite/React dev server | M1 |
| **Install pnpm** | `corepack enable && corepack prepare pnpm@latest --activate` | M1 |
| **Install Tauri system deps** | macOS: Xcode Command Line Tools. Linux: `webkit2gtk-4.1` + deps. | M1 |
| **Create test fixture repos** | Minimal Rust workspace + TS monorepo in `tests/fixtures/` | M2 |
| **ELK.js license review** | ELK.js is EPL-2.0, not MIT. Review compatibility with planned distribution. | M3 |
| **Apple Developer ID** (MVP) | $99/yr for macOS code signing + notarization | MVP |

---

## 10. Risks

| Risk | Likelihood | Impact | Mitigation / Detection |
|------|-----------|--------|----------------------|
| **tauri-specta incompatible with Tauri 2.10.x** | High | No auto-generated TS types | **Already mitigated:** Plan uses manual TS types. Revisit when tauri-specta reaches stable. |
| **ELK + React Flow compound layout integration** | High | Layout breaks, cross-hierarchy edge routing issues | Prove in M3 with fixture data before any real scanning. The `flattenElkGraph` conversion is custom code — test thoroughly. Budget extra time for this milestone. |
| **React Compiler conflicts with React Flow** | Low | Performance regressions or rendering bugs | **Mitigated:** Deferred to M9. Manual `memo()` used until rendering layer is proven. If M9 integration shows issues, exclude graph components via Compiler `sources` config. |
| **tree-sitter anonymous node detection (TypeScript `type` keyword)** | Medium | `import type` misclassified as `import` | M5 unit tests must include explicit test cases for `import type`, inline `import { type Foo }`, and `export type`. |
| **tree-sitter grammar lag for latest TS syntax** | Low | Parse errors on cutting-edge syntax | tree-sitter error recovery produces partial trees. Badge parse failures, don't crash. |
| **cargo_metadata latency on first call** | Medium | >2s first meaningful frame | Cache metadata. Run in `spawn_blocking`. Stream compatibility report first while metadata resolves. |
| **Channel<T> memory leak (tauri-apps/tauri#13133)** | Low | Memory growth on repeated scans | Monitor memory in M4. If severe, clean up channels between scans. |
| **Progressive rendering UX jarring** | Medium | Users confused by partially rendered graph | Package topology renders first as stable compound nodes. File-level details stream in without relayout of package level. |
| **Overlay model adds complexity to query layer** | Low | Slower development | Queries default to merged view. Discovered-only and overlay-only views are additive. Start with merged-only. |
| **Graph adaptation thresholds (120/250 nodes) wrong** | Medium | Default view too dense or too sparse | Make thresholds configurable internally from M3. Test against real repos in M8. |
| **serde tagged enum format mismatch TS types** | Low | Runtime deserialization errors | Write round-trip tests: Rust serialize → TypeScript deserialize. Catch early in M4. |
| **Biome Tailwind class sorting immature** | Low | Inconsistent class ordering | Accept nursery rule limitations or skip class sorting for POC. Not critical path. |
| **Test fixture repos don't cover edge cases** | Medium | False confidence in golden corpus | Design fixtures deliberately: include unsupported constructs, mixed dep kinds, parse-error files. |
| **Path canonicalization issues (macOS case-insensitive)** | Medium | Duplicate nodes for same file | Use `camino::Utf8PathBuf`, normalize early in pipeline. Test with mixed-case imports. Path normalization policy locked in M1. |
| **Scan interleaving / stale UI state** | Medium | Corrupted graph from old scan events | **Mitigated:** Scan lifecycle with `scan_id` and stale-event rejection added in v2. Frontend drops events not matching active scan. |
| **Compatibility report over-promises then revises downward** | Medium | Trust erosion on first use | **Mitigated:** Provisional → final lifecycle. UI shows "provisional" badge until scan completes. Initial structural report explicitly says "source-level analysis pending." |
| **Projection layer complexity (bundle edges, suppression, filtering)** | High | Hardest frontend code, most likely rework area | **Mitigated:** Projection defined as pure function in §12. M3 implements basic collapse/expand. Suppression and filtering layer in M6/M7. Each step is independently testable. |
| **Custom Tauri directory layout fragility** | Low | Build system breaks | M1 scaffold is a spike with go/no-go checkpoint. Fallback: keep `src-tauri/` naming with path dependency to `codeatlas-core`. |

---

## 11. Open Questions (Require Human Input)

**High priority — affect architecture:**

1. **ELK EPL-2.0 license:** ELK.js uses the Eclipse Public License 2.0. This is a weak copyleft license — modifications to ELK itself (not your code) may need disclosure. Is this acceptable for the planned MIT distribution model? If not, the only alternative is a custom layout engine or a significantly less capable library. This must be decided before M3. Yes this is fine. 

2. **No-code-execution guarantee:** Research-strategy §2.1 recommends adding an explicit constraint: "No code execution during analysis (no build.rs, proc macros, package scripts, bundlers)." Should this be added to the PRD as a formal constraint and documented in the compatibility report? This affects how we communicate limits to users.

3. **Golden corpus: synthetic vs real repos:** PRD §22 requires 2+ reference repos per language. Should these be purpose-built synthetic fixtures (fast, deterministic, controlled edge cases) or real open-source repos (realistic, but slower, flakier, harder to maintain)? Recommendation: synthetic fixtures for CI, one real repo (this project) for smoke testing.

**Medium priority — affect implementation approach:**

4. **Project naming:** The PRD calls this "Code Atlas" but the repo is `tauri-poc-zoom-thing`. Should the Tauri app identifier be `com.codeatlas.app`? Should crates be named `codeatlas-core`/`codeatlas-tauri`? Or keep the POC naming informal?

5. **Biome vs ESLint:** Research recommends Biome 2.x (35x faster, single tool). The `.claude/rules/stack.md` says "ESLint + Prettier." This plan assumes Biome. Confirm?

6. **React Compiler:** ~~Should we enable from M1 or defer?~~ **Decided: defer to M9.** Stable (v1.0) but unproven with React Flow compound nodes. M1-M3 already has enough integration risk (Tauri workspace restructure, ELK Web Worker, compound layout). Add in M9 when the rendering layer is proven and we can measure before/after. Use manual `memo()` on custom nodes until then.

---

## 12. Key Technical Decisions for Implementers

### Serde Conventions (Cross-Boundary Types)
All types that cross the Rust → TypeScript boundary use:
```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]  // For flat structs
// OR
#[serde(tag = "type", content = "data", rename_all = "camelCase")]  // For enums
```
This produces JSON with camelCase keys and TypeScript-compatible discriminated unions.

### AnalysisHost / Analysis Pattern
```rust
/// Mutable handle — accepts changes
pub struct AnalysisHost {
    graph: ArchGraph,
    config: RepoConfig,
    profile: GraphProfile,
    // ...
}

/// Immutable snapshot — safe for concurrent queries
pub struct Analysis {
    // All fields are Arc<T> for cheap cloning
}

impl AnalysisHost {
    pub fn snapshot(&self) -> Analysis { /* clone Arcs */ }
    pub fn apply_scan_results(&mut self, results: ScanResults) { /* merge into graph */ }
}

impl Analysis {
    pub fn nodes(&self) -> &[NodeData] { /* ... */ }
    pub fn edges(&self) -> &[EdgeData] { /* ... */ }
    pub fn health(&self) -> &GraphHealth { /* ... */ }
    pub fn compatibility(&self) -> &CompatibilityReport { /* ... */ }
    // ... query methods
}
```

### Edge Category Color Scheme (Okabe-Ito Palette)
| Category | Color | Hex | Dash Pattern |
|----------|-------|-----|-------------|
| `value` | Blue | `#0072B2` | Solid |
| `type_only` | Sky Blue | `#56B4E9` | Dashed (`5,5`) |
| `dev` | Orange | `#E69F00` | Dotted (`2,2`) |
| `build` | Yellow | `#F0E442` | Solid |
| `normal` | Green | `#009E73` | Solid |
| `manual` | Pink | `#CC79A7` | Double line |
| `suppressed` | Gray | `#999999` | Long dashed (`10,5`) |

### Graph Projection Model

The **discovered graph** (in `codeatlas-core`) and the **visible graph** (in React Flow) are different things. The projection pipeline transforms one into the other. This is the most important frontend contract.

```
Discovered Graph (ArchGraph)
    │
    ├─ overlay merge ──→ Merged Graph (discovered + manual edges, with suppression markers)
    │
    ├─ category filter ──→ Filtered Graph (e.g., hide type-only, hide dev)
    │
    ├─ collapse projection ──→ Projected Graph (hidden children, bundled edges)
    │
    └─ React Flow nodes/edges ──→ Visible Graph (positioned by ELK)
```

**Key rules:**

1. **Bundled edges are computed projections, not first-class edges.** When a package is collapsed, all edges between its children and external nodes are replaced by a single bundled edge between the package node and the external target. Each bundled edge carries a `bundledEdgeIds: string[]` array referencing the underlying discovered edge IDs. This allows `DetailPanel` to show provenance for bundled edges.

2. **Suppressed edges exist in the merged graph but are hidden by default.** A "show suppressed" toggle adds them back to the visible graph with dimmed styling + suppression reason tooltip. Suppression filtering happens in the store's projection step, not in the component.

3. **Category filtering happens before collapse projection.** If the user filters to "runtime only" (excludes type-only and dev), the bundled edge counts reflect only the filtered edges. This means the bundled edge between two packages might show "3 imports" with the dev filter on, vs "7 imports" with all categories.

4. **Visible node/edge IDs:** Projected nodes keep their `MaterializedKey` as `id`. Bundled edges get a synthetic ID: `bundle:{source_key}:{target_key}`. This is deterministic and stable across re-layouts.

5. **Layout runs against the projected graph only.** ELK never sees hidden nodes or suppressed edges. This keeps layout fast and correct for the visible state.

6. **The projection is a pure function:** `project(mergedGraph, expandedIds, categoryFilter, showSuppressed) → { nodes, edges }`. It lives in `src/store/graph-store.ts` and is the single source of truth for what React Flow renders.

### Scan Lifecycle

Every scan operation follows these rules:

1. **Scan ID:** Each scan gets a UUID (`scan_id`). The frontend stores the active `scan_id`. Events arriving with a stale `scan_id` are silently dropped.

2. **Scan states:** `idle` → `scanning` → `complete` | `error` | `cancelled`. Only one scan can be active. Starting a new scan cancels any in-progress scan (via `CancellationToken`).

3. **Progressive data:** Phase events (`Phase { scan_id, phase, nodes, edges }`) are additive. The frontend appends nodes/edges to the current graph, runs projection, and triggers ELK re-layout (debounced at 300ms to avoid layout thrash during rapid phase delivery).

4. **Compatibility report enrichment:** The first `CompatibilityReport` event arrives early (structural findings from M2 workspace discovery). A second `CompatibilityReport` event may arrive after scanning with source-level findings (unsupported constructs discovered during parsing). The frontend replaces the report on each event. The UI shows "provisional" badge until `Complete` arrives.

5. **Cancel semantics:** Cancel stops the scan and keeps partial results. The graph shows whatever phases completed. Health indicators reflect the partial state. The compatibility report may be provisional.

6. **Rescan semantics:** A rescan clears the current graph and starts fresh (same scan lifecycle). Viewport position is preserved across rescan by saving/restoring the React Flow viewport transform before/after the new graph renders.

7. **Partial graph queries:** Search, detail panel, and health indicators operate on whatever data is currently in the store, even during an active scan. This is safe because phase events are additive and the store is always in a consistent state.

### Graph Store Shape (Zustand)
```typescript
interface GraphStore {
  // Source data (from scan events)
  discoveredNodes: AppNode[];
  discoveredEdges: AppEdge[];
  overlayEdges: AppEdge[];        // manual edges from config
  suppressedEdgeIds: Set<string>; // edge IDs suppressed by config

  // View state
  expandedNodeIds: Set<string>;
  categoryFilter: Set<EdgeCategory>;  // which categories to show
  showSuppressed: boolean;

  // Projected data (derived from source + view state)
  projectedNodes: AppNode[];      // result of project()
  projectedEdges: AppEdge[];      // includes bundled edges

  // Scan lifecycle
  activeScanId: string | null;
  scanStatus: 'idle' | 'scanning' | 'complete' | 'error' | 'cancelled';

  // React Flow handlers
  onNodesChange: (changes: NodeChange[]) => void;
  onEdgesChange: (changes: EdgeChange[]) => void;

  // Actions
  toggleExpand: (nodeId: string) => void;
  applyScanPhase: (scanId: string, phase: ScanPhase, nodes: AppNode[], edges: AppEdge[]) => void;
  startScan: (scanId: string) => void;
  completeScan: (scanId: string) => void;
  setCategoryFilter: (categories: Set<EdgeCategory>) => void;
  toggleSuppressed: () => void;
}
```

### Directory Discovery Priority
1. Check for `Cargo.toml` with `[workspace]` → Cargo workspace
2. Check for `pnpm-workspace.yaml` → pnpm workspace
3. Check for `package.json` with `workspaces` → npm/yarn workspace
4. If both Cargo and JS workspace found → mixed monorepo (both detectors apply)

---

## 13. ADR Candidates

The following decisions should be recorded in `docs/decisions.md` as they are made during implementation:

1. **ADR-001:** Skip tauri-specta, use manual TypeScript types — RC status, no Channel<T> support, compatibility concerns with Tauri 2.10.x
2. **ADR-002:** Biome 2.x over ESLint + Prettier — performance, single tool, built-in formatting (pending open question #5)
3. **ADR-003:** `edition = "2024"` for all crates — stable since Rust 1.85, async fn in traits, let chains
4. **ADR-004:** MaterializedKey format: `{language}:{entity_kind}:{relative_path}` — no workspace_root, portable from the start, avoids MVP rework
5. **ADR-005:** ScanEvent envelope in codeatlas-tauri, not codeatlas-core — core exports domain types + ScanSink trait, shell adapts to transport
6. **ADR-006:** ELK in Web Worker from M3 — PRD NF3/NF10 require it, API is identical to bundled mode, no added complexity
7. **ADR-007:** Graph store in Zustand with explicit projection pipeline — discovered → filtered → projected → React Flow
8. **ADR-008:** `motion` package over `framer-motion` — rebranded, same API, actively maintained
