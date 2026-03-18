# Code Atlas — Product Requirements Document

**Date:** 2026-03-18
**Status:** Draft v2
**Type:** Phased Product Requirements (POC → MVP → Vision)
**Basis:** Consolidated research (March 17-18, 2026). See `research/consolidated-technical-decisions.md` for technical rationale and `research/consolidated-market-and-product.md` for market context.

---

## 1. Product Summary

Code Atlas is a local-first desktop application that turns a software repository into an interactive, zoomable architecture map. It parses source code locally, builds a hierarchical graph model, and renders it as a nested compound visualization where developers can move from system-level structure to file-level detail in a single continuous canvas.

Beyond structure, Code Atlas provides change intelligence — showing what changed, what is impacted, and what to review next — making it a daily-use tool rather than a one-time curiosity.

The architecture model serves both humans (via the desktop canvas and VS Code bridge) and coding agents (via structured graph queries and MCP).

---

## 2. Problem

Developers working in medium-to-large codebases cannot quickly answer "how does this system fit together?" or "what does this change affect?" Existing tools fail in predictable ways:

- **Dead or sunset:** Sourcetrail (archived 2021, community forks lack modern languages), CodeSee (acquired by GitKraken May 2024, standalone product sunset, "Codemaps" not yet generally available).
- **Cloud-only:** CodeViz sends code to external LLMs — enterprise dealbreaker per the HN launch thread consensus.
- **Static output:** dependency-cruiser, Madge, Swark, GitDiagram produce images or text that go stale immediately.
- **Ecosystem-locked:** Nx Graph requires Nx. Turborepo Graph requires Turborepo. Neither works on arbitrary repos.
- **Flat graphs:** Every tool that auto-generates from code produces a flat node-and-edge diagram. None support hierarchical zoom from system overview to file detail.
- **IDE-constrained:** VS Code extensions lack persistent background processes and native canvas performance. JetBrains diagrams are on-demand, not a persistent workspace.
- **No change awareness:** Structure alone is table stakes. No tool combines architecture visualization with branch/PR diff overlays and blast radius analysis in a local-first tool.

The gap is a **local-first, always-current, hierarchically-zoomable architecture and change-intelligence map** that developers and agents can query.

---

## 3. Product Thesis

If we give developers a fast local desktop app that:
1. Scans a repository and builds a deterministic architecture model
2. Renders that model as a nested zoomable map with adaptive detail
3. Overlays change intelligence (diffs, blast radius, impacted slices)
4. Integrates with VS Code and coding agents

...then the app becomes more valuable than static diagrams because it supports real development tasks: onboarding, refactor planning, dependency inspection, code review, change impact analysis, and agent-assisted development.

The winning product is not the prettiest graph. It is the product that makes engineers faster at understanding, reviewing, and deciding.

---

## 4. Target Users and Jobs To Be Done

### Primary Users
- Senior and staff engineers exploring unfamiliar codebases
- Platform and architecture leads understanding system boundaries
- Engineers entering an existing repo needing fast orientation

### Secondary Users
- New team members during onboarding
- Technical leads planning refactors
- ICs validating dependency direction before changes
- Coding agents needing structured architecture context

### Jobs To Be Done

| JTBD | Core Need | Phase |
|------|-----------|-------|
| **"How does this repo fit together?"** | Top-level structure, boundaries, what talks to what, drill-in without losing orientation | POC |
| **"Show me only what's relevant to my task."** | Dependency slices, entrypoint paths, changed-file neighborhood | MVP |
| **"What does this change affect?"** | Changed nodes, blast radius, impacted slice, review/test hints | MVP |
| **"Let me move between map and editor instantly."** | Open file at range, reveal current file, selection sync | MVP |
| **"Give my agent the same understanding."** | Machine-readable graph, structured slices, stable IDs | Vision |

---

## 5. Product Principles

1. **Local-first.** All analysis on-device. Code never leaves the machine.
2. **Hierarchy over hairball.** Nested structure, adaptive detail. Never show everything at maximum zoom by default.
3. **Deterministic first, generative second.** LLMs may explain the graph; they do not define it.
4. **Every edge has provenance.** Expose evidence class (syntactic/semantic/heuristic). This is the trust advantage.
5. **Workflow over spectacle.** Measure by "helps developer decide faster," not "looks impressive."
6. **Testable core.** Graph logic and transforms testable without rendering.
7. **Replaceable internals.** No assumption of a single parser, layout engine, or renderer.

---

## 6. Phased Scope

### 6.1 Phase 1 — POC

**Purpose:** Validate that a Tauri desktop app can parse a local codebase, build a graph model, and let a developer navigate hierarchical architecture without collapsing into an unreadable hairball — and that the chosen technology stack is viable.

**What we build:**
1. Open a local repository via native file dialog
2. Scan supported source files with tree-sitter
3. Build a hierarchical architecture graph in Rust (petgraph)
4. Render as a zoomable nested map with React Flow + ELK
5. Expand/collapse at multiple nesting levels with adaptive defaults
6. Node detail panel, edge type filtering, search (Cmd+K)
7. Manual rescan after code changes
8. Built-in demo graph fixture
9. Automated tests covering >80% of non-UI logic

**Language support (POC):**
- **Tier 1 (full semantic):** Rust (Cargo.toml, mod hierarchy, use/mod/pub use) and TypeScript/TSX (package.json, import/export)
- **Tier 2 (structural only):** All other file types appear as nodes based on directory structure. No edge detection.

**Graph levels (POC):**
1. Package / crate / app
2. Module / folder / service group
3. File

Function-level and symbol-level visualization are out of scope.

**Success criteria:**
1. A developer points the app at a real Rust or TypeScript project and sees a correct, navigable architecture graph.
2. Initial scan + first render: <3 seconds for ≤300 files, <10 seconds for ≤2,000 files.
3. 3 levels of nesting with working expand/collapse.
4. Zoom, pan, selection, expand/collapse remain smooth at 60fps when visible nodes <200.
5. >80% test coverage on non-UI Rust and TypeScript logic.
6. Zero network calls.
7. A user can identify main packages, find dependencies of a selected node, and locate a file within its architectural context — all from the UI.

**Non-goals (POC):**
- Production polish, branding, onboarding UX
- Function-level or call-site-accurate cross-language graphing
- Git history / blame / churn visualization
- AI-powered summaries, agents, or architecture advice
- Runtime tracing
- Cloud collaboration
- Code editing within the visualization
- Distribution / code signing / notarization
- VS Code integration beyond stretch-goal click-to-open
- Persistence (SQLite)
- File watching (automatic rescan)
- Change intelligence (diff overlays)

**Stretch goals (if time allows after core features):**
- Click-to-open file in VS Code (`code -g file:line:col`)
- Circular dependency highlighting (petgraph SCC — trivial)
- SVG/PNG export

### 6.2 Phase 2 — MVP

**Purpose:** Ship to real users. Adds the features that make Code Atlas a daily-use tool rather than a demo.

**What changes from POC:**
- tree-sitter for TS/JS upgraded to oxc_parser + oxc_resolver (faster, handles TypeScript resolution spec)
- SQLite + FTS5 persistence for snapshots and search indexing
- Stable node IDs across rescans

**What's added:**
1. **File watching** with incremental rescan (notify + tree-sitter incremental, 300-500ms debounce)
2. **Git integration** — branch/PR diff overlay: changed nodes, added/removed edges, changed-slice view
3. **Blast radius visualization** — "if I change this file, what else could break?" (graph traversal, color by distance)
4. **Semantic zoom** — different representations by scale (system overview → focused slice → module detail → file detail), not just label hiding
5. **Slice-based navigation** — upstream, downstream, path between A and B, changed-files-only, service interaction
6. **Click-to-open in VS Code** (`code -g file:line:col`)
7. **Thin VS Code companion extension** — active file tracking, "Show in Code Atlas" context menu, selection sync
8. **Dead code / orphan detection** — zero in-degree nodes that aren't entry points
9. **Mermaid and D2 text export**
10. **Edge confidence/provenance** badges (syntactic vs. semantic)

**Success criteria (MVP):**
1. All POC criteria plus:
2. Graph updates within 2 seconds of file save (watch mode)
3. PR diff overlay correctly highlights changed architecture for a real PR
4. A developer can answer "what does this change affect?" from the UI
5. VS Code round-trip (map → editor → map) works in <1 second

### 6.3 Phase 3+ — Vision

Directional only. Not spec'd in detail.

- **Semantic parsing upgrade:** rust-analyzer + TypeScript compiler/LSP for semantic-grade edges
- **SCIP integration** for normalized cross-language semantic indexes
- **Preemptive plan ghost overlays** — structured plan schema → predicted change overlay before code lands
- **MCP server / CLI / JSON API** for coding agent queries
- **Watch mode with animated graph diffs** (stable IDs + d3-timer position interpolation + diff color coding)
- **Architecture rules** (`.codeatlas/rules.json`) — forbidden imports, layering violations, ownership boundary crossings
- **Ownership/churn/risk overlays** from CODEOWNERS + git history
- **Runtime overlay** via OpenTelemetry service graph (static + runtime view)
- **Local AI summaries** (opt-in, llama-cpp-2 + small code models)
- **Branch comparison** (scan two branches, diff architecture graphs)
- **Interactive HTML export** (self-contained, shareable)
- **Loro CRDT + mDNS for LAN collaboration** (team architecture sessions)
- **WebGPU renderer** (progressive enhancement)

---

## 7. User Stories

### Phase 1 (POC)

#### US-1: Open a codebase
> As a developer, I open the app, select a project directory, and within seconds see a high-level architecture graph showing my packages as nested boxes with dependency edges between them.

**Acceptance criteria:**
- "Open Directory" button triggers native file dialog
- Rust backend scans the directory, builds the graph, serializes to the frontend
- React Flow renders the graph with ELK-computed layout
- Packages appear as group nodes; files inside them as child nodes
- Edges show import relationships between files
- Total time from selection to rendered graph: <3 seconds for ≤300 files

#### US-2: Zoom and pan
> As a developer, I can zoom in to see file-level detail within a package, and zoom out to see the system overview.

**Acceptance criteria:**
- Mouse wheel / trackpad pinch to zoom, click-drag to pan
- MiniMap showing current viewport position
- Fit-to-view button resets to full graph
- Zoom level affects visible detail: zoomed out = package labels only, zoomed in = file names + edge labels visible

#### US-3: Expand and collapse
> As a developer, I can click a package or module node to expand it (revealing contents) or collapse it (showing a summary box).

**Acceptance criteria:**
- Each group node has an expand/collapse toggle
- Collapsed: shows name + summary (e.g., "12 files, 4 exports")
- Expanded: shows child nodes inside the package boundary
- Toggling triggers ELK re-layout of visible nodes
- Default state follows graph adaptation rules (Section 8)

#### US-4: Inspect a node
> As a developer, I can click a node to see its details: file path, type, dependency counts, and which edges connect to it.

**Acceptance criteria:**
- Click a node opens a detail panel (right side, collapsible ~300px)
- Shows: file path, node kind, direct dependencies (in/out count), imports/exports
- Highlights all edges connected to the selected node
- Click an edge in the detail panel to navigate to the connected node

#### US-5: Filter by edge type
> As a developer, I can toggle edge types on/off to reduce visual noise.

**Acceptance criteria:**
- Edge type checkboxes: imports, re-exports
- Hiding an edge type removes those edges and re-runs layout
- At least one edge type must remain active

#### US-6: Search
> As a developer, I can search for a file or package by name and the graph pans to center on it.

**Acceptance criteria:**
- Command palette (Cmd+K) with fuzzy match on node labels
- Selecting a result: centers viewport on node, expands parent packages if collapsed, highlights the node

#### US-7: Demo data
> As someone evaluating the app, I can load a built-in sample graph without having a project ready.

**Acceptance criteria:**
- "Load Demo" button loads a JSON fixture of a representative graph
- The demo graph demonstrates all interaction patterns (expand/collapse, filtering, search)

### Phase 2 (MVP)

#### US-8: See what changed
> As a developer, I can see which parts of the architecture changed on my current branch compared to main.

**Acceptance criteria:**
- Diff overlay shows: added nodes (green), removed nodes (red), modified nodes (amber), connected-but-unchanged (blue/dim)
- "Show only changed slice" filter
- Click-through from changed node to file diff

#### US-9: Understand blast radius
> As a developer, I can select a file and see everything that transitively depends on it.

**Acceptance criteria:**
- Select node → "Show blast radius" action
- Transitively dependent files highlighted, colored by distance (direct = red, 2-hop = orange, 3+ = yellow)
- Count summary: "47 files potentially affected"

#### US-10: VS Code round-trip
> As a developer, I can click a node to open the file in VS Code, and from VS Code I can reveal the current file in Code Atlas.

**Acceptance criteria:**
- Click node → file opens in VS Code at correct line
- VS Code: "Show in Code Atlas" command → atlas pans to that file's node

---

## 8. Dynamic Graph Adaptation

This is a core product requirement. The app must adapt its default presentation to the size of the graph.

### Default Visible Depth by Graph Size

| Graph Size | Default State |
|------------|---------------|
| **Small (<120 visible nodes)** | Top-level packages expanded, modules visible |
| **Medium (120-250 visible nodes)** | Collapsed at package level. User expands on demand. |
| **Large (>250 visible nodes)** | Collapsed. Lower-priority labels hidden. File-level detail deferred until user focuses a region. |

### Edge Density Control

- Dense cross-package edges visible at overview level
- Dense file-level edges only appear when user drills into a limited region
- When a package is collapsed, edges between its children and external nodes are bundled into a single edge between the package and the external target

### Label Visibility Rules

- Package/crate labels: always visible
- Module/folder labels: visible when zoomed in or expanded inside a focused area
- File labels: may abbreviate or hide at low zoom levels

### Thresholds

Node count thresholds (120, 250) are starting points. Make them configurable internally. Test against real repos of varying sizes and tune.

---

## 9. Core User Experience

### Landing State

After scan, the first meaningful screen shows:
- Top-level packages as primary visible nodes
- High-level dependency edges between them
- MiniMap
- Graph controls (zoom, fit-to-view)
- Detail panel placeholder

The default view answers "what are the main pieces of this repo?" in seconds.

### Navigation Flow

**overview** (packages) → **selected package** (expanded, showing modules/files) → **file-level inspection** (detail panel with imports/exports)

The canvas is continuous. The user must not feel like they are switching tools or modes.

### Error Handling

- Clearly distinguish unsupported repo structures, parse failures, and permission issues
- Partial graph generation should still render what is available when safe to do so
- If tree-sitter fails on some files, those files still appear as structural nodes without edges
- Informative error messages, not stack traces

### Detail Panel Design

Right-side collapsible panel (~300px):
- **Tabs:** Overview (name, path, type, stats) | Dependencies (in/out edge lists, clickable) | API (public exports with signatures from tree-sitter)
- Graph canvas 60-70%, detail panel 30-40%
- Panel collapses when nothing selected, slides in on node click
- Breadcrumb navigation: Package > Module > File

---

## 10. Functional Requirements

| ID | Requirement | Priority | Phase | Story |
|----|------------|----------|-------|-------|
| F1 | Select project directory via native dialog | P0 | POC | US-1 |
| F2 | Scan directory with tree-sitter, build graph in Rust | P0 | POC | US-1 |
| F3 | Render graph as interactive React Flow canvas | P0 | POC | US-1 |
| F4 | Nodes nested: packages contain modules/files | P0 | POC | US-1 |
| F5 | Edges show import/re-export relationships | P0 | POC | US-1 |
| F6 | Smooth zoom and pan at 60fps for <200 visible nodes | P0 | POC | US-2 |
| F7 | Expand/collapse on package and module nodes | P0 | POC | US-3 |
| F8 | ELK re-layout on expand/collapse | P0 | POC | US-3 |
| F9 | Default collapse state adapts to graph size | P0 | POC | US-3 |
| F10 | Click node opens detail panel | P1 | POC | US-4 |
| F11 | MiniMap showing viewport position | P1 | POC | US-2 |
| F12 | Edge type filter toggles | P1 | POC | US-5 |
| F13 | Cmd+K search with fuzzy match + navigate to node | P1 | POC | US-6 |
| F14 | Zoom-level detail: labels simplify when zoomed out | P1 | POC | US-2 |
| F15 | Edge bundling when packages are collapsed | P2 | POC | US-3 |
| F16 | Built-in demo graph fixture | P2 | POC | US-7 |
| F17 | Manual rescan button | P1 | POC | — |
| F18 | File watching with incremental rescan | P0 | MVP | — |
| F19 | Git branch/PR diff overlay | P0 | MVP | US-8 |
| F20 | Blast radius visualization | P0 | MVP | US-9 |
| F21 | Click-to-open in VS Code | P0 | MVP | US-10 |
| F22 | VS Code companion extension (thin bridge) | P1 | MVP | US-10 |
| F23 | Semantic zoom (different representations by scale) | P1 | MVP | — |
| F24 | Slice-based navigation presets | P1 | MVP | — |
| F25 | Dead code / orphan detection | P2 | MVP | — |
| F26 | Mermaid / D2 text export | P2 | MVP | — |
| F27 | Edge confidence/provenance badges | P2 | MVP | — |

---

## 11. Non-Functional Requirements

| ID | Requirement | Target | Phase |
|----|------------|--------|-------|
| NF1 | Scan + render time (≤300 files) | <3 seconds | POC |
| NF2 | Scan + render time (≤2,000 files) | <10 seconds | POC |
| NF3 | ELK layout computation (200 nodes) | <500ms in Web Worker | POC |
| NF4 | Interaction framerate (<200 visible nodes) | 60fps | POC |
| NF5 | Application binary size (macOS) | <15MB | POC |
| NF6 | Memory at 500-node graph | <200MB | POC |
| NF7 | Test coverage on non-UI Rust code | >80% | POC |
| NF8 | Test coverage on frontend pure logic | >80% | POC |
| NF9 | Network calls | Zero | POC |
| NF10 | UI thread never blocked by layout or parsing | Always responsive | POC |
| NF11 | Graph update latency after file save (watch mode) | <2 seconds | MVP |
| NF12 | VS Code round-trip (map → editor → map) | <1 second | MVP |

---

## 12. Technical Direction

Full decision records with evidence and alternatives evaluated are in `research/consolidated-technical-decisions.md`. The summary below includes enough rationale to understand each choice without reading the full document.

| Concern | Choice | Why |
|---------|--------|-----|
| Desktop shell | Tauri v2 | Rust-native core, capability-based security, <15MB binary, local-first by design |
| Language | Rust backend + TypeScript/React 19 frontend | Rust for performance-critical parsing/graph; React for rich interactive UI |
| Graph rendering | React Flow v12 (`@xyflow/react`) | Only option with compound nested nodes + React JSX + MIT license |
| Layout engine | ELK.js in Web Worker | Only open-source engine handling compound graphs with port-based edge routing. Dagre has a cross-boundary edge bug. |
| Graph data structure | petgraph StableGraph | Canonical Rust graph crate. Stable indices survive removals. Built-in SCC, topological sort, shortest path. |
| Code parsing (POC) | tree-sitter | Error-tolerant, incremental (sub-ms), multi-language. Same parser as GitHub/Neovim/Zed. |
| Code parsing (MVP, TS/JS) | oxc_parser + oxc_resolver | 3x faster parsing than swc. Resolver handles full TS resolution spec (tsconfig paths, package.json exports, PnP). |
| IPC | Tauri invoke (JSON) + Channel<T> for streaming | invoke for request/response <100KB; Channels for scanning pipelines (ordered, typed, faster than events) |
| Type-safe IPC | tauri-specta v2 | Generates TS bindings from Rust command signatures. Must add at project init. Fallback: manual types. |
| Persistence (MVP) | SQLite + FTS5 | Local-first storage for snapshots, search indexing, export metadata. FTS5 built-in for full-text search. |
| State management | Deferred to /plan | zustand, useReducer, or Jotai — all reasonable. Graph state is a projection of Rust-side source of truth. |
| Frontend styling | Tailwind CSS v4 | Matches stack rules. Utility-first, works with React Flow's node components. |
| Package manager | pnpm (frontend) + Cargo (Rust) | pnpm for frontend (fast, strict). Cargo for Rust ecosystem. |
| Testing | Vitest (frontend) + cargo test + proptest + criterion | 3-layer: Rust unit/integration/benchmarks, frontend jsdom+mockIPC, E2E deferred (macOS WKWebView has no driver) |
| Bundler | Vite | Standard for React + Tauri projects. Web Worker support for ELK. |

### Data Pipeline

```
User selects directory
  → Rust: tree-sitter scans .rs and .ts/.tsx files
  → Rust: builds petgraph, topological sort (parents before children)
  → Rust: serialize graph payload via invoke()
  → React: deserialize, render as React Flow nodes (DOM measures sizes)
  → ELK.js (Web Worker): compute positions + parent container sizes
  → React Flow: apply positions, fitView()
```

### Critical Architecture Constraint
Layout cannot happen in Rust. React Flow needs DOM-measured node sizes before ELK can compute positions. Rust handles parsing, graph construction, and serialization. All layout happens in ELK.js on the frontend.

---

## 13. Milestones

### Phase 1 (POC) Milestones

| # | Milestone | What It Proves |
|---|-----------|---------------|
| M1 | **Scaffold + Hello World** | Tauri v2 + React + Vite builds and launches. Rust toolchain verified. Type-safe IPC wired. |
| M2 | **Static graph rendering** | React Flow renders a hardcoded graph fixture with compound nodes + ELK layout. Expand/collapse works. Graph adaptation defaults work. |
| M3 | **Rust graph pipeline** | tree-sitter parses a real directory. petgraph builds the graph. Data flows through invoke() to React Flow. |
| M4 | **Interactive features** | Node detail panel, edge filtering, search, rescan. |
| M5 | **Polish + demo data** | Dark theme, performance tuning, sample graph fixture, all tests green. |

### Phase 2 (MVP) Milestones

| # | Milestone | What It Proves |
|---|-----------|---------------|
| M6 | **Persistence + stable IDs** | SQLite stores graph snapshots. Nodes have stable identity across rescans. |
| M7 | **File watching** | notify + tree-sitter incremental rescan. Graph updates on save. |
| M8 | **Git integration** | Branch diff overlay. Changed-node highlighting. Blast radius. |
| M9 | **VS Code bridge** | Click-to-open. Companion extension with active file tracking. |
| M10 | **Semantic zoom + slices** | Different representations by scale. Preset slice queries. |

---

## 14. Risks

| Risk | Likelihood | Impact | Mitigation | Phase |
|------|-----------|--------|------------|-------|
| ELK layout too slow for 500+ nodes | Medium | Blocks fluid UX | Web Worker isolates main thread; debounce rapid toggles; profile early with criterion benchmarks | POC |
| tree-sitter import resolution is incomplete | High | Edges are wrong | Start with explicit import/use only; skip dynamic imports; show nodes without edges as fallback | POC |
| Flat-to-hierarchical ELK transform has edge cases | High | Layout breaks | Comprehensive unit tests with proptest; test cross-hierarchy edge routing with INCLUDE_CHILDREN | POC |
| React Flow performance degrades with many compound nodes | Medium | Blocks scale | Expand/collapse keeps visible count <200; memoize custom nodes; profile early | POC |
| tauri-specta integration friction | Medium | Delays type safety | Fall back to manual TypeScript types; specta is nice-to-have | POC |
| Rust is new to the developer | Certain | Slower velocity | Lean on compiler errors; start with minimal Rust surface; expand as comfort grows | POC |
| Graph adaptation thresholds need tuning | Medium | Bad defaults | Make thresholds configurable; test against real repos | POC |
| oxc error recovery vs tree-sitter | Low | Missing edges for some files | Fall back to tree-sitter for failed files | MVP |
| Watch mode instability | Medium | Destroys trust quickly | Stable IDs, debounce, viewport preservation, branch-switch safeguards | MVP |
| VS Code extension sprawl | Medium | Extension absorbs the roadmap | Keep it thin and bridge-first | MVP |
| Barrel file resolution infinite loops | Medium | Crashes or hangs | Cycle detection cutoff in transitive resolution | MVP |

---

## 15. Decisions Log

Authoritative decisions are in `research/consolidated-technical-decisions.md`. Key decisions affecting this PRD:

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Product name | Code Atlas | Communicates hierarchical, explorable nature |
| Visualization library | React Flow + ELK.js | Only option supporting compound nested nodes with React JSX |
| Parsing approach (POC) | tree-sitter | Extensible, error-tolerant, incremental. Same parser as GitHub/Neovim/Zed |
| Parsing approach (MVP, TS/JS) | oxc_parser + oxc_resolver | 3x faster parsing, 28-30x faster resolution, handles full TS resolution spec |
| Live refresh | Manual rescan (POC), file watching (MVP) | Proves same pipeline; watching adds complexity |
| Parsing depth | Imports and re-exports only (POC) | Function calls create denser, harder-to-validate graphs |
| Layout location | ELK.js in Web Worker (frontend) | React Flow needs DOM-measured node sizes |
| Product framing | Architecture + change intelligence + agent context | Structure alone is table stakes in 2026 |
| VS Code integration | Stretch goal (POC), thin bridge (MVP) | Workflow adoption, but not required for core hypothesis validation |
| Persistence | None (POC), SQLite + FTS5 (MVP) | In-memory is sufficient for POC; snapshots and search need persistence |
| Graph adaptation | Required for POC | Core to avoiding the "hairball" problem |
| CodeSee status | Acquired by GitKraken (May 2024), standalone product effectively sunset | Not "dead" but not shipping as an independent product |

### Resolved Open Questions (from original PRD)

| # | Question | Resolution |
|---|----------|------------|
| Q1 | Should monorepos be the primary demo scenario? | **Yes.** The market gap and product thesis both point to monorepos as the core use case. The POC's own codebase (Rust + TypeScript Tauri project) is the primary test case. |
| Q2 | Should edges show multiplicity when packages are collapsed? | **Yes, bundle edges** between collapsed packages. Show individual file-to-file edges when both packages are expanded. This is part of graph adaptation (Section 8). |
| Q3 | What demo graph if user doesn't have a project? | **Ship a JSON fixture** of a representative graph, loadable from a "Load Demo" button. |
| Q4 | Should unsupported file types appear as structural nodes? | **Yes.** Show the full project shape. Users can ignore non-parsed files but the full directory structure provides context. |
| Q5 | Higher launch value for Rust-first or TypeScript-first? | **Both at launch.** The POC targets Rust + TypeScript. At MVP, oxc_resolver gives TypeScript a deeper resolution story. Rust resolution is deterministic from Cargo.toml + mod hierarchy. |
| Q6 | Include a second layout algorithm? | **One good default** (ELK layered, direction DOWN) for POC. A layout switcher is P2 — valuable for evaluation but not required to prove the core thesis. |
