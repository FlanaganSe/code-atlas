---
name: graph_viz_research
description: Graph/network visualization library research for Tauri code architecture app (March 2026) — library selection, Rust crates, data flow architecture, React Flow v12 compound node specifics
type: project
---

Researched graph visualization stack for Tauri POC (code architecture viewer: nested packages, files, services).

**Decision:** `@xyflow/react` v12 + `elkjs` (Web Worker) + `petgraph` + `tree-sitter` (syn eliminated — not error-tolerant)

**Why:** React Flow nodes are arbitrary React components (testable with Vitest + RTL, full JSX for custom node UI). ELK.js is the only layout engine that natively handles compound/hierarchical graphs with port-based edge routing. petgraph covers Rust-side graph construction and analysis. syn parses Rust AST for dependency extraction.

**How to apply:** When planning or implementing graph features, this is the decided stack. Dagre was rejected (cross-boundary edge routing bug with subflows — exact failure mode: cross-hierarchy edges break layout, open upstream issue). Cytoscape was rejected (no React JSX inside nodes, weaker Vitest testability). Sigma.js was rejected (no compound nodes).

**Key constraints discovered:**
- Visual layout MUST happen in JS (ELK.js), NOT Rust — node pixel sizes are unknown until React renders
- ELK.js must run in a Vite Web Worker (~1.45MB bundle); main thread must not block during layout
- Data format: Rust sends `{nodes, edges}` JSON via Tauri `invoke`; use `#[serde(rename_all = "camelCase")]` to bridge `snake_case` Rust → `camelCase` TS
- For large graph payloads (>~100KB), migrate from `invoke` to Tauri Channels — known multi-serialization overhead in Tauri v2 (#5641)
- Expand/collapse pattern keeps simultaneously visible nodes under ~200 for React Flow performance

**React Flow v12 compound node specifics (researched 2026-03-17):**
- `parentId` replaces `parentNode` (fully removed in v12). Parent nodes MUST appear before children in the `nodes` array — violations throw a runtime "Parent node not found" error. Topologically sort the petgraph output before serializing.
- Three levels of nesting (package → module → file) confirmed working in the official sub-flows example
- `extent: 'parent'` constrains child drag to parent bounds; `expandParent: true` auto-expands parent when child hits border (no built-in shrink)
- `node.measured.width` / `node.measured.height` are the v12 fields for post-render dimensions — NOT `node.width`/`node.height`. ELK must read from `.measured`.
- ELK flat↔hierarchical transform required: React Flow uses flat array with `parentId`; ELK requires nested `children` tree. Must write encoder/decoder (~100 lines).
- Cross-hierarchy edges (child → different package) require `'elk.hierarchyHandling': 'INCLUDE_CHILDREN'` at root graph level — without this ELK ignores inter-level edges.
- ELK branch ordering within compound nodes is non-deterministic (discussion #4830, unresolved). Workaround: add `elk.priority` layout options per node.
- After ELK layout, parent node `width`/`height` from ELK output must be explicitly applied to React Flow via `setNodes` — React Flow won't auto-resize containers from ELK output alone.
- `useNodesInitialized()` hook: returns true once all nodes are DOM-measured. Run ELK layout only after this fires. For static-size nodes, can skip and provide sizes directly to ELK.
- Expand/collapse: use `node.hidden` (affects nodes + edges). Spread node objects when toggling — mutation doesn't trigger React re-render. After toggle, re-run ELK with only visible nodes.
- Hybrid React Flow + Pixi.js/WebGL: not justified for POC. Revisit only if >300 nodes simultaneously visible with complex renderers.

**Built-in components relevant to architecture viz (all free/MIT):**
- `<MiniMap>` — `nodeColor` function for coloring by type; `pannable`+`zoomable` props
- `<NodeToolbar>` — floating toolbar for expand/collapse buttons
- `<NodeResizer>` — v12 fixed: children no longer move on parent resize
- `<Panel>` — 9 position options for fixed overlays
- Dark mode: `colorMode` prop on `<ReactFlow />`

**Research file:** `research/consolidated-technical-decisions.md` (Section 2, Graph Rendering Stack)
