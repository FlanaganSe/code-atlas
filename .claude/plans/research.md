# Code Atlas -- Comprehensive Technical Research

**Date:** 2026-03-18
**Scope:** All 14 research domains for POC and MVP planning
**PRD Version:** v5

---

## 1. Core Rust Libraries & APIs

### 1.1 tree-sitter (Rust Bindings)

**What we know:**
- **Crate:** `tree-sitter` on crates.io. The Rust bindings are maintained in the main tree-sitter repo under `lib/binding_rust/`.
- **Grammar crates:** `tree-sitter-typescript` (provides both TypeScript and TSX grammars as separate parsers) and `tree-sitter-rust`.
- **Key API surface:**
  - `Parser` -- stateful, call `parser.set_language(language)` then `parser.parse(source, old_tree)`.
  - `Tree` -- immutable parse result. Call `tree.root_node()` to get the root `Node`.
  - `Node` -- represents a syntax node. Has `kind()`, `child()`, `children()`, `child_by_field_name()`, `utf8_text()`, `start_position()`, `end_position()`.
  - `Query` + `QueryCursor` -- pattern matching against the AST. Define S-expression patterns, run against a node, iterate `QueryMatch` results with captures.
  - `Language` -- loaded from grammar crates via e.g. `tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()`.

- **Incremental parsing:** Pass the previous `Tree` as the second argument to `parser.parse()`. Call `old_tree.edit()` with an `InputEdit` describing the byte range change. Tree-sitter reuses unchanged subtrees. Update time is typically sub-millisecond for small edits.

- **Distinguishing `import type` vs `import` in TypeScript:**
  The tree-sitter-typescript grammar does support `import type` syntax. The `import_statement` node contains an optional `type` keyword child. To distinguish:
  - Parse the `import_statement` node
  - Check for a child with `kind() == "type"` immediately after the `import` keyword
  - For inline type imports (`import { type Foo }`), check the `import_specifier` for a `type` child
  - The `type` keyword is an anonymous node (not a named node), so use `child()` iteration rather than `child_by_field_name()`

  **Query pattern for type imports:**
  ```
  (import_statement "type" @type_keyword) @import
  ```

- **Extracting imports/exports from TypeScript:** Key node types:
  - `import_statement` -- contains `import_clause` and `source` (string literal)
  - `import_clause` -- contains `named_imports` (with `import_specifier` children) or `namespace_import`
  - `export_statement` -- top-level exports
  - `export_clause` -- named exports
  - `lexical_declaration` with `export` -- `export const/let`

- **Extracting mod/use/pub from Rust:** Key node types:
  - `mod_item` -- `mod foo;` or `mod foo { ... }`
  - `use_declaration` -- `use foo::bar;`
  - `visibility_modifier` -- `pub`, `pub(crate)`, etc.
  - `use_wildcard`, `use_list`, `use_as_clause`, `scoped_use_list`

- **Performance:** tree-sitter-rust takes 2-3x as long as rustc's hand-written parser for initial parse, but incremental updates are sub-millisecond. For a typical 2000-file repo, initial parsing of all files should take 1-5 seconds total (parallelizable across files).

**Risks & gotchas:**
- The `type` keyword in `import type` is an anonymous node -- easy to miss if only iterating named children. Must use `child()` or query patterns.
- tree-sitter grammars may lag behind the latest TypeScript/Rust syntax additions. Check grammar version compatibility.
- Error recovery means malformed files still produce a partial tree, which is desirable for our use case but means we must handle `ERROR` and `MISSING` nodes gracefully.
- Tree-sitter queries cannot express cross-file relationships. Import resolution requires separate logic.

**Recommendations:**
- Use tree-sitter `Query` patterns rather than manual AST walking for import/export extraction. Queries are faster and more maintainable.
- Write separate query files (`.scm`) for each language detector.
- For POC, tree-sitter is sufficient for both Rust and TypeScript parsing. The MVP upgrade to oxc_parser for TS is additive.
- Pin grammar crate versions to avoid breaking changes.

### 1.2 petgraph (StableGraph)

**What we know:**
- **Crate:** `petgraph` on crates.io. Current version: 0.7.x (latest stable). Over 2.1M downloads. Actively maintained.
- **StableGraph** is the right choice: indices remain valid after node/edge removal. This is critical for incremental updates and the identity scheme.
- **Type parameters:** `StableGraph<N, E, Ty, Ix>` where N = node weight, E = edge weight, Ty = Directed/Undirected, Ix = index type (u32 default).
- **Key methods:**
  - `add_node(weight) -> NodeIndex` / `add_edge(source, target, weight) -> EdgeIndex`
  - `remove_node(index)` / `remove_edge(index)` -- gaps form but existing indices stay valid
  - `node_weight(index)` / `edge_weight(index)` -- access data
  - `neighbors(node)` / `edges(node)` -- iterate connections
  - `find_edge(source, target)` -- locate specific edges
  - `node_count()` / `edge_count()` -- graph statistics
  - Parallel edges are allowed (important for multiple edge types between same nodes)

- **Algorithms in `petgraph::algo`:** Comprehensive set available:
  - **SCC:** `tarjan_scc()`, `kosaraju_scc()`, `condensation()` -- all work with directed graphs
  - **Topological sort:** `toposort()` -- returns error on cycles
  - **Shortest paths:** `dijkstra()`, `astar()`, `bellman_ford()`, `floyd_warshall()`
  - **Connectivity:** `connected_components()`, `has_path_connecting()`
  - **Cycles:** `is_cyclic_directed()`
  - **Traversal:** `Dfs`, `DfsPostOrder`, `Bfs` visitors
  - **Other:** `dominators()`, `min_spanning_tree()`, `page_rank()`, transitive reduction/closure

- **Compound/hierarchical graph modeling:** petgraph does NOT have built-in compound node support. We must model hierarchy ourselves:
  - **Approach 1 (recommended):** Use a flat `StableGraph` where each node carries a `parent: Option<NodeIndex>` field. Use "contains" edges with a distinct edge kind. Query hierarchy by filtering edges.
  - **Approach 2:** Maintain a separate tree structure (HashMap<NodeIndex, Vec<NodeIndex>>) alongside the graph.
  - Approach 1 is simpler and keeps all data in one structure, making serialization and querying uniform.

**Risks & gotchas:**
- Not all algorithms work with `StableGraph`. Most trait-based algorithms should work because `StableGraph` implements key graph traits, but some may require `Graph`. Test each algorithm needed.
- `StableGraph` uses more memory than `Graph` due to index gaps. Not a concern at our scale (< 50K nodes).
- No built-in serialization. Must implement custom serde for the graph.

**Recommendations:**
- Use `StableGraph<NodeData, EdgeData, Directed, u32>` as the primary graph structure.
- Model hierarchy via a `parent: Option<NodeIndex>` field on `NodeData` plus `EdgeKind::Contains` edges.
- Use `tarjan_scc()` for cycle detection (POC stretch goal).
- Use `Bfs`/`Dfs` for transitive dependency/dependent queries.
- Wrap the raw petgraph in a domain-specific `ArchGraph` struct that enforces invariants (immutable discovered layer, overlay separation).

### 1.3 cargo_metadata

**What we know:**
- **Crate:** `cargo_metadata` on crates.io. Well-maintained, wraps `cargo metadata --format-version 1`.
- **Key struct:** `Metadata` with fields:
  - `packages: Vec<Package>` -- all referenced crates
  - `workspace_members: Vec<PackageId>` -- workspace member IDs
  - `workspace_default_members: WorkspaceDefaultMembers` -- default build members
  - `resolve: Option<Resolve>` -- resolved dependency graph
  - `workspace_root: Utf8PathBuf` -- workspace root path
  - `target_directory: Utf8PathBuf` -- target dir
  - `workspace_metadata: Value` -- custom metadata from Cargo.toml

- **Package struct:** `name`, `version`, `id`, `source`, `dependencies: Vec<Dependency>`, `targets: Vec<Target>`, `features: BTreeMap<String, Vec<String>>`, `manifest_path`, `edition`, `metadata`.

- **Dependency struct:** `name`, `req` (version requirement), `kind` (Normal/Dev/Build), `optional`, `features`, `target` (platform-specific).

- **Resolve struct:** Contains `nodes: Vec<Node>` where each `Node` has `id`, `dependencies`, `deps` (with detailed dep info including `dep_kinds` with `kind` and `target`).

- **Target struct:** `name`, `kind` (lib/bin/test/bench/example), `crate_types`, `src_path`.

- **Usage pattern:**
  ```rust
  let metadata = cargo_metadata::MetadataCommand::new()
      .manifest_path("path/to/Cargo.toml")
      .exec()?;
  ```
  Can also pass `--features`, `--no-default-features`, `--filter-platform`.

**Risks & gotchas:**
- `cargo metadata` invokes cargo as a subprocess. First call may be slow if dependencies need resolution (~2-10 seconds). Subsequent calls with warm cache are fast.
- The `resolve` field may be `None` if `--no-deps` is passed.
- `dep_kinds` on resolve nodes provides the most accurate dependency kind information (normal/dev/build) per-edge.
- Build script outputs, proc-macro expansion results, and conditional compilation are NOT reflected in cargo_metadata output. These are fundamental limitations that must be badged.

**Recommendations:**
- Cache metadata output. Only re-invoke on Cargo.toml/Cargo.lock changes.
- Use `dep_kinds` from the resolve graph (not the Package dependencies) for accurate edge categorization.
- Extract workspace members, their targets, and inter-crate dependencies as the foundation of the Rust graph.

### 1.4 oxc_parser + oxc_resolver (MVP)

**What we know:**
- **Project:** Oxc (Oxidation Compiler) -- high-performance JS/TS toolchain in Rust. Very active development (20K+ GitHub stars, 16K+ commits, latest release March 2026: oxlint v1.56.0, oxfmt v0.41.0).
- **oxc_parser:** Fast, spec-compliant parser for JavaScript and TypeScript. Passes 100% of Test262, 99% of Babel and TypeScript tests. Minimal API: `Parser::new(allocator, source, source_type) -> ParserReturn`. Uses an arena allocator for zero-copy AST.
- **oxc_resolver:** Production-ready module resolver. Used by Nova, swc-node, knip in production.
  - Implements ESM and CommonJS resolution algorithms per Node.js spec
  - Full tsconfig.json support: `paths`, `baseUrl`, `extends`, project references, `${configDir}` substitution
  - Package.json `exports` and `imports` fields with condition name resolution
  - Condition names priority (key order matters, first match wins)
  - Yarn PnP support (behind feature flag)
  - Extension alias support (`.js` -> `.ts`)
  - 40+ configurable options
  - SIMD-accelerated JSON parsing, lock-free concurrent caching
  - API: `Resolver::new(options).sync(directory, specifier)` or `resolveFileSync(file, specifier)`

- **Important note:** oxc_resolver is in a SEPARATE repository (`oxc-project/oxc-resolver`), not in the main oxc repo. The main `oxc` umbrella crate re-exports `oxc_parser` but NOT `oxc_resolver`.

**Risks & gotchas:**
- oxc_parser uses an arena allocator (`oxc_allocator::Allocator`). AST nodes are borrowed from the arena -- cannot be stored long-lived without copying data out.
- API is still pre-1.0. Breaking changes possible between minor versions.
- Must copy/extract the import data we need from the AST before the allocator is dropped.

**Recommendations:**
- For POC: use tree-sitter for TypeScript parsing (simpler API, no arena complexity).
- For MVP: switch to oxc_parser + oxc_resolver for TypeScript. Keep tree-sitter as fallback for files oxc_parser cannot handle.
- oxc_resolver eliminates the need to implement our own TS module resolution. This is a massive complexity reduction.
- Pin crate versions carefully.

### 1.5 serde + serde_json

**What we know:**
- Industry standard for Rust serialization. Performance: 500-1000 MB/s deserialization, 600-900 MB/s serialization.
- Zero-cost derive macros generate serialization code at compile time.
- Key patterns:
  - `#[derive(Serialize, Deserialize)]` on all data types crossing IPC boundary
  - `#[serde(tag = "type")]` for internally tagged enums (useful for edge/node kind variants)
  - `#[serde(rename_all = "camelCase")]` for Tauri IPC compatibility
  - `#[serde(skip)]` for fields that shouldn't be serialized

**Recommendations:**
- Use `serde` derives on all graph data types.
- For large graph payloads, consider streaming JSON chunks rather than single giant documents.
- Use `serde_json::Value` sparingly -- prefer typed deserialization.

### 1.6 notify (File Watching)

**What we know:**
- **Crate:** `notify` v8.2.0 (stable, August 2025). v9.0.0-rc.1 available.
- Cross-platform file system notification library. Used by alacritty, cargo-watch, deno, mdBook, rust-analyzer.
- **API:** `notify::recommended_watcher()` auto-selects best backend per platform. Uses `EventHandler` trait (closures, channel senders).
- Supports `RecursiveMode::Recursive` for watching directory trees.
- MSRV: Rust 1.85.
- macOS uses FSEvents backend. Good performance, some debouncing considerations.

**Recommendations:**
- Use `notify` v8 for MVP file watching.
- Debounce events (300-500ms as PRD specifies) using `notify_debouncer_mini` or custom debounce logic.
- Classify events: source file changes trigger incremental rescan, manifest/config changes trigger broader rescan.
- Send events through a tokio channel to the scan pipeline.

### 1.7 tokio (Async Runtime)

**What we know:**
- Tauri v2 uses tokio internally. Async Tauri commands run on tokio's runtime via `async_runtime::spawn`.
- Key channel types for our architecture:
  - `tokio::sync::mpsc` -- multi-producer, single-consumer. Use for streaming scan results from detectors to the graph builder.
  - `tokio::sync::watch` -- single-producer, multi-consumer. Use for broadcasting graph state updates.
  - `tokio::sync::oneshot` -- single-use request-response. Use for scan cancellation signals.
- Pattern: spawn detector tasks, each sends results through mpsc channel, graph builder consumes.

**Recommendations:**
- Use tokio mpsc channels internally within `codeatlas-core` for streaming detector results.
- Bridge to Tauri's `Channel<T>` at the IPC boundary.
- Use `CancellationToken` from `tokio_util` for scan cancellation.

---

## 2. Tauri v2 Architecture

### 2.1 Command System

**What we know:**
- Commands are Rust functions decorated with `#[tauri::command]`.
- Arguments must implement `serde::Deserialize`. Return types must implement `serde::Serialize`.
- Arguments are passed as JSON objects with camelCase keys by default.
- Error handling: return `Result<T, E>` where E implements `Serialize`. Use `thiserror` for error types.
- Special injected parameters (not from frontend): `AppHandle`, `WebviewWindow`, `State<T>`, `tauri::ipc::Request`.
- Async commands use `async fn` and run on tokio.
- Commands in `lib.rs` cannot be `pub` (glue code limitation). Commands in separate modules should be `pub`.
- Register via `tauri::generate_handler![cmd_a, cmd_b]`.
- **Important:** Functions cannot use borrowed types like `&str` in async commands. Use `String` instead.

**Code pattern:**
```rust
#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Custom(String),
}

impl serde::Serialize for AppError { /* ... */ }

#[tauri::command]
async fn scan_workspace(
    path: String,
    on_progress: tauri::ipc::Channel<ScanEvent>,
    state: tauri::State<'_, AppState>,
) -> Result<ScanResult, AppError> {
    // ...
}
```

### 2.2 Channel<T> Streaming API

**What we know:**
- `tauri::ipc::Channel<T>` where T: Serialize. Fast, ordered data delivery.
- Used internally for download progress, child process output, WebSocket messages.
- On the Rust side: `channel.send(data).unwrap()` sends data to the frontend.
- On the frontend: create a `Channel<T>` instance, set `onmessage` callback, pass as argument to `invoke()`.
- Channels are designed for high-throughput scenarios. Significantly better than the event system for large data streams.
- Can send tagged enums (using `#[serde(tag = "event", content = "data")]`) for discriminated message types.

**Frontend pattern:**
```typescript
import { invoke, Channel } from '@tauri-apps/api/core';

type ScanEvent =
  | { event: 'compatibilityReport'; data: CompatibilityReport }
  | { event: 'packageTopology'; data: PackageNode[] }
  | { event: 'moduleStructure'; data: ModuleNode[] }
  | { event: 'fileEdges'; data: FileEdge[] }
  | { event: 'scanComplete'; data: ScanSummary };

const onEvent = new Channel<ScanEvent>();
onEvent.onmessage = (message) => {
  switch (message.event) {
    case 'compatibilityReport': /* ... */ break;
    case 'packageTopology': /* ... */ break;
    // ...
  }
};

await invoke('scan_workspace', { path: '/path/to/repo', onEvent });
```

**Recommendations:**
- Use Channel<T> for all scan streaming (progressive rendering phases).
- Use regular `invoke()` for request-response queries (node details, search, etc.).
- Keep individual channel messages small. Stream many small messages rather than few large ones.

### 2.3 tauri-specta v2

**What we know:**
- **Crate:** `tauri-specta` v2. Requires Tauri v2 + Specta v2. Latest docs dated January 2026.
- Generates TypeScript binding file from Rust command and event signatures.
- Setup:
  1. Add `#[specta::specta]` attribute to each Tauri command
  2. Derive `specta::Type` on all argument/return types
  3. In main: build with `tauri_specta::ts::builder().commands(...).build()`
  4. In debug mode, writes to a `bindings.ts` file in the frontend source
- Also supports type-safe events with `tauri_specta::Event` derive.
- Generates functions like `commands.scanWorkspace(...)` that wrap `invoke()`.
- Supports extra type exports and constant exports.

**Risks & gotchas:**
- Version pinning is critical. Example pins: `tauri@=2.0.0-beta.22`, `specta@=2.0.0-rc.12`, `tauri-specta@=2.0.0-rc.11`.
- RC versions mean API may still change.
- Complex generic types or lifetime parameters may not be supported.
- Channel<T> parameter type generation needs verification.

**Recommendations:**
- Use tauri-specta for POC. It dramatically improves developer experience.
- If Channel<T> bindings don't work well, fall back to manual TypeScript types for the Channel message types only.
- Generate bindings on every build (in dev mode). Add to `.gitignore` or commit them -- team preference.

### 2.4 Security Model (Capabilities)

**What we know:**
- Tauri v2 uses a capability-based permission system.
- Capabilities defined in `src-tauri/capabilities/` as JSON or TOML files.
- Each capability specifies: identifier, windows it applies to, permissions list.
- Permissions use plugin-namespaced format: `${plugin-name}:${permission-name}`.
- All capabilities in the directory are auto-enabled by default.
- Can be platform-specific with `platforms` array.
- Rust code has full system access. Frontend code only has access through declared capabilities.

**For Code Atlas, we need capabilities for:**
- `dialog:allow-open` -- file/directory picker
- `fs:allow-read-text-file` and broader fs read access (for scanning)
- `shell:allow-execute` -- for `code` CLI integration (MVP)
- Core default capabilities

### 2.5 Plugins

**What we know:**
- **Dialog plugin** (`@tauri-apps/plugin-dialog`): Native file/directory open dialog.
  - Rust: `app.dialog().file().blocking_pick_file()` or async variant
  - TypeScript: `import { open } from '@tauri-apps/plugin-dialog'; const path = await open({ directory: true });`
  - Returns filesystem paths on desktop platforms
- **Shell plugin** (`@tauri-apps/plugin-shell`): Execute child processes. For `code -g file:line:col`.
- **Opener plugin** (`@tauri-apps/plugin-opener`): Open files/URLs with system default handler.
- **File System plugin** (`@tauri-apps/plugin-fs`): Read/write files.
- **Updater plugin** (`@tauri-apps/plugin-updater`): Auto-update mechanism.

### 2.6 Distribution

**What we know:**
- macOS distribution options: DMG installer or App Store.
- Code signing requires Apple Developer ID certificate (~$99/year).
- Notarization is required for apps distributed outside App Store.
- Tauri handles notarization automatically during build if credentials provided via environment variables.
- Auto-update via `tauri-plugin-updater` v2.10.0:
  - Uses public-private key pair for update signature verification (cannot be disabled)
  - Checks a `latest.json` endpoint for updates
  - Can use static JSON (S3, GitHub Releases) or dynamic server
  - Supports Windows, Linux, macOS
- GitHub Releases is the simplest hosting for downloads.
- CI/CD via `tauri-apps/tauri-action@v0` GitHub Action.

---

## 3. Frontend Visualization

### 3.1 React Flow v12 (@xyflow/react)

**What we know:**
- React Flow v12 is the current version. MIT licensed (free tier covers our needs).
- **Sub flows / nested nodes:** Supported via `parentId` property on nodes.
  - Child nodes position relative to parent (position `{x:0, y:0}` = parent's top-left).
  - Parent nodes MUST appear before children in the nodes array.
  - `extent: 'parent'` prevents children from leaving parent bounds.
  - Any node type can be a parent. `type: 'group'` is a convenience type with no handles.
  - Children move with parent when dragged.

- **Custom nodes:** Just React components. Use `Handle` component for connection points. Register via `nodeTypes` prop.

- **Expand/collapse:** NOT built into React Flow core. Must be implemented:
  - Use `hidden` property on nodes to toggle visibility.
  - Maintain expanded/collapsed state separately.
  - On collapse: hide children, remove child edges from visible set, add bundled edges.
  - On expand: show children, restore child edges, remove bundled edges.
  - Re-run ELK layout after toggle.

- **Edge types:** Built-in: bezier (default), straight, step, smoothstep. Custom edges supported.
  - Styling: `style` prop with `stroke`, `strokeWidth`, `strokeDasharray` (e.g., `'5,5'` for dashed).
  - Markers (arrowheads) via `markerEnd`/`markerStart`.
  - Color via `stroke` in style object.
  - Custom edge components with `BaseEdge` helper.

- **MiniMap:** Built-in `<MiniMap />` component.
  - Configurable: `zoomable`, `pannable`, `nodeColor`, `nodeStrokeColor` props.
  - Position prop: 'top-left', 'bottom-right', etc.
  - Custom node rendering via `nodeComponent` (SVG only).

- **Performance considerations:**
  - **Memoize everything:** Custom node/edge components with `React.memo`, callbacks with `useCallback`, arrays with `useMemo`.
  - **Avoid direct node/edge access:** Don't derive state from the nodes array in components. Store derived data separately.
  - **`onlyRenderVisibleElements` prop:** Only render nodes in viewport. Critical for performance.
  - **Collapse large trees:** Toggle `hidden` property rather than rendering all nodes.
  - **Simplify styling:** Shadows, gradients, animations hurt performance at scale.
  - **State management:** React Flow recommends Zustand (they use it internally). Avoid useState/useReducer for diagram state.

**Risks & gotchas:**
- Nested flows (ReactFlow inside ReactFlow) are NOT supported due to transform conflicts. Our use case is fine -- we use sub-flows, not nested React Flow instances.
- DOM-measured node sizing: React Flow needs to render nodes to get their dimensions before layout can compute positions. This creates a render-measure-layout-reposition cycle.
- Edge z-index: edges connected to nested nodes render above regular nodes. May need z-index management.
- React Flow v12 improved child node behavior during resize (children don't move when group is resized).

**Recommendations:**
- Use `parentId` for hierarchy. Model workspace > package > module > file as nested nodes.
- Implement expand/collapse via `hidden` property + state management.
- Use `onlyRenderVisibleElements` for performance.
- Use Zustand for React Flow state (aligns with their internal architecture).
- Create custom node types: `WorkspaceNode`, `PackageNode`, `ModuleNode`, `FileNode`.
- Create custom edge component that supports dashed (suppressed), colored (by category), and confidence-styled edges.

### 3.2 ELK.js

**What we know:**
- JavaScript port of the Eclipse Layout Kernel. Most configurable open-source layout engine for compound graphs.
- **npm:** `elkjs` (latest: 0.9.x).
- **Web Worker support:** Built-in. `elk-worker.js` contains the layout engine and runs in a Web Worker.
  ```javascript
  import ELK from 'elkjs/lib/elk.bundled.js'; // main thread
  // OR
  import ELK from 'elkjs'; // auto-detects Web Worker support
  ```
- **Compound graph support:** ELK natively supports hierarchical nodes via `children` arrays in the graph structure. This is the key advantage over dagre.
- **Key layout options for our use case:**
  - `elk.algorithm: 'layered'` -- hierarchical/layered layout (best for dependency graphs)
  - `elk.direction: 'DOWN'` or `'RIGHT'` -- layout direction
  - `elk.hierarchyHandling: 'INCLUDE_CHILDREN'` -- lay out parent and descendants together
  - `elk.spacing.nodeNode: '80'` -- spacing between sibling nodes
  - `elk.layered.spacing.nodeNodeBetweenLayers: '100'` -- spacing between layers
  - `elk.padding: '[top=20,left=20,bottom=20,right=20]'` -- padding inside compound nodes
  - `elk.edgeRouting: 'ORTHOGONAL'` or `'POLYLINE'` or `'SPLINES'`
  - `elk.nodeSize.constraints: 'NODE_LABELS'` -- size nodes to fit labels
  - `elk.portConstraints: 'FIXED_SIDE'` -- control port/handle placement

- **ELK graph format:**
  ```javascript
  {
    id: 'root',
    layoutOptions: { 'elk.algorithm': 'layered', 'elk.direction': 'DOWN' },
    children: [
      {
        id: 'pkg-a',
        width: 300, height: 200, // or let ELK compute
        layoutOptions: { /* per-node overrides */ },
        children: [
          { id: 'file-1', width: 150, height: 50 },
          { id: 'file-2', width: 150, height: 50 },
        ],
        edges: [/* internal edges */],
      },
    ],
    edges: [
      { id: 'e1', sources: ['file-1'], targets: ['file-2'] },
    ],
  }
  ```

- **Performance:** ELK layout with 200 nodes should be <500ms. For compound graphs with hierarchy, the computation is more expensive than flat graphs. Web Worker ensures main thread stays responsive.

**Risks & gotchas:**
- DOM-measured sizing: React Flow renders nodes first (to get DOM dimensions), then we pass those dimensions to ELK, then apply ELK's computed positions back to React Flow. This creates a visible relayout flash.
  - Mitigation: Start with estimated sizes, then refine. Or use opacity/visibility animation during layout.
- `INCLUDE_CHILDREN` hierarchy handling can be slow for deeply nested large graphs.
- Edge routing for cross-hierarchy edges (edges that leave a compound node and enter another) can produce suboptimal results. May need manual edge bundling for collapsed packages.
- The ELK option namespace is verbose (`org.eclipse.elk.layered.spacing...`). Can use short forms (just `spacing.nodeNode`).

**Recommendations:**
- Use Web Worker for all ELK computation. Never run on main thread.
- Start with `elk.algorithm: 'layered'`, `elk.direction: 'DOWN'`.
- Use `INCLUDE_CHILDREN` for hierarchy handling within expanded packages.
- Implement a render-measure-layout cycle: render with hidden nodes, measure DOM, run ELK, apply positions.
- Debounce layout requests (300ms) during rapid expand/collapse actions.
- Cache layout results per expand/collapse state.

### 3.3 Web Workers in Vite

**What we know:**
- Vite has first-class Web Worker support via two methods:
  1. **Query string import (simpler):**
     ```typescript
     import ElkWorker from './elk.worker?worker';
     const worker = new ElkWorker();
     ```
  2. **URL constructor (recommended, closer to standards):**
     ```typescript
     const worker = new Worker(new URL('./elk.worker.ts', import.meta.url), { type: 'module' });
     ```
- Workers can use ES module imports in both dev and production.
- Configure worker output format in `vite.config.ts`:
  ```typescript
  export default defineConfig({
    worker: { format: 'es' },
  });
  ```

**Recommendations:**
- Use the `?worker` import syntax for simplicity.
- Create an `elk.worker.ts` file that imports `elkjs/lib/elk.bundled.js`, receives graph data via `postMessage`, runs layout, returns positions.
- Wrap in a promise-based API for clean async usage.

### 3.4 Command Palette (cmdk)

**What we know:**
- `cmdk` (by Paco Coursey): Headless React component. Used by Linear, Raycast. Composable API, no default CSS, fuzzy search built-in via `command-score`.
- Also `react-cmdk`: Pre-styled, but less flexible. Last published 3+ years ago.
- shadcn/ui has a `Command` component built on `cmdk` with Tailwind styling.

**Recommendations:**
- Use `cmdk` for Cmd+K search. It's well-maintained, headless (we control styling with Tailwind), and has built-in fuzzy matching.
- Consider using the shadcn/ui Command component as a starting point for styling if it fits the design.

---

## 4. Frontend State Management

### 4.1 Comparison

**Zustand:**
- Single store model (like Redux but simpler). No provider required.
- Selector-based subscriptions prevent unnecessary re-renders.
- React Flow's internal state management uses Zustand. They recommend it.
- Redux DevTools middleware available.
- ~2KB gzipped.
- Best for: global app state, interconnected state that changes as a unit.

**Jotai:**
- Atomic model (like Recoil). Each piece of state is an "atom."
- Surgical re-renders -- only components using a specific atom re-render.
- Derived atoms compose atomically.
- Best for: many independent pieces of state, complex interdependencies.
- Debugging is harder with large atom networks.

**useReducer:**
- Built-in React. No additional dependency.
- Works for local component state. Not suitable for cross-component state.
- No selector optimization -- every consumer re-renders on any dispatch.

### 4.2 Recommendation

**Use Zustand** for this project. Reasons:
1. React Flow recommends and uses Zustand internally. Fighting their architecture creates friction.
2. The graph state (nodes, edges, expanded/collapsed, selection, viewport) is interconnected and changes as a unit during operations like expand/collapse or scan updates.
3. Selector-based subscriptions are sufficient for our re-render optimization needs.
4. Single store is simpler to reason about for streaming updates from the Rust backend.
5. DevTools support helps debugging.

**Store shape:**
```typescript
interface AppStore {
  // Graph data
  nodes: Node[];
  edges: Edge[];
  expandedNodeIds: Set<string>;

  // Scan state
  scanStatus: 'idle' | 'scanning' | 'complete' | 'error';
  compatibilityReport: CompatibilityReport | null;
  graphHealth: GraphHealth | null;
  activeProfile: GraphProfile | null;

  // UI state
  selectedNodeId: string | null;
  searchQuery: string;
  detailPanelOpen: boolean;

  // Actions
  onScanEvent: (event: ScanEvent) => void;
  toggleExpand: (nodeId: string) => void;
  selectNode: (nodeId: string | null) => void;
  // ...
}
```

### 4.3 React 19 Relevant Features

- **React Compiler:** Auto-optimizes memoization. Reduces need for manual `useMemo`/`useCallback`. Note: may conflict with React Flow's memoization patterns -- test carefully.
- **Concurrent rendering:** Prevents long renders from blocking UI. Useful during large graph updates.
- **Activity component** (React 19.2): For pre-rendering hidden views (e.g., pre-render detail panel). Evaluate if beneficial.
- **useOptimistic:** Could be useful for optimistic expand/collapse animations before layout completes.
- Server Components are NOT relevant (Tauri is a desktop app, no server rendering).

---

## 5. TypeScript/JavaScript Module Resolution

### 5.1 Resolution Modes

- **`moduleResolution: 'nodenext'`:** Enforces Node.js ESM rules. No extensionless imports in ESM context. `require()` of ESM allowed (Node v22.12+). Includes `node` condition by default.
- **`moduleResolution: 'bundler'`:** Allows extensionless imports. Does NOT include `node` condition by default. Most common in modern frontend projects.
- **`moduleResolution: 'node16'`:** Like nodenext but `require()` of ESM is an error.
- **`moduleResolution: 'node'`:** Legacy. CommonJS resolution. Still common in older projects.

### 5.2 package.json exports/imports

- `exports` field controls what is accessible to external consumers. Condition names (`import`, `require`, `node`, `browser`, `types`, `default`) determine resolution.
- Key order in `exports` objects is load-bearing -- first match wins.
- `imports` field (with `#` prefix) provides internal aliases.
- `type: "module"` in package.json determines default module format for .js files.

### 5.3 Workspace Detection

**pnpm:** Look for `pnpm-workspace.yaml` at root. Contains `packages:` array with glob patterns.
  ```yaml
  packages:
    - 'packages/*'
    - 'apps/*'
  ```
  Note: pnpm does NOT use package.json `workspaces` field.

**npm/Yarn classic:** Look for `workspaces` field in root `package.json`:
  ```json
  { "workspaces": ["packages/*", "apps/*"] }
  ```

**Yarn Berry/PnP:** Look for `.yarnrc.yml` with `nodeLinker: pnp`. Creates `.pnp.cjs` instead of `node_modules`.

### 5.4 tsconfig.json Parsing

Key fields: `compilerOptions.paths`, `compilerOptions.baseUrl`, `compilerOptions.module`, `compilerOptions.moduleResolution`, `extends`, `references`, `include`, `exclude`.

**Recommendations for POC:**
- Parse tsconfig.json ourselves for profile detection (module, moduleResolution).
- Use basic path-based resolution for imports (relative paths + tsconfig paths).
- Defer full exports/imports condition resolution to MVP (oxc_resolver handles this).

---

## 6. Rust Module System (Parsing Perspective)

### 6.1 Key tree-sitter Nodes for Rust

- **`mod_item`:** `mod foo;` (external) or `mod foo { ... }` (inline). Check for child `declaration_list` to distinguish.
- **`use_declaration`:** `use crate::foo::Bar;`. Children: `scoped_identifier`, `use_list`, `use_wildcard`, `use_as_clause`.
- **`visibility_modifier`:** `pub`, `pub(crate)`, `pub(super)`, `pub(in path)`.
- **`extern_crate_declaration`:** `extern crate foo;`
- **`source_file`:** Root node containing all top-level items.

### 6.2 Module Resolution Logic

Rust module resolution is deterministic from manifest + source structure:
1. `mod foo;` in `src/lib.rs` looks for `src/foo.rs` or `src/foo/mod.rs`.
2. `use crate::foo::bar` traverses the module tree from the crate root.
3. `use super::foo` goes up one module level.
4. `use self::foo` stays in current module.
5. `use external_crate::foo` -- resolved via cargo_metadata dependency graph.

### 6.3 cargo_metadata Mapping

- Each `Package` with a `lib` target = a crate node in the graph.
- `targets` with `kind: ["bin"]` = binary entry points.
- `resolve.nodes[].deps` provides the inter-crate dependency edges with `dep_kinds` for Normal/Dev/Build classification.
- `features` map shows available feature flags and their dependencies.

### 6.4 Limitations

- `#[cfg(...)]` gates require evaluation to know which code is active. cargo_metadata provides default features; beyond that requires build script execution.
- `build.rs` can generate Rust code (into OUT_DIR), define custom cfg flags, and create links. None of this is visible to static analysis.
- `proc_macro` expansion produces code that doesn't exist in source. Cannot be analyzed statically.
- `include!()` macro includes file contents. Can be detected syntactically but the included file needs separate analysis.

---

## 7. Build, Test & Quality

### 7.1 Vitest

**What we know:**
- Blazing-fast test framework built on Vite. Native ES module support, Jest-compatible API.
- Setup: `vitest` in `devDependencies`, `vitest.config.ts` or configure in `vite.config.ts`.
- React component testing: use `@testing-library/react` alongside Vitest.
- **Mocking Tauri IPC:**
  ```typescript
  import { mockIPC, clearMocks } from '@tauri-apps/api/mocks';

  beforeEach(() => {
    mockIPC((cmd, args) => {
      if (cmd === 'scan_workspace') {
        return { /* mock result */ };
      }
    });
  });

  afterEach(() => clearMocks());
  ```
- Events can be mocked with `{ shouldMockEvents: true }` option.
- Channel mocking requires capturing the event callback ID from the args.

**Recommendations:**
- Use Vitest with jsdom for frontend unit tests.
- Mock all Tauri IPC calls in frontend tests.
- Test React Flow interactions with testing-library (click events, state changes).
- Focus frontend tests on: state management logic, data transformation, component behavior. Not visual rendering.

### 7.2 cargo test + proptest

- **proptest** v1.9.0: Property-based testing. Hypothesis-inspired. MSRV 1.84. Maintenance mode (feature-complete).
- Use proptest for graph operations: "for any set of nodes and edges, SCC detection should be idempotent", "adding then removing a node preserves graph invariants", etc.
- **criterion** v0.8.1: Statistics-driven benchmarking. De facto standard.
  - `[dev-dependencies] criterion = { version = "0.8", features = ["html_reports"] }`
  - Benchmark scan performance, graph construction, query speed.

### 7.3 Code Coverage

- **Rust:** Use `cargo-llvm-cov` (LLVM source-based coverage). Cross-platform, accurate. Output: HTML, JSON, text.
  - `cargo install cargo-llvm-cov`
  - `cargo llvm-cov --html`
  - Works on macOS (unlike tarpaulin which is Linux-focused).
- **TypeScript:** Vitest has built-in coverage via `@vitest/coverage-v8`.
  - Configure in `vitest.config.ts`: `coverage: { provider: 'v8', reporter: ['text', 'html'], thresholds: { lines: 80 } }`

### 7.4 CI/CD

**GitHub Actions workflow for Tauri v2:**
1. Trigger: push to release branch or workflow_dispatch.
2. Matrix: macOS arm64 (`aarch64-apple-darwin`), macOS Intel (`x86_64-apple-darwin`), Linux x64, Windows x64.
3. Steps:
   - `actions/checkout@v4`
   - Install system dependencies (Linux only)
   - `actions/setup-node@v4` with pnpm cache
   - `dtolnay/rust-toolchain@stable` + `swatinem/rust-cache@v2`
   - `pnpm install` + frontend build
   - `tauri-apps/tauri-action@v0` for building + optional release
4. Environment: `GITHUB_TOKEN` with read/write permissions.
5. Code signing: platform-specific env vars for certificates.

**Recommendations:**
- Set up CI early (M1 milestone) with: `cargo test`, `cargo clippy`, `pnpm test`, `pnpm typecheck`, `pnpm lint`.
- Add coverage reporting to CI.
- Defer release/distribution workflow to pre-MVP.

---

## 8. Distribution & Deployment

### 8.1 macOS Distribution

- **DMG + direct download** is the simplest path for initial distribution.
- Code signing requires Apple Developer ID ($99/year).
- Notarization: Tauri handles it automatically. Provide credentials via env vars (App Store Connect API key or Apple ID).
- Without signing/notarization: users must right-click > Open and bypass Gatekeeper. Acceptable for POC, not for MVP.

### 8.2 Auto-Update

- `tauri-plugin-updater` v2.10.0.
- Requires generating a signing keypair (separate from Apple code signing).
- Update flow: app checks `latest.json` endpoint -> downloads update -> verifies signature -> installs -> restarts.
- Host `latest.json` + artifacts on GitHub Releases.

### 8.3 Landing Page

- Not in POC scope. For MVP: a simple static site (e.g., on Cloudflare Pages) with download links, feature overview, screenshots.

**Recommendations:**
- POC: no distribution. Run from source or `tauri dev`.
- Pre-MVP: set up Apple Developer account, generate signing keys.
- MVP: GitHub Releases for distribution, tauri-plugin-updater for auto-update.

---

## 9. UX/UI Patterns for Graph Visualization

### 9.1 Zoom Level Best Practices

- **Shneiderman's mantra:** "Overview first, zoom and filter, then details-on-demand."
- Display zoom indicator. Encourage zooming for label clarity.
- At low zoom: show only package labels and aggregated edge counts.
- At medium zoom: show module/folder labels.
- At high zoom: show file labels and individual edges.
- Use `transform` CSS for smooth zoom (React Flow handles this).

### 9.2 Progressive Disclosure

- Hierarchical zoom IS progressive disclosure. Default view shows high-level topology.
- Detail panel slides in on node click -- don't show all details at once.
- Compatibility report: show summary first, expand for details.
- Graph health: show headline metrics, click for detailed lists.

### 9.3 Dark Theme with Tailwind CSS v4

- Tailwind v4 is CSS-first. No `tailwind.config.js` for dark mode.
- Default: uses OS preference via `prefers-color-scheme` automatically.
- Class-based toggle: `@custom-variant dark (&:where(.dark, .dark *));` in global CSS.
- Use CSS custom properties (`@theme` directive) for graph colors that adapt to theme.
- React Flow supports theming via CSS variables.

### 9.4 Detail Panel

- Right-side, collapsible (~300px). Slides in on node click, collapses on deselect.
- Tab navigation: Overview | Dependencies | Exports | Health.
- Use `ResizablePanel` pattern for user-adjustable width.

### 9.5 Loading/Progress States

- Progressive rendering means the user sees partial results immediately.
- Show a progress indicator during scan: "Scanning: 120/450 files..."
- Use skeleton states for the detail panel before data arrives.
- Cancel button visible during scan.

### 9.6 Error States

- Partial results: render what we have + health indicators showing gaps.
- Permission errors: clear message + instructions.
- Empty workspace: "No workspace detected. Code Atlas supports Cargo workspaces and JS/TS workspaces."

### 9.7 Accessibility

- WCAG AA contrast minimum (aim for AAA).
- Keyboard navigation for the graph (React Flow supports keyboard shortcuts).
- ARIA labels on graph controls and interactive elements.
- Colorblind-safe palette for edge categories. Use dual encoding (color + dash pattern).
- Support screen readers for the detail panel (semantic HTML).
- The graph canvas itself is inherently visual, but all information should also be accessible via the detail panel and search.

---

## 10. Architecture Patterns

### 10.1 Cargo Workspace Organization

**Recommended layout:**
```
tauri-poc-zoom-thing/
  Cargo.toml          # virtual manifest (workspace root)
  crates/
    codeatlas-core/   # standalone analysis library
      Cargo.toml
      src/
        lib.rs
        workspace/    # workspace discovery
        detector/     # detector trait + implementations
        graph/        # petgraph wrapper, identity, overlay
        config/       # .codeatlas.yaml parser
        profile/      # graph profile management
        health/       # compatibility report, graph health
    codeatlas-tauri/   # Tauri app (thin shell)
      Cargo.toml      # depends on codeatlas-core
      src/
        lib.rs        # Tauri commands
        main.rs       # Tauri entry point
      capabilities/
      tauri.conf.json
  src/                # React frontend
  package.json
  pnpm-workspace.yaml (if needed)
```

**Key constraint:** `codeatlas-core` must have ZERO dependency on `tauri`, `serde_json`, or any IPC/transport crate. It uses domain types with serde derives. The Tauri shell converts between domain types and IPC types if needed.

**Best practices from matklad (rust-analyzer author):**
- Use a virtual manifest at workspace root (no `src/` at root).
- Flat crate layout under `crates/` directory.
- Name crate directories the same as the crate name.
- Share a single `Cargo.lock` across the workspace.

### 10.2 Detector Architecture

**Pattern: Strategy + Registry**

```rust
// In codeatlas-core
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

pub struct DetectorRegistry {
    detectors: Vec<Box<dyn Detector>>,
}

impl DetectorRegistry {
    pub fn new() -> Self {
        Self {
            detectors: vec![
                Box::new(RustCargoDetector),
                Box::new(TypeScriptImportDetector),
            ],
        }
    }

    pub fn applicable(&self, workspace: &WorkspaceInfo) -> Vec<&dyn Detector> {
        self.detectors.iter()
            .filter(|d| d.applies_to(workspace))
            .map(|d| d.as_ref())
            .collect()
    }
}
```

### 10.3 Two-Layer Graph Model

```rust
pub struct ArchGraph {
    /// Immutable layer: what the scanner actually found
    discovered: StableGraph<NodeData, EdgeData, Directed>,

    /// Overlay layer: user config additions/suppressions
    overlay: GraphOverlay,

    /// Index for fast lookups
    node_index: HashMap<MaterializedKey, NodeIndex>,
}

pub struct GraphOverlay {
    /// Manual edges added via .codeatlas.yaml
    manual_edges: Vec<ManualEdge>,

    /// Edges suppressed in default view
    suppressions: HashMap<EdgeIndex, SuppressionReason>,

    /// Metadata annotations
    metadata: HashMap<NodeIndex, NodeMetadata>,
}

impl ArchGraph {
    /// Query the merged view (discovered + overlay)
    pub fn merged_edges(&self, node: NodeIndex) -> impl Iterator<Item = MergedEdge> {
        // Combine discovered edges (minus suppressed) + manual edges
    }

    /// Query only the discovered layer
    pub fn discovered_edges(&self, node: NodeIndex) -> impl Iterator<Item = &EdgeData> {
        // Only discovered edges, including suppressed ones
    }
}
```

### 10.4 Streaming Architecture

```
[Detector Registry]
    |
    | (mpsc channel per detector)
    v
[Graph Builder] -- merges detector results into StableGraph
    |
    | (output channel with phases)
    v
[Phase Splitter]
    |-- Phase 1: PackageTopology (workspace/package nodes + inter-package edges)
    |-- Phase 2: ModuleStructure (module/folder nodes)
    |-- Phase 3: FileEdges (file nodes + import edges)
    v
[Tauri Channel<T>] --> frontend onmessage handler
```

### 10.5 Identity Scheme Implementation

```rust
/// Current location in this repo/profile/snapshot
#[derive(Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct MaterializedKey {
    pub language: Language,
    pub entity_kind: EntityKind,
    pub relative_path: String,
}

impl MaterializedKey {
    pub fn format(&self) -> String {
        format!("{}:{}:{}", self.language, self.entity_kind, self.relative_path)
    }
}

#[derive(Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct LineageKey(Uuid);

pub struct NodeData {
    pub materialized_key: MaterializedKey,
    pub lineage_key: Option<LineageKey>, // Populated in MVP
    pub label: String,
    pub kind: NodeKind,
    pub parent: Option<NodeIndex>,
    // ...
}
```

---

## 11. Competitive Deep-Dive

### 11.1 CodeViz (YC S24)

- VS Code extension, ~80K installs, v1.6.9 (Dec 2025).
- Sends code to Anthropic/GCP/AWS for LLM-powered analysis.
- **HN feedback (August 2024):** Privacy was the #1 concern. Multiple users stated it was a "deal-breaker" for enterprise. "I'd love to try this against my spaghetti, but I can't send my company's IP to you or anthropic." One user reported a "major incident" from an extension uploading code.
- Pricing: Free (5-10 diagrams/month) -> Pro ($19/mo) -> Teams ($50/seat/mo) -> Enterprise (custom, on-prem).
- Strengths: AI-generated summaries, natural language search, C4/UML models, embeddable diagrams. Good for quick understanding.
- Weaknesses: Cloud dependency, privacy concerns, subscription cost, limited free tier.

### 11.2 GitKraken Codemaps

- Born from CodeSee acquisition (May 2024).
- **Still in early access as of March 2026.** Not GA. The features page says "currently in development."
- CodeSee technology repurposed primarily for GitKraken Automations (workflow automation), not visualization.
- Effectively vaporware until proven otherwise.

### 11.3 dependency-cruiser

- Most robust JS/TS dependency analysis tool.
- Supports custom rules, CI integration, circular dependency detection.
- Produces DOT/SVG/HTML output.
- Limitation: flat graph, static output, no interactive exploration, no hierarchy.

### 11.4 Madge

- Simple JS/TS dependency graphing.
- Easy to set up, generates visual graphs.
- Limitation: graphs become unwieldy for larger projects, no hierarchy, no interactivity.

### 11.5 Swark

- LLM-based architecture visualization. Uses AI to generate Mermaid diagrams.
- "All the logic is encapsulated within the LLM" -- natively supports all languages.
- Limitation: non-deterministic, no provenance, no trust model, generated diagrams are approximations.

### 11.6 SciTools Understand

- 20+ years of deep static analysis engineering. Cross-language.
- Comparison graphs, call hierarchy, control flow.
- Strength: deepest analysis, most mature.
- Weakness: legacy UX, expensive, not modern developer experience.

### 11.7 Nx Graph

- Deep monorepo integration within Nx ecosystem.
- Project graph with import-level granularity, affected detection, plugin-extensible.
- **MCP lesson:** Nx defaults MCP server to minimal mode because "workspace analysis tools are now handled more efficiently by agent skills, which provide domain knowledge as incrementally-loaded instructions rather than tool-call-based data dumps." This validates the PRD's decision to keep the MCP surface thin.
- Limitation: ecosystem-locked (requires Nx).

### 11.8 Competitive Gap Summary

No existing tool provides: (1) upfront compatibility report, (2) build-context-aware profiled graphs, (3) edge provenance with semantic categories, (4) discovered/overlay graph separation, (5) local-first with zero network calls, (6) hierarchical zoom with compound nodes, (7) public surface lens. This combination is the competitive wedge.

---

## 12. Risk Analysis & Unknown-Unknowns

### 12.1 Hardest Technical Challenges

1. **DOM-measured sizing + ELK layout cycle:** React Flow needs DOM dimensions before ELK can compute positions. This creates a render-measure-layout-reposition cycle that can produce visual flicker.
   - Mitigation: Use estimated sizes for initial render, then refine. Animate the reposition.

2. **Cross-hierarchy edge routing with ELK:** When edges cross compound node boundaries (file in package A imports file in package B), ELK must route the edge through the hierarchy. This can produce ugly results.
   - Mitigation: When packages are collapsed, bundle all internal edges into a single inter-package edge.

3. **Expand/collapse state management:** Toggling a package requires: hiding/showing children, adding/removing bundled edges, re-running ELK layout, preserving viewport position. Many edge cases.
   - Mitigation: Comprehensive unit tests. proptest for state invariants.

4. **Progressive rendering UX:** Streaming graph data in phases means the layout changes as new data arrives. This can be jarring.
   - Mitigation: Phase 1 (package topology) should be visually stable. Phase 2-3 add detail within already-positioned packages.

5. **TypeScript import resolution accuracy (POC):** tree-sitter can parse imports but cannot resolve them. Path-based resolution is approximate.
   - Mitigation: Surface unresolved imports via graph health. Upgrade to oxc_resolver in MVP.

### 12.2 Performance Cliffs

| Concern | Threshold | Impact | Mitigation |
|---------|-----------|--------|------------|
| React Flow DOM nodes | >500 visible | Janky scrolling/zoom | `onlyRenderVisibleElements`, collapse packages |
| ELK layout computation | >500 nodes | >1 second layout | Web Worker, only layout visible portion |
| tree-sitter initial parse | >5000 files | >10 second scan | Parallelize across cores |
| Graph serialization over IPC | >1MB JSON | Slow initial render | Stream in phases via Channel |
| Browser memory | >200MB | Tab crashes | Only render expanded packages |

### 12.3 macOS-Specific Concerns

- Tauri uses WKWebView on macOS. Performance is generally good but less predictable than Chromium.
- WKWebView has different CSS rendering quirks. Test early.
- File permission dialogs are macOS-managed. The dialog plugin handles this correctly.
- `cargo metadata` subprocess invocation works fine on macOS.
- Code signing for development: can use ad-hoc signing during development.

### 12.4 Memory Management

- **Rust side:** tree-sitter Trees can be large. Drop them after extracting needed data. Don't keep all parsed trees in memory.
- **Frontend side:** React Flow keeps all nodes in memory. With `onlyRenderVisibleElements`, DOM nodes are created/destroyed on scroll, but data remains in memory.
- For a 2000-file repo: expect ~50MB Rust-side (graph + metadata), ~50MB frontend (React Flow state). Well within 200MB target.
- Release memory from previous scans before starting new ones.

### 12.5 Common Pitfalls in Code Analysis Tools

1. **False precision:** Claiming edges are definitive when they're syntactic approximations. Our edge provenance model addresses this.
2. **Ignoring build context:** Showing a single graph when the real graph depends on configuration. Our profile system addresses this.
3. **Missing the "why":** Showing what's connected without explaining why. Our edge evidence model addresses this.
4. **Stale output:** Static diagrams go stale immediately. Our watch mode (MVP) addresses this.
5. **All-or-nothing:** Either showing everything or nothing. Our progressive disclosure and health badges address this.

---

## 13. Feature Feasibility Assessment

### 13.1 Technically Infeasible or Extremely Risky

None of the POC features are infeasible. The riskiest features are:
- **Progressive rendering with good UX:** Feasible but requires careful design of the render-measure-layout cycle. Risk level: medium.
- **Accurate import resolution (POC):** tree-sitter alone cannot resolve imports. Basic path-based resolution will have gaps. Risk level: medium (mitigated by health indicators).

### 13.2 Features That Could Be Simplified

- **Edge bundling when packages collapse (F29, P2):** Can defer to MVP. Show a simpler aggregate edge count instead.
- **Demo graph fixture (F30, P2):** Can be a static JSON file. No need for a complex demo mode.
- **Non-code file filter (F32, P2):** Simply exclude them. The toggle is not critical for POC.

### 13.3 High-Value Features Potentially Missing

- **Keyboard-first navigation:** Not emphasized in PRD but critical for developer UX. Tab through nodes, arrow keys for neighbors, Enter to expand.
- **URL-like addressing:** Deep links to specific nodes/views (useful for sharing even before export).
- **Undo/redo for graph navigation:** Back button to return to previous expand/collapse state.

### 13.4 Compatibility Report Feasibility

Fully feasible. The detector trait includes `compatibility()` method. Each detector reports what it can/cannot handle. The report is assembled from detector assessments + workspace discovery results.

Prior art: ESLint's config inspector, TypeScript's `--showConfig`, Nx's `nx report` -- all show tool capabilities vs project requirements.

---

## 14. SQLite Integration (MVP)

### 14.1 Crate Choice: rusqlite

**Recommendation: rusqlite** (not sqlx) for this project. Reasons:
1. **Synchronous API:** Graph operations are CPU-bound, not I/O-bound. Async overhead is unnecessary.
2. **`bundled` feature flag:** Compiles SQLite directly into the binary. No system dependency. Deployment is seamless.
3. **Simpler:** No compile-time query checking (unnecessary for our schema), no async runtime complexity.
4. **Lighter weight:** Smaller dependency tree.
5. **rusqlite_migration** v2.4.1: Simple schema migration library. Uses `user_version` (integer at fixed offset in SQLite file) instead of migration tables. Migrations defined as SQL strings in Rust code.

**Do NOT use sqlx:** Using both sqlx and rusqlite creates a semver hazard (both depend on `libsqlite3-sys`). Since Tauri or other dependencies may use rusqlite, stick with rusqlite only.

### 14.2 Schema Design

```sql
-- Snapshots
CREATE TABLE snapshots (
    id TEXT PRIMARY KEY,  -- UUID
    workspace_root TEXT NOT NULL,
    profile_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,  -- ISO 8601
    node_count INTEGER NOT NULL,
    edge_count INTEGER NOT NULL
);

-- Nodes
CREATE TABLE nodes (
    id INTEGER PRIMARY KEY,
    snapshot_id TEXT NOT NULL REFERENCES snapshots(id),
    materialized_key TEXT NOT NULL,
    lineage_key TEXT,  -- UUID, nullable until lineage tracking is active
    label TEXT NOT NULL,
    kind TEXT NOT NULL,  -- 'workspace' | 'package' | 'module' | 'file'
    language TEXT,
    parent_key TEXT,  -- materialized key of parent
    metadata TEXT,  -- JSON blob for extensible metadata
    UNIQUE(snapshot_id, materialized_key)
);

-- Edges
CREATE TABLE edges (
    id INTEGER PRIMARY KEY,
    snapshot_id TEXT NOT NULL REFERENCES snapshots(id),
    source_key TEXT NOT NULL,
    target_key TEXT NOT NULL,
    kind TEXT NOT NULL,  -- 'imports' | 're_exports' | 'contains' | 'depends_on' | 'manual'
    category TEXT NOT NULL,  -- 'value' | 'type_only' | 'dev' | 'build' | 'test' | 'normal' | 'manual'
    confidence TEXT NOT NULL,  -- 'structural' | 'syntactic' | 'resolver_aware'
    source_location TEXT,  -- JSON: {file, line, col}
    resolution_method TEXT,
    overlay_status TEXT  -- NULL or suppression reason
);

-- Lineage tracking (MVP)
CREATE TABLE lineage (
    lineage_key TEXT PRIMARY KEY,  -- UUID
    first_seen_snapshot TEXT NOT NULL,
    current_materialized_key TEXT,
    previous_keys TEXT  -- JSON array of previous materialized keys
);

-- Named views
CREATE TABLE views (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    snapshot_id TEXT REFERENCES snapshots(id),
    viewport TEXT NOT NULL,  -- JSON: {x, y, zoom}
    expanded_nodes TEXT NOT NULL,  -- JSON array of materialized keys
    filters TEXT  -- JSON: active filter state
);

-- Bookmarks
CREATE TABLE bookmarks (
    id TEXT PRIMARY KEY,
    lineage_key TEXT NOT NULL,
    label TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_nodes_snapshot ON nodes(snapshot_id);
CREATE INDEX idx_nodes_key ON nodes(materialized_key);
CREATE INDEX idx_edges_snapshot ON edges(snapshot_id);
CREATE INDEX idx_edges_source ON edges(source_key);
CREATE INDEX idx_edges_target ON edges(target_key);
```

### 14.3 Performance Considerations

- SQLite is fast for our scale. A 5000-node graph with 20,000 edges should serialize in <100ms.
- Use WAL mode for concurrent reads during writes.
- Batch inserts in transactions (MUCH faster than individual inserts).
- For snapshot storage: serialize the full graph once, not node-by-node queries.
- Consider storing the full graph as a single JSON blob for simple snapshots, with the relational schema for queries.

---

## Summary of Key Recommendations

### POC Technology Choices (Confirmed)

| Concern | Choice | Confidence |
|---------|--------|------------|
| Graph library | petgraph StableGraph | High |
| TS parsing (POC) | tree-sitter-typescript | High |
| Rust parsing | tree-sitter-rust + cargo_metadata | High |
| Graph rendering | React Flow v12 (@xyflow/react) | High |
| Layout engine | ELK.js in Web Worker | High |
| Frontend state | Zustand | High |
| IPC streaming | Tauri Channel<T> | High |
| Type-safe IPC | tauri-specta v2 (with manual fallback) | Medium |
| Command palette | cmdk | High |
| Styling | Tailwind CSS v4 | High |
| Testing (frontend) | Vitest + @testing-library/react | High |
| Testing (Rust) | cargo test + proptest | High |
| Benchmarking | criterion 0.8 | High |
| Coverage (Rust) | cargo-llvm-cov | High |
| Coverage (TS) | @vitest/coverage-v8 | High |

### MVP Technology Additions

| Concern | Choice | Confidence |
|---------|--------|------------|
| TS parsing (MVP) | oxc_parser | High |
| TS resolution (MVP) | oxc_resolver | High |
| File watching | notify v8 | High |
| Persistence | rusqlite + rusqlite_migration | High |
| Auto-update | tauri-plugin-updater | High |
| Editor integration | VS Code CLI + companion extension | Medium |

### Critical Path Dependencies

1. **Identity scheme MUST be designed before first scan implementation.** Every downstream feature depends on stable node identity.
2. **Detector trait MUST be defined before implementing any detector.** Retrofit is expensive.
3. **Graph overlay model MUST be in the data model from day one.** Adding later means re-testing all graph operations.
4. **ELK + React Flow integration prototype MUST happen early (M3).** This is the highest-risk frontend integration.
5. **Streaming pipeline (Channel<T>) MUST be validated early (M4).** Progressive rendering depends on it.

---

## Appendix: Version Reference

| Crate/Package | Version | Notes |
|---------------|---------|-------|
| tree-sitter | latest stable | Check crates.io |
| tree-sitter-typescript | latest stable | Includes type import support |
| tree-sitter-rust | latest stable | |
| petgraph | 0.7.x | StableGraph |
| cargo_metadata | latest stable | |
| oxc_parser | 0.118.x | Pre-1.0, pin carefully |
| oxc_resolver | latest stable | Separate repo from oxc |
| serde | 1.x | |
| serde_json | 1.x | |
| notify | 8.2.0 | |
| tokio | 1.x | |
| tauri | 2.x | |
| tauri-specta | 2.0.0-rc.x | RC, pin exact version |
| specta | 2.0.0-rc.x | RC, pin exact version |
| rusqlite | 0.38.x | With `bundled` feature |
| rusqlite_migration | 2.4.1 | |
| proptest | 1.9.0 | |
| criterion | 0.8.1 | |
| thiserror | 2.x | |
| @xyflow/react | 12.x | React Flow |
| elkjs | 0.9.x | |
| cmdk | latest | |
| zustand | 5.x | Check latest |
| @tauri-apps/api | 2.x | |
| @tauri-apps/plugin-dialog | 2.x | |
| @tauri-apps/plugin-updater | 2.10.0 | |
| vitest | latest | |
| tailwindcss | 4.x | CSS-first config |

---

## Modern Practices & Forward-Looking Research (Supplement)

**Date:** 2026-03-18
**Purpose:** Ensure Code Atlas uses cutting-edge tools and patterns where they provide real value, keeping the product relevant and high-quality for years to come.

---

### S1. Modern Rust Ecosystem (2025-2026)

#### S1.1 Error Handling: thiserror + miette

**Current best practice:** thiserror 2.x for defining error types (derives `std::error::Error`, implements `From` for `?` propagation). miette 7.x for rich diagnostic reporting with source spans, code highlights, and suggestions. These are complementary, not competing. anyhow is appropriate for application-level code (scripts, CLI, prototypes) but not for libraries where callers need to match on error variants.

**Recommendation for Code Atlas:**
- `codeatlas-core`: Use **thiserror** for all error enums. Each module (detector, graph, config, workspace) defines its own error type. This enables callers (the Tauri shell, CLI, tests) to match on variants.
- `codeatlas-tauri` (Tauri shell): Use **miette** at the boundary layer to render rich diagnostics for parse errors and unsupported constructs. miette's source-span support is ideal for showing users exactly where a parse failure or unresolvable import occurred in their code. Derive `miette::Diagnostic` on top of thiserror errors.
- Do NOT use anyhow anywhere in the core library. Reserve it for throwaway scripts or build tooling if needed.

**Why this matters for longevity:** miette's diagnostic rendering is directly aligned with the PRD's emphasis on trust and transparency. When users see a parse error or unsupported construct, showing the exact code location with a highlighted span is far more helpful than a generic error message. This approach mirrors what modern compilers (rustc, oxc) do and sets Code Atlas apart from tools that show opaque "analysis failed" messages.

#### S1.2 Async Patterns: tokio + rayon

**Current best practice:** tokio remains the dominant async runtime for Rust (Tauri v2 uses it internally). For CPU-bound parallelism, rayon is the standard choice -- it provides data-parallel iterators that distribute work across cores with zero boilerplate. The two complement each other: tokio for I/O-bound and coordination tasks, rayon for CPU-bound batch work.

**Recommendation for Code Atlas:**
- **tokio**: Continue using for IPC, file watching, channel-based streaming, and Tauri command execution. Already decided in the base research.
- **rayon**: Add for **parallel file parsing** in the scan pipeline. tree-sitter parsing is CPU-bound and embarrassingly parallel (each file is independent). Use `rayon::par_iter()` over the file list to parse files across all available cores. For a 2,000-file repo, this can reduce scan time from ~5 seconds (sequential) to ~1 second (8-core parallel).
- **Bridge pattern**: Use `tokio::sync::mpsc` or `tokio::task::spawn_blocking` to bridge between rayon's thread pool and tokio's async runtime. The standard pattern: spawn a `spawn_blocking` task that runs rayon work, then send results back through a tokio channel.

**Why this matters for longevity:** The PRD targets repos up to 5,000 files with scan times under 10 seconds. rayon makes this achievable on modern hardware without complex thread management. As repo targets grow in future phases, rayon scales naturally with available cores.

#### S1.3 Serialization: serde (with bitcode for internal snapshots)

**Current best practice:** serde remains the industry standard for Rust serialization (1.x stable, 500-1000 MB/s). For IPC and JSON, nothing competes. For internal high-performance storage (snapshots, cache), alternatives exist: **bitcode** (0.6.x) achieves top benchmarks for both speed and compression ratio. **rkyv** (0.8.x) provides zero-copy deserialization but has a steeper API and is Rust-only.

**Recommendation for Code Atlas:**
- **serde + serde_json**: Continue using for all IPC, config files (`.codeatlas.yaml`), and any human-readable serialization. Already decided.
- **bitcode** (MVP/Platform consideration): Evaluate for SQLite snapshot blob serialization. Graph snapshots that store thousands of nodes and edges could benefit from bitcode's ~30-50% smaller payloads and faster deserialization compared to JSON. This would reduce SQLite storage and speed up snapshot loading.
- Do NOT adopt rkyv. Its zero-copy model adds significant API complexity (archived types, alignment requirements) that is not justified at our scale (sub-50K nodes).

**Why this matters for longevity:** serde is not going anywhere. bitcode as an internal optimization for snapshots gives us a clean upgrade path for larger repos without changing any public APIs.

#### S1.4 CLI Framework: clap v4

**Current best practice:** clap v4.x remains the dominant CLI argument parser for Rust. Alternatives (structopt is deprecated into clap, argh by Google is minimal but less featured). clap 4.4+ added 30% faster parsing for nested subcommands and range-based value parsing.

**Recommendation for Code Atlas:**
- Use **clap v4** with derive macros for the Platform-phase CLI surface (`codeatlas scan`, `codeatlas health`, `codeatlas deps`, etc.).
- The derive API (`#[derive(Parser)]`) provides compile-time validation and auto-generated help text.
- Consider using clap's `value_parser!` for typed path arguments and feature-flag enums.

**Why this matters for longevity:** clap is the clear winner in the Rust CLI space with no credible challenger. The derive API is stable and generates excellent help output that matches modern CLI expectations (like `gh`, `cargo`, `pnpm`).

#### S1.5 Logging/Tracing: the `tracing` crate

**Current best practice:** `tracing` (by the tokio team) is the de facto standard for Rust instrumentation. It provides structured, span-based logging with levels (trace, debug, info, warn, error). The `#[instrument]` proc macro instruments functions with zero boilerplate. `tracing-subscriber` provides configurable output formatting (human-readable, JSON, compact). Integration with OpenTelemetry is available for production observability.

**Recommendation for Code Atlas:**
- Add `tracing` and `tracing-subscriber` to `codeatlas-core` from the POC.
- Use `#[instrument]` on key functions: scan entry points, detector methods, graph builder operations, ELK layout requests.
- Use structured fields (not string interpolation) for all log messages: `tracing::info!(file_count = %count, duration_ms = %elapsed, "scan complete")`.
- Configure `tracing-subscriber` with `EnvFilter` so log levels are controllable via `RUST_LOG` environment variable.
- For the Tauri shell, forward tracing output to both stderr and a log file in the app data directory.
- In Platform phase, consider adding `tracing-opentelemetry` for distributed tracing if remote scanner deployment is implemented.

**Why this matters for longevity:** tracing instruments are zero-cost when disabled and provide the diagnostic data needed for performance profiling, debugging, and eventually production monitoring. Adding it from day one means every scan, parse, and layout operation is automatically profiled.

#### S1.6 Rust 2024 Edition Features

**Current best practice:** Rust 2024 Edition shipped with Rust 1.85.0 (February 2025). Key features relevant to Code Atlas:
- **`async fn` in traits** (stable since 1.75.0): No longer need `#[async_trait]` macro for most async traits. The `Detector` trait can use native `async fn detect(...)`.
- **Async closures** (`async || {}`): Available in 2024 edition with `AsyncFn`, `AsyncFnMut`, `AsyncFnOnce` traits in the prelude.
- **`impl Trait` in trait return types**: Enables `fn edges(&self) -> impl Iterator<Item = &EdgeData>` directly in traits.
- **`let` chains in `if let`**: Cleaner pattern matching.
- **Precise capturing in `impl Trait`**: Better lifetime handling for returned iterators and futures.

**Recommendation for Code Atlas:**
- Target **Rust 2024 edition** from the start. Set `edition = "2024"` in all `Cargo.toml` files.
- Use native `async fn` in the `Detector` trait instead of the `async_trait` crate. This removes one proc-macro dependency and improves compile times.
- Use `impl Iterator` in trait methods for graph queries.
- **Caveat:** `async fn` in traits still does not support `dyn Trait` (trait objects). If the `DetectorRegistry` stores `Vec<Box<dyn Detector>>`, async methods will need the `#[async_trait]` workaround OR use a manual `Pin<Box<dyn Future>>` return type. Evaluate whether the registry can use generics instead.

**Why this matters for longevity:** Using the latest edition features from day one avoids future migration costs and aligns with the broader Rust ecosystem's direction. Native async traits reduce compile times and eliminate a common source of confusing error messages.

---

### S2. Modern Frontend Tooling

#### S2.1 Biome vs ESLint + Prettier

**Current best practice:** Biome v2.4 (2026) is a mature, Rust-based linter + formatter. 461 rules, 97% Prettier compatibility, 35x faster than Prettier for formatting, 15-50x faster than ESLint for linting. Biome 2.0 added type inference (~85% of typescript-eslint coverage without running tsc). OXC's oxlint is even faster (50-100x vs ESLint) but covers fewer rules (~300) and lacks a built-in formatter.

ESLint v9 remains the standard with 50M+ weekly downloads, 4000+ plugins, and full type-aware rules. However, it requires Prettier as a separate tool, and the two occasionally conflict.

**Recommendation for Code Atlas:**
- **Use Biome** as the sole linter + formatter for the frontend. Justification:
  1. Code Atlas is a greenfield React + TypeScript project -- Biome's rule coverage is sufficient (no framework-specific plugins needed beyond React/TS).
  2. Single tool replacing ESLint + Prettier eliminates config conflicts and dependency bloat.
  3. 35x faster formatting and 15x faster linting improves DX and CI speed.
  4. Biome's Tailwind CSS class sorting support is built-in.
  5. The project's `.claude/rules/stack.md` lists ESLint + Prettier, but since no code exists yet, switching to Biome has zero migration cost.
- Keep **clippy** for Rust linting (no change).
- Update `.claude/rules/stack.md` to reflect Biome.

**Why this matters for longevity:** Biome's trajectory is clear -- it is gaining adoption rapidly (15M+ monthly downloads in 2025) and is backed by a well-funded team. Consolidating to one tool reduces maintenance burden and aligns with the project's preference for modern, Rust-based tooling.

#### S2.2 React Compiler

**Current best practice:** React Compiler v1.0 shipped October 2025. Production-ready, battle-tested at Meta. Automatic memoization at build time. Works with React 17+. Eliminates the need for manual `useMemo`, `useCallback`, and `React.memo` in most cases. Can memoize code after conditional returns (impossible manually). Performance gains at Meta: initial loads up to 12% faster, some interactions 2.5x faster, neutral memory impact.

**Recommendation for Code Atlas:**
- **Enable React Compiler** from the start. Install `babel-plugin-react-compiler` (or the Vite equivalent).
- Monitor compatibility with React Flow. React Flow v12 uses internal memoization extensively. The React Compiler is designed to coexist with manual memoization (it respects existing `useMemo`/`useCallback`). However, test the combination early in the ELK integration prototype (M3).
- React Flow updated its UI Components to React 19 and Tailwind CSS 4 (October 2025), indicating active maintenance and modern React support.
- Continue using `React.memo` on custom node/edge components as explicit signals to the compiler, but do not add redundant `useMemo`/`useCallback` in new code.

**Why this matters for longevity:** React Compiler is the future of React performance optimization. Starting with it enabled means the codebase benefits from automatic optimization from day one and avoids accumulating manual memoization debt that would need to be cleaned up later.

#### S2.3 UI Component Library: shadcn/ui

**Current best practice:** shadcn/ui is the dominant choice for React + Tailwind projects in 2025-2026. Built on Radix UI primitives (WCAG-compliant accessibility, keyboard navigation, ARIA attributes) and styled with Tailwind CSS. Components are copied into the project (not a dependency), giving full ownership and customization. Includes pre-built `Command` component (built on `cmdk`) for the command palette.

Alternatives: Radix UI (unstyled primitives -- shadcn/ui already wraps these), Ark UI (framework-agnostic via Zag.js state machines, 45+ components, but smaller ecosystem), Headless UI (fewer components, mainly Tailwind Labs).

**Recommendation for Code Atlas:**
- Use **shadcn/ui** for all non-graph UI: dialogs (compatibility report, settings), tabs (detail panel sections), resizable panels, dropdown menus, tooltips, command palette, progress indicators, badges.
- This aligns with the already-decided `cmdk` choice (shadcn/ui's Command component wraps cmdk).
- Do NOT use shadcn/ui for graph-related rendering (that is React Flow's domain).
- Copy components into `src/components/ui/` and customize as needed.

**Why this matters for longevity:** shadcn/ui's copy-into-project model means no version lock-in. Components are plain React + Tailwind + Radix, so they can be maintained indefinitely even if the shadcn/ui project itself becomes inactive. The underlying Radix primitives are the most battle-tested accessible component library in the React ecosystem.

#### S2.4 Animation: Motion (Framer Motion successor)

**Current best practice:** Framer Motion rebranded to **Motion** (motion.dev). Still the most popular React animation library (~32KB gzipped). Declarative API, layout animations, gesture support, AnimatePresence for enter/exit. React Spring offers physics-based animation with more natural motion. Motion One (also from motion.dev) is lighter (~3KB) and uses the Web Animations API (WAAPI) for better mobile performance.

**Recommendation for Code Atlas:**
- Use **Motion (Framer Motion)** for graph transition animations: expand/collapse transitions, layout change animations, panel slide-in/out, node highlight effects.
- Specific use cases:
  - `AnimatePresence` for node appear/disappear during expand/collapse.
  - `layout` prop for smooth position changes after ELK re-layout.
  - `motion.div` for detail panel slide-in.
  - Spring physics for zoom-to-fit and viewport transitions.
- Do NOT animate individual edges (too many DOM elements). Animate edge opacity/color changes via CSS transitions.
- Consider Motion One for extremely performance-sensitive micro-animations if Motion proves too heavy.

**Why this matters for longevity:** Smooth animations during expand/collapse and layout changes are the difference between a professional tool and a prototype. Motion is the most widely adopted animation library in the React ecosystem and the xyflow (React Flow) community frequently uses it in examples.

#### S2.5 Tailwind CSS v4 Patterns and Gotchas

**Current best practice:** Tailwind CSS v4 (January 2025) is a major architectural rewrite. CSS-first config via `@theme` directive (no `tailwind.config.js`). Built on Lightning CSS (Rust-based, 60-80% faster cold builds, 100x faster incremental builds). OKLCH colors by default. Automatic content detection. Container queries are core.

**Key gotchas for Code Atlas:**
1. **Import change**: Replace `@tailwind base; @tailwind components; @tailwind utilities;` with `@import "tailwindcss";`.
2. **Class renames**: `bg-gradient-to-*` becomes `bg-linear-to-*`, `flex-shrink-0` becomes `shrink-0`.
3. **Default border color changed** from `gray-200` to `currentColor`. Add explicit border colors.
4. **Dynamic class names**: If building class names programmatically (e.g., edge color by category), the codemod cannot detect these. Use a safelist or ensure all classes appear statically somewhere.
5. **Plugin changes**: `plugin()`, `addUtilities()`, `matchUtilities()` APIs changed. Simple plugins become `@utility` definitions in CSS.
6. **Upgrade tool**: `npx @tailwindcss/upgrade` handles ~90% of mechanical changes.

**Recommendation for Code Atlas:**
- Start fresh with Tailwind v4 patterns (no migration needed since the project is greenfield).
- Use `@theme` for design tokens: graph colors, edge category colors, confidence colors, health indicator colors.
- Use CSS custom properties (`--color-*`) for theme values that React Flow and custom components both reference.
- Use `@custom-variant dark (&:where(.dark, .dark *));` for class-based dark mode toggling.

**Why this matters for longevity:** Tailwind v4's CSS-first approach means design tokens are standard CSS custom properties, accessible to any part of the stack (React Flow themes, shadcn/ui components, custom CSS). This is more maintainable than JavaScript-based configuration.

#### S2.6 Icons: Lucide

**Current best practice:** Lucide (fork of Feather Icons) offers 1,500+ clean, consistent icons on a 24x24 grid. Optimized for React 19. Tree-shakeable. Phosphor Icons offers more variety (6,000+ icons, 6 weights) but is heavier. Heroicons (by Tailwind Labs) has fewer icons but native Tailwind integration.

**Recommendation for Code Atlas:**
- Use **Lucide React** (`lucide-react`). Reasons:
  1. Clean, consistent stroke-based style fits a developer tool aesthetic.
  2. shadcn/ui uses Lucide as its default icon library -- components already reference Lucide icons.
  3. Tree-shakeable: only icons used are bundled.
  4. Sufficient icon coverage for our UI needs (folder, file, package, search, settings, expand, collapse, warning, check, arrow, link, etc.).
- If specific icons are missing, Phosphor can be added as a supplement (both use standard SVG components).

**Why this matters for longevity:** Aligning with shadcn/ui's default icon library eliminates friction when adopting new shadcn/ui components. Lucide is actively maintained with regular icon additions.

#### S2.7 Type Generation: tauri-specta (confirmed)

**Current best practice:** tauri-specta v2 remains the primary tool for Rust-to-TypeScript type generation in Tauri v2 apps. No significant alternatives have emerged. The base research already covers this well.

**Recommendation for Code Atlas:** No change from base research. Use tauri-specta v2 with manual fallback for Channel<T> types if needed.

---

### S3. Modern Testing Approaches

#### S3.1 Component Testing: Vitest + Storybook (selective)

**Current best practice:** Storybook 8 remains relevant for component development and visual testing. Ladle (Vite-native, 10-50x faster startup) is a lightweight alternative. For Code Atlas, the primary testing concern is React Flow custom nodes, which require both functional testing (interaction behavior) and visual validation (do they look right in the graph context).

**Recommendation for Code Atlas:**
- **Primary**: Use Vitest + @testing-library/react for functional component tests. Test interaction behavior (expand/collapse, selection, edge hover) without visual rendering.
- **Selective Storybook**: Add Storybook for custom node/edge components ONLY if visual iteration during development proves necessary. Do not add it speculatively.
- **Graph structure tests**: Use pure Rust unit tests for graph data model correctness. These are faster and more reliable than testing through the UI.

**Why this matters for longevity:** Testing graph UI at the component level with testing-library validates behavior without brittle visual assertions. Storybook adds value only if the team needs a visual component catalog -- defer the decision until there are enough custom components to justify the overhead.

#### S3.2 E2E Testing for Tauri

**Current best practice:** Tauri v2 supports WebDriver-based E2E testing via `tauri-driver`. Supported frameworks: WebdriverIO, Selenium. Playwright can be used with custom configuration to test the Tauri frontend (mocking IPC calls). **Important caveat: macOS does not provide a desktop WebDriver client**, meaning E2E tests via WebDriver cannot run natively on macOS.

**Recommendation for Code Atlas:**
- **POC**: Skip E2E tests. Focus on Rust unit tests + frontend component tests.
- **MVP**: Add E2E tests using **Playwright** configured to test the frontend in isolation (Vite dev server, mocked Tauri IPC). This sidesteps the macOS WebDriver limitation.
- **Platform**: Evaluate `tauri-driver` + WebdriverIO for full desktop E2E tests running in CI on Linux.
- For smoke testing on macOS during development, use manual testing with the Tauri dev server.

**Why this matters for longevity:** The macOS WebDriver limitation is a real constraint. Playwright-based frontend testing provides the best coverage-to-effort ratio and runs in CI on all platforms.

#### S3.3 Snapshot Testing for Graph Structures

**Current best practice:** Rust has `insta` (by Armin Ronacher of Sentry) for snapshot testing. It serializes data structures to text/JSON/YAML and compares against committed snapshots. `insta review` provides an interactive TUI for reviewing snapshot changes.

**Recommendation for Code Atlas:**
- Use **`insta`** for golden corpus validation in `codeatlas-core`. Snapshot the graph output (nodes, edges, health report) for each reference repository.
- Workflow: scan a reference repo, serialize the `ArchGraph` to JSON, snapshot-test the result. When the graph model changes, `cargo insta review` shows exactly what changed.
- This directly supports the PRD's golden corpus testing requirement.

**Why this matters for longevity:** Snapshot tests catch unintentional changes to graph output that unit tests might miss. They are essential for maintaining correctness across parser and resolver upgrades.

#### S3.4 Visual Regression Testing

**Current best practice:** For open-source projects: Playwright's built-in `toHaveScreenshot()` is free and CI-friendly. For teams with budget: Chromatic (by Storybook maintainers) provides cloud-based visual diffing. Lost Pixel is an open-source alternative that works with Storybook, Ladle, or plain page screenshots.

**Recommendation for Code Atlas:**
- **POC/MVP**: Do not add visual regression testing. The graph layout is inherently dynamic (depends on ELK computation, which may produce slightly different layouts across runs). Visual regression tests would be flaky.
- **Platform**: If the product develops a set of "canonical views" (demo graph, specific reference repo views), consider Playwright screenshot tests for those specific views with generous diff thresholds.

**Why this matters for longevity:** Premature visual regression testing for graph visualizations produces more noise than signal. Defer until the layout stabilizes and canonical views exist.

---

### S4. Developer Experience

#### S4.1 Monorepo Tooling

**Current best practice:** Turborepo and Nx are the leading monorepo build orchestrators. However, for a project with only a frontend package and Rust workspace, the overhead of a full monorepo tool is not justified. pnpm workspaces + Cargo workspaces provide sufficient structure.

**Recommendation for Code Atlas:**
- **Do NOT add Turborepo or Nx.** The project has one frontend package and a Cargo workspace. pnpm scripts + Cargo workspace commands are sufficient.
- Use pnpm scripts for frontend orchestration: `pnpm dev`, `pnpm test`, `pnpm lint`, `pnpm typecheck`.
- Use Cargo workspace commands for Rust: `cargo test --workspace`, `cargo clippy --workspace`.
- If the project grows to multiple frontend packages (e.g., VS Code extension package), re-evaluate.

**Why this matters for longevity:** Avoiding premature tooling complexity keeps the project approachable for contributors. pnpm workspaces + Cargo workspace is the standard for Tauri projects.

#### S4.2 Pre-commit Hooks: lefthook

**Current best practice:** lefthook (written in Go) is faster than husky (Node.js) for pre-commit hooks, especially in polyglot repos. It supports parallel execution, file-based filtering, and native staged-file detection (no separate lint-staged needed). Single `lefthook.yml` config file. 10x faster than husky for large projects.

**Recommendation for Code Atlas:**
- Use **lefthook** for pre-commit hooks. Justification:
  1. Code Atlas is a polyglot project (Rust + TypeScript). lefthook handles both natively.
  2. Built-in staged-file filtering eliminates the need for lint-staged as a separate dependency.
  3. Parallel execution runs Biome (frontend) and `cargo clippy` (Rust) concurrently.
  4. Single `lefthook.yml` config is simpler than husky + lint-staged + separate config files.
- Pre-commit hooks to run: `biome check --staged` (frontend), `cargo clippy` (Rust), `cargo fmt --check` (Rust).
- Pre-push hooks to run: `pnpm typecheck`, `pnpm test`, `cargo test`.

**Why this matters for longevity:** lefthook's polyglot support and parallel execution align perfectly with a Rust + TypeScript project. It is a single binary with no Node.js runtime dependency for the hook runner itself.

#### S4.3 Dev Containers / Nix

**Current best practice:** Nix (via `devenv.sh`) provides fully reproducible development environments. A single `devenv.nix` file replaces Docker, brew, apt, and npm commands. Pins exact compiler versions, system dependencies, and tool versions. Works on macOS (ARM + x86) and Linux. `direnv` integration means `cd`-ing into the project activates the environment automatically. CI integration via `devenv ci`.

**Recommendation for Code Atlas:**
- **POC**: Do NOT add Nix. The contributor base is small (likely solo), and Nix has a steep learning curve.
- **MVP**: Provide a `flake.nix` or `devenv.nix` as an OPTIONAL convenience for contributors. Include Rust stable, Node.js 22, pnpm, and system dependencies (webkit2gtk for Linux, Xcode CLI tools for macOS).
- **Always maintain manual setup instructions** in the README as the primary path.
- Consider a `.devcontainer/` for GitHub Codespaces / VS Code Dev Containers as a lighter alternative.

**Why this matters for longevity:** Nix solves "works on my machine" problems definitively, but its adoption cost is high. Providing it as an optional path respects both experienced Nix users and developers who prefer traditional setup.

#### S4.4 Hot Module Replacement (Tauri + Vite)

**Current best practice:** Tauri v2 + Vite HMR works out of the box for desktop targets. Vite's dev server runs on a local port, and Tauri's webview connects to it. HMR operates via WebSocket. No known gotchas for the macOS desktop + Vite + React combination.

**Known gotchas (from community reports):**
- Mobile targets (Android/iOS) have HMR issues with some frameworks (Nuxt specifically). Not relevant for Code Atlas POC/MVP.
- Tauri's `devUrl` in `tauri.conf.json` must point to Vite's dev server (e.g., `http://localhost:1420`).
- Some Tauri state changes (Rust-side) require a full restart, not just HMR.

**Recommendation for Code Atlas:** No special configuration needed. Tauri + Vite + React HMR works as expected. Focus on ensuring React Flow state is preserved during HMR (Zustand stores persist across HMR by default).

---

### S5. Modern Graph/Visualization Alternatives

#### S5.1 React Flow Alternatives Assessment

**Current best practice:** React Flow v12 (@xyflow/react) remains the leading choice for node-based UIs in React. Alternatives assessed:

| Library | Rendering | Compound Nodes | Maturity | Maintenance | Fit |
|---------|-----------|----------------|----------|-------------|-----|
| **React Flow v12** | DOM/SVG | Via parentId | High | Active (React 19 + TW4 updated Oct 2025) | Best |
| **Reaflow** | DOM/SVG | Yes (nested) | Medium | Active but smaller team | Good |
| **Reagraph** | WebGL (Three.js) | Clustering | Medium | Active | Wrong paradigm |
| **Sigma.js** | WebGL | No | High | Active | Wrong paradigm |
| **JsPlumb** | DOM/SVG | Limited | High | Commercial | Expensive |
| **yFiles** | DOM/SVG/Canvas | Yes | Very High | Commercial | Very expensive |

**Sigma.js** excels at rendering thousands of nodes via WebGL but is designed for network graphs (flat node-link diagrams), not compound hierarchical graphs. It does not support nested nodes, which is a core requirement.

**Reaflow** supports compound graphs and uses ELK internally, making it the closest alternative. However, React Flow has 10x the community, more frequent updates, and better documentation.

**Recommendation for Code Atlas:** **Stick with React Flow v12 + ELK.** No alternative offers a better combination of compound node support, React integration, active maintenance, and community ecosystem. The base research decision is confirmed.

**Why this matters for longevity:** React Flow's recent updates (React 19 + Tailwind CSS 4 compatibility, October 2025) demonstrate ongoing investment. The xyflow team is well-funded and committed to the project.

#### S5.2 WebGPU Rendering Viability

**Current best practice:** Safari 26.0 (September 2025) added WebGPU support enabled by default on macOS, iOS, iPadOS. However, **WKWebView does not have WebGPU access** -- this is a known limitation. WebGPU is only available in Safari's main process, not in the WKWebView used by Tauri.

**Recommendation for Code Atlas:**
- **WebGPU is NOT viable** for graph rendering in a Tauri app on macOS in the near term. WKWebView's lack of GPU access is a blocking constraint.
- For Vision-phase WebGPU exploration, options include:
  1. Wait for Apple to enable WebGPU in WKWebView (no timeline available).
  2. Use a Chromium-based webview (breaks Tauri's platform-native approach).
  3. Render via a separate Metal/Vulkan surface composited with the webview (extremely complex).
- **Practical alternative for large graphs**: Use React Flow's `onlyRenderVisibleElements` + aggressive collapse (already planned) + lazy hydration. These DOM-level optimizations handle 5,000-node graphs adequately.

**Why this matters for longevity:** This is a critical constraint that the PRD's Vision phase should acknowledge. WebGPU in Tauri requires Apple to update WKWebView capabilities. Planning around this avoids wasted effort.

---

### S6. Distribution & Growth

#### S6.1 Homebrew Cask Distribution

**Current best practice:** Homebrew cask is a standard distribution channel for macOS desktop apps. It automatically strips the quarantine attribute, bypassing Gatekeeper warnings for unsigned apps. Tauri apps can be distributed via cask by creating a formula that downloads the `.dmg` from GitHub Releases.

**Recommendation for Code Atlas:**
- **MVP**: Set up a Homebrew cask tap (`homebrew-codeatlas`) in a separate GitHub repo.
- CI/CD pipeline: on tagged release, build the DMG via `tauri-apps/tauri-action`, upload to GitHub Releases, auto-update the cask formula with the new version and SHA256.
- This provides an installation path that avoids Gatekeeper friction: `brew install --cask codeatlas`.
- Complement with direct DMG download from GitHub Releases for users who do not use Homebrew.

**Why this matters for longevity:** Homebrew cask is the expected distribution channel for developer tools on macOS. Linear, Raycast, Warp, and most modern dev tools are available via cask.

#### S6.2 Auto-Update UX Patterns

**Current best practice:** Modern desktop apps (Linear, Raycast, Arc) use a non-disruptive auto-update pattern:
1. Check for updates in the background on launch (or periodically).
2. Download the update silently.
3. Show a subtle, non-modal notification: "Update available. Restart to apply."
4. Apply the update on next restart. Never force-restart during active work.
5. Show a brief changelog or "What's new" after the update.

**Recommendation for Code Atlas:**
- Use `tauri-plugin-updater` (already planned) with the following UX:
  - Check on launch, download silently.
  - Show a non-modal toast/badge in the title bar or status bar: "Update ready -- restart when convenient."
  - Never auto-restart. The user is likely mid-analysis.
  - After restart with a new version, show a dismissable "What's new" panel with a changelog summary.
- Host `latest.json` on GitHub Releases (simplest, already planned).

**Why this matters for longevity:** Intrusive update dialogs erode trust. The "update when convenient" pattern respects the user's workflow and is now the expected behavior for developer tools.

#### S6.3 Telemetry: Opt-in, Privacy-Preserving

**Current best practice:** Privacy-preserving analytics tools: TelemetryDeck (best for native/desktop apps, no consent banners needed, privacy-safe by design), PostHog (full product analytics suite, cookie-free mode, generous free tier of 1M events/month), Plausible (lightweight, GDPR-compliant, but web-focused). All three can be self-hosted.

**Recommendation for Code Atlas:**
- **POC/MVP**: No telemetry. Focus on building trust.
- **Platform**: Add **opt-in telemetry** using anonymized, aggregated events. Implementation:
  1. On first launch, show a clear dialog: "Help improve Code Atlas by sending anonymous usage statistics? [Yes] [No]" with a link to what is collected.
  2. Collect: scan duration, file count, language breakdown, feature usage (which views/features are used), crash reports. Never collect: file paths, file contents, project names, or any PII.
  3. Use **PostHog** with anonymous events (cheaper tier, no PII). Self-host if privacy concerns arise from enterprise users.
  4. Provide a settings toggle to change the choice at any time.
  5. Document the exact events collected in a public privacy page.

**Why this matters for longevity:** Telemetry is essential for product decisions at scale, but for a local-first tool that emphasizes privacy (PRD Principle #1), the implementation must be exemplary. Opt-in with full transparency is the only acceptable approach.

#### S6.4 Open Source Licensing

**Current best practice:** For developer tools in 2025-2026:
- **MIT**: Most permissive, highest adoption (92% of projects). Used by React, Next.js, Vite, Zustand.
- **Apache 2.0**: Includes explicit patent protection. Used by Rust itself, Kubernetes, TensorFlow. Preferred by enterprises for legal clarity.
- **BSL (Business Source License)**: Source-available with commercial restrictions for a set period (typically 3-4 years), then converts to Apache 2.0 or MIT. Used by Sentry, CockroachDB, MariaDB, HashiCorp. Prevents cloud providers from competing with your own product.

**Recommendation for Code Atlas:**
- **Use MIT for `codeatlas-core`** (the analysis library). Maximizes adoption, contribution, and integration potential. Developers and agents should be able to use the analysis engine without licensing friction.
- **Use MIT for the desktop app** as well, initially. If a future SaaS/cloud version emerges, consider BSL for the hosted service layer only.
- Apache 2.0's patent clause is a minor advantage but adds legal complexity that deters casual contributors. MIT's simplicity is the right choice for a project seeking community adoption.
- Add a `LICENSE` file and SPDX headers to all source files.

**Why this matters for longevity:** MIT is the path of least resistance for developer tool adoption. Changing to a more restrictive license later is possible (each release can have a different license), but starting permissive builds community trust.

---

### S7. AI/Agent Integration Patterns

#### S7.1 MCP (Model Context Protocol) Rust Implementation

**Current best practice:** The official Rust MCP SDK is `rmcp` (v0.16.0, crates.io). Implements protocol version 2025-11-25. Uses a macro-based architecture:
- `#[tool]` macro on async functions generates JSON schemas and routing.
- `#[tool_router]` on impl blocks creates dispatchers.
- `#[tool_handler]` generates `ServerHandler` integration.
- Supports stdio transport (`transport-io` feature), with custom transport trait for others.
- Dependencies: tokio, serde, schemars (for JSON Schema generation).

**Example pattern:**
```rust
#[derive(Clone)]
pub struct CodeAtlasMcpServer { tool_router: ToolRouter<Self> }

#[tool_router]
impl CodeAtlasMcpServer {
    #[tool(description = "Get dependencies for a node")]
    async fn get_dependencies(
        Parameters(params): Parameters<DepsParams>,
    ) -> Result<CallToolResult, McpError> { /* ... */ }
}

#[tool_handler]
impl ServerHandler for CodeAtlasMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_11_25,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            instructions: Some("Code Atlas architecture graph server".into()),
        }
    }
}
```

**Recommendation for Code Atlas:**
- Use `rmcp` for the Platform-phase MCP adapter.
- Implement as a thin layer over `codeatlas-core`'s query API.
- Use stdio transport (standard for MCP servers invoked by editors/agents).
- Keep the tool surface small (as the PRD specifies): `get_dependencies`, `get_downstream_impact`, `get_affected_slice`, `search_nodes`.
- Expose graph overview and health as MCP resources (read-only).

**Why this matters for longevity:** MCP is rapidly becoming the standard interface between coding agents and development tools. Having a first-class Rust implementation means the MCP server runs in the same process as `codeatlas-core`, with zero serialization overhead for graph queries.

#### S7.2 Agent-Friendly API Design

**Current best practice (learned from Sourcegraph and Nx):**

**Sourcegraph's approach:** SCIP (Semantic Code Intelligence Protocol) provides normalized cross-language semantic indexes. Their MCP server exposes `sg_find_references` and `sg_go_to_definition` -- high-level semantic queries, not raw graph dumps. Key insight: agents work best with focused, task-oriented tools, not data dumps.

**Nx's approach:** Nx defaults its MCP server to minimal mode because "workspace analysis tools are now handled more efficiently by agent skills, which provide domain knowledge as incrementally-loaded instructions rather than tool-call-based data dumps." Nx provides workspace metadata and generators that agents can invoke.

**Recommendation for Code Atlas:**
- Design MCP tools as **high-level queries**, not graph traversals:
  - "What does module X depend on?" (returns a focused list with provenance)
  - "What is the downstream impact of changing file Y?" (returns affected files with distances)
  - "What is the health status of this workspace?" (returns a structured health report)
- Avoid exposing raw graph nodes/edges via MCP. Agents do not benefit from full graph dumps -- they overflow context windows and require the agent to re-implement graph algorithms.
- Include **node identifiers** (materialized keys) in MCP responses so agents can reference specific nodes in follow-up queries.
- Include **edge category information** so agents can distinguish between runtime and type-only dependencies.

**Why this matters for longevity:** The lessons from Sourcegraph and Nx are clear: effective agent integration is about focused, actionable queries, not raw data access. This aligns with the PRD's decision to keep the MCP surface thin.

#### S7.3 Context Window Management

**Current best practice:** Coding agents (Claude, Cursor, Copilot) have context windows of 100K-1M tokens. A full graph dump of a 5,000-file repo could easily consume 50K+ tokens, leaving little room for the agent's actual task.

**Recommendation for Code Atlas:**
- **Slice-first responses**: Every MCP tool should return a focused slice of the graph, not the whole thing. Maximum response size: ~2,000 tokens per tool call.
- **Progressive detail**: Start with package-level overview (small), let the agent drill down into specific packages.
- **Summary statistics**: Include counts and health indicators so agents can decide whether to drill deeper.
- **Stable identifiers**: Use materialized keys as stable references that agents can pass between tool calls.
- Format: prefer concise structured text (YAML or JSON) over verbose natural-language descriptions.

**Why this matters for longevity:** As agents become the primary consumers of architecture data, the ability to provide relevant context without overwhelming the context window is a key differentiator.

---

### S8. Accessibility & Inclusive Design

#### S8.1 WCAG 2.2 AA Compliance

**Current best practice:** WCAG 2.2 (October 2023) is the current W3C standard. Key criteria relevant to Code Atlas:
- **2.4.11 Focus Appearance (AA):** Focus indicators must be sufficiently bold and high-contrast.
- **2.4.12 Focus Not Obscured (AA):** Focused elements must not be hidden by other content.
- **2.5.7 Dragging Movements (AA):** Alternatives for drag operations (relevant for graph panning).
- **2.5.8 Target Size (AA):** Minimum 24x24 CSS pixel touch/click targets.
- **1.4.11 Non-text Contrast (AA):** UI components and graphical objects need 3:1 contrast ratio.

**Recommendation for Code Atlas:**
- Apply WCAG 2.2 AA principles to all non-graph UI (panels, dialogs, menus, search). shadcn/ui + Radix provides WCAG compliance out of the box for these components.
- For the graph canvas: full WCAG compliance is aspirational but not fully achievable (inherently visual, spatial data). Provide **alternative access** to all graph information via:
  1. The detail panel (semantic HTML, ARIA labels, screen-reader-friendly).
  2. The command palette search (keyboard-first, ARIA roles).
  3. A keyboard-navigable node list (as an alternative to visual graph exploration).
- Ensure all interactive elements have focus indicators (Tailwind's `ring-*` utilities).
- Provide a "Skip to content" link for keyboard users.

**Why this matters for longevity:** Accessibility is both an ethical imperative and increasingly a legal requirement. Building it in from the start is dramatically easier than retrofitting.

#### S8.2 Keyboard-First Graph Navigation

**Current best practice:** React Flow provides basic keyboard support (Tab through nodes, arrow keys for viewport, Enter to select). For a code architecture tool used by developers (who are keyboard-heavy users), more sophisticated keyboard navigation is expected.

**Recommendation for Code Atlas:**
- Implement keyboard shortcuts:
  - `Tab`/`Shift+Tab`: cycle through visible nodes.
  - `Enter`: expand/collapse selected node, or open detail panel.
  - `Escape`: deselect, close panel.
  - Arrow keys (when node selected): navigate to connected nodes (left = dependents, right = dependencies).
  - `Cmd+K`: search (already planned).
  - `Cmd+[`/`Cmd+]`: navigation history (back/forward through selected nodes).
  - `Cmd+0`: fit to view.
- Display a keyboard shortcut cheat sheet (accessible via `?`).
- Ensure all keyboard actions are discoverable via tooltips or a help panel.

**Why this matters for longevity:** Developer tools that are keyboard-navigable earn loyalty. This is a feature that competitors (CodeViz, GitKraken Codemaps) have not prioritized, creating a differentiation opportunity.

#### S8.3 Screen Reader Compatibility

**Current best practice:** Graph visualizations are inherently challenging for screen readers. Best approaches:
1. Provide an accessible data table as an alternative view (nodes as rows, edges as columns).
2. Use `aria-describedby` on the canvas to point to a hidden text description.
3. Ensure the detail panel is fully semantic HTML with proper headings, lists, and labels.
4. Use ARIA live regions to announce graph state changes (scan progress, node selection).

**Recommendation for Code Atlas:**
- The graph canvas itself will not be screen-reader-accessible (visual-spatial data cannot be meaningfully linearized).
- All information available in the graph MUST also be accessible through:
  1. **Detail panel**: Fully semantic HTML. Node name, type, dependencies list, dependents list, health status.
  2. **Search**: Cmd+K search results with ARIA roles.
  3. **Health dashboard**: Fully accessible tables and lists.
  4. **Future (MVP)**: A tree-view alternative to the graph canvas showing the same hierarchy in a navigable list.

**Why this matters for longevity:** While the graph canvas is inherently visual, making all graph data accessible through alternative interfaces ensures the product is usable by developers with visual impairments.

#### S8.4 Colorblind-Safe Palette

**Current best practice:** The Okabe-Ito palette (Color Universal Design) provides 8 colors distinguishable by colorblind and non-colorblind users. Paul Tol's qualitative palettes offer 5-10 colorblind-safe colors. Key principles: avoid red/green combinations, use no more than 8 colors, use dual encoding (color + pattern/dash).

**Recommendation for Code Atlas:**
- Use the **Okabe-Ito palette** for edge categories:
  - Value import: `#0072B2` (blue)
  - Type-only import: `#56B4E9` (sky blue) + dashed line
  - Dev dependency: `#E69F00` (orange) + dotted line
  - Build dependency: `#F0E442` (yellow)
  - Normal dependency: `#009E73` (green)
  - Manual (config overlay): `#CC79A7` (pink) + double line
  - Suppressed: `#999999` (gray) + very dashed
- Always pair color with a secondary visual encoding (dash pattern, line weight, or label). This ensures edges are distinguishable even in grayscale or for users with any type of color vision deficiency.
- Provide a "High contrast" mode that increases line weight and saturation for all edges.
- Test the palette with a colorblind simulation tool (e.g., Sim Daltonism for macOS).

**Why this matters for longevity:** Edge categories are a core differentiating feature of Code Atlas. If users cannot distinguish them visually, the feature fails. Dual encoding (color + pattern) is a simple solution that works for everyone.

---

### S9. Performance Monitoring

#### S9.1 Frontend Performance Profiling

**Current best practice:** React DevTools Profiler for component-level performance analysis. Chrome DevTools Performance tab for frame-by-frame analysis (WKWebView supports remote debugging via Safari Web Inspector on macOS). React Flow's `onlyRenderVisibleElements` is the primary performance lever.

**Recommendation for Code Atlas:**
- Use **Safari Web Inspector** (Develop menu > App Name) for profiling the Tauri webview.
- Key metrics to monitor:
  - FPS during pan/zoom (target: 60fps with <200 visible nodes).
  - ELK layout computation time (target: <500ms for 200 nodes).
  - Time to first meaningful render after scan start (target: <2 seconds).
  - Memory usage (target: <200MB total).
- Add `tracing` spans (Rust side) for scan duration, parse time per file, graph construction time.
- Emit timing events via Channel<T> for frontend performance dashboards.

**Why this matters for longevity:** Performance budgets set during the POC become the baseline for all future development. Instrumenting early prevents regressions.

#### S9.2 Rust Profiling Tools

**Current best practice for macOS:**
- **`cargo-flamegraph`**: Standard tool for CPU flame graphs. Works on macOS via DTrace. Install via `cargo install flamegraph`.
- **`samply`**: Modern alternative with Firefox Profiler web UI integration. Better macOS support than flamegraph in some cases.
- **`dhat`**: Heap allocation profiler. Useful for identifying allocation hotspots in the scan pipeline.
- **macOS Instruments**: Apple's profiling tool. Works with Rust binaries. Provides CPU, memory, and allocation profiling.
- **`heaptrack`**: Linux-only heap profiler. Not available on macOS.

**Recommendation for Code Atlas:**
- Use **`cargo-flamegraph`** for CPU profiling of scan operations.
- Use **`samply`** as an alternative when flamegraph has issues on macOS.
- Use **`dhat`** for heap allocation profiling during scan (tree-sitter allocations, graph construction).
- Use **`criterion`** benchmarks (already planned) for continuous performance tracking.
- Add criterion benchmarks for: single-file parse time, 100-file parallel parse time, graph construction time, ELK layout input generation time.

**Why this matters for longevity:** Rust performance is a key selling point for Code Atlas. Profiling tools ensure that performance remains excellent as the codebase grows.

#### S9.3 Runtime Performance Monitoring

**Recommendation for Code Atlas:**
- Emit scan timing events via `tracing`: total scan time, per-detector time, per-phase time (workspace discovery, parsing, graph construction, streaming).
- Display these timings in the UI health dashboard (MVP).
- For Platform phase: log scan timings to a local SQLite table for trend analysis (are scans getting slower over time?).
- Use `std::time::Instant` for precise timing in Rust, not wall-clock time.

---

### S10. Future-Proofing

#### S10.1 WASM Potential

**Current state:** petgraph compiles to WASM (petgraph-wasm project exists, work-in-progress). tree-sitter has WASM support via `tree-sitter-wasm-build-tool`, and prebuilt WASM binaries exist for language parsers (Sourcegraph maintains a set). However, ABI incompatibilities between Clang and Rust for `wasm32-unknown-unknown` require workarounds.

**Recommendation for Code Atlas:**
- **Design `codeatlas-core` to be WASM-compatible** from the start, even though WASM compilation is not a POC/MVP goal:
  1. Avoid platform-specific APIs (use `std::path::Path` abstractions, not raw syscalls).
  2. Keep filesystem I/O behind a trait abstraction (`trait FileSystem`) so it can be replaced with in-memory or WASM-compatible implementations.
  3. Avoid `tokio` in the core library's public API. Use `async fn` with trait bounds that work in both tokio and WASM runtimes (wasm-bindgen-futures).
  4. Do not depend on `cargo_metadata` (subprocess invocation) in the core library. Instead, define a `WorkspaceMetadata` struct that the caller populates. The Tauri shell calls `cargo metadata` and converts the result; a WASM version would receive the data differently.
- **Platform/Vision**: Compile `codeatlas-core` to WASM for a browser-based version. Use tree-sitter's prebuilt WASM grammars. This enables a "try online" experience without installation.

**Why this matters for longevity:** A WASM-compiled core enables a web version, VS Code webview integration, and online demos. Designing for WASM compatibility from the start has minimal cost but opens significant future opportunities.

#### S10.2 Plugin System Design

**Current best practice for Rust plugin systems:**
- **WASM plugins (via Extism/wasmtime):** Best for security (sandboxed), cross-language (plugins can be written in any language that compiles to WASM). Used by Zed editor, moonrepo. Overhead: ~10-100us per function call.
- **Rhai scripting:** Embedded scripting language for Rust. Small footprint (~160-300KB gzipped WASM). Rust-like syntax. Good for configuration and simple rules but limited for complex analysis.
- **Native Rust plugins (dynamic linking):** Fastest but unsafe. Not recommended.

**Recommendation for Code Atlas:**
- **Platform phase**: Implement plugins via **WASM** (wasmtime or Extism). This aligns with the PRD's detector seam architecture and provides:
  1. Safety: WASM plugins run in a sandbox. A buggy plugin cannot crash the host.
  2. Cross-language: Plugin authors can use Rust, Go, TypeScript, Python.
  3. Distribution: WASM binaries are small and portable.
  4. The internal `Detector` trait maps naturally to a WASM interface (`.wit` file defining inputs/outputs).
- **Supplement with Rhai** for lightweight configuration rules (`.codeatlas.yaml` `rules` section). Architecture rules like "no import from package X to package Y" can be expressed as Rhai scripts evaluated against the graph.
- Define the plugin interface (`.wit` file) during Platform design, even if the runtime is not implemented until later.

**Why this matters for longevity:** WASM-based plugins are the emerging standard for extensible developer tools (Zed, moonrepo, Spin). They provide safety guarantees that native plugins cannot, and their cross-language support maximizes the potential contributor base.

#### S10.3 Cross-Platform Considerations

**Current state for Tauri v2:**
- **macOS**: Primary target. WKWebView. Works well. Code signing required for distribution ($99/year Apple Developer).
- **Linux**: Requires `webkit2gtk` 4.1 (available on Ubuntu 22.04+). Tauri apps work but may have visual differences due to GTK theming. No code signing required.
- **Windows**: Uses WebView2 (Chromium-based, pre-installed on Windows 10+). `.msi` installers require WiX (Windows-only build). `.exe` installers use NSIS (can be built on any platform). Code signing via Authenticode certificates.

**Key gotchas:**
1. **Cross-compilation is not supported.** Must build on each target platform (use CI matrix).
2. **Path handling**: Windows uses `\` separators and has drive letters. Use `std::path::Path` throughout, never string manipulation on paths.
3. **File system case sensitivity**: macOS (default) and Windows are case-insensitive. Linux is case-sensitive. Import resolution must handle this.
4. **Font rendering**: Differs across platforms. Use system fonts or bundle a specific font for consistent graph labels.
5. **Webview differences**: WKWebView (macOS) vs WebView2 (Windows) vs WebKitGTK (Linux) have slightly different CSS rendering. Test on all platforms.

**Recommendation for Code Atlas:**
- **POC**: macOS only. Do not invest in cross-platform testing.
- **MVP**: Add Linux CI build. Test manually on Linux.
- **Platform**: Add Windows CI build. Address path handling and case sensitivity issues.
- Use `std::path::Path` everywhere. Use the `camino` crate (`Utf8PathBuf`) for paths that need to be serialized (petgraph node data, IPC).
- Add a CI matrix early: `[macos-latest, ubuntu-latest]` for tests, `[macos-latest, ubuntu-latest, windows-latest]` for builds.

**Why this matters for longevity:** Cross-platform support expands the addressable market significantly. Designing with cross-platform in mind from the start (path handling, case sensitivity) prevents expensive retrofits.

---

### Supplement Summary: Key Additions to Technology Choices

| Concern | Choice | Phase | Confidence |
|---------|--------|-------|------------|
| Error handling (core) | thiserror 2.x | POC | High |
| Error diagnostics (shell) | miette 7.x | POC | High |
| CPU-bound parallelism | rayon | POC | High |
| Structured logging | tracing + tracing-subscriber | POC | High |
| Frontend linter + formatter | Biome v2.x (replaces ESLint + Prettier) | POC | High |
| React optimization | React Compiler v1.0 | POC | Medium-High |
| UI components | shadcn/ui (Radix + Tailwind) | POC | High |
| Animation | Motion (Framer Motion) | POC | Medium |
| Icons | Lucide React | POC | High |
| Pre-commit hooks | lefthook | POC | High |
| Snapshot testing (Rust) | insta | POC | High |
| CLI framework | clap v4 | Platform | High |
| MCP server | rmcp (official Rust SDK) | Platform | High |
| Plugin system | WASM (wasmtime/Extism) + Rhai | Platform | Medium |
| Colorblind palette | Okabe-Ito | POC | High |
| Rust edition | 2024 | POC | High |
| License | MIT | POC | High |
| Homebrew distribution | Homebrew cask tap | MVP | High |
| Telemetry | PostHog (opt-in, anonymous) | Platform | Medium |
| Dev environment | Nix/devenv (optional) | MVP | Low |
| E2E testing | Playwright (frontend isolation) | MVP | Medium |
