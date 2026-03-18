# Code Atlas — Product Requirements Document

**Date:** 2026-03-18
**Status:** Draft v5
**Type:** Phased Product Requirements (POC → MVP → Platform → Vision)
**Basis:** Draft v4 PRD, critical architecture review (March 2026), competitive landscape research (March 2026). v5 incorporates: discovered/declared graph separation, edge taxonomy, compatibility report, graph-shaping change handling, public surface lens, config overlay model, identity scheme refinement, scope calibration for honest support contracts, and selective scope adjustments (demote interactive HTML export, FTS5, binary size gate; simplify MCP surface).

---

## 1. Product Summary

Code Atlas is a local-first desktop application that builds a **profiled, evidence-backed architecture graph** from a software repository and renders it as an interactive, zoomable map. It parses source code locally, resolves dependencies within an explicit build context (workspace configuration, module resolution mode, feature flags, condition sets), and produces a hierarchical visualization where developers move from system-level structure to file-level detail on a single continuous canvas.

Before a developer invests time in the graph, the tool provides an upfront **compatibility report** — a clear support contract declaring what is Supported, Partially supported, and Unsupported for the target repository. This is not a footnote. It is a first-class product surface that establishes trust before the visualization does.

The architecture graph is the core product primitive. It serves two surfaces in the near term, with a third emerging in the Vision phase:

1. **Human interface** — interactive canvas with hierarchical zoom, change overlays, saved views, and editor integration.
2. **Agent interface** — structured graph queries via CLI/JSON API, with a thin MCP adapter for coding agent interoperability.
3. **Factory interface** *(Vision)* — architecture-constrained scaffolding, convention enforcement, and portfolio observability. This is aspirational direction, not a shaping force for the first two phases.

The graph is not a picture. It is a queryable, evidence-backed model of how a codebase fits together — parameterized by the build context that determines what "fits together" means, **honest about what it cannot resolve**, and configurable by the repo owner through overlays that supplement (but never silently mutate) what the scanner discovered.

---

## 2. Problem

Developers working in medium-to-large codebases cannot quickly answer "how does this system fit together?" or "what does this change affect?" Existing tools fail in predictable ways:

- **Dead or sunset:** Sourcetrail (archived 2021), CodeSee (acquired by GitKraken May 2024, standalone product sunset).
- **Cloud-dependent:** CodeViz sends code to external servers for AI processing — enterprise dealbreaker. Now offers on-prem, but the default path involves external data flow.
- **Static output:** dependency-cruiser, Madge, Swark, GitDiagram produce images or text that go stale immediately and cannot be queried.
- **Ecosystem-locked:** Nx Graph requires Nx. Turborepo Graph requires Turborepo.
- **Flat graphs:** Every auto-generated tool produces a flat node-and-edge diagram. None support hierarchical zoom with adaptive detail.
- **No build-context awareness:** No tool exposes which graph profile (tsconfig, Cargo features, platform target, workspace scope, condition sets) produced the visualization. Users cannot verify or adjust the analysis context.
- **No trust model:** Tools either show all edges at equal confidence or hide the distinction entirely. Users cannot inspect why a relationship exists, whether the graph is complete, or which language constructs the tool could not analyze.
- **No upfront support assessment:** No tool tells you what it *can* and *cannot* analyze about your specific repo before you invest time in the results. Users discover gaps after trusting the map, which is worse than knowing the limits upfront.
- **No change awareness:** Structure alone is table stakes. No local-first tool combines architecture visualization with branch change overlays and static downstream impact analysis.
- **No shareability beyond screenshots:** Personal exploration tools that produce nothing shareable for onboarding, review, or refactor planning.

The gap is a **local-first, build-context-aware, hierarchically-zoomable architecture and change-intelligence tool** that developers and agents can trust, query, configure, and share — one that is honest about its scope before asking users to trust its output.

### What has changed since this gap was last assessed

GitKraken's Codemaps (from the CodeSee acquisition) and CodeViz have both shipped updates in this space. CodeViz now markets automatic architecture modeling, PR visualization, shared workspaces, version history, embeddable diagrams, API access, and on-prem deployment. GitKraken Codemaps remains in early access as of March 2026 — not GA. Nx defaults its MCP server to minimal mode because broad workspace-analysis tools proved less efficient than focused skills and direct CLI. The competitive wedge is no longer "visual code maps don't exist." It is **trustworthy local graphs with explicit evidence, build-context awareness, repo-local configurability, upfront support honesty, and agent interoperability.** See Section 21 for full competitive positioning.

---

## 3. Product Thesis

If we give developers a fast local desktop app that:

1. Runs an upfront **compatibility report** declaring what it can and cannot analyze for a specific repo — before the graph renders
2. Discovers workspace structure and builds a **profiled architecture graph** — parameterized by build context, not assumed to be singular
3. Renders that graph as a nested zoomable map with adaptive detail
4. Exposes graph health, edge provenance, **edge categories** (value vs. type-only, runtime vs. dev), and **unsupported construct badges** so users can verify what they see and understand what the tool *couldn't* analyze
5. Accepts **repo-local configuration as overlays** — manual edges, view-level suppressions, metadata annotations — that supplement but never silently mutate the discovered graph
6. Overlays change intelligence (branch change overlays with **graph-shaping change awareness**, static downstream impact with edge-category filters, affected slices)
7. Produces **saved views, snapshots, and shareable exports** for onboarding, review, and planning workflows
8. Provides a **public surface lens** showing what each package exposes to consumers, not just internal import structure
9. Integrates with editors and coding agents as a shared architecture context layer

...then the app becomes more valuable than static diagrams because it supports real development tasks: onboarding, refactor planning, dependency inspection, code review, change impact analysis, and agent-assisted development.

The winning product is the one engineers trust. Trust comes from:
- **Upfront support honesty:** users know what the tool can and cannot handle for their repo before they invest time.
- **Explicit profiles:** users know what build context produced the graph.
- **Edge provenance and taxonomy:** every relationship carries evidence and semantic category.
- **Graph health + unsupported construct transparency:** the tool tells you what it couldn't resolve *and* which language constructs it didn't model.
- **Immutable discovered graph:** config overlays supplement the graph; they never silently delete discovered edges.
- **Repo-local configuration:** an escape hatch for when static analysis isn't enough.
- **Workspace-first framing:** the graph starts from packages/crates, not incidental file connectivity.

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

### Target repo scale (v1)

Code Atlas v1 targets real-world TypeScript and Rust monorepos — the kinds of workspaces developers actually work in. Performance targets are validated against repos up to ~5,000 source files, with architecture designed to scale beyond that through lazy hydration and slice-first workflows in future versions. The product is honest about its scope: supported repo shapes get accurate graphs; partially-supported shapes get graphs with clear badges; unsupported shapes get an upfront explanation.

### Jobs To Be Done

| JTBD | Core Need | Phase |
|------|-----------|-------|
| **"Can this tool handle my repo?"** | Upfront compatibility report: Supported, Partial, Unsupported — with specific reasons | POC |
| **"How does this repo fit together?"** | Workspace/package topology first, then module/file drill-down. Build-context-aware. | POC |
| **"Can I trust this graph?"** | Graph health indicators, unresolved imports, unsupported construct badges, active profile display, edge provenance | POC |
| **"Show me only what's relevant to my task."** | Dependency slices, entrypoint paths, changed-file neighborhood | MVP |
| **"What does this change affect?"** | Changed nodes, static downstream impact with edge-category filters, affected slice, graph-shaping change alerts | MVP |
| **"What does this package expose?"** | Public surface lens: package exports, public modules, API boundaries | MVP |
| **"Let me save and share what I found."** | Named views, bookmarks, graph snapshots, static export (SVG/PNG) | MVP |
| **"Let me move between map and editor instantly."** | Open file at range, reveal current file, selection sync | MVP |
| **"Give my agent the same understanding."** | CLI/JSON API with MCP adapter, structured graph queries, stable IDs | Platform |

---

## 5. Product Principles

1. **Local analysis, explicit egress.** All scanning and graph construction happen on-device. Code is never transmitted for analysis. Data leaves the machine only through explicit user actions — export, MCP query responses, shared artifacts — and the product clearly labels what is being shared when it is.
2. **Truth-model-first.** The graph model, its correctness, and its provenance are the product. The canvas is a surface, not the core.
3. **Profiled, not absolute.** There is no single "correct graph" for a repository. The graph is parameterized by build context (workspace scope, tsconfig, Cargo features/targets, resolution mode, condition sets). The active profile is always visible.
4. **Honest about limits.** The tool reports what it couldn't resolve *and* which constructs it didn't model (cfg gates, build scripts, proc macros, dynamic imports, framework conventions). Graph health, unsupported construct badges, and the upfront compatibility report are first-class UX, not debug features.
5. **Configurable truth, immutable discovery.** Static analysis alone cannot fully describe real-world architecture. The tool accepts repo-local configuration as **overlays** — manual edges, view-level suppressions, metadata annotations — that supplement the discovered graph without mutating it. The discovered graph is append-only. Configuration never silently deletes observed edges; it can suppress them in views with visible annotations.
6. **Hierarchy over hairball.** Nested structure, adaptive detail. The default view is workspace/package topology, not a file-import explosion.
7. **Every edge has provenance and category.** Expose evidence class (syntactic/resolver-aware/heuristic) and semantic category (value import, type-only import, dev dependency, manual override). This is the trust advantage. The discovered graph is the source of truth; overlays are visibly layered on top.
8. **Separated core.** The analysis engine is a standalone Rust library/service callable over stdio/IPC, not logic embedded in the desktop shell. This enables CLI, MCP, headless scans, editor bridges, and future remote support.
9. **Workflow over spectacle.** Measure by "helps developer decide faster," not "looks impressive."
10. **Testable core.** Graph logic and transforms testable without rendering. Correctness measured against a golden corpus of reference repositories.
11. **Detector seams.** The architecture assumes language/framework analysis is pluggable internally from day one. Public plugins wait; internal extension points do not.
12. **Replaceable internals.** No assumption of a single parser, layout engine, or renderer.

---

## 6. Phased Scope

### 6.1 Phase 1 — POC

**Purpose:** Validate that the analysis core can discover workspace structure, build a profiled architecture graph with an immutable discovered layer, and that the Tauri desktop shell can render it as a navigable hierarchical map with trust signals. Prove the truth model, the scanner/shell separation, the discovered/declared graph separation, the edge taxonomy data model, the internal detector architecture, and the compatibility report — not the canvas polish.

**Supported repo archetype (POC):**

The POC does *not* claim equal support for "Rust and TypeScript monorepos" in the abstract. It targets a specific, declared archetype:

- **Primary archetype:** Tauri-style monorepo — Cargo workspace with one or more crates + pnpm/npm/yarn workspace with TypeScript packages. The project's own repository is the primary test case.
- **TypeScript POC scope:** Workspace package discovery, tree-sitter parsing, import resolution via tsconfig `paths`/`baseUrl` discovery and basic path-based resolution. NOT full `exports`/`imports` conditions, NOT PnP, NOT project references. Unsupported constructs are badged. The compatibility report flags these as "Partial" or "Unsupported" with specific reasons.
- **Rust POC scope:** `cargo metadata` for workspace members, packages, dependencies, targets, and default features. Tree-sitter for mod/use/pub use hierarchy. NOT cfg-aware beyond default features, NOT build.rs-generated code, NOT proc-macro expansion. Unsupported constructs are badged. The compatibility report flags these as "Partial" with specific reasons.

**What we build:**

1. **Analysis core as a separate Rust library** — the scanner, graph builder, and query engine live in a standalone crate (`codeatlas-core`) callable over stdio/IPC. The Tauri app is a thin desktop client. This separation is a POC architectural requirement.
2. Open a local repository via native file dialog
3. **Compatibility report** — runs during workspace discovery, before or alongside the first scan. Produces a structured assessment:
   - Per-language support status: **Supported**, **Partial** (with specific unsupported constructs), **Unsupported** (with reasons)
   - Detected constructs that affect analysis completeness (Yarn PnP, project references, conditional exports, `build.rs`, proc macros, `cfg` gates, generated code, custom conditions)
   - Package manager detection and support status
   - Module resolution mode detection
   - This is the product's **support contract** — users know what to expect before trusting the map
4. **Workspace discovery** — detect workspace structure automatically:
   - Rust: `cargo metadata --format-version 1` for workspace members, packages, dependencies, targets, features
   - TypeScript/JS: detect `pnpm-workspace.yaml`, `package.json` `workspaces` field; discover applicable `tsconfig.json` files
5. **Graph profile** — expose the active analysis context (see Section 8) including what the profile *does not* cover (unsupported constructs)
6. Scan supported source files with tree-sitter via internal **detector seams** (language-specific detector modules with a defined interface), resolve imports using workspace-aware path resolution
7. **Edge taxonomy data model** — from the first scan, edges carry not only evidence class but also semantic category: value import vs. type-only import (`import type`), normal vs. dev vs. build dependency (from manifests), structural containment, re-export. Tree-sitter can distinguish `import type { Foo }` from `import { Foo }` in TypeScript; `cargo metadata` distinguishes normal/dev/build dependencies. Capture this from day one.
8. Build a hierarchical architecture graph in Rust (petgraph) with **immutable discovered graph + config overlay separation** (see Section 11). The discovered graph represents what the scanner found. Config overlays add manual edges and suppress edges in views. These are distinct layers in the data model.
9. **Progressive rendering** — stream graph data to the frontend via Tauri `Channel<T>`:
   - Phase 1: workspace/package topology (renders immediately)
   - Phase 2: module/folder structure
   - Phase 3: file-level nodes and import edges
   - User can interact with partial results; cancel is supported
10. Render as a zoomable nested map with React Flow + ELK
11. **Workspace/package lens as default view** — top-level shows packages/crates/apps with inter-package dependency edges. File-import graph is a drill-down, not the landing page.
12. Expand/collapse at multiple nesting levels with adaptive defaults
13. **Compatibility report display** — shown during/after first scan, before the user invests time in the graph. Accessible from the profile panel thereafter.
14. **Graph health indicators** — active profile badge, unresolved import count, parse failure count, confidence summary, **unsupported construct badges** (see Section 9)
15. **Basic edge provenance and category** — each edge carries its evidence class (syntactic, resolver-aware, structural) and semantic category (value, type-only, dev, build, normal) visible on hover/click
16. Node detail panel with edge evidence, search (Cmd+K)
17. Manual rescan after code changes
18. **Repo-local config schema designed** — `.codeatlas.yaml` schema designed, documented, and parseable in the POC. The config model uses the **overlay architecture**: discovered graph is immutable; config creates overlays that add manual edges and suppress edges in views. At minimum `ignore` paths and `entrypoints` are functional in POC. (See Section 11)
19. Built-in demo graph fixture
20. Automated tests: golden corpus + coverage targets (see Section 22)

**Graph levels (POC):**
1. Workspace / monorepo root
2. Package / crate / app
3. Module / folder
4. File

Function-level and symbol-level visualization are out of scope.

**Identity scheme (POC):**
The identity scheme is foundational — it affects every downstream feature (watch mode, viewport preservation, diff overlays, graph snapshots, saved views, selection sync, agent references). The POC must define two concepts:

- **Materialized node key:** where this node is in this repo/profile/snapshot. Format: `{workspace_root}:{language}:{entity_kind}:{relative_path}`. Used for current-session addressing.
- **Logical lineage key:** what should survive rename/move for bookmark preservation and diff history. Policy: a rename is a new materialized key but preserves the lineage key for up to one rename hop. Tracking uses a hybrid approach: git rename detection as the primary signal, supplemented by broader rescans when branch switches or manifest/config changes are detected. Content and AST fingerprints are a designed extension point, activated in Platform. This is designed in the POC, persisted in MVP.

See Section 10 for full identity scheme specification.

**Detector architecture (POC):**
Language analysis is organized as internal detector modules with a defined `Detector` trait/interface. The POC ships two built-in detectors (Rust, TypeScript). The interface is internal — no public plugin API yet — but it is explicitly designed so that framework-specific detectors (route conventions, generated clients, test↔source relationships) can be added without invasive changes. See Section 14.

**Success criteria:**
1. A developer points the app at the project's own Tauri monorepo (or a similar Cargo + TS workspace) and sees a workspace-level package graph with correct inter-package dependencies.
2. **Compatibility report** is displayed during/after first scan showing Supported/Partial/Unsupported status per language, with specific reasons for any gaps.
3. The active graph profile is visible in the UI. The user can see what workspace scope, resolution mode, and feature set produced the graph.
4. **Unsupported construct badges** clearly show what the scanner did NOT analyze (e.g., "cfg gates not evaluated," "build.rs output not included," "package.json exports conditions not resolved").
5. Graph health indicators show: total nodes, edges resolved, imports unresolved, files with parse failures, unsupported constructs detected.
6. Clicking an edge shows its evidence class, semantic category, and source location.
7. **Edge categories** are captured from scan data: value vs. type-only imports (TypeScript), normal vs. dev vs. build dependencies (Cargo).
8. Initial scan with progressive rendering: first meaningful frame (package topology) <2 seconds for ≤2,000 files. Full graph <10 seconds.
9. 3+ levels of nesting with working expand/collapse.
10. Zoom, pan, selection, expand/collapse remain smooth at 60fps when visible nodes <200.
11. **Golden corpus:** correct graph on the project's own repo + at least 2 reference repos per supported language. Explicit unsupported-construct detection verified. Compatibility report accuracy verified.
12. >80% test coverage on non-UI Rust and TypeScript logic (coverage is hygiene, not the headline metric).
13. Zero network calls.
14. The analysis core (`codeatlas-core`) is callable from a test harness independent of the Tauri shell.
15. `.codeatlas.yaml` is parseable; at minimum `ignore` paths and `entrypoints` are functional. The overlay model (add/suppress, not add/remove) is implemented in the data model.
16. **Discovered graph immutability** is enforced: config overlays cannot delete edges from the discovered layer.

**Non-goals (POC):**
- Production polish, branding, onboarding UX
- Function-level or call-site-accurate cross-language graphing
- Git history / blame / churn visualization
- AI-powered summaries, agents, or architecture advice
- Runtime tracing or service interaction detection
- Cloud collaboration
- Code editing within the visualization
- Distribution / code signing / notarization
- VS Code integration beyond stretch-goal click-to-open
- Persistence (SQLite)
- File watching (automatic rescan)
- Change intelligence (diff overlays)
- Dead code or orphan detection
- Remote workspace support (local workspaces only)
- Mermaid/D2 export
- Public plugin API
- Full TS exports/imports condition resolution (deferred to MVP with oxc_resolver)
- cfg-aware Rust analysis beyond default features (deferred to MVP)
- Public surface lens (deferred to MVP)
- Interactive HTML export (deferred to Platform)

**Stretch goals (if time allows):**
- Click-to-open file in VS Code (local-only; `code -g file:line:col` with `code-insiders` fallback; see Section 15)
- Circular dependency highlighting (petgraph SCC — trivial once graph is correct)
- SVG/PNG export of current view
- User-adjustable graph profile (change feature set, toggle dev dependencies)

### 6.2 Phase 2 — MVP

**Purpose:** Ship to real users. Adds the features that make Code Atlas a daily-use tool rather than a demo. Focus remains on trust and workflow fit. The critical additions beyond POC are **honest change intelligence**, **public surface lens**, **edge-category-aware impact analysis**, and **shareability**.

**What changes from POC:**
- tree-sitter for TS/JS upgraded to **oxc_parser + oxc_resolver** (faster, handles full TypeScript resolution spec including tsconfig paths, project references, package.json exports/imports with condition names, Yarn PnP support)
- Rust profile expanded: **cfg-aware graph** using `cargo metadata` features + targets; the profile UI lets users select feature combinations and target triples; cfg-gated code paths are visually distinguished. This remains an approximation — `cargo metadata` cannot capture `build.rs`-emitted custom cfgs or proc-macro expansion. The compatibility report and unsupported construct badges make this explicit.
- SQLite persistence for snapshots, stable identity + lineage across rescans
- User-configurable graph profiles via **canonical presets** (Node ESM-first, Node CJS-first, Bundler) plus toggleable dimensions (dependency kinds, Cargo features/targets, TS condition sets). Full free-form profile editing deferred to Platform.
- `.codeatlas.yaml` fully functional with overlay model: manual edges (add), view-level suppressions (suppress), metadata annotations, framework detector hints
- Compatibility report updated to reflect expanded analysis capabilities (oxc_resolver, cfg-aware Rust)

**What's added:**

1. **File watching** with incremental rescan (notify + tree-sitter incremental, 300-500ms debounce). Branch switches and manifest/config changes trigger broader rescans rather than incremental updates.
2. **Change overlay against base ref** — compare current branch to a user-selected base ref (default: merge-base with main). This is a **change overlay on the current graph**, not a true architecture diff. Changed files are mapped to current graph nodes. Shows added/modified nodes and their structural neighbors. Explicitly labeled as "change overlay" — the base ref and mode (committed/working tree) are always visible. **Graph-shaping changes** (manifests, lockfiles, tsconfigs, `.codeatlas.yaml`) are classified separately and trigger alerts — see Section 12.
3. **Static downstream impact with edge-category filters** — "select a file, see everything that transitively depends on it in this graph profile." Color by graph distance. **Filter by edge category:** exclude type-only imports, exclude dev/test edges, show only runtime impact path. Explicitly labeled as static reachability, not runtime impact prediction.
4. **Public surface lens** — a filtered view showing what each package exposes to consumers:
   - TypeScript: items reachable from `package.json` `exports`/`main`/`module` entry points
   - Rust: public items (`pub`) reachable from the crate's `lib` target
   - Cross-package edges filtered to show only those that cross public surface boundaries
   - Useful for onboarding ("what can I use from this package?"), refactor planning ("is this a public API I'd be breaking?"), and understanding package contracts
5. **Semantic zoom** — different representations by scale (system overview → focused slice → module detail → file detail)
6. **Slice-based navigation** — upstream, downstream, path between A and B, changed-files-only
7. **Click-to-open in VS Code** — local workspaces only; `code -g file:line:col` with URI scheme fallback (see Section 15)
8. **Thin VS Code companion extension** — active file tracking, "Show in Code Atlas" context menu, selection sync. Local workspaces only.
9. **Edge provenance inspector** — click any edge to see: evidence class, semantic category (value/type-only/dev/build/manual), source file + range, resolution method used, confidence level, active profile at scan time, whether the edge is suppressed in any overlay
10. **Graph health dashboard** — expanded from POC indicators: full unresolved import list, ambiguous resolution list, generated/ignored file list, fallback logic explanations, unsupported construct inventory, profile comparison, compatibility report history
11. **Named views and bookmarks** — save a viewport position + expand/collapse state + active filters as a named view. Bookmark individual nodes or slices.
12. **Graph snapshots** — persist a full graph state (nodes, edges, evidence, profile) to SQLite. Snapshots enable future architecture diff.
13. **Static export** — export the current view as SVG or PNG. These are the MVP shareable artifacts for onboarding, review, and planning.
14. **Built-in detector modules** for high-value patterns: test↔source file relationships, barrel file/re-export chains. Internal, not plugin-based.

**De-prioritized from original PRD:**
- **Dead code / orphan detection** — deferred. "Zero in-degree and not an entry point" is a weak heuristic with high false-positive rates (dynamic imports, public package exports, CLI targets, tests, proc-macro crates, convention-based entrypoints). When it ships, it will be named **"possibly unreferenced internal module"**, require explicit entrypoint configuration via `.codeatlas.yaml`, and carry a low-confidence badge.
- **Service interaction slices** — deferred to Platform phase. Static import edges cannot detect HTTP/gRPC/Kafka/database communication. Requires framework-specific detectors or runtime data (OpenTelemetry).
- **Mermaid/D2 text export** — low priority. Does not address trust, workspace awareness, or workflow fit.
- **True architecture diff (graph-to-graph comparison)** — deferred. Requires dual scans, snapshot comparison, and lineage-based identity matching. Snapshots land in MVP; the diff feature lands in Platform. The MVP ships "change overlay against base ref" instead, which is lighter and very useful but is not the same thing.
- **Interactive HTML export** — deferred to Platform. SVG/PNG covers the core sharing need. Interactive HTML introduces privacy concerns (file paths and names embedded in shareable artifacts), artifact size concerns, and staleness problems. It does not create competitive differentiation — CodeViz already offers richer export formats. The wedge is trust and honesty, not export fidelity.
- **Full-text search indexing (FTS5)** — deferred to Platform. In-memory fuzzy search over the current graph is sufficient for MVP workflows. FTS5 becomes valuable when searching across many persisted snapshots, which is a Platform-era use case.

**Success criteria (MVP):**
1. All POC criteria plus:
2. Compatibility report updated to reflect expanded analysis (oxc_resolver, cfg-aware Rust)
3. Graph updates within 2 seconds of file save (watch mode)
4. Branch switches and manifest/config changes trigger broader rescans with compatibility report refresh
5. Change overlay correctly highlights changed files and their structural neighbors for a real branch, with base ref visible and labeled as "change overlay"
6. **Graph-shaping changes** (Cargo.toml, package.json, lockfiles, tsconfig.json, .codeatlas.yaml) are classified distinctly in change overlays with alerts
7. A developer can answer "what does this change affect (statically)?" from the UI, **filtered by edge category** (e.g., "show only runtime impact, excluding type-only and dev edges")
8. **Public surface lens** correctly identifies package exports/public API for both TypeScript (package.json exports/main) and Rust (pub items from lib target)
9. VS Code round-trip (map → editor → map) works in <1 second for local workspaces
10. Graph health dashboard accurately reports unresolved imports, resolution ambiguities, and unsupported constructs
11. User can switch graph profiles (e.g., toggle dev dependencies, change Cargo feature set, select TS condition sets via presets and toggleable dimensions) and see the graph update
12. TS/JS resolution is correct for repos using `exports`/`imports` conditions, project references, and tsconfig paths (verified against golden corpus)
13. Named views can be saved and restored. SVG/PNG export produces useful shareable artifacts.
14. `.codeatlas.yaml` overlays are respected: manual edges appear with "manual" evidence class, suppressed edges are hidden in default view (visible with toggle), metadata annotations display in node detail panel
15. **Discovered graph immutability** is maintained: suppressed edges are still present in the data model and visible when the user toggles "show suppressed"
16. **Golden corpus expanded:** correct graph on 5+ reference repos across both languages, including repos with supported and unsupported constructs

### 6.3 Phase 3 — Platform

**Purpose:** Make the architecture graph a shared context layer for humans, agents, and automation. The **CLI/JSON API is the primary automation interface**; MCP is a thin adapter, not the core.

**What's added:**

1. **CLI / JSON API** — the primary automation and scripting interface. The analysis core already supports stdio from POC. Platform adds a stable, documented command surface:
   - `codeatlas scan <dir>` — full scan with JSON output
   - `codeatlas health <dir>` — compatibility report + graph health
   - `codeatlas deps <node-id>` — direct imports/dependents with provenance
   - `codeatlas impact <file-path>` — static transitive dependents with edge-category filters
   - `codeatlas search <query>` — fuzzy search across graph
   - `codeatlas cycles` — all dependency cycles
   - `codeatlas affected <changed-files>` — architecture slice for a set of changed files
   - `codeatlas profile` — active graph profile
   - `codeatlas snapshot list|show|diff` — snapshot management
2. **Thin MCP adapter** — exposes a focused subset of CLI capabilities as MCP resources and tools for coding agent interoperability:
   - **Resources** (read-only): `graph://overview`, `graph://health`, `graph://profile`
   - **Tools** (read-only queries): `get_dependencies`, `get_downstream_impact`, `get_affected_slice`, `search_nodes`
   - Designed with MCP's consent/control model: all read-only, no mutation
   - Deliberately minimal — full graph dumps are kept out of the prompt path to manage token usage
   - The CLI/JSON surface is always richer; MCP is the adapter
3. **True architecture diff** — graph-to-graph comparison using persisted snapshots. Shows added/removed/modified nodes and edges between two graph states. Uses lineage-based identity matching for rename/move awareness. Requires at least one base snapshot.
4. **Architecture rules** (`.codeatlas.yaml` `rules` section) — forbidden imports, layering violations, ownership boundary crossings. Rules operate on repo-local metadata (tags, layers, owners), not just import topology.
5. **Possibly unreferenced internal module detection** — with explicit entrypoint configuration from `.codeatlas.yaml`, low-confidence badges, and opt-in activation
6. **Plugin system** — user-defined analysis rules, custom node types, framework-specific detectors. The internal detector seam from POC becomes a public API.
7. **Interactive HTML export** — self-contained interactive snapshot viewable in browser. Ships here rather than MVP because it requires solving privacy (redaction controls), artifact size management, and staleness labeling. Not a competitive differentiator — trust and honesty are.
8. **FTS5 search indexing** — full-text search across persisted snapshots. Enables cross-snapshot search and historical queries.
9. **Identity scheme enhancement** — content and AST fingerprints supplement git rename detection for more robust lineage tracking across major refactors.
10. **Remote scanner deployment model** — architecture for running the analysis core on a remote host (container, SSH, Codespaces), with the desktop app connecting over a network transport. Design only — implementation in Vision.

### 6.4 Phase 4+ — Vision

Directional only. Not spec'd in detail. These items are aspirational and should not shape the first two phases.

**Architecture intelligence:**
- **Semantic parsing upgrade:** rust-analyzer + TypeScript compiler/LSP for semantic-grade edges (type-aware references, implementations, call hierarchy)
- **SCIP integration** for normalized cross-language semantic indexes
- **Watch mode with animated graph diffs** (stable IDs + d3-timer position interpolation + diff color coding)
- **Ownership/churn/risk overlays** from CODEOWNERS + git history

**Factory capabilities:**
- **Architecture-constrained scaffolding** — generate new modules/services that conform to detected conventions and existing architecture boundaries
- **Portfolio observability** — cross-project architecture health, dependency currency, drift detection
- **Preemptive plan ghost overlays** — structured plan schema → predicted change overlay before code lands (from markdown plans or structured formats)
- **Greenfield project creation** — scaffold new projects from architecture templates that enforce discovered conventions

**Collaboration and scale:**
- **Loro CRDT + mDNS for LAN collaboration** (team architecture sessions)
- **WebGPU renderer** (progressive enhancement for very large graphs)
- **Remote workspace support** — VS Code Remote / Codespaces / SSH, using the remote scanner deployment model from Platform
- **Local AI summaries** (opt-in, small code models for natural-language architecture explanations)
- **Full free-form profile editor** — arbitrary profile dimension configuration beyond presets and toggles

---

## 7. User Stories

### Phase 1 (POC)

#### US-1: Open a codebase and see workspace topology
> As a developer, I open the app, select a project directory, and within seconds see a high-level graph showing my workspace packages as nested boxes with inter-package dependency edges.

**Acceptance criteria:**
- "Open Directory" button triggers native file dialog
- Rust backend discovers workspace structure (`cargo metadata` for Rust, workspace config detection for JS/TS)
- Progressive rendering: package-level topology appears first (<2 seconds), file-level detail streams in afterward
- Active graph profile badge is visible (workspace root, detected package manager, resolution mode)
- Packages appear as primary group nodes; inter-package edges show dependency relationships
- File-level import edges appear when user drills into a package

#### US-2: Understand repo compatibility and analysis limits
> As a developer, I can see — before investing time in the graph — what the tool supports, partially supports, and cannot analyze in my repo, so I know what to expect and what to be cautious about.

**Acceptance criteria:**
- **Compatibility report** is displayed during/after first scan with structured Supported/Partial/Unsupported status per language and construct
- Report shows specific reasons for any partial or unsupported status (e.g., "Yarn PnP detected — not supported in POC," "2 crates use build.rs — generated code not analyzed," "exports conditions not evaluated")
- Report is accessible from the profile panel after initial display
- Profile badge/panel shows: workspace root, detected packages, language coverage, resolution approach
- For Rust: Cargo workspace members, active target(s), default features, **unsupported constructs** (cfg gates beyond default features, build.rs output, proc macros)
- For TypeScript: detected tsconfig(s), module resolution mode, workspace package manager, **unsupported constructs** (exports/imports conditions not evaluated in POC, dynamic imports, framework conventions)
- Unsupported construct badges are visually prominent, not buried in settings

#### US-3: Verify graph health
> As a developer, I can see at a glance whether the graph is complete or has gaps, so I know how much to trust it.

**Acceptance criteria:**
- Health indicator shows: total nodes, edges resolved, unresolved imports (count + list on click), files with parse failures (count + list on click), unsupported constructs (count + type + list on click)
- Unresolved imports are visually distinct (dashed edges or warning badges)
- Parse failures appear as structural nodes with a warning indicator
- Unsupported construct nodes are marked with an explanatory badge ("cfg-gated: not evaluated in this profile")

#### US-4: Zoom and pan
> As a developer, I can zoom in to see file-level detail within a package, and zoom out to see the workspace overview.

**Acceptance criteria:**
- Mouse wheel / trackpad pinch to zoom, click-drag to pan
- MiniMap showing current viewport position
- Fit-to-view button resets to full graph
- Zoom level affects visible detail: zoomed out = package labels only, zoomed in = file names + edge labels visible

#### US-5: Expand and collapse
> As a developer, I can click a package or module node to expand it (revealing contents) or collapse it (showing a summary box).

**Acceptance criteria:**
- Each group node has an expand/collapse toggle
- Collapsed: shows name + summary (e.g., "12 files, 4 exports")
- Expanded: shows child nodes inside the package boundary
- Toggling triggers ELK re-layout of visible nodes
- Default state follows graph adaptation rules (Section 13)

#### US-6: Inspect a node and its edges
> As a developer, I can click a node to see its details and inspect the evidence and category behind its edges.

**Acceptance criteria:**
- Click a node opens a detail panel (right side, collapsible ~300px)
- Shows: file path, node kind, direct dependencies (in/out count), declared exports
- Highlights all edges connected to the selected node
- Click an edge: shows evidence class (syntactic, resolver-aware, structural), **semantic category** (value, type-only, dev, build, normal, manual), source location, resolution method
- Click an edge target to navigate to the connected node

#### US-7: Search
> As a developer, I can search for a file or package by name and the graph pans to center on it.

**Acceptance criteria:**
- Command palette (Cmd+K) with fuzzy match on node labels
- Selecting a result: centers viewport on node, expands parent packages if collapsed, highlights the node

#### US-8: Demo data
> As someone evaluating the app, I can load a built-in sample graph without having a project ready.

**Acceptance criteria:**
- "Load Demo" button loads a JSON fixture of a representative multi-package graph
- The demo graph demonstrates all interaction patterns (expand/collapse, filtering, search, health indicators, edge provenance with categories, unsupported construct badges, compatibility report)

### Phase 2 (MVP)

#### US-9: See what changed on this branch
> As a developer, I can see which files changed on my current branch and how they relate to the architecture, with clear alerts when changes affect the graph structure itself.

**Acceptance criteria:**
- Base ref selector (default: merge-base with main; user can change)
- **Change overlay** shows: modified nodes (amber), nodes with new files (green), structurally affected neighbors (blue/dim)
- **Graph-shaping changes** (manifests, lockfiles, tsconfigs, `.codeatlas.yaml`) are classified separately with a prominent alert: "These changes may affect the graph structure — consider rescanning"
- "Show only changed slice" filter
- Active base ref is always visible in the UI, labeled "Change overlay against {base-ref}"
- Click-through from changed node to file diff
- **Explicitly does not claim to show added/removed architecture edges** — this is file-change mapping onto the current graph, not a graph-to-graph comparison

#### US-10: Understand static downstream impact
> As a developer, I can select a file and see everything that transitively depends on it in the current graph profile, filtered by edge category.

**Acceptance criteria:**
- Select node → "Show downstream impact" action
- Transitively dependent files highlighted, colored by graph distance (direct = red, 2-hop = orange, 3+ = yellow)
- **Edge-category filters:** toggle type-only imports, dev/test edges, build edges. Default: runtime-only impact (excludes type-only and dev edges)
- Count summary: "47 files in static downstream path (runtime only)" with filter state visible
- Label explicitly says "static" — does not claim runtime impact prediction

#### US-11: VS Code round-trip (local workspaces)
> As a developer working locally, I can click a node to open the file in VS Code, and from VS Code I can reveal the current file in Code Atlas.

**Acceptance criteria:**
- Click node → file opens in VS Code at correct line (local workspaces only)
- VS Code: "Show in Code Atlas" command → atlas pans to that file's node
- Works with `code` and `code-insiders` CLI; falls back to `vscode://` URI scheme

#### US-12: Save and share architecture views
> As a developer, I can save a view of the architecture and share it with teammates for onboarding, review, or planning.

**Acceptance criteria:**
- Save current viewport + expand/collapse state + filters as a named view
- Bookmark individual nodes (persistent across rescans via lineage key)
- Export current view as SVG or PNG
- Named views survive rescans (resolved via lineage keys)

#### US-13: Understand package public surface
> As a developer, I can see what each package exposes to consumers — its public API boundary — separate from its internal structure.

**Acceptance criteria:**
- **Public surface lens** toggle shows only public-facing items per package:
  - TypeScript: items reachable from `package.json` `exports`/`main`/`module` entry points
  - Rust: `pub` items reachable from the crate's `lib` target
- Cross-package edges filtered to only those crossing public surface boundaries
- Package summary shows: "N public exports, M internal modules"
- Useful for answering "what can I use from this package?" and "would this refactor break a public API?"

---

## 8. Graph Profile

This is a foundational product concept. There is no single "correct graph" for a repository. The architecture graph is parameterized by build context.

### Why profiles exist

- **Rust:** Cargo features are explicit conditional compilation. `#[cfg(feature = "X")]` gates entire modules, impls, and dependency edges. Workspace membership, targets (lib/bin/example/test/bench), and platform cfgs further shape the graph. `cargo metadata` output changes based on `--features`, `--no-default-features`, and `--filter-platform` flags. **What cargo metadata gives:** full dependency graph with resolved versions, feature activations, workspace structure, package targets, dependency kinds (normal/dev/build). **What it misses:** build script output (custom cfg flags, generated code in OUT_DIR), proc-macro expansion results, actual source code or AST, type information, which code paths are reachable, and host-vs-target dependency distinction. A crate with 10 feature flags has 1,024 possible combinations; add cross-platform and test/release modes and the configuration space is enormous. `cfg_attr(target_os, path = "...")` can change which files are even part of the module tree. There is no way to get a complete picture without executing build scripts and proc macros — any model that claims completeness without code execution is approximate.
- **TypeScript/JS:** The meaning of an import depends on `moduleResolution` mode (`node16`/`nodenext`/`bundler`), `tsconfig.json` `paths`/`baseUrl`/`references`, and `package.json` `exports`/`imports` fields with condition names (`import`, `require`, `browser`, `development`, `production`). The same import statement can resolve to different files under different conditions. Key differences: `bundler` mode allows extensionless imports and does NOT include the `node` condition by default; `nodenext` forbids extensionless imports in ESM context and allows `require()` of ESM (reflecting Node v22.12+); `node16` treats `require()` of ESM as an error. Condition key ordering in `exports` objects is load-bearing — first match wins. TypeScript project references create explicit inter-project dependency boundaries where dependents see `.d.ts` output, not source. Yarn PnP replaces `node_modules` with a different resolution mechanism entirely. Mixed-format monorepos (some packages CJS, some ESM) need per-package resolution configuration.
- **Both:** Dev dependencies, build dependencies, and test targets create edges that may or may not belong in an "architecture" view. The edge taxonomy (Section 9) captures these categories so users can filter by what matters for their task.

### Profile dimensions

| Dimension | Rust | TypeScript/JS | Default |
|-----------|------|---------------|---------|
| **Workspace scope** | Workspace members from `cargo metadata` | Packages from workspace config | All workspace members |
| **Dependency kinds** | normal / dev / build | production / dev / peer / optional | Normal/production only |
| **Features** | `--features`, `--all-features`, `--no-default-features` | N/A | Default features |
| **Platform/target** | `--filter-platform <triple>` | N/A | Host platform |
| **Resolution mode** | N/A | `moduleResolution` from tsconfig | Auto-detected from tsconfig |
| **Condition sets** | N/A | `exports`/`imports` condition names (import, require, browser, etc.) | Auto-detected from moduleResolution mode |
| **Project references** | N/A | tsconfig `references` array | Auto-detected |
| **Include external deps** | Show crates outside workspace | Show node_modules packages | No (workspace-internal only) |
| **Unsupported constructs** | cfg beyond selected features, build.rs output, proc-macro expansion | Dynamic imports, framework conventions, PnP (POC only) | Badged in profile display |

### Canonical starting profiles

Based on module resolution research, three profiles cover 90%+ of real-world TS/JS repos:

| Profile | module | moduleResolution | conditionNames | extensionless imports |
|---------|--------|-----------------|----------------|----------------------|
| **Node ESM-first** | `nodenext` | `nodenext` | `["node", "import"]` | Forbidden |
| **Node CJS-first** | `nodenext` | `nodenext` | `["node", "require"]` | Forbidden |
| **Bundler** | `esnext`/`preserve` | `bundler` | `["import"]` (no `node`) | Allowed |

The POC detects which profile applies by reading `tsconfig.json` `compilerOptions.module`/`moduleResolution` and `package.json` `type` field. Mixed-format monorepos (some packages CJS, some ESM) need per-package profile selection — designed in POC, functional in MVP.

### POC behavior

The POC auto-detects a reasonable default profile and displays it. The profile is read-only in the POC (user cannot change it). The profile badge shows what was detected **and what was not modeled**. MVP adds user-adjustable profiles via canonical presets and toggleable dimensions.

### MVP behavior

MVP provides **canonical presets** (the three profiles above, plus a Rust workspace default) and **toggleable dimensions** (dependency kinds, Cargo features, Cargo targets, TS condition sets). This covers 90%+ of real-world needs without the complexity of a full free-form profile editor. Full free-form editing is deferred to Vision.

### Profile display

The active profile is always visible in the UI — either as a persistent badge or an expandable panel. Users must never wonder "what analysis context produced this graph?" The profile display includes:
- What was analyzed (workspace scope, resolution mode, features)
- What was NOT analyzed (unsupported constructs, with explanations)
- Confidence summary (percentage of imports resolved, count of unsupported constructs detected)
- Link to the full compatibility report

---

## 9. Graph Health & Trust

Trust is the product's primary differentiator. Graph health is not a debug feature — it is a first-class UX concern that belongs in the POC.

### Health indicators (POC)

| Indicator | What it shows | UX treatment |
|-----------|--------------|--------------|
| **Compatibility report** | Supported/Partial/Unsupported per language and construct | Shown during/after first scan; accessible from profile panel |
| **Active profile** | Workspace root, detected packages, resolution mode, features, condition sets | Persistent badge, expandable to full detail |
| **Resolution completeness** | X of Y imports resolved successfully | Header metric |
| **Unresolved imports** | List of imports that could not be resolved to a target | Warning badge on affected nodes; click to see list |
| **Parse failures** | Files that tree-sitter could not parse | Warning indicator on node; file still appears as structural node |
| **Edge confidence** | Each edge labeled as syntactic / resolver-aware / structural | Hover/click on edge to see evidence |
| **Edge category** | Each edge labeled with semantic category (value / type-only / dev / build / normal / manual) | Hover/click on edge; filterable in downstream impact |
| **Unsupported constructs** | Language constructs detected but not modeled (cfg gates, build.rs, proc macros, dynamic imports, framework conventions) | Prominent badge per construct type; click for explanation and list of affected locations |

### Edge evidence model

Every edge carries:
- **Kind:** imports, re_exports, contains, depends_on, manual
- **Category:** value | type_only | dev | build | test | peer | normal — the semantic role of this relationship. This is orthogonal to how the edge was discovered.
  - `value` — a runtime import that will be present in compiled output
  - `type_only` — `import type` / `export type` in TypeScript; guaranteed elided at compile time
  - `dev` — from `devDependencies` (JS/TS) or `[dev-dependencies]` (Rust)
  - `build` — from `[build-dependencies]` (Rust) or build tooling imports
  - `test` — edge exists only in test code
  - `peer` — from `peerDependencies` (JS/TS)
  - `normal` — standard runtime dependency (default)
  - `manual` — added via `.codeatlas.yaml` overlay
- **Confidence class:** `structural` (directory hierarchy only) < `syntactic` (parsed import statement) < `resolver-aware` (import resolved to target via path/workspace resolution) < `semantic` (type-aware, post-MVP) < `runtime` (observed, Vision)
- **Source location:** file, line range where the relationship was detected
- **Resolution method:** which resolver produced this edge (tree-sitter path, oxc_resolver, cargo metadata, manual config, etc.)
- **Overlay status:** whether this edge is suppressed in any config overlay (and the suppression reason)

### Why edge categories matter

Without edge categories, "static downstream impact" lights up test files, type-only consumers, and build tooling alongside production runtime dependents. That noise undermines trust. With categories:
- **"What breaks if I change this file?"** → filter to value + normal edges only
- **"What tests cover this area?"** → filter to test edges
- **"What's the full dependency surface?"** → show all categories

Edge categories are captured from scan data — tree-sitter distinguishes `import type` from `import` in TypeScript; `cargo metadata` provides `dep_kinds` with `normal`/`dev`/`build` classification. The data is already available; it just needs to be preserved in the edge model.

### Unsupported construct model

Every unsupported construct detected is tracked:
- **Construct type:** e.g., `cfg_gate`, `build_script`, `proc_macro`, `dynamic_import`, `framework_convention`, `exports_condition`
- **Location:** file + line range where it was detected
- **Impact:** what the graph might be missing because of this construct
- **How to address:** either "switch to a profile that includes this" (for features/targets) or "this will be supported in a future version" or "add a manual dependency in .codeatlas.yaml"

### MVP additions

- Full unresolved import list with clickable navigation to source
- Ambiguous resolution list (imports with multiple possible targets under different conditions)
- Generated/ignored file list
- Fallback logic explanations (when tree-sitter fell back, why)
- Unsupported construct inventory (full list, grouped by type, with links to affected nodes)
- Profile comparison (toggle profiles, see what changes — e.g., "enabling feature X adds 12 nodes and 34 edges")

---

## 10. Identity Scheme

The identity scheme is the most foundational design decision in the product. It affects watch mode, viewport preservation, diff overlays, graph snapshots, saved views, bookmarks, selection sync, agent references, and shareability. Getting it wrong creates silent correctness bugs across every downstream feature.

### Two-key design

| Concept | Purpose | Format | Stable across |
|---------|---------|--------|---------------|
| **Materialized key** | Current location in this repo/profile/snapshot | `{workspace_root}:{language}:{entity_kind}:{relative_path}` | Same scan |
| **Lineage key** | Logical identity that survives rename/move | UUID assigned at first observation, linked across renames via hybrid detection | Rescans, renames, branch comparisons |

### Rename/move policy

- **Same path, same materialized key.** A file at the same relative path always gets the same materialized key.
- **Rename detection (hybrid approach):** Primary signal is git rename detection (`-M`). Supplemented by broader rescans triggered by branch switches and manifest/config changes (these events can cause many paths to change simultaneously, where fine-grained rename detection is unreliable). Content and AST fingerprints are a designed extension point, activated in Platform for more robust cross-refactor lineage.
- **What "same node" means for saved views:** A bookmark targets the lineage key. If the file was renamed, the bookmark follows it. If the file was deleted, the bookmark is marked as "target removed."
- **What "same node" means for diff:** Change overlay uses materialized keys (current location). Architecture diff (Platform) uses lineage keys (logical identity).

### Branch switch and config change behavior

When the tool detects a branch switch (HEAD change) or a graph-shaping file change (manifests, tsconfigs, lockfiles, `.codeatlas.yaml`):
- Incremental rename detection is not attempted for these events — too many files may have changed
- A broader rescan is triggered instead
- The compatibility report is refreshed
- Saved views attempt lineage-key resolution after the rescan completes

### POC scope

- Materialized keys are generated and used for all node addressing.
- Lineage key design is documented and the data model supports it, but lineage tracking (rename detection, UUID persistence) activates in MVP when SQLite lands.

---

## 11. Repo-Local Configuration

Static analysis cannot fully describe the architecture of real-world repositories. Some dependencies exist by convention, not by import. Some paths should be ignored. Some files are entrypoints that the scanner cannot infer. Real repos need **declared truth** alongside discovered truth.

### The overlay model

The discovered graph — what the scanner actually found in source code and manifests — is **immutable**. It represents observed reality. Repo-local configuration creates **overlays** on top of the discovered graph:

- **Manual edges** (`add`): declare dependencies that exist by convention, runtime behavior, or other mechanisms the scanner cannot observe. These edges carry a `manual` evidence class badge.
- **View-level suppressions** (`suppress`): mark discovered edges as "expected but not architecturally interesting" or "known dead code pending removal." **Suppressed edges remain in the discovered graph.** They are hidden in the default view but visible when the user toggles "show suppressed edges." They appear dimmed/dashed with the suppression reason.
- **Metadata annotations** (`packages`): tags, layers, owners, and other metadata attached to packages/nodes.
- **Framework hints** (`frameworks`): directives for built-in detector modules.
- **Declarations** (`declarations`): user-provided annotations about constructs the scanner cannot analyze.

### Why this model matters

If configuration can silently delete observed edges, the provenance model becomes ambiguous. Exports, snapshots, rules, and agent queries cannot distinguish "this edge doesn't exist" from "someone hid this edge." The overlay model preserves the integrity of the discovered graph while giving users the controls they need.

### Why this matters before MVP

Without repo-local config:
- Orphan/unreferenced detection has no concept of "entrypoint" (high false positives)
- Framework-convention dependencies are invisible (route files, generated clients, convention-based plugins)
- Generated/vendor paths pollute the graph
- Rules and layer enforcement have no metadata to operate on
- Every downstream feature that depends on "what is this node's role?" must guess

### Configuration file: `.codeatlas.yaml`

Located at workspace root. Versioned with the repo. Schema designed in POC, progressively functional.

```yaml
# .codeatlas.yaml
version: 1

# Paths to ignore (glob patterns)
ignore:
  - "dist/**"
  - "node_modules/**"
  - "target/**"
  - "**/*.generated.ts"
  - "vendor/**"

# Explicit entrypoints and public API roots
entrypoints:
  - path: "apps/web/src/main.tsx"
    kind: app
  - path: "packages/sdk/src/index.ts"
    kind: public-api
  - path: "crates/cli/src/main.rs"
    kind: binary

# Dependency overlays
dependencies:
  # Manual edges: declare dependencies the scanner cannot observe
  add:
    - from: "apps/web"
      to: "packages/config"
      reason: "Runtime config loaded via environment, not imported"

  # View-level suppressions: hide edges in default view without deleting them
  # The discovered edge remains in the graph; it is hidden with an annotation
  suppress:
    - from: "packages/utils"
      to: "packages/legacy"
      reason: "Import is dead code, pending removal in Q2"

# Package metadata
packages:
  "packages/sdk":
    tags: [public, stable]
    layer: api
    owner: platform-team
  "apps/web":
    tags: [internal]
    layer: application
    owner: web-team

# Framework detector hints
frameworks:
  - type: "next-pages"
    root: "apps/web"
  - type: "generated-client"
    output: "packages/api-client/src/generated"
    source: "packages/api-client/openapi.yaml"

# Declarations about unsupported constructs
declarations:
  - type: "convention-based-routing"
    path: "apps/web/src/pages/**"
    note: "Next.js file-based routing; scanner cannot infer route dependencies"
```

### POC functionality

- Schema defined and documented
- Parser implemented (validate `.codeatlas.yaml` on scan)
- `ignore` paths respected during scan
- `entrypoints` parsed and stored (used for display; functional for orphan detection in Platform)
- **Overlay model implemented in data model:** discovered graph layer is distinct from config overlay layer. `add` creates manual edges in the overlay. `suppress` marks discovered edges as suppressed. The data model enforces immutability of the discovered layer.
- Other sections parsed but surfaced as "recognized, not yet functional"

### MVP functionality

- All sections functional
- Manual dependency edges appear in graph with a `manual` evidence class badge
- Suppressed edges hidden in default view; visible with "show suppressed" toggle (dimmed/dashed with reason)
- Package tags/layers/owners displayed in node detail panel
- Framework detector hints fed to built-in detector modules
- Declarations surfaced alongside unsupported construct badges

---

## 12. Diff Semantics

Change intelligence is a high-value feature, but the PRD must be precise about what "diff" means. There are two fundamentally different features, and the product ships them at different times.

### Feature 1: Change overlay against base ref (MVP)

**What it is:** Map changed files onto the current graph. This is an **affected set visualization**, not an architecture diff.

**How it works:**
1. User selects a base ref (default: merge-base of current branch and `main`/`master`)
2. Code Atlas computes `git diff <base-ref>...HEAD` to determine changed files
3. Changed files are classified into two groups:
   - **Code changes:** source files, test files, assets — mapped to current graph nodes normally
   - **Graph-shaping changes:** manifests (`Cargo.toml`, `package.json`), lockfiles, tsconfigs, `.codeatlas.yaml`, workspace configs (`pnpm-workspace.yaml`) — these can alter workspace membership, dependency resolution, and profile semantics
4. Code changes are overlaid on the current graph: modified nodes, new file nodes (if in scope), structurally affected neighbors
5. **Graph-shaping changes trigger a distinct alert** rather than being shown as ordinary changed files. The alert explains what *might* have changed (workspace membership, dependency graph, resolution config) and suggests rescanning.
6. **What this does NOT show:** removed nodes, removed edges, added edges between existing nodes, or any structural change that only exists in the base ref's graph. It shows the *current* graph with change annotations.

**Why graph-shaping change handling matters:**
If `Cargo.toml` adds a new dependency, the change overlay should not simply highlight `Cargo.toml` as "modified" alongside source files. That change may have added new edges to the graph, changed the workspace structure, or altered resolution behavior. Treating it as a normal file change is misleading. Treating it as a distinct class is honest.

**Graph-shaping files (classified by the scanner):**
- `Cargo.toml`, `Cargo.lock`
- `package.json`, `pnpm-lock.yaml`, `yarn.lock`, `package-lock.json`
- `tsconfig.json`, `tsconfig.*.json`
- `pnpm-workspace.yaml`
- `.codeatlas.yaml`
- `.gitignore` (affects what the scanner sees)

**Why this is still very useful:**
- Answers "what files changed and what do they connect to?"
- Supports code review: "show me the blast zone of this branch"
- Feeds static downstream impact: "what depends on the changed files?"
- Graph-shaping change alerts prevent false confidence
- Fast — no dual scan required

### Feature 2: Architecture diff (Platform)

**What it is:** Compare two full graph states. Show added/removed/modified nodes and edges.

**How it works:**
1. Both graph states must be available as persisted snapshots (SQLite, from MVP)
2. Lineage keys are used to match nodes across snapshots (survives renames)
3. Diff shows: nodes present in A but not B (removed), nodes present in B but not A (added), nodes in both but with different edges (modified), edges present in one but not the other
4. This is a real structural comparison, not file-change mapping

**Why it's deferred:**
- Requires snapshot persistence (MVP)
- Requires lineage-based identity (MVP)
- Requires scanning the base ref's source (either from checkout or cached snapshot)
- Significantly more complex than change overlay

### Diff modes (MVP)

| Mode | What's compared | When it's used |
|------|----------------|----------------|
| **Committed** | `base-ref...HEAD` | Default: shows what this branch has changed |
| **Working tree** | `HEAD` vs working tree | Toggled via UI switch: shows uncommitted changes |

### Explicit decisions

- **Base ref is user-visible and changeable.** The UI always shows what ref is being compared against.
- **Rename detection:** Git's default rename detection (`-M`) is used. Renames appear as a remove + add with a "renamed" badge in the change overlay.
- **Generated files:** Respect `.codeatlas.yaml` ignore paths and the project's `.gitignore`. Generated files that are tracked by git are included only if they match the active non-code file filter.
- **Submodules:** Shown as single nodes with changed/unchanged status. Submodule internals are not analyzed.
- **Label honesty:** The feature is labeled "Change overlay against {base-ref}" in the UI, NOT "Architecture diff" or "PR diff."
- **`.gitignore` respected in overlay calculations.** Files matched by `.gitignore` are excluded from both baseline scanning and overlay change calculations.

---

## 13. Dynamic Graph Adaptation

The app adapts its default presentation to the size of the graph.

### Default Visible Depth by Graph Size

| Graph Size | Default State |
|------------|---------------|
| **Small (<120 visible nodes)** | Top-level packages expanded, modules visible |
| **Medium (120-250 visible nodes)** | Collapsed at package level. User expands on demand. |
| **Large (>250 visible nodes)** | Collapsed. Lower-priority labels hidden. File-level detail deferred until user focuses a region. |

### Rendering strategy

The scanner eagerly processes all source files (tree-sitter is fast enough for repos up to the target size). However, the **renderer** only creates React Flow DOM nodes for packages that are expanded. Collapsed packages are single compound nodes with summary labels. This keeps the visible node count manageable regardless of total graph size.

The API is designed so that lazy scanning (deferring file-level analysis of unexpanded packages) can be added as a future optimization for repos beyond the MVP target size. The current architecture does not preclude this — it is an additive capability.

### Edge Density Control

- Dense cross-package edges visible at overview level
- Dense file-level edges only appear when user drills into a limited region
- When a package is collapsed, edges between its children and external nodes are bundled into a single edge between the package and the external target

### Label Visibility Rules

- Package/crate labels: always visible
- Module/folder labels: visible when zoomed in or expanded inside a focused area
- File labels: may abbreviate or hide at low zoom levels

### Non-code file policy

Code files (source, manifests, configs) are the default node set. Assets, build output, generated files, and other non-code files are **hidden by default** and opt-in via a filter toggle. `.codeatlas.yaml` `ignore` paths are always excluded. Rationale: the default should answer "how does the code fit together?" not "what files exist?"

### Thresholds

Node count thresholds (120, 250) are starting points. Make them configurable internally. Test against real repos.

---

## 14. Detector Architecture

Language and framework analysis must be organized as pluggable **detector modules** from the POC, even though a public plugin API is deferred to Platform.

### Why this matters early

- Imports alone are not the whole architecture. Framework conventions (file-based routing, generated clients, test↔source relationships, schema-driven code) create real dependencies that import scanning misses.
- Hardcoding "imports + folders + manifests" into the graph model makes later framework support invasive.
- The POC ships two detectors. The MVP ships 3-4. Platform opens the API. If the seam isn't there from the start, each new detector is a refactor.

### Detector trait (internal, Rust)

```rust
/// A detector module that can discover nodes and edges in a repository.
pub trait Detector: Send + Sync {
    /// Human-readable name (e.g., "rust-cargo", "typescript-imports")
    fn name(&self) -> &str;

    /// What language/framework this detector handles
    fn language(&self) -> Language;

    /// Whether this detector applies to the given workspace
    fn applies_to(&self, workspace: &WorkspaceInfo) -> bool;

    /// Report what this detector can and cannot analyze (for compatibility report)
    fn compatibility(&self, workspace: &WorkspaceInfo) -> CompatibilityAssessment;

    /// Discover nodes and edges, streaming results
    fn detect(
        &self,
        workspace: &WorkspaceInfo,
        profile: &GraphProfile,
        config: &RepoConfig,
        sink: &dyn DetectorSink,
    ) -> Result<DetectorReport>;
}

/// Report of what the detector found AND what it couldn't analyze
pub struct DetectorReport {
    pub nodes_discovered: usize,
    pub edges_discovered: usize,
    pub unsupported_constructs: Vec<UnsupportedConstruct>,
    pub parse_failures: Vec<ParseFailure>,
}

/// Detector's contribution to the compatibility report
pub struct CompatibilityAssessment {
    pub language: Language,
    pub status: SupportStatus, // Supported, Partial, Unsupported
    pub details: Vec<CompatibilityDetail>,
}
```

### POC detectors

1. **`rust-cargo`** — cargo metadata + tree-sitter mod/use/pub use + crate dependency graph. Captures dependency kind (normal/dev/build) as edge category.
2. **`typescript-imports`** — workspace package discovery + tsconfig + tree-sitter import/export resolution. Captures `import type` vs `import` as edge category (type_only vs value).

### MVP detectors (built-in)

3. **`test-source`** — map test files to their corresponding source files (by naming convention + import analysis)
4. **`barrel-reexport`** — trace barrel file (index.ts) re-export chains to actual source

### Platform detectors (public API)

5+ — user-defined detectors via the public plugin API. Examples: Next.js page routes, generated GraphQL clients, OpenAPI-generated types.

---

## 15. Editor & Workspace Topology

### Scope declaration

**Code Atlas v1 (POC + MVP) supports local workspaces only.** The codebase must be on the local filesystem where the Tauri app is running. Remote workspaces (VS Code Remote, Codespaces, SSH, containers) are explicitly out of scope until the Platform phase.

### Why this matters

VS Code extensions can run in local, web, or remote extension hosts. A "thin companion extension" works straightforwardly for local workspaces but requires a workspace-side scanning component for remote workspaces. The analysis core separation (Principle 8) ensures this is an additive capability later, not a rewrite.

### Click-to-open specification (POC stretch / MVP)

| Mechanism | How | Fallback |
|-----------|-----|----------|
| CLI | `code -g file:line:col` | Try `code-insiders -g file:line:col` |
| URI scheme | `vscode://file/{absolute-path}:{line}:{col}` | N/A |

- CLI may not be on PATH. The app should check for `code` availability and suggest `Shell Command: Install 'code' command in PATH` if not found.
- This is **local-only**. Remote file reveal requires the remote scanner architecture from Platform.

### VS Code companion extension (MVP)

- **Scope:** Bridge only. Does not perform scanning or analysis.
- **Capabilities:** `onDidChangeActiveTextEditor` for file tracking, "Show in Code Atlas" context menu, selection sync.
- **Communication:** WebSocket bridge (tokio-tungstenite server in Tauri, client in extension).
- **Constraint:** Local workspaces only in MVP.

---

## 16. Analysis Core Architecture

The analysis engine (`codeatlas-core`) must be a standalone Rust library/binary from the POC.

### Why this is a POC requirement, not a future refactor

If the scanner is built as "logic inside the Tauri app," every future surface — CLI, MCP server, headless base-ref scans for architecture diff, editor bridges, remote workspaces — becomes a rewrite instead of a new client. The cost of separation at POC is low (clean crate boundary). The cost of separation after MVP is a major refactor.

### Architecture

```
┌─────────────────────────────────────────────────┐
│                 codeatlas-core                    │
│  (standalone Rust crate, no Tauri dependency)     │
│                                                   │
│  ┌─────────┐  ┌──────────┐  ┌─────────────────┐ │
│  │Workspace│  │ Detector │  │   Graph Model    │ │
│  │Discovery│  │ Registry │  │  (petgraph +     │ │
│  │         │  │          │  │   evidence +     │ │
│  │         │  │          │  │   identity +     │ │
│  │         │  │          │  │   overlay)       │ │
│  └─────────┘  └──────────┘  └─────────────────┘ │
│  ┌─────────┐  ┌──────────┐  ┌─────────────────┐ │
│  │ Profile │  │  Config  │  │     Query        │ │
│  │ Manager │  │  Parser  │  │    Engine        │ │
│  │         │  │ + Overlay │  │                 │ │
│  └─────────┘  └──────────┘  └─────────────────┘ │
│  ┌──────────────────────────────────────────────┐│
│  │    Compatibility │  Streaming Output         ││
│  │    Report Engine │  (Channel)                ││
│  └──────────────────────────────────────────────┘│
└──────────────────────┬──────────────────────────┘
                       │ stdio / IPC / direct call
          ┌────────────┼────────────┐
          │            │            │
   ┌──────┴──────┐ ┌──┴───┐ ┌─────┴──────┐
   │ Tauri Shell │ │ CLI  │ │MCP Adapter │
   │ (desktop)   │ │      │ │            │
   └─────────────┘ └──────┘ └────────────┘
```

### API pattern (following rust-analyzer's architecture invariant)

The core exposes two primary types:
- **`AnalysisHost`** — mutable handle. Accepts workspace changes (file edits, config updates, rescan requests). Holds the current graph state (both discovered layer and overlay layer).
- **`Analysis`** — immutable snapshot obtained from `AnalysisHost`. Safe for concurrent queries. All query methods live here.

**API design rules (from rust-analyzer's Architecture Invariant):**
- Uses **domain terminology** (nodes, edges, profiles, health, compatibility, overlays), not transport terminology (JSON, channels, commands)
- All arguments and return types are conceptually serializable POD types with public fields
- The API is designed for an ideal consumer — **NOT influenced by Tauri's IPC model, MCP's tool schema, or any specific transport**
- Syntax trees, hir types, and internal graph structures are absent from the public API
- The graph model exposes both the discovered layer and the overlay layer — consumers can query either or the merged view

### IPC boundary

- **Direct call:** Tauri shell links `codeatlas-core` as a Rust dependency and calls it in-process. This is the POC path — zero overhead.
- **stdio:** The core can be compiled as a standalone binary that reads commands from stdin and writes results to stdout (JSON or MessagePack). This enables CLI, MCP, and headless use.
- **The key constraint:** `codeatlas-core` must have NO dependency on Tauri, WebView, or any UI framework. It is a pure analysis library.

### POC scope

- `codeatlas-core` exists as a separate Cargo crate within the workspace
- All scanning, graph building, profile management, compatibility reporting, overlay management, and query logic lives in this crate
- The Tauri app depends on it as a library (direct call)
- A minimal test harness can exercise `codeatlas-core` without the Tauri shell
- The stdio/CLI surface is stretch — the crate boundary is the requirement

---

## 17. Core User Experience

### Landing State

After scan, the first meaningful screen shows:
- **Compatibility report summary** (Supported/Partial/Unsupported — expandable for detail)
- **Workspace/package topology** as primary visible nodes (not files)
- High-level dependency edges between packages
- Active graph profile badge (including unsupported construct summary)
- Graph health summary (resolved/unresolved/failures/unsupported)
- MiniMap
- Graph controls (zoom, fit-to-view)
- Detail panel placeholder

The default view answers "can I trust this graph?" and "what are the main pieces of this repo and how do they depend on each other?" in seconds.

### Navigation Flow

**compatibility report** (trust first) → **workspace overview** (packages) → **selected package** (expanded, showing modules/files) → **file-level inspection** (detail panel with imports/exports/provenance/categories)

The canvas is continuous. The user must not feel like they are switching tools or modes.

### Error Handling

- Clearly distinguish unsupported repo structures, parse failures, and permission issues
- Partial graph generation renders what is available — with health indicators showing the gaps
- If tree-sitter fails on some files, those files still appear as structural nodes without edges, with a parse failure badge
- Informative error messages, not stack traces

### Detail Panel Design

Right-side collapsible panel (~300px):
- **Tabs:** Overview (name, path, type, stats) | Dependencies (in/out edge lists with evidence class and category, clickable, filterable by category) | Declared Exports (public exports with signatures from tree-sitter) | Health (unsupported constructs affecting this node, unresolved imports from this node)
- Graph canvas 60-70%, detail panel 30-40%
- Panel collapses when nothing selected, slides in on node click
- Breadcrumb navigation: Workspace > Package > Module > File

---

## 18. Functional Requirements

| ID | Requirement | Priority | Phase | Story |
|----|------------|----------|-------|-------|
| F1 | Select project directory via native dialog | P0 | POC | US-1 |
| F2 | Workspace discovery (Cargo workspaces, JS/TS workspaces) | P0 | POC | US-1 |
| F3 | Compatibility report (Supported/Partial/Unsupported per language and construct) | P0 | POC | US-2 |
| F4 | Graph profile detection and display (including unsupported constructs) | P0 | POC | US-2 |
| F5 | Analysis core as separate crate (`codeatlas-core`) | P0 | POC | — |
| F6 | Internal detector trait + registry (with compatibility assessment method) | P0 | POC | — |
| F7 | Rust detector (cargo metadata + tree-sitter mod/use) with edge categories (normal/dev/build) | P0 | POC | US-1 |
| F8 | TypeScript detector (workspace + tsconfig + tree-sitter imports) with edge categories (value/type-only) | P0 | POC | US-1 |
| F9 | Build petgraph StableGraph with edge evidence, categories, and identity keys | P0 | POC | US-1 |
| F10 | Discovered graph / config overlay separation (immutable discovered layer) | P0 | POC | — |
| F11 | Progressive scan with streaming via Channel<T> | P0 | POC | US-1 |
| F12 | Render graph as interactive React Flow canvas | P0 | POC | US-1 |
| F13 | Workspace/package lens as default view | P0 | POC | US-1 |
| F14 | Nodes nested: workspace > packages > modules > files | P0 | POC | US-1 |
| F15 | Edges show import/re-export relationships with evidence class and category | P0 | POC | US-6 |
| F16 | Graph health indicators (resolved, unresolved, failures, unsupported) | P0 | POC | US-3 |
| F17 | Unsupported construct badges | P0 | POC | US-2 |
| F18 | Smooth zoom and pan at 60fps for <200 visible nodes | P0 | POC | US-4 |
| F19 | Expand/collapse on package and module nodes | P0 | POC | US-5 |
| F20 | ELK re-layout on expand/collapse | P0 | POC | US-5 |
| F21 | Default collapse state adapts to graph size | P0 | POC | US-5 |
| F22 | Click node opens detail panel with edge provenance and categories | P1 | POC | US-6 |
| F23 | Materialized node key scheme | P0 | POC | — |
| F24 | Lineage key design (documented, data model supports it) | P1 | POC | — |
| F25 | `.codeatlas.yaml` schema designed, parser implemented, ignore paths functional, overlay model in data model | P1 | POC | — |
| F26 | MiniMap showing viewport position | P1 | POC | US-4 |
| F27 | Cmd+K search with fuzzy match + navigate to node | P1 | POC | US-7 |
| F28 | Zoom-level detail: labels simplify when zoomed out | P1 | POC | US-4 |
| F29 | Edge bundling when packages are collapsed | P2 | POC | US-5 |
| F30 | Built-in demo graph fixture | P2 | POC | US-8 |
| F31 | Manual rescan button | P1 | POC | — |
| F32 | Non-code file filter (hidden by default, opt-in) | P2 | POC | — |
| F33 | File watching with incremental rescan (broader rescan on branch/config changes) | P0 | MVP | — |
| F34 | Change overlay against base ref with graph-shaping change classification | P0 | MVP | US-9 |
| F35 | Static downstream impact with edge-category filters | P0 | MVP | US-10 |
| F36 | Public surface lens (package exports / public API boundary view) | P1 | MVP | US-13 |
| F37 | Click-to-open in VS Code (local only) | P0 | MVP | US-11 |
| F38 | VS Code companion extension (thin bridge, local only) | P1 | MVP | US-11 |
| F39 | Semantic zoom (different representations by scale) | P1 | MVP | — |
| F40 | Slice-based navigation presets | P1 | MVP | — |
| F41 | User-adjustable graph profiles (canonical presets + toggleable dimensions) | P1 | MVP | — |
| F42 | Edge provenance inspector (full detail with category) | P1 | MVP | — |
| F43 | Graph health dashboard (full, including compatibility report history) | P1 | MVP | — |
| F44 | oxc_parser + oxc_resolver for TS/JS (full resolution spec) | P0 | MVP | — |
| F45 | cfg-aware Rust graph (features + targets from profile, with honest limits badged) | P1 | MVP | — |
| F46 | SQLite persistence (snapshots, stable identity, lineage) | P0 | MVP | — |
| F47 | `.codeatlas.yaml` fully functional (all sections, overlay model: add/suppress) | P1 | MVP | — |
| F48 | Named views and bookmarks | P1 | MVP | US-12 |
| F49 | Graph snapshots (persist full graph state) | P1 | MVP | — |
| F50 | Static export (SVG/PNG) | P1 | MVP | US-12 |
| F51 | Test↔source detector | P2 | MVP | — |
| F52 | Barrel/re-export chain detector | P2 | MVP | — |
| F53 | CLI / JSON API (primary automation interface) | P0 | Platform | — |
| F54 | Thin MCP adapter (focused subset of CLI capabilities) | P1 | Platform | — |
| F55 | Architecture diff (graph-to-graph via snapshots) | P1 | Platform | — |
| F56 | Architecture rules enforcement | P1 | Platform | — |
| F57 | Public plugin API (detector seam) | P2 | Platform | — |
| F58 | Interactive HTML export (with redaction controls) | P2 | Platform | — |
| F59 | FTS5 search indexing (cross-snapshot search) | P2 | Platform | — |

---

## 19. Non-Functional Requirements

| ID | Requirement | Target | Phase |
|----|------------|--------|-------|
| NF1 | First meaningful frame (package topology) ≤2,000 files | <2 seconds | POC |
| NF2 | Full scan + render (≤2,000 files) | <10 seconds | POC |
| NF3 | ELK layout computation (200 nodes) | <500ms in Web Worker | POC |
| NF4 | Interaction framerate (<200 visible nodes) | 60fps | POC |
| NF5 | Application binary size (macOS) | Monitor, no hard gate. Tauri naturally produces small binaries; track but do not sacrifice functionality for size. | POC |
| NF6 | Memory at 500-node graph | <200MB | POC |
| NF7 | Test coverage on non-UI Rust code | >80% | POC |
| NF8 | Test coverage on frontend pure logic | >80% | POC |
| NF9 | Network calls | Zero | POC |
| NF10 | UI thread never blocked by layout or parsing | Always responsive | POC |
| NF11 | Scan cancellation | User can cancel and keep partial results | POC |
| NF12 | `codeatlas-core` testable without Tauri shell | Required | POC |
| NF13 | Golden corpus: correct graph on reference repos | See Section 22 | POC |
| NF14 | Compatibility report displayed during/after first scan | Before user invests time in graph | POC |
| NF15 | Graph update latency after file save (watch mode) | <2 seconds | MVP |
| NF16 | VS Code round-trip (map → editor → map, local) | <1 second | MVP |

---

## 20. Technical Direction

Full decision records in `docs/decisions.md`. Summary below.

| Concern | Choice | Why |
|---------|--------|-----|
| Desktop shell | Tauri v2 | Rust-native core, capability-based security, small binary, local-first by design |
| Language | Rust backend + TypeScript/React 19 frontend | Rust for performance-critical parsing/graph; React for rich interactive UI |
| **Analysis core** | **Separate `codeatlas-core` crate, no Tauri dependency** | **Enables CLI, MCP, headless scans, editor bridges, future remote support without rewrite** |
| **Graph data model** | **Immutable discovered layer + config overlay layer** | **Preserves provenance integrity; config supplements but never mutates discoveries** |
| Graph rendering | React Flow v12 (`@xyflow/react`) | Compound nested nodes + React JSX + MIT license |
| Layout engine | ELK.js in Web Worker | Only open-source engine handling compound graphs with port-based edge routing |
| Graph data structure | petgraph StableGraph | Stable indices survive removals. Built-in SCC, topological sort, shortest path. |
| **Edge model** | **Evidence class + semantic category + overlay status** | **Categories enable meaningful impact filtering; overlay status preserves provenance** |
| Workspace discovery (Rust) | `cargo_metadata` crate | Machine-readable package/workspace/dependency/feature/target information |
| Workspace discovery (JS/TS) | Detect pnpm-workspace.yaml / package.json workspaces | Standard workspace config files for all major package managers |
| Code parsing (POC) | tree-sitter | Error-tolerant, incremental, multi-language. Distinguishes `import type` vs `import` in TypeScript grammar. |
| Code parsing (MVP, TS/JS) | oxc_parser + oxc_resolver | 3x faster parsing. Resolver handles full TS resolution spec (tsconfig paths, project references, package.json exports/imports, condition names, PnP). |
| **Detector architecture** | **Internal `Detector` trait from POC with compatibility assessment** | **Framework support and new languages are additive, not invasive; compatibility report is detector-driven** |
| **Repo-local config** | **`.codeatlas.yaml` with overlay model (add/suppress, not add/remove)** | **Escape hatch for static analysis limits; preserves discovered graph immutability** |
| Import resolution (POC, Rust) | Cargo.toml + mod hierarchy + use paths | Rust module resolution is deterministic from manifest + source structure |
| Import resolution (POC, TS) | Path-based with tsconfig discovery | Sufficient for POC; oxc_resolver at MVP for full spec |
| Streaming IPC | Tauri Channel<T> | Ordered streaming from Rust to frontend for progressive scan |
| Request/response IPC | Tauri invoke (JSON) | For payloads <100KB |
| Type-safe IPC | tauri-specta v2 | Generates TS bindings from Rust command signatures. Fallback: manual types. |
| Persistence (MVP) | SQLite (no FTS5 in MVP) | Local-first storage for snapshots, stable identity, lineage. FTS5 deferred to Platform for cross-snapshot search. |
| State management | Deferred to /plan | zustand, useReducer, or Jotai — all reasonable |
| Frontend styling | Tailwind CSS v4 | Utility-first, works with React Flow's node components |
| Package manager | pnpm (frontend) + Cargo (Rust) | pnpm for frontend (fast, strict). Cargo for Rust ecosystem. |
| Testing | Vitest (frontend) + cargo test + proptest + criterion | 3-layer: Rust unit/integration/benchmarks, frontend jsdom+mockIPC, E2E deferred |
| Bundler | Vite | Standard for React + Tauri projects. Web Worker support for ELK. |
| **Automation interface** | **CLI/JSON primary, MCP as thin adapter** | **CLI is richer, stable, and scriptable; MCP adapts a focused subset for agent interoperability** |
| **Compatibility report** | **First-class POC feature, detector-driven** | **Trust requires upfront honesty about what the tool can analyze for a specific repo** |

### Data Pipeline

```
User selects directory
  → codeatlas-core: workspace discovery (cargo metadata / workspace config detection)
  → codeatlas-core: parse .codeatlas.yaml (if exists)
  → codeatlas-core: determine graph profile (workspace scope, features, resolution mode)
  → codeatlas-core: run detector compatibility assessments → produce compatibility report
  → codeatlas-core: stream compatibility report to frontend (renders immediately)
  → codeatlas-core: run detector registry (each detector scans its language/framework)
  → codeatlas-core: merge detector results into petgraph StableGraph with edge evidence + categories
  → codeatlas-core: apply config overlays (manual edges added, suppressions marked)
  → codeatlas-core: compute graph health + unsupported construct inventory
  → codeatlas-core: stream graph phases via output channel:
      Phase 1: workspace/package nodes + inter-package edges (renders immediately)
      Phase 2: module/folder nodes
      Phase 3: file nodes + file-level import edges
  → Tauri shell: forward stream to React frontend
  → React: render each phase as React Flow nodes (DOM measures sizes)
  → ELK.js (Web Worker): compute positions + parent container sizes
  → React Flow: apply positions, fitView()
  → React: display compatibility report summary + graph health indicators + unsupported construct badges
```

### Critical Architecture Constraints

1. **`codeatlas-core` must not depend on Tauri.** It is a standalone analysis library.
2. **Layout cannot happen in Rust.** React Flow needs DOM-measured node sizes before ELK can compute positions.
3. **Identity scheme must be designed before first scan implementation.** Every downstream feature depends on stable node identity.
4. **Streaming is not optional.** Giant JSON payloads via `invoke()` produce poor UX. Use `Channel<T>` to stream the graph in phases.
5. **Detector registry is not optional.** Language analysis must go through the detector trait from the first implementation.
6. **Discovered graph immutability is not optional.** The data model must enforce separation between the discovered layer and the config overlay layer from the first implementation.

---

## 21. Competitive Positioning

### The market in 2026

Code architecture visualization is no longer an empty category. Multiple tools exist:

- **CodeViz** (YC S24) — VS Code extension (~80K installs, v1.6.9, Dec 2025) and web platform. LLM-powered analysis that sends code to Anthropic/GCP/AWS for processing. Offers interactive diagrams, natural language search, C4/UML models, export to Mermaid/draw.io/PNG. Pricing: Free (5 diagrams/month) → Pro ($19/mo) → Teams ($50/seat/mo) → Enterprise (custom, on-prem). **Privacy is the #1 concern** — HN launch thread users called sending code to cloud LLMs a "deal-breaker" for enterprise. On-prem listed but unverified in production.
- **GitKraken Codemaps** — born from the CodeSee acquisition (May 2024). **Still in early access as of March 2026 — not GA.** The features page says "currently in development." The CodeSee technology has been repurposed primarily for GitKraken Automations (workflow automation), not visualization. Codemaps is effectively vaporware until proven otherwise.
- **SciTools Understand** — legacy leader in deep static analysis. Cross-language, comparison graphs, call hierarchy, control flow. 20+ years of engineering. Not a modern UX.
- **Nx Graph** — deep monorepo integration within its ecosystem. Project graph visualization, affected detection, project-graph plugins for extensibility. Ecosystem-locked. Nx now defaults its MCP server to minimal mode — broad workspace analysis tools proved less efficient than focused skills.
- **Sourcegraph** — cross-repo code navigation via SCIP. Not an architecture visualization tool, but SCIP indexes overlap.
- **dependency-cruiser, Madge, Swark** — static output tools. Useful for CI checks, not for interactive exploration.

### Where Code Atlas wins

The wedge is **evidence-backed, profile-aware, repo-configurable local architecture graphs that are honest about coverage, provide upfront support contracts, and offer meaningful edge taxonomy for impact analysis.** Specifically:

1. **Upfront support contract.** No existing tool tells you what it can and cannot handle for your specific repo before you invest time. Code Atlas runs a compatibility report first. This is the strongest trust signal in the category.
2. **Build-context-aware graphs.** No existing tool exposes the analysis profile or lets users understand which workspace scope, feature set, resolution mode, and condition sets produced the graph. This is the most defensible technical advantage.
3. **Honesty about limits.** No tool reports unsupported constructs alongside the graph. Most tools show all edges at equal confidence. Code Atlas badges what it couldn't analyze and provides a configuration escape hatch.
4. **Edge provenance and taxonomy.** No tool exposes evidence chains for relationships. No tool distinguishes value imports from type-only imports, runtime deps from dev deps, or parsed edges from manual overrides. Every Code Atlas edge carries its source, evidence class, semantic category, and resolution method.
5. **Immutable discovered graph with overlay model.** Config supplements but never silently mutates discovered relationships. This means exports, snapshots, and agent queries reflect reality plus clearly-labeled modifications.
6. **Privacy.** Zero network calls. Code never leaves the machine during analysis. Egress is explicit and user-controlled. Enterprise unlock that cloud-first tools cannot match without an on-prem deployment.
7. **Hierarchical zoom.** No tool provides workspace → package → module → file zoom in a single continuous canvas with adaptive defaults.
8. **Public surface lens.** No tool provides a filtered view of what each package exposes to consumers vs. its internal structure.
9. **Agent readiness.** The same graph model serves humans (canvas) and coding agents (CLI/JSON + MCP adapter). Architecture-aware AI context, not raw file dumps. Deliberately minimal MCP surface manages token usage.
10. **Change intelligence with honest semantics.** Change overlays with explicit base-ref semantics, graph-shaping change alerts, static downstream impact with edge-category filters, affected slices — all labeled for what they are, not oversold.

### Where competitors are stronger

1. **SciTools Understand** — deeper cross-language analysis, 20+ years of static analysis engineering. Comparison graphs. Call hierarchy.
2. **Nx Graph** — deeper monorepo integration within its ecosystem, import-level granularity, PR-level diffs via Nx Cloud, plugin-extensible project graph.
3. **CodeViz** — AI-generated architecture summaries, shared workspaces, version history, embeddable diagrams, API access. ~80K VS Code installs. If privacy and trust-provenance are not concerns, it provides immediate value. Its on-prem offering, if real, could partially close the privacy gap for enterprises.
4. **Sourcegraph** — most mature cross-repo code navigation via SCIP.

### Competitive wedge (summary)

The durable wedge is: **trustworthy, evidence-backed, profile-aware, repo-configurable, local architecture graphs that tell you what they can handle upfront, distinguish edge categories for meaningful impact analysis, and are honest about what they can and cannot analyze.** This is narrower and stronger than "visual code maps don't exist" (they do) or "privacy is enough" (it helps but isn't sufficient alone).

---

## 22. Acceptance: Golden Corpus

Coverage targets (>80%) are engineering hygiene. The product's moat is trust, and trust is measured against real repositories, not abstract percentages.

### Golden corpus requirements

The golden corpus is a set of reference repositories that the scanner must produce correct, verifiable graphs for. Each supported archetype has at least one reference repo.

**POC corpus:**

| Repo archetype | Example | What we verify |
|----------------|---------|---------------|
| **This project** (Tauri monorepo: Cargo workspace + TS) | `tauri-poc-zoom-thing` | All workspace packages discovered, inter-package deps correct, self-referential correctness, compatibility report accurate |
| **Rust workspace** | A public Cargo workspace with multiple crates, features, targets | Workspace members correct, crate dependency graph matches `cargo metadata`, mod/use hierarchy correct, edge categories (normal/dev/build) correct, unsupported constructs (cfg gates, build.rs, proc-macros) detected and badged, compatibility report accurate |
| **TS/JS monorepo** | A public pnpm/npm/yarn workspace with multiple packages | Workspace packages discovered, inter-package deps from manifests correct, import resolution correct for basic tsconfig paths, edge categories (value/type-only) correct, unsupported constructs (exports conditions, dynamic imports) detected and badged, compatibility report accurate |

**MVP corpus expansion:**

| Repo archetype | What we verify |
|----------------|---------------|
| TS monorepo with `exports`/`imports` conditions | oxc_resolver correctly handles condition-based resolution |
| TS monorepo with project references | Inter-project reference edges correct; public surface lens identifies declaration boundaries |
| Rust workspace with non-default features | Feature-gated code paths correctly included/excluded based on profile |
| Repo with `.codeatlas.yaml` | Config overlays correctly applied: manual edges appear with `manual` badge, suppressed edges hidden in default view but visible with toggle |
| Repo with graph-shaping changes on a branch | Change overlay correctly classifies manifest/lockfile/tsconfig changes as graph-shaping |

### What "correct" means

For each golden corpus repo:
1. All workspace packages/crates are discovered (no missing, no spurious)
2. **Compatibility report** accurately reflects what is supported, partial, and unsupported
3. Inter-package dependency edges match manifest declarations
4. File-level import edges resolve to correct targets (verified against known-good resolution for a sample of imports)
5. **Edge categories** are correct: type-only imports classified as type_only, dev deps as dev, etc.
6. Unsupported constructs are detected (no false silence — if the repo uses cfg gates, the scanner must badge them)
7. Unresolved imports are accurately reported (no false completeness claims)
8. Graph health metrics are accurate
9. **Config overlays** applied correctly: manual edges present, suppressed edges hidden in default view

### What "correct" does NOT mean

- 100% of all imports resolved (some are genuinely unresolvable without full semantic analysis)
- Framework-convention dependencies discovered automatically (that's what `.codeatlas.yaml` is for)
- proc-macro expansion (explicitly out of scope, badged as unsupported)

---

## 23. Milestones

### Phase 1 (POC) Milestones

| # | Milestone | What It Proves |
|---|-----------|---------------|
| M1 | **Scaffold + Architecture** | Tauri v2 + React + Vite builds and launches. `codeatlas-core` exists as a separate crate with no Tauri dependency. Detector trait defined (with compatibility assessment method). Identity scheme designed. `.codeatlas.yaml` schema defined with overlay model. Discovered/overlay graph layer separation in data model. |
| M2 | **Workspace discovery + compatibility report** | `cargo metadata` integration works. JS/TS workspace detection works. Graph profile is computed and displayable. Compatibility report is generated from detector assessments and displayed. Unsupported construct detection framework works. |
| M3 | **Static graph rendering** | React Flow renders a hardcoded graph fixture with compound nodes + ELK layout. Expand/collapse works. Graph adaptation defaults work. |
| M4 | **Rust detector + streaming** | Rust detector parses a real directory via tree-sitter. petgraph builds the graph with edge evidence and categories (normal/dev/build). Data streams via Channel<T> with progressive rendering. Cancel works. Unsupported Rust constructs (cfg, build.rs, proc-macro) detected and badged. |
| M5 | **TypeScript detector** | TypeScript detector parses a real TS workspace. Import resolution with tsconfig paths. Edge categories captured (value vs type-only). Unsupported TS constructs (exports conditions, dynamic imports) detected and badged. |
| M6 | **Graph health + provenance + config** | Health indicators display. Edge evidence and categories are inspectable. Unresolved imports are surfaced. Unsupported construct badges work. `.codeatlas.yaml` ignore paths and entrypoints functional. Overlay model (add/suppress) functional in data model. |
| M7 | **Interactive features** | Node detail panel with category-aware edge display, search, edge filtering, rescan. |
| M8 | **Golden corpus + polish** | All POC golden corpus repos produce correct graphs with accurate compatibility reports. Edge categories verified. Dark theme, performance tuning, sample graph fixture, all tests green. |

### Phase 2 (MVP) Milestones

| # | Milestone | What It Proves |
|---|-----------|---------------|
| M9 | **Persistence + stable identity + lineage** | SQLite stores graph snapshots (no FTS5). Materialized keys verified across rescans. Lineage tracking via git rename detection. Broader rescans on branch/config changes. |
| M10 | **oxc_resolver + cfg-aware Rust** | Full TS resolution spec (exports, imports, conditions, project references). Rust profile with feature/target selection. Compatibility report updated for expanded capabilities. Golden corpus expanded. |
| M11 | **File watching** | notify + incremental rescan. Graph updates on save. Viewport preserved. Branch switches trigger broader rescan. |
| M12 | **Change overlay + graph-shaping changes** | Change overlay against base ref. Changed-node highlighting. Base ref visible. Labeled as "change overlay." Graph-shaping changes (manifests, lockfiles, tsconfigs) classified separately with alerts. |
| M13 | **Static downstream impact + edge-category filters** | Select node, see transitive dependents with distance coloring. Filter by edge category (exclude type-only, dev). |
| M14 | **Public surface lens** | Package exports / public API boundary view. TypeScript: package.json exports/main. Rust: pub items from lib target. Cross-package edge filtering. |
| M15 | **VS Code bridge** | Click-to-open (local). Companion extension with file tracking. |
| M16 | **Saved views + export** | Named views, bookmarks, SVG/PNG export. |
| M17 | **Full config + health dashboard + profiles** | `.codeatlas.yaml` fully functional with overlay model. User can switch profiles via presets and toggleable dimensions. Full graph health dashboard with compatibility report history. |
| M18 | **Semantic zoom + slices + detectors** | Different representations by scale. Preset slice queries. Test↔source and barrel detectors. |

---

## 24. Risks

| Risk | Likelihood | Impact | Mitigation | Phase |
|------|-----------|--------|------------|-------|
| **cargo metadata adds startup latency** | Medium | Slower first render | Cache workspace metadata; only re-invoke on Cargo.toml changes | POC |
| **Graph profile + unsupported construct UX overwhelms users** | Medium | Adoption friction | Default profile is auto-detected and reasonable; unsupported constructs are badged but not blocking; progressive disclosure; compatibility report is a summary, not a wall of text | POC |
| **ELK layout too slow for 500+ nodes** | Medium | Blocks fluid UX | Web Worker isolates main thread; debounce rapid toggles; profile early | POC |
| **tree-sitter import resolution is incomplete** | High | Missing/wrong edges | Surface unresolved imports via graph health; unsupported construct badges explain gaps; compatibility report sets expectations upfront | POC |
| **Flat-to-hierarchical ELK transform edge cases** | High | Layout breaks | Comprehensive unit tests with proptest; test cross-hierarchy edge routing | POC |
| **React Flow performance with many compound nodes** | Medium | Blocks scale | Expand/collapse keeps visible count <200; only render expanded packages; memoize; profile early | POC |
| **Progressive rendering UX is jarring** | Medium | Bad first impression | Package topology is visually stable; file-level details stream in without relayout of package-level | POC |
| **tauri-specta integration friction** | Medium | Delays type safety | Fall back to manual TypeScript types | POC |
| **Rust is new to the developer** | Certain | Slower velocity | Lean on compiler; start with minimal Rust surface; `codeatlas-core` keeps Rust scope focused | POC |
| **Detector trait over-designed or under-designed** | Medium | Refactor later | Keep trait minimal in POC; only two implementations; evolve from real usage | POC |
| **`.codeatlas.yaml` schema churn** | Medium | Breaking changes for users | Version the schema; keep POC surface minimal; expand in MVP | POC |
| **Overlay model adds data model complexity** | Low | Slower development | The separation is straightforward — two graph layers with a merged query view. Worth the cost for provenance integrity. | POC |
| **oxc error recovery vs tree-sitter** | Low | Missing edges | Fall back to tree-sitter for failed files | MVP |
| **Watch mode instability** | Medium | Destroys trust | Stable IDs, debounce, viewport preservation, branch-switch triggers broader rescans | MVP |
| **Change overlay misunderstood as architecture diff** | Medium | Trust erosion | Explicit labeling, documentation, UI copy that says "change overlay" not "diff" | MVP |
| **Graph-shaping change classification false positives** | Low | Alert fatigue | Only classify specific file types (manifests, lockfiles, tsconfigs); user can dismiss | MVP |
| **VS Code extension sprawl** | Medium | Extension absorbs roadmap | Keep it thin and bridge-first | MVP |
| **Barrel file resolution infinite loops** | Medium | Crashes or hangs | Cycle detection cutoff in transitive resolution | MVP |
| **Public surface detection misses non-standard exports** | Medium | Incomplete lens | Start with standard patterns (package.json exports, pub items); declare limitations | MVP |
| **MCP protocol evolution** | Low | API instability | Follow protocol versioning; keep MCP surface thin; CLI/JSON is primary | Platform |

---

## 25. Decisions Log

Authoritative decisions in `docs/decisions.md`. Key decisions affecting this PRD:

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Product name | Code Atlas | Communicates explorable, layered architecture |
| Graph model philosophy | Profiled, not absolute | No single "correct graph" — depends on build context |
| **Discovered/declared separation** | **Immutable discovered graph + config overlay layer** | **Provenance integrity: config supplements but never silently mutates discoveries. Suppressed edges remain queryable.** |
| **Edge taxonomy** | **Evidence class + semantic category (value/type-only/dev/build/test/manual)** | **Makes downstream impact analysis meaningful instead of noisy. Data already available from tree-sitter and cargo metadata.** |
| **Compatibility report** | **First-class POC feature, detector-driven** | **Trust requires upfront honesty about what the tool can handle for a specific repo. Badges inside the graph are too late.** |
| **Config overlay model** | **add/suppress, not add/remove** | **"remove" silently deletes observed edges, breaking provenance. "suppress" hides in default view while preserving the edge in the data model.** |
| Default view | Workspace/package topology | Answers "what are the main pieces?" before file-level detail |
| Visualization library | React Flow + ELK.js | Only option supporting compound nested nodes with React JSX |
| **Analysis core architecture** | **Separate `codeatlas-core` crate** | **Enables CLI, MCP, headless scans without rewriting** |
| **Detector architecture** | **Internal `Detector` trait from POC with compatibility assessment** | **Language/framework analysis pluggable; compatibility report is detector-driven; public API deferred** |
| **Repo-local config** | **`.codeatlas.yaml` designed in POC with overlay model** | **Escape hatch for static analysis limits; preserves discovered graph immutability** |
| **Identity scheme** | **Two-key: materialized + lineage, hybrid detection** | **Current addressing + rename/move survival. Git rename as primary signal; broader rescans on branch/config changes; content fingerprints as Platform extension.** |
| **Unsupported construct model** | **Badge what we can't analyze, don't hide it** | **Trust requires honesty about limits** |
| **Change overlay vs architecture diff** | **Change overlay in MVP, true diff in Platform** | **Honest about what file-change mapping can show** |
| **Graph-shaping change handling** | **Classify manifest/lockfile/tsconfig changes separately in overlays** | **These changes can invalidate the graph itself; treating them as normal file changes is misleading** |
| **Public surface lens** | **MVP feature** | **"What does this package expose?" is more useful early than interactive HTML export** |
| **Automation interface** | **CLI/JSON primary, MCP as thin adapter** | **CLI is richer, stable, scriptable. MCP adapts a focused subset. Avoids the Nx lesson of broad MCP being less efficient than focused tools.** |
| **Interactive HTML export** | **Deferred to Platform** | **SVG/PNG covers core sharing. HTML has privacy/size/staleness issues. Not a competitive differentiator — trust and honesty are.** |
| **FTS5 search** | **Deferred to Platform** | **In-memory fuzzy search over current graph is sufficient for MVP. FTS5 valuable for cross-snapshot search.** |
| **Binary size** | **Monitor, no hard gate** | **Tauri naturally produces small binaries. Gating on <15MB risks bad tradeoffs against functionality.** |
| **Profile UI (MVP)** | **Canonical presets + toggleable dimensions** | **Covers 90%+ of real-world needs without free-form editor complexity. Full editor deferred to Vision.** |
| **Golden corpus** | **Reference repos as headline acceptance metric** | **Trust measured against real repos, not coverage %** |
| **Saved views + export** | **MVP, not deferred** | **Without shareability, the product stays personal-only** |
| **Factory interface** | **Vision phase only; does not shape POC/MVP** | **Trust-first; prove the graph before building on top of it** |
| Workspace discovery | cargo metadata + JS workspace config detection | Machine-readable workspace structure, not directory walking |
| Parsing approach (POC) | tree-sitter | Extensible, error-tolerant, incremental |
| Parsing approach (MVP, TS/JS) | oxc_parser + oxc_resolver | Faster parsing, full TS resolution spec |
| Language tier naming | "syntactic + resolver-aware" not "full semantic" | Honest about POC-level analysis capabilities |
| Trust model | Compatibility report + graph health + edge provenance + categories + unsupported construct badges in POC | Trust is the moat; cannot defer it |
| Scan delivery | Progressive streaming via Channel<T> | First meaningful frame fast; user can interact with partial results |
| "Blast radius" | Renamed to "static downstream impact" | Honest about what static analysis can determine |
| "PR diff" | Renamed to "change overlay against base ref" | Local-first app; not a graph diff; honest about semantics |
| "Dead code detection" | Deferred past MVP, renamed | Zero in-degree is a weak heuristic with high false-positive rates |
| Service interaction slices | Deferred to Platform (requires runtime data) | Static imports cannot detect HTTP/gRPC/Kafka communication |
| Non-code files | Hidden by default, opt-in | Prevents architecture map from becoming a file browser |
| Remote workspaces | Out of scope for v1 (local only) | Remote VS Code extension host requires workspace-side scanning |
| Stable node identity | Two-key scheme designed in POC, lineage tracking persisted in MVP | Foundational for all downstream features |
| Live refresh | Manual rescan (POC), file watching (MVP) | Proves same pipeline; watching adds complexity |
| VS Code integration | Stretch (POC), thin bridge (MVP), local only | Workflow adoption, not required for core hypothesis |
| Persistence | None (POC), SQLite without FTS5 (MVP) | In-memory sufficient for POC; FTS5 deferred to Platform |
| Product framing | Architecture intelligence → factory capabilities (Vision) | Build the trustworthy understanding layer first |
| Egress model | Local analysis, explicit egress | Analysis is local; data leaves only through explicit user actions with clear labeling |
| Rendering strategy | Eager scan, lazy render | Scanner processes all files; React Flow only renders expanded packages. API supports future lazy scanning. |

### Resolved Open Questions

| # | Question | Resolution |
|---|----------|------------|
| Q1 | Should monorepos be the primary demo scenario? | **Yes.** Workspace discovery is a POC feature. The POC's own codebase (Rust + TypeScript Tauri project) is the primary test case. |
| Q2 | Should edges show multiplicity when packages are collapsed? | **Yes, bundle edges** between collapsed packages. Show individual file-to-file edges when both packages are expanded. |
| Q3 | What demo graph if user doesn't have a project? | **Ship a JSON fixture** of a representative multi-package graph with health indicators, edge provenance with categories, unsupported construct badges, and compatibility report. |
| Q4 | Should unsupported file types appear as structural nodes? | **Not by default.** Code + manifests are the default node set. Other files are opt-in via filter toggle. |
| Q5 | Is the POC analysis "full semantic"? | **No.** POC is syntactic + resolver-aware. Semantic analysis (rust-analyzer, TS compiler, SCIP) is Vision phase. Terminology updated accordingly. |
| Q6 | Include a second layout algorithm? | **One good default** (ELK layered, direction DOWN) for POC. Layout switcher is P2. |
| Q7 | What is the graph identity scheme? | **Two-key:** materialized key (`{workspace_root}:{language}:{entity_kind}:{relative_path}`) for current addressing, lineage key (UUID + hybrid rename tracking) for persistence and diff. Content/AST fingerprints are a Platform extension. |
| Q8 | How does "PR diff" work without GitHub/GitLab integration? | **It doesn't.** Feature is "change overlay against base ref" using local git. Base ref is user-visible and changeable. This is file-change mapping onto the current graph, not a graph-to-graph comparison. Graph-shaping changes are classified separately. |
| Q9 | Does the VS Code extension work with remote workspaces? | **Not in v1.** Local workspaces only. Remote support requires workspace-side scanning component (Platform phase). |
| Q10 | What about dead code detection? | **Deferred.** Will ship as "possibly unreferenced internal module" with explicit entrypoint configuration via `.codeatlas.yaml` and low-confidence badges. |
| Q11 | Should the scanner be embedded in the Tauri app? | **No.** The analysis core (`codeatlas-core`) is a separate crate with no Tauri dependency. The desktop app calls it in-process. This enables CLI, MCP, and headless surfaces without a rewrite. |
| Q12 | When does repo-local configuration ship? | **Schema designed in POC with overlay model.** `ignore` and `entrypoints` functional in POC. Full overlay model (add/suppress) functional in data model from POC. Full feature set in MVP. |
| Q13 | What is the headline acceptance metric? | **Golden corpus correctness**, not coverage percentage. Coverage (>80%) is hygiene. Trust is measured against reference repos. |
| Q14 | What about saved views and shareability? | **MVP.** Named views, bookmarks, SVG/PNG export. Interactive HTML deferred to Platform. |
| Q15 | Should the POC support both Rust and TypeScript? | **Yes**, but with declared limits per language and an upfront compatibility report. The project itself is a Tauri app (both languages). Unsupported constructs are badged, not hidden. |
| Q16 | Should `.codeatlas.yaml` support removing discovered edges? | **No.** The discovered graph is immutable. Config uses `suppress` to hide edges in the default view while preserving them in the data model. This preserves provenance integrity. |
| Q17 | Should the MCP surface be broad or focused? | **Focused.** CLI/JSON is the primary automation interface. MCP adapts a minimal subset (overview, health, deps, impact, search, affected). Full graph dumps stay out of the prompt path. |
| Q18 | Should binary size have a hard gate? | **No.** Monitor it, but do not sacrifice functionality. Tauri naturally produces small binaries. |
| Q19 | Should edge categories be captured from day one? | **Yes.** Tree-sitter and cargo metadata already provide this data (import type vs import, dep_kinds). Capturing it early is low cost; retrofitting it later requires rescanning all golden corpus and revalidating edge correctness. |
| Q20 | Should interactive HTML export be in MVP? | **No.** SVG/PNG covers the core sharing workflow. HTML export has privacy (embedded file paths), size, and staleness concerns. It is not the competitive wedge — trust and honesty are. Deferred to Platform where redaction controls can ship alongside it. |
