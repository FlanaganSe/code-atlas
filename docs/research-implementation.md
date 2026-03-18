# Code Atlas — Implementation Research

**Date:** 2026-03-18
**Status:** Consolidated research for implementation planning
**Scope:** Libraries, APIs, architecture patterns, code patterns, tooling decisions

---

## 1. Rust Core Libraries

### 1.1 tree-sitter (POC Parser)

- **Crate:** `tree-sitter` (Rust bindings in main repo under `lib/binding_rust/`)
- **Grammars:** `tree-sitter-typescript` (separate TypeScript + TSX parsers), `tree-sitter-rust`
- **API surface:**
  - `Parser` → `parser.set_language(language)` → `parser.parse(source, old_tree)` → `Tree`
  - `Tree.root_node()` → `Node` with `kind()`, `child()`, `children()`, `child_by_field_name()`, `utf8_text()`, `start_position()`, `end_position()`
  - `Query` + `QueryCursor` for S-expression pattern matching against AST
  - `Language` loaded from grammar crates: `tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()`
- **Incremental parsing:** Pass previous `Tree` to `parser.parse()`, call `old_tree.edit()` with `InputEdit`. Reuse unchanged subtrees. Sub-millisecond update for small edits.
- **Distinguishing `import type` vs `import`:**
  - `import_statement` node contains optional `type` keyword child (anonymous node)
  - Inline type imports: `import_specifier` has `type` child
  - Use `child()` iteration (not `child_by_field_name()` since it's anonymous)
  - Query pattern: `(import_statement "type" @type_keyword) @import`
- **Key TS node types:** `import_statement`, `import_clause`, `named_imports`, `import_specifier`, `namespace_import`, `export_statement`, `export_clause`, `lexical_declaration` with `export`
- **Key Rust node types:** `mod_item`, `use_declaration`, `visibility_modifier`, `use_wildcard`, `use_list`, `use_as_clause`, `scoped_use_list`, `extern_crate_declaration`, `source_file`
- **Performance:** 2-3x slower than rustc's hand-written parser for initial parse. 2,000-file repo: 1-5 seconds total (parallelizable).
- **Gotchas:**
  - `type` keyword is anonymous — easy to miss with only named children iteration
  - Grammars may lag latest syntax
  - Error recovery produces partial trees with `ERROR`/`MISSING` nodes — handle gracefully
  - Queries cannot express cross-file relationships
- **Recommendations:**
  - Use `Query` patterns over manual AST walking (faster, more maintainable)
  - Separate `.scm` query files per language detector
  - Pin grammar crate versions

### 1.2 petgraph (StableGraph)

- **Crate:** `petgraph` 0.7.x, 2.1M+ downloads, actively maintained
- **StableGraph:** indices remain valid after node/edge removal — critical for incremental updates
- **Type:** `StableGraph<N, E, Ty, Ix>` (N=node weight, E=edge weight, Ty=Directed, Ix=u32 default)
- **Key methods:** `add_node`, `add_edge`, `remove_node`, `remove_edge`, `node_weight`, `edge_weight`, `neighbors`, `edges`, `find_edge`, `node_count`, `edge_count`
- **Parallel edges allowed** (important for multiple edge types between same nodes)
- **Algorithms:** `tarjan_scc()`, `kosaraju_scc()`, `toposort()`, `dijkstra()`, `astar()`, `has_path_connecting()`, `is_cyclic_directed()`, `Dfs`, `Bfs`, `dominators()`, `page_rank()`
- **Hierarchy modeling:** No built-in compound nodes. Use `parent: Option<NodeIndex>` field on `NodeData` + `EdgeKind::Contains` edges (recommended over separate tree structure — simpler, uniform serialization/querying)
- **Gotchas:**
  - Not all algorithms work with `StableGraph` — test each one needed
  - More memory than `Graph` due to index gaps (not a concern at <50K nodes)
  - No built-in serialization — must implement custom serde
- **Recommendations:**
  - Use `StableGraph<NodeData, EdgeData, Directed, u32>`
  - Wrap in domain `ArchGraph` struct enforcing invariants
  - Use `tarjan_scc()` for cycle detection, `Bfs`/`Dfs` for transitive queries

### 1.3 cargo_metadata

- **Crate:** `cargo_metadata`, wraps `cargo metadata --format-version 1`
- **Key structs:** `Metadata` (packages, workspace_members, workspace_default_members, resolve, workspace_root, target_directory), `Package` (name, version, id, dependencies, targets, features, manifest_path, edition), `Dependency` (name, req, kind: Normal/Dev/Build, optional, features, target), `Resolve` (nodes with deps and dep_kinds), `Target` (name, kind: lib/bin/test/bench/example, src_path)
- **Usage:**
  ```rust
  cargo_metadata::MetadataCommand::new()
      .manifest_path("path/to/Cargo.toml")
      .exec()?;
  ```
  Supports `--features`, `--no-default-features`, `--filter-platform`
- **Gotchas:**
  - First call may be slow (~2-10s) if deps need resolution. Warm cache is fast.
  - `resolve` is `None` if `--no-deps` passed
  - Use `dep_kinds` from resolve nodes (not Package dependencies) for accurate edge categorization
  - Build script outputs, proc-macro expansion, and conditional compilation NOT reflected
- **Recommendations:** Cache output. Re-invoke only on Cargo.toml/Cargo.lock changes.

### 1.4 oxc_parser + oxc_resolver (MVP)

- **Project:** Oxc (Oxidation Compiler), 20K+ GitHub stars, very active
- **oxc_parser:** Fast, spec-compliant JS/TS parser. 100% Test262, 99% Babel/TS tests. Uses arena allocator for zero-copy AST. API: `Parser::new(allocator, source, source_type) -> ParserReturn`
- **oxc_resolver:** Production-ready module resolver (used by Nova, swc-node, knip):
  - ESM + CJS resolution per Node.js spec
  - Full tsconfig.json: `paths`, `baseUrl`, `extends`, project references, `${configDir}` substitution
  - `package.json` `exports`/`imports` with condition name resolution (key order matters — first match wins)
  - Yarn PnP support (behind feature flag)
  - Extension alias (`.js` → `.ts`), 40+ options
  - SIMD-accelerated JSON parsing, lock-free concurrent caching
  - API: `Resolver::new(options).sync(directory, specifier)`
- **Important:** oxc_resolver is in a SEPARATE repo (`oxc-project/oxc-resolver`), not in main oxc repo
- **Gotchas:**
  - Arena allocator: AST nodes are borrowed — extract data before allocator dropped
  - Pre-1.0 API, breaking changes possible
- **Recommendations:** tree-sitter for POC, oxc for MVP. Pin crate versions carefully.

### 1.5 File System Libraries

- **`ignore` crate:** File walking with `.gitignore` support. Use for determining what's eligible to scan.
- **`camino` crate:** Safer UTF-8 path handling (`Utf8PathBuf`) — critical for serialized paths.
- **`notify` v8.2.0:** File watching. Cross-platform (FSEvents on macOS). Use `recommended_watcher()` + `PollWatcher` fallback.
  - Known caveats: editor save behavior varies, network/WSL paths miss events, Docker-on-macOS issues, macOS ownership rules can block events, large dir trees can be lossy
  - Use `notify_debouncer_mini` or custom debounce (300-500ms)
  - Classify: source changes → incremental rescan, manifest/config changes → broader rescan
- **`globset`:** For glob pattern matching (ignore paths in `.codeatlas.yaml`)

### 1.6 Supporting Crates

| Crate | Purpose | Phase |
|-------|---------|-------|
| `serde` + `serde_json` | Serialization (500-1000 MB/s deser, 600-900 MB/s ser). Use `#[serde(tag = "type")]` for tagged enums, `#[serde(rename_all = "camelCase")]` for IPC | POC |
| `serde_yaml` + `schemars` | Config parsing + JSON Schema publication for `.codeatlas.yaml` | POC |
| `thiserror` 2.x | Error types in `codeatlas-core`. Each module defines own error enum. | POC |
| `miette` 7.x | Rich diagnostic rendering at Tauri shell boundary. Source-span support ideal for parse errors. | POC |
| `tracing` + `tracing-subscriber` | Structured logging with `#[instrument]` macro. Use `EnvFilter` via `RUST_LOG`. | POC |
| `rayon` | Parallel file parsing. `par_iter()` over file list. Bridge to tokio via `spawn_blocking`. | POC |
| `tokio` | Async runtime (Tauri uses internally). `mpsc` for streaming, `watch` for broadcasting, `oneshot` for cancellation, `CancellationToken` from `tokio_util` | POC |
| `rusqlite` + `rusqlite_migration` | SQLite with `bundled` feature. Synchronous API. WAL mode. Do NOT use sqlx (semver hazard). | MVP |
| `proptest` 1.9.0 | Property-based testing for graph invariants | POC |
| `criterion` 0.8.1 | Benchmarking scan/graph/layout | POC |
| `insta` | Snapshot testing for golden corpus validation | POC |
| `cargo-llvm-cov` | Rust code coverage (works on macOS, unlike tarpaulin) | POC |
| `clap` v4 | CLI argument parser with derive macros | Platform |
| `rmcp` v0.16.0 | Official Rust MCP SDK. `#[tool]` macro, stdio transport. | Platform |

### 1.7 Rust 2024 Edition Features

Target `edition = "2024"` from the start:
- **`async fn` in traits** (stable since 1.75.0): No `#[async_trait]` macro needed
- **`impl Trait` in trait return types**: `fn edges(&self) -> impl Iterator<Item = &EdgeData>` in traits
- **`let` chains**, precise capturing, async closures
- **Caveat:** `async fn` in traits doesn't support `dyn Trait`. If `DetectorRegistry` stores `Vec<Box<dyn Detector>>`, async methods need manual `Pin<Box<dyn Future>>` return type or keep `#[async_trait]` for that case.

---

## 2. Tauri v2 Architecture

### 2.1 Command System

- Commands: `#[tauri::command]` on Rust functions
- Args: `serde::Deserialize`. Returns: `serde::Serialize`. JSON objects with camelCase keys.
- Errors: `Result<T, E>` where E: Serialize. Use `thiserror`.
- Injected params (not from frontend): `AppHandle`, `WebviewWindow`, `State<T>`, `tauri::ipc::Request`
- Async: `async fn` on tokio. **Cannot use `&str` in async commands — use `String`.**
- Commands in `lib.rs` cannot be `pub`. In separate modules: `pub`.
- Register: `tauri::generate_handler![cmd_a, cmd_b]`

### 2.2 Channel<T> Streaming

- `tauri::ipc::Channel<T>` where T: Serialize. Fast, ordered delivery.
- Rust: `channel.send(data).unwrap()`
- Frontend: `Channel<T>` instance → `onmessage` callback → pass as `invoke()` arg
- Use tagged enums: `#[serde(tag = "event", content = "data")]`
- **Use Channel<T> for all scan streaming (progressive rendering). Use regular `invoke()` for request-response queries.**
- Keep individual messages small. Many small > few large.

### 2.3 tauri-specta v2

- Generates TypeScript bindings from Rust command/event signatures
- `#[specta::specta]` + `specta::Type` derives → `tauri_specta::ts::builder().commands(...).build()`
- RC versions — pin exact versions
- If Channel<T> bindings don't work, fall back to manual TS types for channel messages only

### 2.4 Security Model (Capabilities)

- Capability-based permissions in `src-tauri/capabilities/` (JSON/TOML)
- Plugin-namespaced: `${plugin-name}:${permission-name}`
- **Needed:** `dialog:allow-open`, scoped fs read, `shell:allow-execute` (MVP for `code` CLI)
- Rust has full system access. Frontend only through declared capabilities.
- Use capabilities aggressively. Keep scanning entirely in backend Rust. Expose only narrow commands: `open_directory`, `start_scan`, `cancel_scan`, `open_in_editor`, `export_view`, `check_for_updates`.

### 2.5 Plugins

| Plugin | Purpose |
|--------|---------|
| `@tauri-apps/plugin-dialog` | Native file/directory picker |
| `@tauri-apps/plugin-shell` | Child processes (`code -g file:line:col`). Scope exact binaries/args. |
| `@tauri-apps/plugin-opener` | Open files/URLs with system default |
| `@tauri-apps/plugin-fs` | File read/write |
| `@tauri-apps/plugin-updater` | Auto-update (requires signing keypair; cannot disable signatures) |

**Pin Tauri plugin versions conservatively.** Plugins evolve somewhat independently of core stability.

---

## 3. Frontend Stack

### 3.1 React Flow v12 (@xyflow/react)

- **Sub flows / nested nodes:** `parentId` property. Child positions relative to parent. Parent MUST appear before children in array. `extent: 'parent'` constrains children.
- **Custom nodes:** React components + `Handle` for connections. Register via `nodeTypes` prop.
- **Expand/collapse:** NOT built in. Implement via `hidden` property + state management:
  - Collapse: hide children, remove child edges, add bundled edges
  - Expand: show children, restore edges, remove bundled edges
  - Re-run ELK layout after toggle
- **Edges:** Built-in bezier/straight/step/smoothstep. Custom edges via `BaseEdge`. Style with `stroke`, `strokeDasharray` (dashed: `'5,5'`). Markers via `markerEnd`/`markerStart`.
- **MiniMap:** Built-in `<MiniMap />` with `zoomable`, `pannable`, `nodeColor` props
- **Performance rules:**
  - Memoize all custom components with `React.memo`, callbacks with `useCallback`
  - `onlyRenderVisibleElements` prop — critical
  - Collapse large trees via `hidden` property
  - Avoid shadows/gradients/animations at scale
  - Zustand for state (React Flow uses it internally)
- **Gotchas:**
  - No nested React Flow instances (transform conflicts)
  - DOM-measured sizing: render → measure → layout → reposition cycle
  - Edge z-index: edges to nested nodes render above regular nodes

### 3.2 ELK.js Layout

- JavaScript port of Eclipse Layout Kernel. Best open-source for compound graphs.
- **Web Worker support built in:** `import ELK from 'elkjs/lib/elk.bundled.js'`
- **Key options:**
  - `elk.algorithm: 'layered'` (hierarchical — best for dependency graphs)
  - `elk.direction: 'DOWN'` or `'RIGHT'`
  - `elk.hierarchyHandling: 'INCLUDE_CHILDREN'`
  - `elk.spacing.nodeNode`, `elk.layered.spacing.nodeNodeBetweenLayers`, `elk.padding`
  - `elk.edgeRouting: 'ORTHOGONAL'` | `'POLYLINE'` | `'SPLINES'`
- **Graph format:** hierarchical `children` arrays with `id`, `width`, `height`, `layoutOptions`, `edges`
- **Performance:** 200 nodes <500ms. Compound graphs more expensive. Worker ensures main thread responsive.
- **Gotchas:**
  - Render-measure-layout cycle: React Flow renders → get DOM dimensions → ELK computes → apply positions → visible relayout flash. Mitigation: estimated sizes first, then refine.
  - `INCLUDE_CHILDREN` can be slow for deeply nested large graphs
  - Cross-hierarchy edges can produce suboptimal routing
- **Recommendations:**
  - **Never run on main thread**
  - Start with `layered` + `DOWN`
  - Debounce layout requests (300ms) during rapid expand/collapse
  - Cache layout results per expand/collapse state

### 3.3 Web Workers in Vite

```typescript
// Query string import (simpler):
import ElkWorker from './elk.worker?worker';
const worker = new ElkWorker();

// URL constructor (standards-based):
const worker = new Worker(new URL('./elk.worker.ts', import.meta.url), { type: 'module' });
```

Configure in `vite.config.ts`: `worker: { format: 'es' }`

### 3.4 State Management: Zustand

- React Flow recommends and uses Zustand internally
- Graph state (nodes, edges, expanded, selection, viewport) is interconnected — changes as a unit
- Selector-based subscriptions prevent unnecessary re-renders
- DevTools middleware available
- ~2KB gzipped

**Store shape:**
```typescript
interface AppStore {
  nodes: Node[];
  edges: Edge[];
  expandedNodeIds: Set<string>;
  scanStatus: 'idle' | 'scanning' | 'complete' | 'error';
  compatibilityReport: CompatibilityReport | null;
  graphHealth: GraphHealth | null;
  activeProfile: GraphProfile | null;
  selectedNodeId: string | null;
  searchQuery: string;
  detailPanelOpen: boolean;
  // Actions...
}
```

### 3.5 Frontend Tooling

| Tool | Purpose | Notes |
|------|---------|-------|
| **Biome v2.x** | Linter + formatter (replaces ESLint + Prettier). 35x faster formatting, 15x faster linting. Tailwind CSS class sorting built-in. | Greenfield — zero migration cost |
| **React Compiler v1.0** | Automatic memoization at build time. Up to 12% faster initial loads. | Test compatibility with React Flow early (M3) |
| **shadcn/ui** | UI components (Radix + Tailwind). Copy-into-project model. Pre-built Command (cmdk wrapper), dialogs, tabs, panels, badges. | Use for all non-graph UI |
| **cmdk** | Command palette for Cmd+K search. Headless, fuzzy search built-in. | shadcn/ui wraps it |
| **Motion (Framer Motion)** | Animations: expand/collapse transitions, layout changes, panel slide-in, node highlights. `AnimatePresence`, `layout` prop, spring physics. Do NOT animate individual edges. | Medium priority — adds polish |
| **Lucide React** | Icons. Clean stroke-based style. shadcn/ui default. Tree-shakeable. | Aligns with component library |
| **lefthook** | Pre-commit hooks. Polyglot (Rust + TS). Parallel execution. Single `lefthook.yml`. | Runs `biome check`, `cargo clippy`, `cargo fmt --check` |

### 3.6 React 19 Features

- **React Compiler:** Auto-optimizes memoization. May conflict with React Flow patterns — test early.
- **Concurrent rendering:** Prevents long renders from blocking UI during graph updates.
- Server Components NOT relevant (Tauri desktop app).

---

## 4. Architecture Patterns

### 4.1 Repository Layout

```
tauri-poc-zoom-thing/
  Cargo.toml          # virtual manifest (workspace root)
  crates/
    codeatlas-core/   # standalone analysis library (NO Tauri dependency)
      src/
        lib.rs
        workspace/    # workspace discovery
        detector/     # detector trait + implementations
        graph/        # petgraph wrapper, identity, overlay
        config/       # .codeatlas.yaml parser
        profile/      # graph profile management
        health/       # compatibility report, graph health
    codeatlas-tauri/   # Tauri app (thin shell)
      src/
        lib.rs        # Tauri commands
        main.rs       # Tauri entry point
      capabilities/
      tauri.conf.json
  src/                # React frontend
  package.json
```

**Key constraint:** `codeatlas-core` has ZERO dependency on `tauri`, `serde_json`, or any IPC/transport crate.

### 4.2 Core API Pattern (rust-analyzer style)

- **`AnalysisHost`** — mutable handle. Accepts workspace changes, file edits, config updates, rescan requests. Holds current graph state.
- **`Analysis`** — immutable snapshot from `AnalysisHost`. Safe for concurrent queries. All query methods live here.
- Uses **domain terminology** (nodes, edges, profiles, health), NOT transport terminology
- All types are conceptually serializable POD with public fields
- Not influenced by Tauri IPC, MCP tool schema, or any specific transport

### 4.3 Two-Layer Graph Model

```rust
pub struct ArchGraph {
    discovered: StableGraph<NodeData, EdgeData, Directed>,  // Immutable
    overlay: GraphOverlay,                                    // Config additions/suppressions
    node_index: HashMap<MaterializedKey, NodeIndex>,          // Fast lookups
}

pub struct GraphOverlay {
    manual_edges: Vec<ManualEdge>,
    suppressions: HashMap<EdgeIndex, SuppressionReason>,
    metadata: HashMap<NodeIndex, NodeMetadata>,
}
```

Query surfaces: discovered only, overlay only, or merged view.

### 4.4 Streaming Pipeline

```
[Detector Registry]
    | (mpsc channel per detector)
    v
[Graph Builder] -- merges into StableGraph
    | (output channel with phases)
    v
[Phase Splitter]
    |-- Phase 1: PackageTopology
    |-- Phase 2: ModuleStructure
    |-- Phase 3: FileEdges
    v
[Tauri Channel<T>] --> frontend onmessage handler
```

### 4.5 Identity Scheme

```rust
#[derive(Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct MaterializedKey {
    pub language: Language,
    pub entity_kind: EntityKind,
    pub relative_path: String,
}

#[derive(Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct LineageKey(Uuid);  // Populated in MVP

pub struct NodeData {
    pub materialized_key: MaterializedKey,
    pub lineage_key: Option<LineageKey>,
    pub label: String,
    pub kind: NodeKind,
    pub parent: Option<NodeIndex>,
}
```

**Critical:** Do NOT persist `workspace_root` inside exported/materialized keys (breaks portability, leaks paths). Use `{repo_fingerprint}:{profile_fingerprint}:{entity_kind}:{relative_path}` for external identity.

### 4.6 Detector Trait

```rust
pub trait Detector: Send + Sync {
    fn name(&self) -> &str;
    fn language(&self) -> Language;
    fn applies_to(&self, workspace: &WorkspaceInfo) -> bool;
    fn compatibility(&self, workspace: &WorkspaceInfo) -> CompatibilityAssessment;
    fn detect(
        &self,
        workspace: &WorkspaceInfo,
        profile: &GraphProfile,
        config: &RepoConfig,
        sink: &dyn DetectorSink,
    ) -> Result<DetectorReport>;
}
```

### 4.7 Domain Types (Define Early)

`WorkspaceSnapshot`, `GraphProfile`, `ProfileFingerprint`, `CompatibilityReport`, `UnsupportedConstruct`, `GraphHealth`, `NodeId`, `MaterializedKey`, `LineageKey`, `EdgeEvidence`, `EdgeCategory`, `OverlayState`, `SavedView`, `GraphSnapshot`

---

## 5. TypeScript/JavaScript Resolution

### 5.1 Resolution Modes

| Mode | Behavior |
|------|----------|
| `nodenext` | Enforces Node.js ESM rules. No extensionless imports in ESM. `require()` of ESM allowed (Node v22.12+). Includes `node` condition. |
| `bundler` | Allows extensionless imports. Does NOT include `node` condition. Most common in modern frontend. |
| `node16` | Like nodenext but `require()` of ESM is error. |
| `node` | Legacy CJS resolution. |

### 5.2 Key Resolution Facts

- `exports` field: condition names (`import`, `require`, `node`, `browser`, `types`, `default`). **Key order is load-bearing — first match wins.**
- `imports` field: internal aliases with `#` prefix
- `type: "module"` determines default module format for .js files
- TypeScript also consults `types`/`typings`/`typesVersions` — runtime-only lens is insufficient for TS libraries
- `customConditions` in tsconfig should be carried in the profile model
- Mixed-format monorepos (some CJS, some ESM) need per-package resolution config

### 5.3 Workspace Detection

- **pnpm:** `pnpm-workspace.yaml` → `packages:` array with globs. Does NOT use `package.json` `workspaces` field.
- **npm/Yarn classic:** `package.json` `workspaces` field
- **Yarn PnP:** `.yarnrc.yml` with `nodeLinker: pnp`. Creates `.pnp.cjs` instead of `node_modules`.

### 5.4 Condition-Set Profiles

Implement explicit condition-set profiles:
- Node ESM-like: `["node","import"]`
- Node CJS-like: `["node","require"]`
- Bundler-like: configurable condition stack

Capture exact condition order used. Persist on snapshots. Show in UI.

### 5.5 POC vs MVP Scope

- **POC:** tree-sitter parsing, workspace package discovery, tsconfig `paths`/`baseUrl` basic resolution. NOT full `exports`/`imports` conditions, NOT PnP, NOT project references.
- **MVP:** `oxc_parser` + `oxc_resolver` for full resolution spec. Support `paths`, `baseUrl`, `references`, `exports`, `imports`, condition sets. Explicitly detect and report Yarn PnP and mixed ESM/CJS.

---

## 6. Rust Module System (Parsing)

### 6.1 Module Resolution

Deterministic from manifest + source structure:
1. `mod foo;` in `src/lib.rs` → `src/foo.rs` or `src/foo/mod.rs`
2. `use crate::foo::bar` traverses from crate root
3. `use super::foo` / `use self::foo` — relative navigation
4. `use external_crate::foo` — via `cargo_metadata` dependency graph

### 6.2 cargo_metadata Mapping

- `Package` with `lib` target → crate node
- `targets` with `kind: ["bin"]` → binary entry points
- `resolve.nodes[].deps` → inter-crate edges with `dep_kinds` (Normal/Dev/Build)
- `features` map → feature flags and their dependencies

### 6.3 Limitations (Badge, Don't Hide)

- `#[cfg(...)]` gates need build execution for non-default features
- `build.rs` generates code (OUT_DIR), defines custom cfg flags
- `proc_macro` expansion — invisible to static analysis
- `include!()` macro — detectable syntactically, included file needs separate analysis
- `cfg_attr(target_os, path = "...")` can change which files are in the module tree
- **Cargo resolver v2** changes feature unification behavior — must be part of profile

---

## 7. SQLite Integration (MVP)

### 7.1 Schema

```sql
CREATE TABLE snapshots (
    id TEXT PRIMARY KEY, workspace_root TEXT NOT NULL,
    profile_hash TEXT NOT NULL, created_at TEXT NOT NULL,
    node_count INTEGER NOT NULL, edge_count INTEGER NOT NULL
);
CREATE TABLE nodes (
    id INTEGER PRIMARY KEY, snapshot_id TEXT NOT NULL REFERENCES snapshots(id),
    materialized_key TEXT NOT NULL, lineage_key TEXT,
    label TEXT NOT NULL, kind TEXT NOT NULL, language TEXT,
    parent_key TEXT, metadata TEXT,
    UNIQUE(snapshot_id, materialized_key)
);
CREATE TABLE edges (
    id INTEGER PRIMARY KEY, snapshot_id TEXT NOT NULL REFERENCES snapshots(id),
    source_key TEXT NOT NULL, target_key TEXT NOT NULL,
    kind TEXT NOT NULL, category TEXT NOT NULL, confidence TEXT NOT NULL,
    source_location TEXT, resolution_method TEXT, overlay_status TEXT
);
CREATE TABLE lineage (
    lineage_key TEXT PRIMARY KEY, first_seen_snapshot TEXT NOT NULL,
    current_materialized_key TEXT, previous_keys TEXT
);
CREATE TABLE views (
    id TEXT PRIMARY KEY, name TEXT NOT NULL,
    snapshot_id TEXT REFERENCES snapshots(id),
    viewport TEXT NOT NULL, expanded_nodes TEXT NOT NULL, filters TEXT
);
CREATE TABLE bookmarks (
    id TEXT PRIMARY KEY, lineage_key TEXT NOT NULL,
    label TEXT, created_at TEXT NOT NULL
);
```

### 7.2 Operational Notes

- WAL mode for concurrent reads during writes
- Batch inserts in transactions (MUCH faster)
- 5,000-node/20,000-edge graph serializes in <100ms
- Content-addressed dedupe for large evidence strings
- Deterministic write ordering
- Treat DB + WAL files as a unit
- No FTS5 until Platform

---

## 8. Testing Strategy

### 8.1 Test Pyramid

1. **Rust unit tests** — resolvers, config parsing, overlay merge, identity generation, query logic
2. **Rust integration tests** — reference repos, compatibility reports, graph snapshots (insta)
3. **Property tests (proptest)** — graph invariants, overlay immutability, path normalization, collapse/expand
4. **Performance tests (criterion)** — scan latency, layout latency, snapshot size
5. **Frontend tests (Vitest + testing-library)** — selectors, filters, state transitions, Tauri IPC mocking
6. **E2E (MVP, Playwright)** — frontend isolation tests, mocked IPC. macOS WebDriver NOT available.

### 8.2 Golden Corpus Dimensions

- Rust workspaces with features/targets
- TS monorepos with project refs + exports/imports + aliases
- Mixed-language repos
- Overlays and suppressions
- Repos with graph-shaping branch deltas
- Mixed ESM/CJS workspace
- Yarn PnP repo (as deliberate Partial/Unsupported case)
- cfg-heavy Rust, proc-macro/build-script examples
- Generated-code paths and ignore rules

### 8.3 Mocking Tauri IPC

```typescript
import { mockIPC, clearMocks } from '@tauri-apps/api/mocks';
beforeEach(() => { mockIPC((cmd, args) => { /* ... */ }); });
afterEach(() => clearMocks());
```

---

## 9. CI/CD & Distribution

### 9.1 CI Lanes

1. **ci-fast:** lint/typecheck/unit tests
2. **ci-integration:** detector integration + golden subset
3. **release:** signed multi-platform artifacts + updater JSON
4. **nightly:** full corpus + perf + flaky diagnostics

### 9.2 GitHub Actions Matrix

- macOS arm64, macOS Intel, Linux x64, Windows x64
- Steps: checkout → system deps (Linux) → Node/pnpm → Rust toolchain → `pnpm install` → `tauri-apps/tauri-action@v0`
- `GITHUB_TOKEN` with read/write permissions
- Platform-specific env vars for signing certificates

### 9.3 Distribution

- **POC:** No distribution. Run from source or `tauri dev`.
- **MVP:** GitHub Releases + signed installers. Homebrew cask tap (`brew install --cask codeatlas`).
- macOS: DMG + notarization (Apple Developer ID, $99/yr). Without signing: right-click > Open bypass only.
- Windows: NSIS/MSI. Code signing affects SmartScreen. EV certs get immediate reputation.
- Linux: AppImage (universal), `.deb` once releases stabilize
- **Updater:** `tauri-plugin-updater`, static JSON on GitHub Releases. Non-modal "Update ready — restart when convenient." Never auto-restart.

### 9.4 Signing Requirements

- Tauri updater **requires** signatures (cannot disable)
- macOS notarization required for Developer ID distribution (free Apple accounts cannot notarize)
- Generate signing keypair separate from Apple code signing
- Update flow: check `latest.json` → download → verify signature → install → restart

---

## 10. Performance Budgets & Cliffs

| Metric | Target | Phase |
|--------|--------|-------|
| First meaningful frame (package topology, ≤2,000 files) | <2 seconds | POC |
| Full scan + render (≤2,000 files) | <10 seconds | POC |
| ELK layout (200 nodes) | <500ms in Worker | POC |
| Interaction framerate (<200 visible nodes) | 60fps | POC |
| Memory at 500-node graph | <200MB | POC |
| Graph update after file save (watch mode) | <2 seconds | MVP |
| VS Code round-trip (local) | <1 second | MVP |

| Performance Cliff | Threshold | Mitigation |
|-------------------|-----------|------------|
| React Flow DOM nodes | >500 visible | `onlyRenderVisibleElements`, collapse |
| ELK computation | >500 nodes | Worker, layout only visible portion |
| tree-sitter initial parse | >5000 files | rayon parallelization |
| IPC serialization | >1MB JSON | Stream via Channel<T> |
| Browser memory | >200MB | Only render expanded packages |

---

## 11. Cross-Platform Considerations

- **macOS:** WKWebView. Primary target. Code signing required for distribution.
- **Linux:** webkit2gtk 4.1 (Ubuntu 22.04+). Visual differences from GTK theming.
- **Windows:** WebView2 (Chromium). `.msi` needs WiX (Windows-only), `.exe` uses NSIS.
- **Cross-compilation not supported** — must build on each target (CI matrix)
- **Path handling:** Use `std::path::Path` / `camino`, never string manipulation
- **Case sensitivity:** macOS/Windows case-insensitive, Linux sensitive. Import resolution must handle this.
- **WebGPU NOT viable** in Tauri (WKWebView lacks access). Use DOM optimizations instead.

---

## 12. Version Reference

| Crate/Package | Version | Notes |
|---------------|---------|-------|
| tree-sitter | latest stable | |
| petgraph | 0.7.x | StableGraph |
| cargo_metadata | latest stable | |
| oxc_parser | 0.118.x+ | Pre-1.0, pin carefully |
| oxc_resolver | latest stable | Separate repo |
| serde / serde_json | 1.x | |
| notify | 8.2.0 | |
| tokio | 1.x | |
| tauri | 2.x | |
| tauri-specta | 2.0.0-rc.x | Pin exact |
| rusqlite | 0.38.x | `bundled` feature |
| proptest | 1.9.0 | |
| criterion | 0.8.1 | |
| thiserror | 2.x | |
| @xyflow/react | 12.x | |
| elkjs | 0.9.x | EPL-2.0 license — legal review needed |
| zustand | 5.x | |
| cmdk | latest | |
| tailwindcss | 4.x | CSS-first config |
| vitest | latest | |
| rmcp | 0.16.0 | Platform phase |
