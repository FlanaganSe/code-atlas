# Research: Tauri Network Code Visualization POC

**Date:** 2026-03-17
**Goal:** Build a Tauri v2 desktop app that visualizes code architecture as interactive, zoomable, nested graphs — like the screenshot showing packages/frontend → packages/api → services/auth with file-level detail inside each box.

---

## Executive Summary

**Recommended stack:**
- **Renderer:** `@xyflow/react` v12 (React Flow) — nodes are React components, compound nesting via `parentId`
- **Layout engine:** `elkjs` v0.11.1 in a Vite Web Worker — only engine that handles compound hierarchical graphs + port-based edge routing
- **Rust graph logic:** `petgraph` for graph construction/analysis + `syn` for Rust AST parsing
- **IPC:** Tauri `invoke` (JSON) for POC; upgrade to `Channel<T>` for streaming codebase scanning later
- **Type-safe IPC:** `tauri-specta` v2 — add at project init (painful to retrofit)
- **Testing:** 3-layer strategy — Rust (proptest + criterion benchmarks), frontend (mockIPC + Vitest jsdom), visual regression (deferred)

**Critical constraint:** Visual layout cannot happen in Rust. React Flow needs DOM-measured node sizes before ELK can compute positions.

---

## 1. Dev Environment Setup (macOS)

### Versions
| Tool | Version | Notes |
|------|---------|-------|
| Rust | 1.84.0 stable | aarch64-apple-darwin is Tier 1 (Apple Silicon native) |
| Tauri | 2.10.3 (crate), 2.10.1 (CLI/JS) | Stable since 2024-10-02 |
| Node.js | 22 LTS | Matches stack.md constraint |
| pnpm | 10.32.1 | Enable via `corepack enable` |
| Rust edition | **2021** | Edition 2024 has unconfirmed Tauri bug (#10412). Migrate later via `cargo fix --edition` |

### Installation Steps
```sh
# 1. Xcode CLI tools (hard prerequisite)
xcode-select --install

# 2. Rust via rustup
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
source "$HOME/.cargo/env"
rustup component add rustfmt clippy rust-analyzer

# 3. Node.js 22 LTS (via installer or fnm/nvm)
# 4. pnpm
corepack enable && corepack use pnpm@latest

# 5. Cargo tools
cargo install tauri-cli --version "^2" --locked
cargo install cargo-nextest --locked
cargo install cargo-audit --locked
cargo install --locked bacon   # replaces archived cargo-watch
```

### Verification Script
Save as `scripts/check-env.sh`:
```sh
#!/usr/bin/env bash
set -euo pipefail
PASS=0; FAIL=0
check() {
  local label="$1"; local cmd="$2"; local min_ver="$3"
  if version=$(eval "$cmd" 2>/dev/null); then
    echo "  [OK] $label: $version"; PASS=$((PASS+1))
  else
    echo "  [FAIL] $label: not found (need $min_ver)"; FAIL=$((FAIL+1))
  fi
}
echo "=== Rust / Cargo ==="
check "rustc"         "rustc --version"          "1.77.2+"
check "cargo"         "cargo --version"          "1.77.2+"
check "rustfmt"       "rustfmt --version"        "any"
check "clippy"        "cargo clippy --version"   "any"
check "rust-analyzer" "rust-analyzer --version"  "any"
check "cargo-nextest" "cargo nextest --version"  "any"
check "cargo-audit"   "cargo audit --version"    "any"
check "bacon"         "bacon --version"          "any"
check "tauri-cli"     "cargo tauri --version"    "2.x"
echo ""
echo "=== Node / pnpm ==="
check "node"          "node --version"           "22+"
check "pnpm"          "pnpm --version"           "10+"
echo ""
echo "=== macOS System ==="
check "xcode-select"  "xcode-select -p"          "any"
echo ""
echo "=== Summary: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ] && echo "All prerequisites met." || echo "Fix the failures above."
```

### macOS Gotchas
- Xcode must be launched once after install (license agreement)
- `~/.cargo/bin` must be on PATH (rustup patches `~/.zshrc` but needs new shell)
- Rosetta NOT required on Apple Silicon
- Intel Mac: `x86_64-apple-darwin` demoted to Tier 2 (Aug 2025) — still works but reduced CI coverage

---

## 2. Tauri v2 Platform Details

### WebView Engines
| Platform | Engine | Capability |
|----------|--------|------------|
| macOS | WKWebView (WebKit) | Safari 17+ — WebGL2, Canvas 2D |
| Windows | WebView2 (Chromium) | Modern Chrome — WebGL2, WebGPU (flag) |
| Linux | WebKit2GTK | Safari ~14 — WebGL2, Canvas 2D |

WebGL2 is the safe rendering ceiling across all platforms.

### IPC Architecture
- Commands are JSON-RPC. `invoke()` serializes via `serde_json`
- **Known issue:** ~6x overhead from multiple serde passes on large payloads ([#5641](https://github.com/tauri-apps/tauri/issues/5641))
- **Channels:** `Channel<T>` for ordered streaming (Rust → frontend). Use for codebase scanning pipeline
- **Raw bytes:** `tauri::ipc::Response` bypasses JSON — ~5ms/10MB on macOS, ~200ms on Windows
- **Events:** `app.emit()` evaluates JS directly — signals only, not data streaming

**POC strategy:** Use `invoke` for graph payloads (<100KB JSON). Add Channels when scanning pipeline is built.

### Capability / Permission Model
Capabilities live in `src-tauri/capabilities/*.json`:
- `windows` — window labels (or `["*"]`)
- `permissions` — plugin permission strings (e.g., `"fs:default"`)
- Security boundaries enforced by **window label**, not title

### State Management
- Tauri wraps managed state in `Arc` — do NOT add your own `Arc`
- Use `Mutex<T>` for interior mutability (`std::sync::Mutex` unless holding across `.await`)
- **Gotcha:** `State<'_, T>` type mismatch = **runtime panic**, not compile error. Use type aliases.

```rust
app.manage(Mutex::new(AppState::default()));

#[tauri::command]
fn get_graph(state: State<'_, Mutex<AppState>>) -> GraphPayload {
    state.lock().unwrap().graph.to_payload()
}
```

### Type-Safe IPC: tauri-specta
`tauri-specta` v2 generates TypeScript bindings from Rust command signatures. Full autocomplete + compile-time IPC checking. **Must be added at project init** — painful to retrofit.

### Canonical Project Structure
```
├── package.json / index.html
├── src/                        # React frontend
└── src-tauri/
    ├── Cargo.toml
    ├── build.rs
    ├── tauri.conf.json
    ├── src/
    │   ├── main.rs             # thin: calls app_lib::run()
    │   └── lib.rs              # all setup, manage(), generate_handler!
    ├── icons/
    └── capabilities/
        └── default.json
```

`main.rs` must be thin — all Tauri config in `lib.rs` (mobile compatibility).

### Channel Streaming Pattern
```rust
#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum AnalysisUpdate {
    NodeDiscovered { node: GraphNode },
    Done { stats: AnalysisStats },
}

#[tauri::command]
async fn analyze_codebase(
    path: String,
    on_progress: Channel<AnalysisUpdate>,
) -> Result<(), String> {
    for node in scan_files(&path) {
        on_progress.send(AnalysisUpdate::NodeDiscovered { node })?;
    }
    on_progress.send(AnalysisUpdate::Done { stats })?;
    Ok(())
}
```

No built-in cancellation — use `AtomicBool` cancel flag or drop the Channel from frontend.

---

## 3. Graph Visualization Libraries

### Recommendation: React Flow + ELK.js

| Component | Choice | Why |
|-----------|--------|-----|
| Renderer | `@xyflow/react` v12 | Nodes are React components, compound nesting via `parentId`, 35K stars, MIT |
| Layout | `elkjs` v0.11.1 | Only engine with compound graph + port-based edge routing. Run in Web Worker |
| Rust graph | `petgraph` | Canonical Rust graph crate. StableGraph, topo sort, SCC detection |
| Code parsing | `syn` (Rust-only) or `tree-sitter` (multi-lang) | AST extraction for building the graph |

### Why React Flow + ELK?
1. **Compound nodes** — `parentId` + `extent: 'parent'` maps directly to packages-containing-files
2. **Custom node rendering** — arbitrary React JSX (file paths, metrics, expand/collapse controls)
3. **Testability** — nodes are React components (Vitest + RTL); ELK calls are pure async functions
4. **Layout flexibility** — ELK supports `layered` (DAGs), `mrtree`, `radial`, `force` algorithms
5. **Scale strategy** — expand/collapse limits visible nodes to ~200; full graph lives in petgraph

### What was eliminated
| Library | Reason |
|---------|--------|
| dagre | No compound/nested graph support — hard blocker |
| Sigma.js | No compound nodes at all |
| vis-network | Abandoned upstream, poor TS/React support |
| Cytoscape.js | Can't render React JSX inside nodes; weaker Vitest testability. Fallback if >500 simultaneous nodes needed |

### Data Flow Pipeline (Critical Architecture)
```
Rust backend
  parse source (syn/tree-sitter)
  build petgraph::StableGraph
  serialize {nodes, edges} JSON
        |  Tauri invoke
        v
React frontend
  deserialize → render nodes (DOM sizes measured)
        |  pass measured sizes
        v
ELK.js (Web Worker)
  compute positions + edge bend points (compound-aware)
        |  Promise resolves
        v
React Flow
  apply positions → rendered layout
```

### Recommended Data Format

**Rust (serde):**
```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub kind: NodeKind,       // "package" | "file" | "service" | "module"
    pub parent_id: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: EdgeKind,       // "depends_on" | "imports" | "calls"
}
```

**TypeScript (immutable):**
```typescript
type NodeKind = "package" | "file" | "service" | "module";
type EdgeKind = "depends_on" | "imports" | "calls";

type RawGraphNode = {
  readonly id: string;
  readonly label: string;
  readonly kind: NodeKind;
  readonly parentId?: string;
  readonly metadata?: Record<string, unknown>;
};

type GraphPayload = {
  readonly nodes: readonly RawGraphNode[];
  readonly edges: readonly RawGraphEdge[];
};
```

---

## 4. Testing Strategy

### 3-Layer Approach

```
E2E (skip for POC)    — tauri-driver Linux-only; macOS WKWebView has no driver
                            |
Integration            — Rust: tauri::test mock runtime
                       — Frontend: mockIPC() + Vitest jsdom
                            |
Unit                   — Rust: mod tests (pure graph algorithms)
                       — Frontend: pure functions (zoom reducer, data transforms)
                            |
Benchmarks             — Rust: criterion at 10/100/1000 nodes
```

### Layer 1: Rust (Highest Value)
- **Unit tests:** `mod tests` for graph algorithms — no Tauri dependency
- **Property tests:** `proptest` generates arbitrary graph inputs, auto-shrinks failures
- **IPC integration:** `tauri::test::mock_builder()` + `assert_ipc_response()` — headless on macOS
- **Benchmarks:** `criterion` at 10/100/1000 nodes — **primary A/B comparison tool**
- **Parameterized:** `rstest` with `#[case]` per layout algorithm behind a shared `LayoutAlgorithm` trait

```rust
#[rstest]
#[case(ForceDirected::default())]
#[case(Dagre::default())]
fn all_layouts_position_all_nodes(#[case] layout: impl LayoutAlgorithm) {
    let graph = make_test_graph(10);
    let result = layout.compute(&graph, &LayoutConfig::default());
    assert_eq!(result.positions.len(), 10);
}
```

### Layer 2: Frontend (jsdom, Fast)
- `mockIPC()` from `@tauri-apps/api/mocks` intercepts all `invoke()` calls
- Extract testable logic as pure functions: zoom reducer, data transforms, layout normalization
- `toMatchInlineSnapshot()` for structural regression on graph data shapes
- `test.each()` for parameterized graph configs
- `fast-check` for property-based testing of data transformations

**Gotcha:** Any `invoke()` call without `mockIPC()` first throws in jsdom — `window.__TAURI_INTERNALS__` doesn't exist.

```typescript
// vitest.config.ts
export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test-setup.ts'],
    globals: true,
  },
})

// src/test-setup.ts
import { beforeEach } from 'vitest'
import { clearMocks } from '@tauri-apps/api/mocks'
beforeEach(() => { clearMocks() })
```

### Layer 3: Visual Regression (Deferred)
Vitest 4.0 (Oct 2025) stabilized Browser Mode with `toMatchScreenshot()`. Add when a layout algorithm is selected for production. Two Vitest configs needed (jsdom + browser mode).

### CI Matrix (POC)
| Job | Runs on | What |
|-----|---------|------|
| `cargo test` | macOS + Linux | Rust unit + integration (headless) |
| `cargo bench` | Linux | Algorithm benchmarks |
| `pnpm test` | macOS + Linux | Frontend jsdom tests |

---

## 5. Tauri-Specific Constraints

1. **`main.rs` must be thin** — all setup in `lib.rs` for mobile compatibility
2. **No built-in command cancellation** — use `AtomicBool` or Channel drop
3. **`State<'_, T>` type mismatch = runtime panic** — use type aliases
4. **Layout cannot happen in Rust** — React Flow needs DOM pixel dimensions
5. **IPC JSON overhead >100KB** — use Channels for scanning pipelines
6. **Windows binary IPC regression** — ~200ms/10MB vs ~5ms on macOS

---

## 6. Cargo.toml Template

```toml
[package]
name = "tauri-poc-zoom-thing"
version = "0.1.0"
edition = "2021"
rust-version = "1.77.2"

[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
strip = true

[dev-dependencies]
proptest = "1"
criterion = { version = "0.5", features = ["html_reports"] }
rstest = "0.24"

[[bench]]
name = "layout_bench"
harness = false
```

---

---

## 7. React Flow v12 Compound Nodes — Deep Dive

**Date:** 2026-03-17
**Scope:** Compound node mechanics, ELK integration specifics, expand/collapse patterns, Pro features, and render-at-scale strategy.

---

### 7.1 Compound Node Mechanics (`parentId` + `extent: 'parent'`)

**How it works:**
- Any node becomes a container by assigning `parentId` on child nodes. The child's `position` is relative to the parent's top-left (`{x:0, y:0}`).
- When a parent moves, all children move with it automatically.
- `extent: 'parent'` constrains drag to within the parent boundary. Without it, children can be dragged outside the parent visually but remain logically parented.
- `expandParent: true` on a child automatically enlarges the parent container when the child is dragged to the border — no built-in collapse/shrink.

**Hard ordering constraint:** Parent nodes MUST appear before their children in the `nodes` array. Violation throws a "Parent node not found" runtime error (not a compile error). This is a documented, enforced invariant — issue #4438 was closed as "not a bug." When building the array from a petgraph traversal, do a topological sort before serialization.

**Node type flexibility:** Any node type can be a parent — including `type: 'group'` (no handles, pure container) or custom types. The `group` type is the idiomatic choice for packages/modules.

**Edge rendering:** Edges between nodes that share a common ancestor render above all nodes by default. Cross-boundary edges (child → top-level) also render above. Control with `zIndex` in `defaultEdgeOptions`.

**v12 change:** `node.parentNode` renamed to `node.parentId` (was deprecated since 11.11.0, fully removed in v12). Node dimensions after measurement are now in `node.measured.width` / `node.measured.height` — NOT `node.width`/`node.height`. Layout libraries (dagre, ELK) must read from `.measured`.

---

### 7.2 Multi-Level Nesting (package → module → file)

**Confirmed working up to 3 levels deep** from the official sub-flows example:

```
Group '4' (package)            type: 'group', 300×300px
  Node '4a' (file)             parentId: '4', extent: 'parent'
  Group '4b' (module)          parentId: '4'
    Node '4b1' (file)          parentId: '4b'
    Node '4b2' (file)          parentId: '4b'
```

Edges across nesting levels work (e.g., `'3'` → `'4b'` is an inter-level edge). No documented depth limit beyond the ordering constraint. Three levels (package → module → file) is directly supported.

**Practical constraint:** Each additional nesting level multiplies the flat-to-hierarchical conversion complexity for ELK. Plan for a transform function that recursively builds the ELK `children` tree from React Flow's flat `parentId` references.

---

### 7.3 Expand / Collapse Pattern

**The Pro example** uses `useExpandCollapse` hook (closed-source, requires Pro subscription). It uses **dagre** for relayout, tracks state in `node.data`, and uses `node.hidden` to hide children — not deletion.

**The open pattern (reconstructed from docs + discussions):**

```typescript
// Core primitive: node.hidden affects both node and its edges
const hideDescendants = (nodes: Node[], collapsedId: string): Node[] =>
  nodes.map((n) =>
    isDescendantOf(n, collapsedId, nodes)
      ? { ...n, hidden: true }   // spread required — mutation won't trigger re-render
      : n
  );

// After toggling hidden, trigger ELK relayout on visible nodes only
// ELK receives only non-hidden nodes in its children arrays
```

**Layout after collapse:** The Pro example uses dagre for relayout post-collapse. For ELK + compound nodes, you must re-run ELK with only the visible nodes (non-hidden) after each toggle. This is an async operation — show a loading state or debounce rapid toggles.

**State management:** Track `isExpanded: boolean` in `node.data`. The expand/collapse button lives inside the custom node component. On click: update `node.data.isExpanded`, re-compute the `hidden` flag for all descendants, trigger ELK relayout.

**Performance note:** Keep simultaneously-visible nodes under ~200 (prior research constraint still applies). Collapse at the package level first, expand on demand.

---

### 7.4 React Flow + ELK.js Integration — Specifics

**The fundamental impedance mismatch:** React Flow uses a **flat array with `parentId` references**. ELK requires a **nested `children` tree**. You must write a flat↔hierarchical encoder/decoder. This is non-trivial but well-understood (community discussion #3495 documents the pattern).

**Flat → ELK conversion:**
```typescript
// Pseudo-code for the critical transform
function toElkGraph(nodes: Node[], edges: Edge[]): ElkNode {
  const rootNodes = nodes.filter(n => !n.parentId);
  const toElk = (node: Node): ElkNode => ({
    id: node.id,
    width: node.measured?.width ?? 150,
    height: node.measured?.height ?? 50,
    layoutOptions: node.data.elkLayoutOptions,
    children: nodes
      .filter(n => n.parentId === node.id)
      .map(toElk),
    edges: edges
      .filter(e => e.source === node.id || e.target === node.id) // inner edges only
      .map(toElkEdge),
  });
  return {
    id: 'root',
    children: rootNodes.map(toElk),
    edges: edges.filter(isRootLevelEdge).map(toElkEdge),
  };
}
```

**ELK → React Flow conversion (critical v12 detail):**
After `elk.layout()` resolves, walk the nested result and reconstruct the flat array with ELK-computed `x,y` as React Flow `position`. Parent nodes get their `width`/`height` from ELK's output (ELK auto-sizes compound nodes based on children + padding).

**The sizing problem:** React Flow must render nodes before ELK can know their sizes (for dynamic/content-driven nodes). The hook pattern is:
1. Render with placeholder positions
2. `useNodesInitialized()` returns `true` (all nodes measured)
3. Read `node.measured.width` / `node.measured.height`
4. Run ELK layout
5. Apply positions → `fitView()`

This is a one-time render flash. For static-size nodes (all packages 200×60, all files 150×40) you can skip step 2 and provide sizes directly to ELK.

**Known ELK + compound node issues:**
- Render order of branches within a compound node is non-deterministic without explicit `elk.priority` layout options (discussion #4830 — no maintainer resolution)
- Edge routing across hierarchy boundaries ("cross-hierarchy edges") requires `'elk.hierarchyHandling': 'INCLUDE_CHILDREN'` at the root level — without this, inter-level edges are ignored by ELK
- Parent node sizing from ELK output must be explicitly applied back to React Flow (`setNodes` with width/height from ELK result) — React Flow won't auto-resize containers from ELK output alone

**Key ELK options for compound architecture diagrams:**
```typescript
const elkOptions = {
  // Root graph
  'elk.algorithm': 'layered',
  'elk.direction': 'DOWN',
  'elk.hierarchyHandling': 'INCLUDE_CHILDREN',  // CRITICAL for cross-boundary edges
  'elk.layered.spacing.nodeNodeBetweenLayers': '80',
  'elk.spacing.nodeNode': '40',

  // Per compound node (package/module)
  'elk.padding': '[top=30, left=20, bottom=20, right=20]',  // header space
  'elk.nodeSize.constraints': 'MINIMUM_SIZE PORTS NODES',
};
```

**Bundle size:** ELK.js is ~1.45MB uncompressed (vs dagre's ~39KB). Running it in a Vite Web Worker is non-negotiable for keeping the main thread responsive.

**Performance with 100–500 compound nodes:** No official benchmarks exist for compound-node scenarios. The official stress test uses 450 flat nodes (15×30 grid) and demonstrates smooth interaction. For compound nodes:
- ELK computation time scales with total node+edge count — expect 200–800ms for 500 nodes on modern hardware (async in Worker, so no UI block)
- React Flow rendering itself should handle 200 visible nodes comfortably; 500 visible may show frame drops on complex custom node components
- Mitigation: expand/collapse to keep simultaneously-rendered node count under 200

---

### 7.5 Hybrid Strategy: React Flow + Canvas/WebGL for Dense Views

**The question:** Should we use React Flow for high-level view and Pixi.js/WebGL for dense file-level detail?

**Short answer: Not for the POC. Revisit if >300 nodes are simultaneously visible.**

**Reasoning:**

React Flow's stress test demonstrates 450 nodes at interactive framerates with simple node styles. The bottleneck is React re-renders, not canvas drawing. Key mitigation strategies available without switching to WebGL:

1. `node.hidden` collapses entire subtrees — keeps visible count low
2. Custom node memoization (`memo()` + stable `data` references) cuts re-renders
3. Avoid storing selected nodes in the main nodes array (separate state slice)
4. Simple CSS on nodes (avoid animations, box-shadows, gradients) — styling is the primary render cost

**When WebGL becomes justified:** If simultaneously-visible nodes exceed ~500 with complex custom renderers, or if file-level detail requires rendering hundreds of text labels, a Pixi.js canvas layer becomes worthwhile. The recommended pattern would be a zoom-level split:
- **Zoom out:** React Flow showing packages + inter-package edges only
- **Zoom in (per package):** Pixi.js canvas rendering file-level detail on demand

This hybrid would require a custom `useZoomLevel` hook and conditional renderer swapping — significant complexity for the POC phase. Defer unless performance proves insufficient.

**Cytoscape.js** remains the fallback if >500 simultaneous nodes are needed (WebGL rendering via the `cytoscape-renderer-canvas` approach), but it loses JSX-inside-nodes and Vitest RTL testability.

---

### 7.6 React Flow Pro Features — Architecture Visualization Relevance

**Included in `@xyflow/react` (free, MIT):**
- `<MiniMap>` — bird's-eye view, supports `nodeColor` function per node type. Clickable + pannable with `pannable` / `zoomable` props. Renders outside the viewport, ideal for architecture nav.
- `<Controls>` — zoom in/out, fit view, lock/unlock interaction
- `<Background>` — dot/grid/cross patterns. v12 adds `patternClassName` for Tailwind styling.
- `<Panel>` — fixed overlays at 9 positions (top-left, top-center, top-right, etc.)
- `<NodeResizer>` / `<NodeResizeControl>` — drag handles on nodes. v12 fix: children no longer move on parent resize (extent+expand corrected)
- `<NodeToolbar>` — floating toolbar above nodes (good for expand/collapse buttons)
- `<ViewportPortal>` — render elements in viewport coordinate space without a custom node
- `useConnection` — access active in-progress connection (for edge-drawing UX)
- Dark mode via `colorMode` prop (`'light' | 'dark' | 'system'`), all default styles use CSS variables

**Pro examples (paid, not open source):**
- "Expand and Collapse" — `useExpandCollapse` hook + dagre relayout (most relevant)
- "Auto Layout" — `useAutoLayout` hook switching between dagre/elkjs/d3-force at runtime
- "Parent Child Relation" — drag nodes into containers with detach button
- "Dynamic Layouting" — vertical tree with placeholder nodes

**Assessment:** The free tier has everything needed for the POC. The Pro "Expand and Collapse" example would save implementation time but the pattern is reconstructible from docs (as described in §7.3). Pro subscription may be worthwhile if the expand/collapse implementation stalls.

---

### 7.7 Options & Recommendation for Compound Node Implementation

**Option A: React Flow flat nodes + ELK Web Worker (recommended)**
- Full compound nesting (3 levels confirmed), custom JSX nodes, `MiniMap`, `NodeToolbar`
- Requires: flat↔hierarchical encoder/decoder for ELK, `useNodesInitialized` timing, topological node ordering from Rust
- Trade-offs: ELK bundle size (1.45MB, Worker-contained), initial render flash, cross-boundary edge routing needs `hierarchyHandling: INCLUDE_CHILDREN`
- Risk: ELK branch ordering non-determinism in compound nodes (workaround: add `elk.priority` options)

**Option B: React Flow with dagre for layout, manual compound positioning**
- Dagre is 39KB vs ELK's 1.45MB, simpler integration
- Hard blocker: dagre has an open bug preventing correct subflow layout when any node in the subflow connects to a node outside it (cross-boundary edges break layout). This is exactly the package→service cross-package dependency case.
- Verdict: Not viable for this use case.

**Option C: Hybrid React Flow (overview) + Pixi.js canvas (file detail)**
- Handles scale beyond 500 visible nodes
- Much higher implementation complexity: dual renderer, zoom-level routing, event coordination
- Not justified for POC; revisit if React Flow performance proves insufficient at target graph sizes

**Recommendation: Option A.** The ELK encoder/decoder is a one-time investment of ~100 lines. The `hierarchyHandling: INCLUDE_CHILDREN` option resolves cross-boundary edge routing. The flat↔hierarchical problem is well-documented and solvable. All three nesting levels (package → module/service → file) are confirmed to work in React Flow v12. The POC's expand/collapse strategy (keep <200 visible nodes) keeps React Flow rendering comfortably within performance bounds.

---

## Sources

### Tauri
- [Tauri v2 Release](https://v2.tauri.app/release/)
- [IPC Concepts](https://v2.tauri.app/concept/inter-process-communication/)
- [Calling Rust](https://v2.tauri.app/develop/calling-rust/)
- [Calling Frontend (Channels)](https://v2.tauri.app/develop/calling-frontend/)
- [State Management](https://v2.tauri.app/develop/state-management/)
- [Capabilities](https://v2.tauri.app/security/capabilities/)
- [WebView Versions](https://v2.tauri.app/reference/webview-versions/)
- [Project Structure](https://v2.tauri.app/start/project-structure/)
- [Tests & Mocking](https://v2.tauri.app/develop/tests/)
- [tauri-specta](https://github.com/specta-rs/tauri-specta)
- [IPC overhead #5641](https://github.com/tauri-apps/tauri/issues/5641)
- [Edition 2024 bug #10412](https://github.com/tauri-apps/tauri/issues/10412)

### Graph Visualization
- [React Flow performance](https://reactflow.dev/learn/advanced-use/performance)
- [React Flow testing](https://reactflow.dev/learn/advanced-use/testing)
- [React Flow ELK example](https://reactflow.dev/examples/layout/elkjs)
- [elkjs GitHub](https://github.com/kieler/elkjs)
- [ELK layered algorithm](https://eclipse.dev/elk/reference/algorithms/org-eclipse-elk-layered.html)
- [Cytoscape.js WebGL preview](https://blog.js.cytoscape.org/2025/01/13/webgl-preview/)
- [petgraph](https://crates.io/crates/petgraph)
- [syn](https://crates.io/crates/syn)
- [React Flow v12 sub-flows guide](https://reactflow.dev/learn/layouting/sub-flows)
- [React Flow sub-flows example](https://reactflow.dev/examples/grouping/sub-flows)
- [React Flow expand-collapse Pro example](https://reactflow.dev/examples/layout/expand-collapse)
- [React Flow hidden nodes](https://reactflow.dev/examples/nodes/hidden)
- [React Flow ELK example](https://reactflow.dev/examples/layout/elkjs)
- [React Flow ELK multiple handles](https://reactflow.dev/examples/layout/elkjs-multiple-handles)
- [React Flow performance guide](https://reactflow.dev/learn/advanced-use/performance)
- [React Flow stress test (450 nodes)](https://reactflow.dev/examples/nodes/stress)
- [React Flow built-in components](https://reactflow.dev/learn/concepts/built-in-components)
- [React Flow v12 migration guide](https://reactflow.dev/learn/troubleshooting/migrate-to-v12)
- [React Flow v12 release blog](https://xyflow.com/blog/react-flow-12-release)
- [React Flow Pro examples](https://reactflow.dev/pro/examples)
- [React Flow layouting overview](https://reactflow.dev/learn/layouting/layouting)
- [React Flow NodeResizer API](https://reactflow.dev/api-reference/components/node-resizer)
- [xyflow issue #4438 (node ordering constraint)](https://github.com/xyflow/xyflow/issues/4438)
- [xyflow discussion #3495 (ELK + subflows)](https://github.com/xyflow/xyflow/discussions/3495)
- [xyflow discussion #4830 (ELK subflow render order)](https://github.com/xyflow/xyflow/discussions/4830)
- [xyflow discussion #2821 (auto-resize parent)](https://github.com/xyflow/xyflow/discussions/2821)
- [xyflow discussion #1265 (expand/collapse)](https://github.com/xyflow/xyflow/discussions/1265)
- [ELK layout options reference](https://eclipse.dev/elk/reference/options.html)
- [elkjs GitHub](https://github.com/kieler/elkjs)
- [react-flow-elk-mixed-layout demo](https://github.com/dipockdas/react-flow-elk-mixed-layout)
- [xyflow Pro Platform open source announcement](https://xyflow.com/blog/react-flow-pro-platform-open-source)

### Rust Tooling
- [Rust releases](https://releases.rs/)
- [Tauri v2 Prerequisites](https://v2.tauri.app/start/prerequisites/)
- [aarch64 Tier 1 RFC](https://rust-lang.github.io/rfcs/3671-promote-aarch64-apple-darwin-to-tier-1.html)
- [cargo-watch archived](https://github.com/watchexec/cargo-watch)
- [cargo-nextest](https://nexte.st/)
- [bacon](https://github.com/Canop/bacon)

### Testing
- [proptest](https://github.com/proptest-rs/proptest)
- [Vitest 4.0 Browser Mode](https://www.infoq.com/news/2025/12/vitest-4-browser-mode/)
- [CrabNebula E2E (macOS, paid)](https://docs.crabnebula.dev/plugins/tauri-e2e-tests/)
- [tauri::test::assert_ipc_response](https://docs.rs/tauri/latest/tauri/test/fn.assert_ipc_response.html)

---

# Market Landscape: Code/Architecture Visualization Tools (March 2026)

---

## Current State — Existing Products

### 1. CodeSee

**Status:** Acquired by GitKraken, May 2024. Had announced shutdown in February 2024 before the acquisition intervened. The standalone codesee.io product is effectively sunset — it now exists as a component of GitKraken's "DevEx Platform." The first concrete output of the acquisition is GitKraken Automations (workflow automation), not visualization. A feature called "Codemaps" (interactive codebase dependency mapping) was announced for early access but remains in development as of early 2026 and is not generally available.

**What it did well:** Automated cross-repo dependency visualization, code health metrics, visual PR reviews with diff context, service/module ownership tracking.

**What was missing:** Required cloud service (code sent to CodeSee servers), no local/offline mode, heavy SaaS pricing, no hierarchical zoom levels.

**Sources:**
- [GitKraken acquires CodeSee (finsmes, May 2024)](https://www.finsmes.com/2024/05/gitkraken-acquires-codesee.html)
- [GitKraken Codemaps early access](https://www.gitkraken.com/solutions/codemaps)
- [GitKraken Code Dependency Mapping](https://www.gitkraken.com/features/code-dependency-mapping)

---

### 2. Sourcetrail

**Status:** Original repository archived December 2021 by CoatiSoftware. Active community forks exist — the most prominent (petermost/Sourcetrail) has 3,000+ commits with releases into late 2025 including support for Clang/LLVM 20 and Qt 6.9. Not dead, but not officially maintained.

**What it does well:** Interactive source explorer for C/C++/Java/Python. Bidirectional navigation between code and graph — clicking a node jumps to source. True call graph + type hierarchy + file dependency views in one tool. Desktop native (Qt). Local only.

**What's missing:** No modern language support (no Rust, TypeScript, Go in official build). No hierarchical zoom metaphor — shows a flat graph for the selected symbol, not a nested overview. No git history integration. UI is dated (2015-era Qt). No AI. No monorepo awareness.

**Sources:**
- [GitHub CoatiSoftware/Sourcetrail (archived)](https://github.com/CoatiSoftware/Sourcetrail)
- [Community fork xiota/Sourcetrail](https://github.com/xiota/Sourcetrail)
- [Sourcetrail HN thread (Aug 2024)](https://news.ycombinator.com/item?id=41179446)

---

### 3. SciTools Understand

**Status:** Actively maintained commercial product. Has a VS Code extension. Targets aerospace/defense/automotive markets (MISRA, AUTOSAR compliance). Enterprise pricing, no public price list.

**What it does well:** The most comprehensive static analysis + visualization suite available. Supports 20+ languages including Ada, COBOL, FORTRAN, Jovial, C/C++, Java, Python, JavaScript. Call graphs, dependency graphs, UML class diagrams, metrics dashboards, treemaps. Works on codebases with millions of LOC. Desktop native standalone. AI-powered summaries added recently.

**What's missing:** Very high price (enterprise only). UI designed for legacy/embedded developers, not web/cloud developers. Visualization is generated on demand and not a live browsable canvas. No real-time or incremental analysis.

**Sources:**
- [SciTools Understand features](https://scitools.com/features)
- [G2 reviews 2025](https://www.g2.com/products/understand/reviews)

---

### 4. dependency-cruiser

**Status:** Actively maintained. ~1.13M weekly npm downloads, 6,460 GitHub stars. CLI-only, no interactive UI.

**What it does well:** JavaScript/TypeScript/CoffeeScript/ESM dependency graphing with rule enforcement. Generates SVGs via Graphviz and navigable HTML reports. CI-friendly: fails the build on rule violations (circular deps, forbidden imports). Good for monorepos.

**What's missing:** No interactive UI. Graphviz output is static images. No zoom/pan. No hierarchical visualization. JS/TS only (no Rust, Go, Python). Cannot visualize at runtime. Output is a flat directed graph with no nesting.

**Sources:**
- [dependency-cruiser vs madge npm trends](https://npmtrends.com/dependency-cruiser-vs-madge-vs-react-graph-vis)
- [UpgradeJS comparison](https://www.upgradejs.com/blog/application-architecture-visualization.html)

---

### 5. Madge

**Status:** Actively maintained. ~1.8M weekly npm downloads, 9,995 GitHub stars. CLI-only.

**What it does well:** Fast JS/TS circular dependency detection, simple graph images, very low barrier to entry.

**What's missing:** Same gaps as dependency-cruiser: no interactive UI, no zoom, no nesting, JS/TS only. Known performance and readability problems on large projects ("the resulting graph was pretty huge and hard to read").

**Sources:**
- [npm trends comparison](https://npmtrends.com/dependency-cruiser-vs-madge-vs-react-graph-vis)
- [DEV.to: Skott, the new Madge](https://dev.to/antoinecoulon/introducing-skott-the-new-madge-1bfl)

---

### 6. Nx Graph

**Status:** Actively maintained, bundled with Nx monorepo tool. Free within the Nx ecosystem.

**What it does well:** Interactive browser-based dependency graph for Nx workspaces. Filtering, focus on affected subgraphs, task pipeline visualization. Import-level granularity — knows which specific exports changed affected which packages. PR-level graph diffs via Nx Cloud.

**What's missing:** Locked to Nx monorepos. Cannot analyze arbitrary codebases. No file-level or function-level zoom. No cross-language view. Cannot show nested hierarchical structure below the project/package level.

**Sources:**
- [Nx vs Turborepo comprehensive guide](https://generalistprogrammer.com/comparisons/turborepo-vs-nx)
- [Turborepo, Nx, Lerna: truth about monorepo tooling 2026](https://dev.to/dataformathub/turborepo-nx-and-lerna-the-truth-about-monorepo-tooling-in-2026-71)

---

### 7. Turborepo Graph

**Status:** Actively maintained by Vercel. CLI-based, browser visualization or Graphviz export.

**What it does well:** Task dependency graph visualization for Turborepo monorepos. Fast (Rust core). Shows which packages are affected by a change.

**What's missing:** Simpler than Nx — package-level only, no import-level granularity, no file-level drill-down. Locked to Turborepo workspaces.

---

### 8. VS Code Extensions

**Status:** Fragmented. No single dominant extension owns this space.

Notable extensions:

- **CodeViz (YC S24):** VS Code extension generating interactive call graphs and AI-powered C4 architecture diagrams. Uses LLMs (sends code to Anthropic). $19/month for full features. Traction at Amazon, Microsoft, Roblox. Key criticisms from HN launch: (1) code leaves machine — enterprise dealbreaker, (2) web-dev-biased categorization (a robotics repo got categorized as frontend/backend), (3) free tier shows almost nothing, (4) no support for non-VS Code editors.
- **Dependency Graph (sz-p):** Minimal VS Code plugin showing local project dependency graph. Static.
- **Dependency Cruiser Extension:** Wraps dependency-cruiser in VS Code. Static SVG output.
- **Swark:** Uses GitHub Copilot LLM API to auto-generate Mermaid architecture diagrams from a folder. Local LLM only (only Copilot sees code). No interactivity — produces static Mermaid markdown.
- **Code Graph (CodeAtlas):** Maps function references across files in VS Code.

**Overall gap:** All VS Code extensions are constrained by the extension host model — no persistent background process, no native UI chrome, limited canvas performance. None provide a dedicated pan/zoom hierarchical workspace in a standalone desktop window.

**Sources:**
- [CodeViz HN launch](https://news.ycombinator.com/item?id=41393458)
- [CodeViz VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=CodeViz.codeviz)
- [Swark GitHub](https://github.com/swark-io/swark)

---

### 9. JetBrains Dependency Diagrams

**Status:** Built into IntelliJ IDEA, WebStorm, Rider. Active and maintained.

**What it does well:** Module/project dependency diagrams. Rider's dependency analysis builds graphs without compilation. Java/Kotlin class hierarchy + UML in IntelliJ. TypeScript import/export visualization in WebStorm.

**What's missing:** Tied to the IDE being open. Diagrams are generated on demand, not a persistent browsable canvas. No function-level call graph. No git history overlay. No zoom metaphor for navigating hierarchical levels. Requires JetBrains subscription.

**Sources:**
- [IntelliJ module dependency diagram](https://www.jetbrains.com/help/idea/project-module-dependencies-diagram.html)
- [Rider visual dependency analysis](https://www.jetbrains.com/help/rider/Visual_Dependency_Analysis.html)

---

### 10. Newer tools (2024-2026)

**GitDiagram (2025):** Web tool — replace "hub" with "diagram" in a GitHub URL to get an AI-generated (Claude 3.5 Sonnet) interactive architecture diagram. Zero install, fast. Limitations: public GitHub repos only, static output, no zoom levels, one-shot — not a live model.
- [gitdiagram.com](https://gitdiagram.com/)

**CodeCharta (MaibornWolff, active 2026):** Open source. Transforms static analysis metrics from SonarQube/Tokei/Code Maat into an interactive 3D "city" metaphor — files are buildings, folders are districts, height/color = metrics. Local only. Gap: this is metrics visualization, not dependency/import visualization. No call graphs, no dependency edges. Steep setup (requires SonarQube pipeline).
- [CodeCharta GitHub](https://github.com/MaibornWolff/codecharta)

**IcePanel (active 2025-2026):** Collaborative SaaS C4 model diagramming. Interactive, zoomable, model-based (changes sync across all diagrams). Has an MCP server for AI integration. Gap: manually maintained — it does not parse your repo. You draw the diagram; IcePanel keeps it consistent. Strong for team-maintained architecture documentation, not auto-generated visualization.
- [IcePanel](https://icepanel.io/)

**GitHub Next Repo Visualization (2021, prototype):** Circle-packing of repo file structure, animated over git history. "Usable prototype" status, not a product. No dependency edges, no hierarchical drill-down.
- [GitHub Next Repo Visualization](https://githubnext.com/projects/repo-visualization/)

**Codemap.app (active 2025):** Web-based graph visualization for TypeScript, Python, Java, PHP, Ruby, Go, Terraform. Privacy-first (code never sent to servers — runs in browser). Known OOM crashes on larger projects. No AI. No hierarchical nested view. Flat graph only.
- [Codemap.app](https://codemap.app/)

---

## Constraints — What Can't Change

1. **Privacy is a hard gate for enterprise.** The CodeViz HN launch thread is the clearest data point: sending code to external servers is a non-starter at many companies. Any tool requiring cloud LLM inference loses enterprise accounts by policy, not preference.

2. **Static output cannot be a daily driver.** Madge, dependency-cruiser, Swark, GitDiagram all produce static images or Mermaid text. Developers want to pan, zoom, click-to-navigate. Static outputs are a reporting artifact, not a working tool.

3. **IDE extension constraints are real.** VS Code extensions cannot sustain background processes, have limited canvas performance, and are not standalone windows. JetBrains tools require the IDE running. Neither delivers a dedicated visualization workspace.

4. **Tool lock-in limits value.** Nx graph only works inside Nx. Turborepo graph only works inside Turborepo. Dependency-cruiser is JS/TS-only. Tools locked to a build system or language cannot serve the general developer population.

5. **LLM-based tools break on large codebases.** Swark, CodeViz, GitDiagram all rely on LLMs to infer architecture. On codebases beyond the LLM context window, they silently truncate or hallucinate. This makes them unreliable for large codebases — the exact case where visualization is most needed.

---

## Gaps — What You Cannot Do Today

These are the most significant unmet needs, ordered by developer impact:

**1. Hierarchical zoom from system to file to function — in one tool.**
No existing tool does all three levels fluidly. C4 model tools (IcePanel, Structurizr) do system → container → component but stop at code boundaries. Source explorers (Sourcetrail) do file → function but not system overview. Nothing provides all levels in a single interactive canvas with smooth zoom transitions.

**2. Auto-generated and always current.**
Every tool requires either manual maintenance (IcePanel, C4 diagrams) or a one-shot generation (GitDiagram, Swark) that goes stale immediately. No tool watches the codebase and updates the visualization as files change. Developers identify documentation decay as a core long-term problem — existing tooling reinforces it.

**3. Multi-language in a single view.**
Most tools are language-specific. Real codebases mix Rust + TypeScript + Python + SQL. No tool visualizes cross-language dependency flows (e.g., a TypeScript frontend calling a Rust Tauri backend calling a Python ML service).

**4. Local/offline, no cloud dependency.**
Enterprise developers need code to stay local. CodeViz loses enterprise accounts here by design. Sourcetrail had this right (desktop, local) but is archived and language-limited. Codemap.app claims local (browser-only) but crashes on large projects. There is no modern, actively maintained, locally-running desktop app for architecture visualization.

**5. Git history overlaid on the dependency graph.**
Gource visualizes git history as animation but has no dependency graph. GitLens 17.1 shows file history but not dependency impact. No tool shows "this module changed 47 times in the last 90 days and is imported by 23 other modules" as a first-class visual. This compound view — churn × centrality — is the highest-value insight for identifying technical debt and refactoring risk.

**6. Large codebase performance with graceful degradation.**
Codemap.app reports OOM on large projects. Madge's output becomes unreadable. Sourcetrail slows on large C++ codebases. No tool handles 500K+ LOC gracefully in an interactive visualizer. The expand/collapse paradigm (show summaries, drill into what you need) is the right pattern but no tool implements it cleanly across all zoom levels.

**7. Runtime vs. static view.**
All existing tools are static analysis only. Runtime call graphs (what actually gets invoked vs. what the import graph implies) can differ substantially — dynamic imports, plugin systems, conditional feature flags, duck-typed calls. No tool bridges static and runtime views.

---

## Options — Approaches for a New Desktop App

### Option A: Deep single-language tool (Rust-first)

Build the best-in-class Rust codebase visualizer using `syn` for AST-accurate parsing, petgraph for graph algorithms, React Flow + ELK for hierarchical visualization. Target Rust developers — a completely underserved audience (Sourcetrail never got Rust support, Understand has limited Rust support, no other tool addresses Rust).

**Trade-offs:**
- Pro: Depth over breadth. Rust parsing with syn is production-grade. Clear differentiation from web-dev-biased tools. Rust community is large, growing fast, and has no good option.
- Pro: Faster to ship; correctness is easier to validate for a single language's import semantics.
- Con: Smaller total addressable market than a multi-language tool.
- Con: Feels niche outside the Rust community.

### Option B: Multi-language via tree-sitter, language-agnostic core

Use tree-sitter (the same parser used by GitHub, Neovim, Zed) to parse 20+ languages into a common graph format. Build language plugins as separate modules. Hierarchical visualization at module → file → function level across any language mix.

**Trade-offs:**
- Pro: Works on any codebase. Directly addresses the cross-language gap. Tree-sitter grammars for Rust, TypeScript, Python, Go, C, Java, Swift are battle-tested at GitHub scale. The Rust crate `tree-sitter` exists and is well-maintained.
- Pro: Language support becomes adding a grammar + import resolver, not a new parser.
- Con: Higher complexity — each language has different import/dependency semantics (TypeScript re-exports, Rust `pub use`, Python `__init__.py`, Go module system are all distinct).
- Con: Risk of broad-but-shallow: visualization exists but dependency resolution has edge-case bugs per language.

### Option C: Local-first with git history overlay (the "codebase health" angle)

Focus on the intersection of dependency structure + git churn + code size metrics. A visual map where position = dependency centrality, height/color = churn or complexity. Targets "where is our technical debt?" rather than "how does this work?". CodeCharta exists in this space but is deeply flawed (requires SonarQube, no dependency edges, city metaphor obscures flow, steep setup).

**Trade-offs:**
- Pro: Highly differentiated. Directly answers actionable refactoring questions, not just structural curiosity.
- Pro: Git history is language-agnostic.
- Con: More abstract — developers most often want to see concrete dependency flow, not just heat maps.
- Con: Requires integrating git log analysis, code complexity metrics, and visualization simultaneously — a broader scope to ship a v1.

---

## Recommendation

**Option B (multi-language tree-sitter core) with Option A's depth applied to the launch languages.**

The highest-value unmet need is a **local-first, hierarchically-zoomable, always-current desktop visualizer that works for any codebase**. The four properties that make a tool "daily driver" worthy vs. novelty:

1. **Local only** — no code leaves the machine. This is the enterprise unlock. It also enables file-watching for live updates.
2. **Always current** — file-watch integration keeps the graph updated on save, like a live document not a snapshot.
3. **Hierarchical zoom** — system → module → file → function in a single canvas with smooth transitions. No existing tool does this.
4. **Multi-language** — tree-sitter as the parsing layer makes the core graph builder language-agnostic. Each language gets an import-resolution module on top of the common AST walk.

**Why tree-sitter over custom parsers:** GitHub uses it. Neovim uses it. Zed uses it. The grammars are correct and community-maintained. Building on it means language support is a grammar + import resolver away, not a new parser implementation. The `tree-sitter` Rust crate integrates cleanly with a Tauri backend.

**Why desktop native (Tauri) over web/IDE extension:** Enterprise code privacy, persistent background file-watching, native OS file system access, no extension host limitations. Tauri gives a sub-10MB binary, ~30-40MB idle memory, sub-500ms cold start. These characteristics matter for a tool that runs alongside the IDE all day.

**The differentiating feature to ship first:** Expand/collapse hierarchical nesting with file-watch auto-update, for Rust and TypeScript (the languages this POC already targets). That one feature — seeing your monorepo as nested boxes that stay current as you edit — is something no existing tool delivers. Everything else (AI summaries, git history overlay, runtime tracing, broader language support) is additive on top of that foundation.

---

## Market Landscape Sources

- [CodeSee acquired by GitKraken (May 2024)](https://www.finsmes.com/2024/05/gitkraken-acquires-codesee.html)
- [GitKraken DevEx Platform launch](https://www.gitkraken.com/blog/gitkraken-launches-devex-platform-acquires-codesee)
- [Sourcetrail GitHub (archived)](https://github.com/CoatiSoftware/Sourcetrail)
- [Sourcetrail HN thread](https://news.ycombinator.com/item?id=41179446)
- [SciTools Understand](https://scitools.com/features)
- [dependency-cruiser vs madge npm trends](https://npmtrends.com/dependency-cruiser-vs-madge-vs-react-graph-vis)
- [UpgradeJS architecture visualization](https://www.upgradejs.com/blog/application-architecture-visualization.html)
- [Skott: the new Madge (DEV.to)](https://dev.to/antoinecoulon/introducing-skott-the-new-madge-1bfl)
- [Nx vs Turborepo 2025](https://generalistprogrammer.com/comparisons/turborepo-vs-nx)
- [Turborepo, Nx, Lerna 2026 (DEV.to)](https://dev.to/dataformathub/turborepo-nx-and-lerna-the-truth-about-monorepo-tooling-in-2026-71)
- [JetBrains IntelliJ module dependency diagram](https://www.jetbrains.com/help/idea/project-module-dependencies-diagram.html)
- [JetBrains Rider visual dependency analysis](https://www.jetbrains.com/help/rider/Visual_Dependency_Analysis.html)
- [JetBrains WebStorm module dependency diagram](https://www.jetbrains.com/help/webstorm/module-dependency-diagram.html)
- [CodeViz YC S24 HN launch](https://news.ycombinator.com/item?id=41393458)
- [CodeViz VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=CodeViz.codeviz)
- [CodeViz YC company profile](https://www.ycombinator.com/companies/codeviz)
- [Swark GitHub](https://github.com/swark-io/swark)
- [GitDiagram](https://gitdiagram.com/)
- [GitHub Next Repo Visualization](https://githubnext.com/projects/repo-visualization/)
- [CodeCharta GitHub](https://github.com/MaibornWolff/codecharta)
- [IcePanel](https://icepanel.io/)
- [Codemap.app](https://codemap.app/)
- [Ask HN: What developer tool do you wish existed in 2026?](https://news.ycombinator.com/item?id=46345827)
- [Ask HN: Visualize Software Architecture/Concepts](https://news.ycombinator.com/item?id=41219304)
- [On navigating a large codebase (RoyalSloth)](https://blog.royalsloth.eu/posts/on-navigating-a-large-codebase/)
- [Code Compass: Challenges of navigating unfamiliar codebases (arxiv 2024)](https://arxiv.org/html/2405.06271v1)
- [GitLens 17.1 Visual History](https://www.gitkraken.com/blog/gitlens-17-1-visual-history-reimagined)
- [G2 reviews: Understand by SciTools 2025](https://www.g2.com/products/understand/reviews)
