# Code Atlas — Consolidated Technical Decisions

**Date:** 2026-03-18
**Status:** Active
**Basis:** All research conducted March 17-18, 2026 (see Sources section for full list)

This document records every major technical choice for Code Atlas, what was considered, and why the choice was made. It is the single authoritative reference. When a decision here conflicts with any other document, this document wins.

---

## 1. Application Shell

### Context
Code Atlas is a local-first desktop application that parses local codebases and renders interactive architecture visualizations. The shell must provide native filesystem access, a secure process model, and a modern web-based rendering surface.

### Options Evaluated

| Option | Strengths | Weaknesses |
|--------|-----------|------------|
| **Tauri v2** | Rust-native core, capability-based security, small binary (<15MB), sub-500ms cold start, split core/webview model | WKWebView on macOS (not Chromium), smaller ecosystem than Electron |
| **Electron** | Largest ecosystem, consistent Chromium behavior, proven at scale (VS Code) | Large binaries (~150MB+), higher memory, no Rust-native core, weaker default security |
| **Wails v3** | Go-native, decent footprint | v3 is alpha-era in March 2026, too immature for a product bet |
| **Neutralino** | Tiny binary | Thin runtime, no Rust core, limited capabilities |

### Decision
**Tauri v2** (stable since October 2024).

### Evidence
- Tauri v2.10.3 (crate), 2.10.1 (CLI/JS) as of March 2026. Stable, well-documented.
- Capability-based permission model enforces minimal privilege per window. Security boundaries are window-label-based.
- Split process model: Rust core has full OS access, webview is sandboxed. Exactly the architecture Code Atlas needs — parsing/graph logic in Rust, rendering in the webview.
- Binary size target (<15MB) is achievable with Tauri; not with Electron.
- The product's local-first, Rust-core, security-conscious nature maps directly to Tauri's design values.
- Wails v3 was explicitly checked and found to be alpha-era in March 2026 (public roadmap confirms).

### Risks and Mitigations
- **WKWebView differences from Chromium:** WebGL2 is supported across all Tauri platforms (macOS, Windows, Linux). WebGPU is available on macOS Tahoe 26+ and Windows (WebView2). Test rendering on all platforms.
- **Smaller plugin ecosystem than Electron:** Offset by Rust crate ecosystem. Most functionality needed (filesystem, process, IPC) is covered by Tauri's built-in plugins.
- **Electron fallback:** If the product later needs embedded editor functionality or uniform Chromium behavior, Electron remains a viable migration. The architecture (Rust core + web frontend) translates.

### Open Items
- Monitor Tauri v2's IPC performance for large payloads. Known ~6x overhead from multiple serde passes on JSON >100KB (issue #5641). Mitigation: use `Channel<T>` for streaming when payloads exceed ~100KB.

---

## 2. Graph Rendering Stack

### Context
The core product requirement is a zoomable, pannable canvas showing nested compound nodes (packages containing modules containing files) with edges between them at all hierarchy levels. This is not a flat node-and-edge graph — it requires first-class compound/hierarchical node support.

### Options Evaluated

| Library | Compound Nodes | Rendering | React Integration | Testability |
|---------|---------------|-----------|-------------------|-------------|
| **React Flow v12** | Yes (`parentId`, 3 levels confirmed) | DOM (React components) | Native | Vitest + RTL |
| **Sigma.js + Graphology** | No | WebGL | Wrapper | Limited |
| **Dagre** | No (cross-boundary edge bug) | N/A (layout only) | N/A | N/A |
| **Cytoscape.js** | Yes (compound nodes) | Canvas | Wrapper | Weaker with Vitest |
| **G6 v5 (AntV)** | Yes ("Combo" concept) | Canvas/SVG/WebGL/WASM | Extension | Uneven docs |
| **yFiles** | Yes (best-in-class) | SVG/Canvas | Native | Good |
| **Kookie Flow** | Unknown | WebGL | React-like | Unknown |

### Decision
**React Flow v12 (`@xyflow/react`)** for rendering + **ELK.js (`elkjs`)** in a Web Worker for layout.

### Evidence

**Why React Flow wins:**
- Compound nesting via `parentId` is first-class. Three nesting levels (package → module → file) confirmed working in the official sub-flows example.
- Nodes are React components — arbitrary JSX, testable with Vitest + RTL, styled with Tailwind.
- Built-in components are sufficient: `<MiniMap>`, `<NodeToolbar>`, `<NodeResizer>`, `<Panel>`, dark mode via `colorMode` prop. All free/MIT.
- The expand/collapse pattern (using `node.hidden` + ELK re-layout on visible nodes) is well-documented.

**Why ELK wins over Dagre:**
- ELK handles compound/nested layout with port-based edge routing. Dagre has an open bug preventing correct subflow layout when any subflow node connects to an external node — exactly the cross-package dependency case Code Atlas needs. React Flow's own docs call this out.
- ELK supports `INCLUDE_CHILDREN` for cross-hierarchy edge routing.
- ELK is ~1.45MB (vs Dagre's 39KB) but runs in a Web Worker, so no main-thread impact.

**What was eliminated and why:**
- **Sigma.js:** Early research recommended it for dense network visualization. Deep evaluation eliminated it — Sigma has no compound node support at all. Its data model (Graphology) is a flat graph. This is a hard blocker for nested package visualization.
- **Dagre:** Cross-boundary edge layout bug (open upstream issue). Not viable for packages with dependencies on nodes outside the package.
- **Cytoscape.js:** Supports compound nodes but cannot render React JSX inside nodes. Weaker Vitest testability. Remains a fallback only if >500 simultaneous visible nodes are needed (unlikely given expand/collapse strategy).

### Risks and Mitigations
- **ELK bundle size (1.45MB):** Run in Web Worker. Non-negotiable.
- **ELK branch ordering non-determinism in compound nodes:** Add `elk.priority` layout options per node (workaround from discussion #4830).
- **Flat-to-hierarchical transform:** React Flow uses flat arrays with `parentId`; ELK requires nested `children` tree. ~100 lines of encoder/decoder, well-documented pattern (discussion #3495).
- **React Flow performance at scale:** Keep simultaneously-visible nodes under ~200 via expand/collapse. Custom node memoization. Simple CSS (avoid animations, box-shadows, gradients in node styles).

### Open Items
- Evaluate **G6 v5** if React Flow performance becomes a proven bottleneck. G6 has WASM-backed layouts and WebGL rendering. Needs hands-on compound node ("Combo") quality evaluation.
- **yFiles** ($17K/developer) is the strongest commercial option. Evaluate only if graph UX quality becomes a strategic moat and budget is available.
- **Kookie Flow** claims 50K+ nodes at 60fps via WebGL but compound node support is unknown.

---

## 3. Parsing and Semantic Analysis Pipeline

### Context
This is the most important technical decision beyond the POC. Code Atlas needs to extract imports, exports, module declarations, and dependency relationships from source code. The quality of these extractions directly determines the trustworthiness of the architecture map.

### Options Evaluated

| Approach | Speed | Accuracy | Language Coverage | Incremental | Error Tolerant |
|----------|-------|----------|------------------|-------------|----------------|
| **tree-sitter only** | Fast | Syntactic (imports/use) | 20+ grammars | Yes (sub-ms) | Yes |
| **oxc (TS/JS only)** | 3-5x faster than tree-sitter | Syntactic + resolution | TS/JS only | No | Less |
| **Language servers (LSP)** | Varies | Semantic (types, refs) | Per-language | Via LSP | Varies |
| **SCIP indexers** | Batch | Semantic (precise) | Per-indexer | No | N/A |
| **`syn` (Rust only)** | Fast | AST-accurate | Rust only | No | No (panics on invalid) |

### Decision
**Hybrid pipeline, phased:**

| Phase | TS/JS Parser | TS/JS Resolver | Rust Parser | Other Languages |
|-------|-------------|---------------|-------------|-----------------|
| **POC** | tree-sitter | Path-based (simple) | tree-sitter | Structural only (dir hierarchy) |
| **MVP** | oxc_parser + oxc_module_lexer | oxc_resolver | tree-sitter | tree-sitter |
| **Post-MVP** | oxc + TS compiler/LSP | oxc_resolver + LSP | tree-sitter + rust-analyzer | tree-sitter + LSP adapters |

### Evidence

**Why tree-sitter for POC:**
- Battle-tested at GitHub/Neovim/Zed scale. The `tree-sitter` Rust crate (v0.26.7) integrates cleanly.
- Error-tolerant — produces partial ASTs for invalid/incomplete code. Files that fail to parse still appear as structural nodes.
- Incremental parsing via `tree.edit()` + `parser.parse(new_source, Some(&old_tree))` — typically sub-millisecond for small edits.
- Language support = grammar + import resolver. Grammars exist for Rust, TypeScript, Python, Go, Java, and many more.
- Import extraction queries validated against `node-types.json` for TypeScript and Rust. Key patterns: TS uses `(import_statement source: (string))` for imports, `(export_statement source: (string))` for re-exports, `(call_expression function: (import))` for dynamic imports. Rust uses `(use_declaration argument: (scoped_identifier))` for `use`, `(mod_item name: (identifier))` for `mod`, and `(use_declaration (visibility_modifier))` for `pub use`. Each grammar crate also exposes built-in `TAGS_QUERY` patterns (`@definition.function`, `@definition.class`, etc.) for code navigation.

**Why tree-sitter is not enough long-term:**
- It does not inherently solve: symbol resolution, definition/reference accuracy, type-driven import resolution, implementation relationships, cross-file semantic identity.
- For a trusted architecture product, every edge should carry provenance (syntactic vs. semantic vs. heuristic). Tree-sitter can only produce syntactic edges.

**Why oxc for TS/JS at MVP:**
- `oxc_parser` is 3x faster than swc, 5x faster than Biome for JS/TS parsing.
- `oxc_module_lexer` extracts imports without full AST when only imports are needed.
- `oxc_resolver` (v11.19.1) is 28-30x faster than webpack's `enhanced-resolve` and handles the entire TypeScript resolution spec: ESM/CJS, `tsconfig.json` paths/baseUrl/extends/references, `package.json` exports/imports fields with condition names, Yarn PnP, symlinks.
- Used by Turbopack (Nova), swc-node, knip, rspack — production-proven.
- tree-sitter remains as fallback if oxc fails on a specific file.

**Why `syn` was eliminated for Rust parsing:**
- Not error-tolerant (panics on invalid Rust code).
- Not incremental.
- tree-sitter provides multi-language consistency and error recovery.

**Rust module resolution is deterministic:**
- Crate root from `Cargo.toml` (`src/lib.rs` or `src/main.rs`)
- `mod.rs` style or `foo.rs` style (mutually exclusive per module)
- `use` paths map to module hierarchy: `use crate::foo::bar`
- Workspace via `[workspace]` members + path dependencies

**TypeScript barrel files** create transitive dependency chains. Atlassian reported 75% faster builds after removing them. Resolution strategy: parse barrel exports → trace to actual source → create direct edges → cache results.

### Risks and Mitigations
- **tree-sitter import resolution is incomplete:** Start with explicit `import`/`use` only. Skip dynamic imports. Show nodes without edges as fallback.
- **oxc error recovery vs tree-sitter:** Fall back to tree-sitter for files where oxc fails.
- **Barrel file resolution infinite loops:** Cycle detection cutoff in transitive resolution.
- **Parser is not `Send`/`Sync`:** Create one parser per thread for parallel parsing.

### Open Items
- **SCIP integration:** Sourcegraph's normalized semantic index format provides precise/syntactic confidence separation. Strongly consider for post-MVP if cross-language semantic quality matters.
- **rust-analyzer integration:** For Rust semantic edges (references, implementations, type hierarchy). Post-MVP.
- **TypeScript compiler/language service:** For TS semantic edges beyond what oxc provides. Post-MVP.

---

## 4. Graph Data Model

### Context
The graph data model is the core data structure that everything else depends on. It must represent hierarchical containment (packages contain modules contain files), dependency relationships (imports, exports, re-exports), and support efficient traversal, filtering, and serialization.

### Decision
**petgraph `StableGraph`** in Rust, with a defined node/edge schema.

### Evidence
- `petgraph` (v0.8.3) is the canonical Rust graph crate. `StableGraph` provides stable indices that remain valid after node/edge removals — critical for incremental updates and graph diffing.
- Built-in algorithms: topological sort, SCC detection (Tarjan's/Kosaraju's), shortest path (Dijkstra, A*), k-shortest paths, articulation points, bridges.
- Zero-copy filtering: `NodeFiltered`, `EdgeFiltered`, `Reversed` adaptors work transparently with all traversal algorithms.

### Node Hierarchy (POC)

```
repository
  └── workspace / package / crate / app
       └── module / folder / subsystem
            └── file
```

Symbol-level nodes (functions, types, traits) are out of POC scope. The schema should accommodate them without restructuring.

### Edge Types (POC)

| Edge Kind | Description | POC? |
|-----------|-------------|------|
| `contains` | Parent-child hierarchy | Yes |
| `imports` | File-to-file import | Yes |
| `re_exports` | Barrel re-export | Yes |

Additional edge types for post-POC: `exports`, `defines`, `references`, `implements`, `calls`, `depends_on_runtime`, `owned_by`, `changed_in`, `predicted_change`, `violates_rule`.

### Serialization Format (Rust → Frontend)

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub kind: NodeKind,        // "package" | "module" | "file"
    pub parent_id: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: EdgeKind,        // "imports" | "re_exports"
}
```

**Critical ordering constraint:** Parent nodes MUST appear before their children in the serialized array. React Flow throws "Parent node not found" runtime error on violation. Topological sort the petgraph output before serialization.

### Stable Node Identity (MVP)

For watch mode, PR overlays, and snapshot comparison, nodes need stable IDs across rescans. Recommended scheme:
- `{repo_root_hash}:{language}:{entity_kind}:{relative_path}:{symbol_fqn}`
- Hash fallback for synthetic group nodes

### Edge Evidence Model (Post-POC)

Every edge should eventually carry:
```json
{
  "kind": "imports",
  "confidence": "syntactic",
  "source": "tree-sitter",
  "evidence": { "file": "src/lib.rs", "range": { "start": [18, 4], "end": [18, 19] } },
  "version": "scan-000142"
}
```

Confidence levels: `heuristic` < `syntactic` < `semantic` < `runtime` < `user`.

### Risks and Mitigations
- **petgraph has no built-in graph diff:** Implement custom `GraphDiff` struct (added/removed/modified nodes and edges) using content-addressed identity.
- **StableGraph memory overhead:** Stable indices use `Option<T>` slots. Acceptable for expected graph sizes.

---

## 5. IPC and Type Safety

### Context
The Tauri IPC bridge connects Rust (graph logic) to React (rendering). Graph payloads, scan progress updates, and user commands all cross this boundary. Type mismatches at this boundary cause runtime errors, not compile errors.

### Decision
- **`invoke`** for request/response commands (graph payloads <100KB JSON)
- **`Channel<T>`** for streaming (scan progress, graph deltas during rescan)
- **`tauri-specta` v2** for type-safe bindings (generates TypeScript from Rust command signatures)
- **`serde(rename_all = "camelCase")`** on all DTOs to bridge Rust `snake_case` → TypeScript `camelCase`

### Evidence
- Tauri commands are JSON-RPC. `invoke()` serializes via `serde_json`.
- Known IPC overhead: ~6x from multiple serde passes on large payloads (issue #5641). For graph payloads <100KB, this is acceptable.
- `Channel<T>` provides ordered streaming from Rust to frontend. Use for codebase scanning pipeline (progress + incremental node discovery). No built-in cancellation — use `AtomicBool` cancel flag or Channel drop from frontend.
- `tauri-specta` (v2.0.0-rc.21) generates TypeScript bindings from Rust command signatures. Must be added at project init — painful to retrofit.
- Events (`app.emit()`) evaluate JS directly — use for signals only, not data streaming.

### Risks and Mitigations
- **`State<'_, T>` type mismatch:** Causes runtime panic, not compile error. Use type aliases to reduce risk.
- **`tauri-specta` maturity:** Still at rc.21. Can fall back to manual TypeScript types if specta causes friction. Specta is nice-to-have, not a hard dependency.
- **Raw bytes path:** `tauri::ipc::Response` bypasses JSON (~5ms/10MB on macOS, ~200ms on Windows). Available if graph payloads grow beyond what JSON IPC can handle.

---

## 6. State Management (Frontend)

### Context
The frontend needs to manage: React Flow node/edge state, viewport position, selected node, expand/collapse state, filter settings, search state, and detail panel content.

### Decision
**Deferred to implementation planning.** The PRD explicitly left this as a plan-phase decision. Options include zustand, useReducer, or Jotai — all are reasonable.

### Constraints (from research)
- Graph state should be derivable from the Rust-side graph (source of truth). Frontend state is a projection.
- Expand/collapse state lives in `node.data.isExpanded`. Toggling requires spreading node objects (mutation won't trigger React re-render).
- Viewport-only state stays local to the frontend. No need to round-trip viewport position through Rust.
- Avoid storing selected nodes in the main nodes array — use a separate state slice to minimize re-renders.

---

## 7. Layout Strategy

### Context
ELK.js computes the positions of all nodes (including compound container sizing). The question is whether layout should happen in Rust or JavaScript, and how to handle the impedance mismatch between React Flow's flat array model and ELK's nested tree model.

### Decision
**Layout in JavaScript (ELK.js in a Vite Web Worker).** Layout cannot happen in Rust.

### Evidence
React Flow must render nodes in the DOM before their pixel dimensions are known. ELK needs those dimensions to compute positions. The pipeline is:

```
1. Rust: build petgraph, topological sort, serialize {nodes, edges} JSON
2. React: deserialize, render nodes (DOM measures sizes via node.measured.width/height)
3. ELK.js (Web Worker): compute positions + parent container sizes
4. React Flow: apply positions, fitView()
```

For static-size nodes (all packages 200×60, all files 150×40), step 2 can be skipped — provide sizes directly to ELK. For dynamic/content-driven nodes, use `useNodesInitialized()` hook to wait for DOM measurement.

### Flat ↔ Hierarchical Transform
- React Flow: flat array with `parentId` references
- ELK: nested `children` tree
- Transform is ~100 lines, well-documented (xyflow discussion #3495)
- After ELK layout, parent node `width`/`height` from ELK output must be explicitly applied to React Flow via `setNodes` — React Flow won't auto-resize containers from ELK output alone.

### Key ELK Options

```typescript
const elkOptions = {
  'elk.algorithm': 'layered',
  'elk.direction': 'DOWN',
  'elk.hierarchyHandling': 'INCLUDE_CHILDREN',  // CRITICAL for cross-boundary edges
  'elk.layered.spacing.nodeNodeBetweenLayers': '80',
  'elk.spacing.nodeNode': '40',
  // Per compound node:
  'elk.padding': '[top=30, left=20, bottom=20, right=20]',
  'elk.nodeSize.constraints': 'MINIMUM_SIZE PORTS NODES',
};
```

### Risks and Mitigations
- **ELK computation time:** Expect 200-800ms for 500 nodes on modern hardware. Async in Worker, no UI block. Debounce rapid toggles.
- **ELK has no incremental layout** (GitHub issue #100: explicitly "out of scope"). Use `elk.interactiveLayout: true` with `elk.position` on unchanged nodes to preserve stability during re-layout.
- **Initial render flash:** One-time flash while DOM measures → ELK computes → positions apply. Mitigate with loading indicator or fixed-size nodes.

---

## 7b. File Watching and Incremental Updates (MVP)

### Context
For the graph to stay current as developers edit code, the app needs filesystem watching with incremental rescan. This must be reliable, debounced, and respect `.gitignore`.

### Decision
- **Default watcher:** `notify` v8.2.0 (stable) — standard Rust crate used by rust-analyzer, alacritty, deno, mdBook. FSEvents on macOS, inotify on Linux, ReadDirectoryChangesW on Windows.
- **Debouncing:** `notify-debouncer-full` — adds rename event matching and path updating for editors that do atomic saves (write temp → rename). **Recommended debounce window: 300-500ms.**
- **`.gitignore` handling:** `ignore` crate (by BurntSushi/ripgrep author, 80M+ downloads). Fast recursive directory iteration respecting `.gitignore`, `.ignore`, `.git/info/exclude`. `WalkParallel` for parallel traversal.
- **Large-repo option:** **Watchman** (Meta) — not just a watcher but a file watching service with source-control-aware query support, including minimized change reasoning for large repos. Relevant for rebases, stacked branches, and minimizing noisy update sets. Use as an optional backend when available, with `notify` as the default fallback.

### Incremental Parse Pipeline
```
File change (notify + debouncer-full, 300ms)
  → Filter via ignore crate (.gitignore)
  → Incremental tree-sitter reparse (sub-ms via tree.edit() + parser.parse(new_source, Some(&old_tree)))
  → Extract imports/exports from changed ASTs
  → Compute GraphDiff against petgraph StableGraph
  → Apply diff to StableGraph
  → Send GraphDiff via Tauri Channel
  → Frontend: apply diff to React Flow state
  → ELK re-layout (Web Worker, interactiveLayout:true, previous positions)
  → Animate to new positions + diff highlights
```

### Animation Stack (for graph updates)

| Layer | Tool | Use For |
|-------|------|---------|
| Position interpolation | `d3-timer` | Node position animation (React Flow compatible) |
| Enter/exit animation | `Motion` (Framer Motion) v12 | Node appear/disappear with `AnimatePresence`. WAAPI-based for 120fps. Use only for node *content* animations — Motion's layout animations conflict with React Flow position management (xyflow Discussion #2995). |
| Visual state changes | CSS transitions | Border, background, opacity (GPU-composited) |
| Highlight effects | CSS `@keyframes` | Glow/pulse (use sparingly — `box-shadow` triggers paint) |

### Diff Color Coding
- Green: added nodes/edges
- Red: removed nodes/edges
- Amber: modified (content changed, node still exists)
- Blue/dim: unchanged but affected (connected to changed nodes)
- Gray: unchanged and unaffected

---

## 8. Persistence

### Context
The POC runs entirely in-memory — the graph is rebuilt from source on every scan. For MVP and beyond, persistence enables snapshots, search indexing, export provenance, and faster restarts.

### Decision
- **POC:** No persistence. In-memory petgraph, rebuilt on scan.
- **MVP:** SQLite + FTS5.

### Evidence
SQLite is the strongest default for a local-first application:
- Local-first storage with zero server infrastructure
- Snapshots (graph versions across rescans/branches)
- Configuration persistence
- FTS5 for full-text search (file names, symbol names, paths, summaries) — built into SQLite, tunable for footprint
- Export metadata (provenance timestamps, evidence links)

### Search Implementation
- **Rust-side fuzzy search:** `nucleo` (used by Helix editor). Modified Smith-Waterman algorithm, ~6x faster than skim, handles Unicode graphemes correctly.
- **Frontend command palette:** `cmdk` — unstyled, composable React command palette component with built-in fuzzy filtering, keyboard navigation, and grouping. Standard for Cmd+K interfaces.

### Post-MVP Options
- **Tantivy:** Add only if search becomes a first-class subsystem requiring advanced ranking or larger full-text corpora.
- **DuckDB:** Attractive for cross-snapshot analytics and architecture drift reports. Not needed as canonical store.

---

## 9. Testing Strategy

### Context
Testability must be designed in from the start. POC decisions need evidence, not taste. macOS WKWebView has no WebDriver support, constraining E2E options.

### Decision
**3-layer strategy:**

| Layer | What | Tools | Phase |
|-------|------|-------|-------|
| **Rust unit + integration** | Graph algorithms, parsing, serialization | `mod tests`, proptest, rstest, criterion benchmarks | POC |
| **Rust IPC integration** | Command handlers via mock runtime | `tauri::test::mock_builder()`, `assert_ipc_response()` | POC |
| **Frontend unit** | Pure functions (transforms, filters, search), IPC wrappers | Vitest + jsdom, mockIPC, fast-check | POC |
| **Benchmarks** | Layout/parse performance at 10/100/1000 nodes | criterion (Rust), Vitest bench (JS) | POC |
| **Visual regression** | Layout correctness screenshots | Vitest browser mode + `toMatchScreenshot()` | Post-POC |
| **E2E** | Full app flows | Deferred (macOS WKWebView has no driver) | Deferred |

### Evidence
- **Rust tests are highest value.** Graph algorithms, parsing, and serialization are pure Rust — highly testable without any Tauri dependency.
- **proptest** generates arbitrary graph inputs and auto-shrinks failures. Use for graph invariants (no orphan edges, parent ordering, SCC detection correctness).
- **criterion** benchmarks at 10/100/1000 nodes are the primary A/B comparison tool for layout algorithm evaluation.
- **rstest** with `#[case]` enables parameterized tests per layout algorithm.
- **mockIPC()** from `@tauri-apps/api/mocks` intercepts all frontend `invoke()` calls. Any `invoke()` call without `mockIPC()` first throws in jsdom — `window.__TAURI_INTERNALS__` doesn't exist.
- **Desktop E2E is blocked on macOS.** CrabNebula offers macOS support but requires paid subscription + `tauri-plugin-automation` in source. Skip for POC. Plan Linux/Windows CI for E2E later.

### Test Coverage Target
- **>80%** on non-UI Rust code (graph algorithms, parsing, serialization)
- **>80%** on frontend pure logic (transforms, filters, search, state management)
- **0%** visual/E2E for POC (reliance on unit tests + manual smoke tests)

---

## 10. Security Model

### Context
Code Atlas reads local source code. It must never expose that code to external services or grant unnecessary permissions.

### Decision
Minimal Tauri capabilities. Zero network calls.

### Specifics
- **Capabilities:** `core:default` + `dialog:allow-open` (native file dialog). Additional permissions (shell for VS Code integration) added only when the feature ships.
- **No permissions for:** shell (except VS Code bridge at MVP), fs write, process, opener (except VS Code URI at MVP).
- **No remote script loading.** All assets bundled locally.
- **Tight CSP** in Tauri config. No CDN-loaded scripts in the webview.
- **Rust backend reads directories directly** — the frontend has no filesystem access.

### Evidence
Tauri v2's capability system enforces permissions per window label. This is a stronger default security posture than Electron's "everything is allowed unless you restrict it."

---

## 11. VS Code Integration

### Context
VS Code is where most target users already work. The question is not whether to integrate but when and how deeply.

### Decision (Phased)

| Phase | Feature | Mechanism | Effort |
|-------|---------|-----------|--------|
| **POC stretch** | Click-to-open file in VS Code | `code -g file:line:col` via tauri-plugin-shell | Low |
| **MVP** | Thin companion extension: active file tracking, "Show in Code Atlas" | WebSocket bridge (tokio-tungstenite server in Tauri, client in extension) | Medium |
| **Post-MVP** | Semantic enrichment: symbol providers, call hierarchy | `vscode.executeDocumentSymbolProvider`, `vscode.prepareCallHierarchy` via WebSocket | High |

### Evidence
- Two mechanisms for opening files: `code -g` CLI flag, or `vscode://file/` URI scheme. Both work cross-platform.
- A companion extension enables richer integration: `onDidChangeActiveTextEditor` for file tracking, `vscode.executeReferenceProvider` for "find all usages", SCM API for quick diff gutter decorations.
- **VS Code webview is NOT suitable as primary canvas.** ZenML explicitly abandoned React Flow in a VS Code webview: "too heavy for an extension." Webview constraints: sandboxed iframes, no HMR, async postMessage only, context lost when hidden.
- **The Tauri app is the primary product. The extension is a bridge.**
- Precedent: GitKraken + GitLens, Docker Desktop + Docker extension, Figma + Figma for VS Code, Sourcegraph + Cody.

### Risks and Mitigations
- **VS Code extension sprawl:** Keep it thin and bridge-first. The extension should enhance when present, not be required. Code Atlas works fully standalone without the extension.

---

## 12. Agent/MCP Interoperability

### Context
Coding agents (Claude Code, Cursor, etc.) need structured repository context. Code Atlas's graph model is exactly the kind of data agents need to understand codebase structure, predict change impact, and make better decisions.

### Decision
- **POC:** Not in scope.
- **MVP:** Design graph query API with agent use in mind (clean, structured, JSON-serializable responses).
- **Post-MVP:** Ship a local MCP server exposing graph queries.

### Recommended MCP Capabilities (Post-MVP)

| Tool | Description |
|------|-------------|
| `get_architecture_overview()` | Top-level package graph |
| `get_dependencies(node_id)` | Direct imports/dependents |
| `get_blast_radius(file_path)` | Transitively affected files |
| `search_nodes(query)` | Fuzzy search across graph |
| `get_circular_dependencies()` | All dependency cycles |
| `get_impacted_slice(diff)` | Architecture slice for a diff/PR |
| `get_path(from, to)` | Shortest path between two nodes |

### Evidence
- CodeCanvas already markets MCP server integration in March 2026. This is becoming table stakes for developer tools.
- `tauri-mcp` crate exists on lib.rs. Known compatibility issue: Claude Desktop has disconnection issues with Rust-based MCP servers — Node.js wrapper workaround exists.
- The same data model that powers the UI can power CLI, JSON API, and MCP surfaces.

---

## Appendix A: Version Snapshot (March 17-18, 2026)

### NPM
| Package | Version |
|---------|---------|
| `@tauri-apps/cli` | 2.10.1 |
| `@tauri-apps/api` | 2.10.1 |
| `@xyflow/react` | 12.10.1 |
| `elkjs` | 0.11.1 |
| `typescript` | 5.9.3 |
| `vite` | 8.0.0 |
| `@biomejs/biome` | 2.4.7 |

### Cargo
| Crate | Version |
|-------|---------|
| `tauri` | 2.10.3 |
| `tree-sitter` | 0.26.7 |
| `petgraph` | 0.8.3 |
| `notify` | 8.2.0 (9.0.0-rc.2 available) |
| `notify-debouncer-mini` | 0.7.0 |
| `ignore` | 0.4.25 |
| `tauri-specta` | 2.0.0-rc.21 |

### System
| Tool | Version |
|------|---------|
| Rust stable | 1.94.0 (2026-03-05) |
| Node.js | 22+ LTS |
| pnpm | 10.x |
| Rust edition | 2021 (edition 2024 had tauri-build bug #10412; fix merged but unconfirmed in stable release) |

---

## Appendix B: Eliminated Technologies

| Technology | Reason | Decision |
|------------|--------|----------|
| Sigma.js + Graphology | No compound node support. Hard blocker for nested hierarchy. | Eliminated |
| Dagre | Open bug prevents correct subflow layout with external connections | Eliminated |
| vis-network | Abandoned upstream, poor TS/React support | Eliminated |
| Cytoscape.js | No React JSX inside nodes, weaker Vitest testability | Fallback only (>500 visible nodes) |
| `syn` for Rust parsing | Not error-tolerant (panics), not incremental | Eliminated (use tree-sitter) |
| Cosmograph | No compound nodes, flat graph only | Eliminated |
| deck.gl / Graph.gl | Not actively maintained, no compound nodes | Eliminated |
| Electron | Larger binaries, higher memory, no Rust-native core | Available as fallback if Tauri hits blockers |
| Wails v3 | Alpha-era in March 2026 | Too immature |
| Neutralino | Thin runtime, no Rust core | Insufficient for this product |

---

## Sources

All research was conducted March 17-18, 2026. Primary sources include official documentation for Tauri v2, React Flow v12, ELK.js, tree-sitter, petgraph, oxc, and VS Code API. Registry checks were run locally via `npm view` and `cargo search/info`. Competitor claims are based on public product pages, not hands-on trials. Full source lists are available in the original research files.
