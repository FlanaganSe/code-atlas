# PRD: Code Atlas — Local-First Architecture Visualization POC

**Date:** 2026-03-17
**Status:** Draft
**Type:** Proof of Concept
**Basis:** `.claude/plans/research.md`, `.plans/2026-03-17-tauri-poc-research.md`, market landscape analysis

---

## 1. Product Summary

Code Atlas is a local-first desktop application that turns a software repository into an interactive, zoomable architecture map. It parses source code locally, builds a hierarchical graph model, and renders it as a nested compound visualization where developers can move from system-level structure to file-level detail in a single canvas.

The first version is a proof of concept. Its purpose is to validate that a Tauri desktop app can parse a local codebase, build a graph model, and let a developer navigate hierarchical architecture without collapsing into an unreadable hairball — and that the chosen technology stack is viable for a production product.

---

## 2. Problem

Developers working in medium-to-large codebases cannot quickly answer "how does this system fit together?" Existing tools fail in predictable ways:

- **Dead or stalled:** Sourcetrail (archived 2021), CodeSee (acquired/sunset 2024)
- **Cloud-only:** CodeViz sends code to external LLMs — enterprise dealbreaker
- **Static output:** dependency-cruiser, Madge, Swark, GitDiagram produce images or text that go stale immediately
- **Ecosystem-locked:** Nx Graph requires Nx. Turborepo Graph requires Turborepo. Neither works on arbitrary repos.
- **Flat graphs:** Every tool that auto-generates from code produces a flat node-and-edge diagram. None support hierarchical zoom from system overview to file detail.
- **IDE-constrained:** VS Code extensions lack persistent background processes, native canvas performance, and standalone windows. JetBrains diagrams are on-demand, not a persistent workspace.

The gap is not "another dependency graph." The gap is a **local-first, always-current, hierarchically zoomable architecture map** that can be explored like an application, not read like generated documentation.

---

## 3. Why Now

- Tauri v2 is mature (stable since Oct 2024) — fast local desktop app with a secure Rust core and modern React frontend.
- React Flow v12 + ELK.js makes nested, compound graph interfaces feasible in a way they were not a few years ago.
- Sourcetrail's archival and CodeSee's sunset leave a clear market opening for local-first architecture visualization.
- No tool in the current landscape combines local execution, hierarchical zoom, and live code awareness.

---

## 4. Product Thesis

If we give developers a fast desktop app that scans a repository locally, maintains a structured graph of architecture entities, renders that graph as a nested zoomable map, and adapts the amount of detail to graph size, then the app becomes more valuable than a static diagram because it supports real development tasks: onboarding, refactor planning, dependency inspection, and architectural drift detection.

---

## 5. Target Users

**Primary:** Senior and staff engineers exploring unfamiliar codebases. Platform and architecture leads understanding system boundaries. Engineers entering an existing repo needing fast orientation.

**Secondary:** New team members during onboarding. Technical leads planning refactors. ICs validating dependency direction before changes.

**Not targeted by POC:** Non-technical stakeholders. Teams needing collaborative review workflows. Users expecting cloud sync or hosted analysis.

---

## 6. Product Principles

### 6.1 Local-first
All analysis happens on-device. Repository contents never leave the machine.

### 6.2 Progressive disclosure
The app must never show the entire codebase at maximum detail by default. It reveals more detail as the user focuses.

### 6.3 Hierarchy over hairball
The product prefers nested architectural structure over a flat all-nodes-at-once network.

### 6.4 Testable core
Graph creation, graph transforms, and user-facing state transitions must be testable independent of rendering.

### 6.5 Replaceable internals
The graph model should not assume a single parser, layout engine, or renderer. This is an architecture goal, not a POC deliverable — the plan should separate concerns enough that future changes don't require a rewrite.

---

## 7. Goals and Success Criteria

### POC Goals

| # | Goal | What It Proves |
|---|------|---------------|
| G1 | Zoomable, pannable, nested graph of a real codebase | Core interaction is viable |
| G2 | Source code to graph data to visual layout pipeline | Data pipeline works end-to-end |
| G3 | Expand/collapse at multiple nesting levels | Hierarchical navigation pattern works |
| G4 | Graph adapts to size (collapsed defaults, label hiding) | Progressive disclosure is feasible |
| G5 | Automated tests cover non-UI logic | Technology stack supports disciplined development |
| G6 | Technology choices validated | Tauri v2 + React Flow + ELK + petgraph + tree-sitter work well together |

### Measurable Success Criteria

1. A developer can point the app at a real Rust or TypeScript project and see a correct, navigable architecture graph.
2. Initial scan + first render completes in under 3 seconds for a project up to 300 files, under 10 seconds for up to 2,000 files.
3. The graph supports 3 levels of nesting (package/crate, module/folder, file) with working expand/collapse.
4. Zoom, pan, selection, and expand/collapse remain smooth on modern hardware when visible nodes are kept under 200.
5. Automated tests cover >80% of non-UI logic (both Rust and TypeScript).
6. The app runs entirely locally — zero network calls.
7. A user can identify the main packages, find what depends on a selected node, and locate a file within its architectural context — all from the UI.

---

## 8. Non-Goals

The POC is explicitly not building:

- Production polish, branding, onboarding UX
- Full function-level or call-site-accurate cross-language graphing
- Git history / blame / churn visualization
- AI-powered summaries, agents, or architecture advice
- Runtime tracing or network packet inspection
- Cloud collaboration, shared workspaces, or SaaS features
- Code editing within the visualization
- Distribution / code signing / notarization
- Perfect visual scalability for every possible monorepo size
- Polished export or reporting workflows
- Mobile targets (iOS/Android)
- Support for languages beyond Rust and TypeScript as fully parsed

---

## 9. POC Scope

### 9.1 What we are building

A desktop app that:
1. Opens a local repository via native file dialog
2. Scans supported source files with tree-sitter
3. Builds a hierarchical architecture graph in Rust
4. Renders it as a zoomable nested map with React Flow + ELK
5. Lets the user inspect dependencies, filter edges, and search nodes
6. Supports manual rescan after code changes

### 9.2 Language support

**Tier 1 — full semantic support:**
- **Rust:** Detect packages via `Cargo.toml`, modules via `mod.rs`/`lib.rs`, edges from `use`/`mod`/`pub use` statements.
- **TypeScript/TSX:** Detect packages via `package.json`, modules via `index.ts`/`index.tsx`, edges from `import`/`export` statements.

**Tier 2 — structural only (no AST parsing):**
- All other file types appear as nodes based on directory structure. No edge detection. This shows the full project shape for languages we don't parse yet.

### 9.3 Graph levels

1. Package / crate / app
2. Module / folder / service group
3. File

Function-level or symbol-level visualization is out of scope.

### 9.4 Relationship types

- Containment through hierarchy (parent-child nesting)
- Import/use edges (file-to-file and module-to-module)
- Re-export edges (`pub use`, barrel files)

Function call edges, protocol-level labels (fetch, gRPC), and runtime relationships are out of scope.

### 9.5 Rescan, not live refresh

The POC uses a manual "Rescan" button. Filesystem watching with automatic graph updates is a compelling differentiator — the market research identifies "always current" as a key gap — but it adds significant complexity (debouncing, incremental re-scan, graph diffing, viewport preservation) without proving any core hypothesis that a rescan button doesn't also prove. If time allows after core features are complete, file watching is the highest-value stretch goal.

---

## 10. User Stories

### US-1: Open a codebase
> As a developer, I open the app, select a project directory, and within seconds see a high-level architecture graph showing my packages as nested boxes with dependency edges between them.

**Acceptance criteria:**
- "Open Directory" button triggers native file dialog
- Rust backend scans the directory, builds the graph, serializes to the frontend
- React Flow renders the graph with ELK-computed layout
- Packages appear as group nodes; files inside them appear as child nodes
- Edges show import relationships between files
- Total time from selection to rendered graph: <3 seconds for a 300-file project

### US-2: Zoom and pan
> As a developer, I can zoom in to see file-level detail within a package, and zoom out to see the system overview, with smooth transitions.

**Acceptance criteria:**
- Mouse wheel / trackpad pinch to zoom, click-drag to pan
- MiniMap showing current viewport position
- Fit-to-view button resets to full graph
- Zoom level affects visible detail: zoomed out = package labels only, zoomed in = file names + edge labels visible

### US-3: Expand and collapse
> As a developer, I can click a package or module node to expand it (revealing contents) or collapse it (showing a summary box).

**Acceptance criteria:**
- Each group node has an expand/collapse toggle
- Collapsed: shows name + summary (e.g., "12 files, 4 exports")
- Expanded: shows child nodes inside the package boundary
- Toggling triggers ELK re-layout of visible nodes
- Default state follows graph adaptation rules (Section 11)

### US-4: Inspect a node
> As a developer, I can click a node to see its details: file path, type, dependency counts, and which edges connect to it.

**Acceptance criteria:**
- Click a node opens a detail panel (right side or bottom)
- Shows: file path, node kind, direct dependencies (in/out count), imports/exports
- Highlights all edges connected to the selected node
- Click an edge in the detail panel to navigate to the connected node

### US-5: Filter by edge type
> As a developer, I can toggle edge types on/off to reduce visual noise.

**Acceptance criteria:**
- Edge type checkboxes: imports, re-exports
- Hiding an edge type removes those edges and re-runs layout
- At least one edge type must remain active

### US-6: Search
> As a developer, I can search for a file or package by name and the graph pans to center on it.

**Acceptance criteria:**
- Search input with keyboard shortcut (Cmd+K)
- Fuzzy match on node labels
- Selecting a result: centers viewport on node, expands parent packages if collapsed, highlights the node

### US-7: Demo data
> As someone evaluating the app, I can load a built-in sample graph without having a project ready.

**Acceptance criteria:**
- "Load Demo" button loads a JSON fixture of a representative graph
- The demo graph demonstrates all interaction patterns (expand/collapse, filtering, search)

---

## 11. Dynamic Graph Adaptation

This is a core product requirement, not a technical nice-to-have. The app must adapt its default presentation to the size of the graph being visualized.

### 11.1 Default visible depth by graph size

- **Small graphs (<120 visible nodes):** Open deeper by default — top-level packages expanded, modules visible.
- **Medium graphs (120-250 visible nodes):** Start collapsed at the package level. User expands on demand.
- **Large graphs (>250 visible nodes):** Start collapsed. Hide lower-priority labels. Defer file-level detail until the user focuses a region.

### 11.2 Edge density control

- Dense cross-package edges are visible at the overview level.
- Dense file-level edges only appear when the user drills into a limited region.
- When a package is collapsed, edges between its children and external nodes are bundled into a single edge between the package and the external target.

### 11.3 Label visibility rules

- Package and crate labels are always visible.
- Module and folder labels are visible when zoomed in or when expanded inside a focused area.
- File labels may abbreviate or hide entirely at low zoom levels.

---

## 12. Core User Experience

### 12.1 Landing state

The first meaningful screen after scan shows:
- Top-level packages as the primary visible nodes
- High-level dependency edges between them
- A minimap
- Graph controls (zoom, fit-to-view)
- A detail panel placeholder

The default view answers "what are the main pieces of this repo?" in seconds.

### 12.2 Navigation flow

The product supports a natural drill-in flow:

**overview** (packages) -> **selected package** (expanded, showing modules/files) -> **file-level inspection** (detail panel with imports/exports)

The user must not feel like they are switching tools or modes. The canvas is continuous.

### 12.3 Error handling

- The app must clearly distinguish unsupported repo structures, parse failures, and permission issues.
- Partial graph generation should still render what is available when safe to do so.
- If tree-sitter fails on some files, those files still appear as structural nodes without edges.

---

## 13. Functional Requirements

| ID | Requirement | Priority | Story |
|----|------------|----------|-------|
| F1 | Select a project directory via native dialog | P0 | US-1 |
| F2 | Scan directory with tree-sitter, build graph in Rust | P0 | US-1 |
| F3 | Render graph as interactive React Flow canvas | P0 | US-1 |
| F4 | Nodes are nested: packages contain modules/files | P0 | US-1 |
| F5 | Edges show import/re-export relationships | P0 | US-1 |
| F6 | Smooth zoom and pan at 60fps for <200 visible nodes | P0 | US-2 |
| F7 | Expand/collapse on package and module nodes | P0 | US-3 |
| F8 | ELK re-layout on expand/collapse | P0 | US-3 |
| F9 | Default collapse state adapts to graph size | P0 | US-3 |
| F10 | Click node opens detail panel | P1 | US-4 |
| F11 | MiniMap showing viewport position | P1 | US-2 |
| F12 | Edge type filter toggles | P1 | US-5 |
| F13 | Cmd+K search with fuzzy match + navigate to node | P2 | US-6 |
| F14 | Zoom-level detail: labels simplify when zoomed out | P2 | US-2 |
| F15 | Edge bundling when packages are collapsed | P2 | US-3 |
| F16 | Built-in demo graph fixture | P2 | US-7 |
| F17 | Manual rescan button | P1 | - |

---

## 14. Non-Functional Requirements

| ID | Requirement | Target |
|----|------------|--------|
| NF1 | Scan + render time (300-file project) | <3 seconds |
| NF2 | Scan + render time (2,000-file project) | <10 seconds |
| NF3 | ELK layout computation (200 nodes) | <500ms (Web Worker, non-blocking) |
| NF4 | Interaction framerate (<200 visible nodes) | 60fps |
| NF5 | Application binary size (macOS) | <15MB |
| NF6 | Memory at 500-node graph | <200MB |
| NF7 | Test coverage on non-UI Rust code | >80% |
| NF8 | Test coverage on frontend pure logic | >80% |
| NF9 | Network calls | Zero — fully local |
| NF10 | UI thread never blocked by layout or parsing | Always responsive |

---

## 15. Technical Direction

### 15.1 Application platform

- Tauri v2 desktop application
- Rust stable (1.94.0 as of 2026-03-17) via rustup
- React 19 + TypeScript 5.x + Vite frontend
- pnpm package management
- Tailwind CSS v4 for UI styling

### 15.2 Graph rendering

**Chosen:** React Flow (`@xyflow/react` v12) for rendering + ELK.js (`elkjs`) in a Web Worker for layout.

**Why this wins:** React Flow is the only viable option for the product's core requirement of nested compound architecture views. It supports compound nesting via `parentId`, renders nodes as React components (testable, flexible), and integrates with ELK — the only layout engine that handles compound graphs with port-based edge routing. Research confirmed 3 levels of nesting working correctly.

**What was eliminated and why:**
- **Sigma.js + Graphology:** Early research recommended this for dense network visualization. Deep technical evaluation eliminated it — Sigma has no compound node support at all, which is a hard blocker for nested package visualization. Sigma is the right tool for a flat large-network renderer; this product is not that.
- **Dagre:** No compound/nested graph support. Also has an open bug preventing correct subflow layout when any subflow node connects to an external node — exactly the cross-package dependency case.
- **Cytoscape.js:** Cannot render React JSX inside nodes. Weaker Vitest testability. Falls back to this only if >500 simultaneous visible nodes are needed (unlikely given expand/collapse strategy).

### 15.3 Graph construction

- **Rust graph library:** `petgraph` (StableGraph) — stable indices, topological sort, SCC detection.
- **Code parser:** `tree-sitter` with language grammars — multi-language, incremental, battle-tested at GitHub/Zed/Neovim scale.
- **IPC:** Tauri `invoke` (JSON) for graph payloads. Type-safe IPC via `tauri-specta` v2 recommended (generates TypeScript bindings from Rust commands).

**Critical architecture constraint:** Layout cannot happen in Rust. React Flow needs DOM-measured node sizes before ELK can compute positions. Rust is responsible for parsing, graph construction, and serialization. All layout happens in ELK.js on the frontend.

### 15.4 Data pipeline

```
User selects directory
  -> Rust: tree-sitter scans .rs and .ts/.tsx files
  -> Rust: builds petgraph, topological sort (parents before children)
  -> Rust: serialize graph payload via invoke()
  -> React: deserialize, render as React Flow nodes (DOM measures sizes)
  -> ELK.js (Web Worker): compute positions + parent container sizes
  -> React Flow: apply positions, fitView()
```

### 15.5 Test strategy

Three layers:
1. **Rust unit + integration tests** — graph algorithms, parsing, serialization. Property-based tests with proptest for graph invariants. Benchmarks with criterion at 10/100/1000 nodes.
2. **Frontend unit tests** — pure functions (transforms, filters, search), IPC wrappers with mockIPC, state transitions. Vitest + jsdom.
3. **Visual/E2E — deferred for POC.** macOS WKWebView has no WebDriver support. Rely on unit tests and manual smoke tests.

### 15.6 Security

- Minimal Tauri capabilities: `core:default` + `dialog:allow-open`
- No shell, fs write, process, or opener permissions
- No remote script loading
- Tight CSP in Tauri config
- Rust backend reads directories directly — no frontend filesystem access

---

## 16. Milestones (Suggested)

These are implementation phases. Exact breakdown happens in `/plan`.

| # | Milestone | What It Proves |
|---|-----------|---------------|
| M1 | **Scaffold + Hello World** | Tauri v2 + React + Vite builds and launches. Rust toolchain verified. Type-safe IPC wired. |
| M2 | **Static graph rendering** | React Flow renders a hardcoded graph fixture with compound nodes + ELK layout. Expand/collapse works. Graph adaptation defaults work. |
| M3 | **Rust graph pipeline** | tree-sitter parses a real directory. petgraph builds the graph. Data flows through invoke() to React Flow. |
| M4 | **Interactive features** | Node detail panel, edge filtering, search, rescan. |
| M5 | **Polish + demo data** | Dark theme, performance tuning, sample graph fixture, all tests green. |

---

## 17. What "Good" Looks Like in a Demo

1. Open a real repo and render the top-level architecture map in under 3 seconds.
2. See packages as nested boxes. Understand the system structure without reading code.
3. Click a package. See its immediate neighborhood highlighted.
4. Expand into a module, then inspect a file's imports and exports.
5. Filter the graph to show only import edges. See the graph simplify.
6. Search for a file by name. Watch the graph pan and zoom to it.
7. Hit rescan after editing code. See the graph update.

If the demo only shows a static pretty graph, the POC failed. The product must be interactive and navigable enough that a developer can explain their architecture to a teammate using only the app.

---

## 18. Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| ELK layout too slow for 500+ nodes | Medium | Blocks fluid UX | Web Worker isolates main thread; debounce rapid toggles; profile early with criterion benchmarks |
| tree-sitter import resolution is incomplete | High | Edges are wrong | Start with explicit `import`/`use` only; skip dynamic imports; show nodes without edges as fallback |
| Flat-to-hierarchical ELK transform has edge cases | High | Layout breaks | Comprehensive unit tests with proptest; test cross-hierarchy edge routing with `INCLUDE_CHILDREN` |
| React Flow performance degrades with many compound nodes | Medium | Blocks scale | Expand/collapse keeps visible count <200; memoize custom node components; profile early |
| tauri-specta integration friction | Medium | Delays type safety | Can fall back to manual TypeScript types; specta is nice-to-have, not a hard dependency |
| Rust is new to the developer | Certain | Slower velocity | Lean on compiler errors; start with minimal Rust surface; expand as comfort grows |
| Graph adaptation thresholds need tuning | Medium | Bad defaults | Make thresholds configurable internally; test against real repos of varying sizes |

---

## 19. Open Questions

These should be answered before or during milestone planning. They should not block starting the POC.

| # | Question | Default If Unanswered |
|---|----------|----------------------|
| Q1 | Should monorepos with multiple packages be the primary demo scenario? | **Yes** — the screenshot inspiration and the market gap both point to monorepos as the core use case. |
| Q2 | Should edges show multiplicity when packages are collapsed (e.g., "5 imports" as one thick edge)? | **Yes, bundle edges** between collapsed packages. Show individual file-to-file edges when both packages are expanded. |
| Q3 | What graph should we demo if the user doesn't have a project handy? | **Ship a JSON fixture** of the POC's own codebase, loadable from a "Load Demo" button. |
| Q4 | Should unsupported file types appear as structural nodes by default? | **Yes** — show the full project shape. Users can ignore non-parsed files but seeing the full directory structure provides context. |
| Q5 | Is launch value higher for Rust-first or TypeScript-first? | **Both at launch.** The POC targets its own codebase (Rust + TypeScript) as the primary test case. |
| Q6 | Should we include a second layout algorithm or ship with one? | **One good default** (ELK layered, direction DOWN). A layout switcher (P2) is valuable for evaluation but not required to prove the core thesis. |

---

## 20. Decisions Recorded in This PRD

These are product and technical decisions that resolve conflicts between prior documents or open questions from research.

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Product name | Code Atlas | Communicates hierarchical, explorable nature better than "CodeGraph" |
| Visualization library | React Flow + ELK.js | Only option supporting compound nested nodes (Sigma.js eliminated — no compound support) |
| Parsing approach | tree-sitter (multi-language) | Extensible beyond Rust/TS. Same parser used by GitHub, Neovim, Zed. Language support = grammar + import resolver |
| Live refresh | Manual rescan for POC | Proves the same pipeline. File watching is highest-priority post-POC feature |
| Parsing depth | Imports and re-exports only | Function calls create denser, harder-to-validate graphs. Add in v2 |
| Layout in Rust vs JS | Layout in JS (ELK Web Worker) | React Flow needs DOM-measured node sizes. Layout cannot happen server-side |
| State management | Frontend concern, decided in /plan | Not a product decision. zustand, useReducer, or similar — let the plan decide |
| Code in PRD | No code in this PRD | Implementation details (Rust structs, file paths, state tables) belong in the /plan phase |
| Graph adaptation | Required for POC | Core to avoiding the "hairball" problem that kills every competitor |

---

## 21. Research Conclusions Applied

The following findings from the research phase materially shaped this PRD:

- The strongest market gap is **local-first + always-current + hierarchical zoom**. No existing tool delivers all three.
- Tauri v2 is the right platform — sub-10MB binary, sub-500ms cold start, native filesystem access, secure Rust core.
- React Flow v12 + ELK.js is the only viable rendering direction for nested compound architecture views. Three nesting levels confirmed working. The flat-to-hierarchical ELK encoder/decoder is ~100 lines and well-documented.
- Sigma.js was recommended by early research but eliminated by deep technical evaluation — no compound node support, which is a hard blocker for the product concept.
- The graph must adapt to graph size — progressive disclosure is the difference between a useful tool and a hairball generator.
- Testability must be designed in because POC decisions need evidence, not taste.
- macOS WKWebView has no WebDriver, so the test strategy emphasizes unit, integration, and benchmark layers over desktop E2E.
- The market gap is not "another dependency graph" but a **local-first architecture atlas** — the product should feel like exploring a map, not reading a diagram.
