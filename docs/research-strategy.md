# Code Atlas — Strategy & Product Research

**Date:** 2026-03-18
**Status:** Consolidated research for planning
**Scope:** Product direction, competitive landscape, UX, distribution, risks, build sequence, PRD amendments

---

## 1. Product Direction & Thesis Validation

### 1.1 What The PRD Gets Right (Keep)

All research sources agree on these as foundational:

- **Profiled graphs, not "the graph."** Visible product concept, not just internal detail.
- **Compatibility report before trust is requested.** Unusually strong product judgment.
- **Immutable discovered graph plus overlays.** Best decision in the PRD.
- **Workspace/package topology as landing surface.** Correct for comprehension and performance.
- **Thin MCP adapter, richer CLI/JSON core.** Broad MCP-first design is a trap (Nx lesson).
- **Separate `codeatlas-core` crate from start.** Non-negotiable.
- **Progressive streaming.** Giant payloads → poor perceived performance.
- **Unsupported construct badges.** Hidden incompleteness destroys trust.
- **Edge taxonomy (value/type-only/dev/build).** Data already available from tree-sitter and cargo metadata.

### 1.2 Core Product Framing

The product is not "a map of the repo." It is:

1. A **build-context-aware architecture index**
2. With an **interactive graph UI**
3. And a **trust model** telling users what the index means and what it does not mean

Feature order must follow: detect → declare support honestly → build index → query/visualize → layer workflows.

### 1.3 Highest-Value User Outcomes

1. "Can I trust this graph?" — compatibility report, graph health, edge provenance
2. "Can I answer impact questions fast?" — downstream impact with edge-category filters
3. "Can I open this in my editor immediately?" — click-to-open
4. "Does it stay correct as my branch changes?" — watch mode + change overlay

Highest-value user flow:
> open repo → read compatibility report → trust package topology → inspect one node/edge → run downstream impact with runtime-only defaults → jump to editor → save/share view

**Default static impact to runtime-only edges.** Type-only imports are erased from JS output; Cargo distinguishes normal/dev/build. Showing everything makes impact analysis noisy.

---

## 2. Recommended PRD Amendments

### 2.1 Changes Before Implementation

1. **Do not persist `workspace_root` in exported/materialized keys.** Use portable address: `{repo_fingerprint}:{profile_fingerprint}:{entity_kind}:{relative_path}`. Store absolute roots only in local session metadata. Prevents path leakage, enables portable snapshots.

2. **Add Cargo resolver version + `required-features` to profile model.** Resolver v2 changes feature unification — same feature list can behave differently. Honor per-target `required-features` from `cargo metadata`.

3. **Broaden TS public-surface lens beyond runtime exports.** Add `types`/`typings`/`typesVersions` and TS version metadata. A runtime-only lens under-explains public API for TS libraries.

4. **Narrow Rust public-surface MVP promise.** rustdoc JSON is nightly-dependent/unstable. Ship MVP lens based on `lib` targets, public modules, `pub use` chains, crate entrypoints. Deeper item-level extraction as experimental.

5. **Use system `git` first for change overlays.** Don't start with libgit2. `git merge-base` and `git diff -M` express the right semantics and honor user's config/worktrees/submodules. Wrap behind a thin VCS façade.

6. **Simplify first VS Code bridge.** Use URI handlers + deep links for "Show in Code Atlas" / "Open in VS Code." Only add socket bridge once active-file sync justifies it. Remote VS Code extensions run with remote workspace, not local filesystem.

7. **Add explicit constraints to PRD:**
   - No code execution during analysis (no `build.rs`, proc macros, package scripts, bundlers)
   - Path normalization policy (canonicalize, separate display paths, define symlink behavior)
   - Explicit egress policy (updater/telemetry/crash reporting are network features → opt-in if privacy is core)
   - License review gate for dependencies (ELK/elkjs is EPL-2.0, not MIT)
   - Profile fingerprinting on every scan result

8. **"Unresolved import" is not a single failure type.** The UI should explain *why*: path alias outside active config, package exports condition mismatch, generated file missing, unsupported PnP, dynamic import, ambiguous mixed-module resolution.

### 2.2 Scope Narrowing

- **Public surface lens:** keep, but narrow MVP to package/crate-level public roots and public syntax inventory
- **VS Code bridge:** stretch or late MVP, not early MVP anchor
- **Barrel detector:** MVP only for workspace-owned files, with cycle cutoffs
- **Architecture diff:** keep deferred; change overlay language must not become sloppy
- **Watch mode:** not trivial even once identity exists — build explicit fallback/recovery

### 2.3 Explicit De-Scopes (Until Platform)

- Cross-repo/global graph federation
- Cloud sync and collaborative sessions
- Runtime instrumentation as primary dependency source
- Broad language support beyond Rust + TS/JS
- AI-generated architecture narratives as core workflow
- Rich semantic API signatures in MVP
- Remote workspace support

---

## 3. Competitive Landscape

### 3.1 Competitors

| Tool | Strengths | Weaknesses |
|------|-----------|------------|
| **CodeViz** (YC S24, ~80K VS Code installs) | AI summaries, natural language search, C4/UML, embeddable diagrams, shared workspaces. | Sends code to cloud (Anthropic/GCP/AWS). Privacy is #1 HN concern ("deal-breaker" for enterprise). $19-50/seat/mo. |
| **GitKraken Codemaps** | Born from CodeSee acquisition (May 2024). | **Still in early access as of March 2026 — not GA.** Features page says "currently in development." Effectively vaporware. |
| **SciTools Understand** | Deepest cross-language analysis, 20+ years. Comparison graphs, call hierarchy, control flow. | Legacy UX, expensive. |
| **Nx Graph** | Deep monorepo integration, import-level granularity, affected detection, plugin-extensible. MCP defaults to minimal mode. | Ecosystem-locked (requires Nx). |
| **Sourcegraph** | Most mature cross-repo code navigation via SCIP. | Not an architecture visualization tool. |
| **dependency-cruiser** | Most robust JS/TS dep analysis. Custom rules, CI integration, circular detection. | Flat graph, static output, no interaction, no hierarchy. |
| **Madge** | Simple JS/TS dep graphing, easy setup. | Unwieldy at scale, no hierarchy, no interactivity. |
| **Swark** | LLM-based, all-language-capable. | Non-deterministic, no provenance, no trust model. |

### 3.2 Competitive Wedge

No existing tool provides ALL of:
1. Upfront compatibility report
2. Build-context-aware profiled graphs
3. Edge provenance with semantic categories
4. Discovered/overlay graph separation
5. Local-first with zero network calls
6. Hierarchical zoom with compound nodes
7. Public surface lens

**Durable wedge:** trustworthy, evidence-backed, profile-aware, repo-configurable, local architecture graphs that tell you what they can handle upfront, distinguish edge categories for meaningful impact analysis, and are honest about what they cannot analyze.

### 3.3 Where Competitors Are Stronger

- **SciTools:** Deeper analysis, comparison graphs, call hierarchy
- **Nx Graph:** Deeper monorepo integration, PR-level diffs via Cloud, plugin-extensible
- **CodeViz:** AI summaries, shared workspaces, version history, embeddable diagrams. If privacy not a concern, immediate value.
- **Sourcegraph:** Most mature cross-repo navigation (SCIP)

---

## 4. Trust System Design

### 4.1 Compatibility Report UX

Not a static badge wall. It should answer:
- What was detected
- What was modeled
- What was not modeled
- How much that matters
- What the user can do about it

Sections: support summary, active profile, unsupported constructs, unresolved imports by reason, suggested mitigations.

### 4.2 Detector Compatibility Contract

Each detector returns:
- `support_level`: supported | partial | unsupported
- `capability_flags`: e.g., `handles_exports_conditions`, `handles_dynamic_import`
- `known_gaps`: machine-readable codes
- `repo_specific_findings`: concrete encountered constructs

**Compatibility report ships before full graph.**

### 4.3 Trust Signals (Always Visible)

- Active profile badge
- Support level
- Resolution completeness
- Unsupported construct count
- Manual edge count
- Suppressed edge count
- Last scan time
- Scan fingerprint

### 4.4 UX Rules

- No "green" status if unsupported constructs found in scanned scope
- Every unresolved edge carries a reason
- Every suppression remains auditable
- Never overstate confidence: static downstream impact labeled "static-only"
- Include edge-category filtering (runtime/type/dev/build/manual)

---

## 5. Edge Taxonomy & Impact Analysis

### 5.1 Edge Evidence Model

Every edge carries:
- **Kind:** imports, re_exports, contains, depends_on, manual
- **Category:** value | type_only | dev | build | test | peer | normal | manual
- **Confidence class:** structural < syntactic < resolver-aware < semantic (post-MVP) < runtime (Vision)
- **Source location:** file + line range
- **Resolution method:** tree-sitter path, oxc_resolver, cargo_metadata, manual config
- **Overlay status:** discovered, suppressed (with reason), manual

### 5.2 Why Categories Matter

Without categories, "static downstream impact" lights up test files, type-only consumers, and build tooling alongside production runtime dependents. With categories:
- "What breaks if I change this?" → value + normal edges only
- "What tests cover this area?" → test edges
- "Full dependency surface?" → all categories

### 5.3 Diff Semantics

**MVP: Change overlay** (file-change mapping onto current graph, NOT graph-to-graph comparison)
- Classify changes: code changes vs graph-shaping changes (manifests, lockfiles, tsconfigs, `.codeatlas.yaml`)
- Graph-shaping changes trigger distinct alerts
- Label honestly: "Change overlay against {base-ref}"

**Platform: Architecture diff** (graph-to-graph via snapshots + lineage keys)

---

## 6. Watch Mode & Incremental Analysis

### 6.1 Change Classification

Classify file events into:
- `manifest_graph_shaping` → full rebuild
- `resolver_graph_shaping` → full rebuild
- `code_structural` → file-level incremental
- `non_graph` → ignore

### 6.2 Broad Rescan Triggers

Branch switch, lockfile change, workspace config change, tsconfig change, `Cargo.toml`/`Cargo.lock` change, `.codeatlas.yaml` change, `.gitignore` change, path-classification rule change.

Show banner: "graph's structural assumptions changed, compatibility refresh in progress."

### 6.3 Reliability Controls

- Track event lag and overflow counters
- Detect impossible sequences
- Auto-trigger safety full-rescan when confidence drops
- Native watcher primary → debounced coalescer → poll fallback → hard recovery on overflow

---

## 7. Distribution & Privacy

### 7.1 Distribution Path

1. **POC:** No distribution. `tauri dev` only.
2. **MVP:** GitHub Releases + signed installers + Homebrew cask tap
3. **Post-stable:** winget for Windows, maybe Scoop

### 7.2 Auto-Update Strategy

- **POC:** No updater
- **MVP:** Optional updater after explicitly documenting egress behavior
- **Channels:** alpha, beta, stable
- **UX:** Non-modal "Update ready — restart when convenient." Never auto-restart.
- **Critical:** Do NOT silently add background update checks if "no code leaves your machine" is the product message.

### 7.3 Privacy Architecture

Product-level commitments:
- No network during analysis by default
- No source-content telemetry by default
- Explicit opt-in for diagnostics export
- Redaction path for logs/screenshots if support bundle generated
- All egress types treated explicitly: update checks, crash reporting, telemetry, license checks, cloud AI

### 7.4 Telemetry

- POC/MVP: No telemetry
- Platform: Opt-in only. Anonymous events (scan duration, file count, language breakdown, feature usage). Never: file paths, contents, project names, PII. Use PostHog with anonymous events.

---

## 8. Security Model

### 8.1 Tauri Capabilities

Use aggressively:
- Minimum plugin permissions
- Scoped file access
- No accidental command overexposure
- Scanning entirely in backend Rust
- Narrow commands only: `open_directory`, `start_scan`, `cancel_scan`, `open_in_editor`

### 8.2 Repo Safety

The app must NOT:
- Execute repo code
- Run package scripts
- Invoke build scripts
- Follow dangerous shell paths loosely

### 8.3 Supply Chain

- Lockfile pinning
- `cargo audit` + `cargo deny` + `pnpm audit` in CI
- Signed release artifacts
- Reproducibility checks for core binaries

---

## 9. UX Patterns

### 9.1 Graph Adaptation

| Graph Size | Default State |
|------------|---------------|
| Small (<120 visible) | Top-level packages expanded, modules visible |
| Medium (120-250) | Collapsed at package level |
| Large (>250) | Collapsed. Lower-priority labels hidden. |

Scanner eagerly processes all files. Renderer only creates DOM nodes for expanded packages.

### 9.2 Navigation Flow

compatibility report (trust first) → workspace overview (packages) → selected package (expanded) → file-level inspection (detail panel)

Canvas is continuous. No mode-switching feeling.

### 9.3 Detail Panel

Right-side, collapsible (~300px). Tabs: Overview | Dependencies | Exports | Health.
- Edge lists with evidence class and category, clickable, filterable
- Breadcrumb: Workspace > Package > Module > File

### 9.4 Accessibility

- WCAG 2.2 AA for all non-graph UI (shadcn/ui + Radix provides this)
- All graph info accessible via detail panel, search, health dashboard
- Keyboard shortcuts: Tab/Shift-Tab through nodes, Enter expand/collapse, Escape deselect, arrow keys to connected nodes, Cmd+K search, Cmd+0 fit-to-view
- **Colorblind-safe (Okabe-Ito palette)** with dual encoding (color + dash pattern):
  - Value: `#0072B2` (blue, solid)
  - Type-only: `#56B4E9` (sky blue, dashed)
  - Dev: `#E69F00` (orange, dotted)
  - Build: `#F0E442` (yellow)
  - Normal: `#009E73` (green)
  - Manual: `#CC79A7` (pink, double line)
  - Suppressed: `#999999` (gray, very dashed)

### 9.5 Best Early UX Wins

- Open in editor / copy path / copy node ID
- Show why an import resolved or failed
- Show manual/suppressed edges clearly
- "Trust details" panel reachable from everywhere
- Saved views for onboarding/review/planning

### 9.6 UX Traps to Avoid

- Overly smooth visuals masking low-confidence analysis
- Giant default graph dumps without guided entry points
- Pretending dynamic/runtime links are statically proven
- Polished marketing onboarding before graph is trustworthy
- Rich export before JSON export and saved views

---

## 10. Documentation Strategy

### 10.1 Structure (Diataxis Model)

- tutorials, how-to guides, reference, explanation
- Repo split: `docs/prd.md`, `docs/decisions.md`, `docs/architecture.md`, `.plans/`, `docs/reference/`

### 10.2 Must-Exist Before MVP

- Compatibility model and support contract
- Graph profile explanation
- `.codeatlas.yaml` schema reference
- Privacy and egress policy
- Supported platforms and known limits
- Release/install docs
- Troubleshooting guide for unresolved imports and unsupported constructs

### 10.3 Most Important Doc

A precise **support matrix** matching the compatibility report vocabulary exactly.

---

## 11. Unknown Unknowns & Risk Ledger

### 11.1 High-Signal Risks

| Risk | Why It Matters | Early Probe |
|------|---------------|-------------|
| TS condition-resolution drift | Different stacks produce materially different graphs | Snapshot and display condition set in every run |
| `exports` ordering subtleties | Order semantically meaningful; mistakes silently alter resolution | Unit tests with fixture packages focused on condition order |
| CJS↔ESM edge cases (Node 22+) | `require`/ESM interop alters reachable modules | Dedicated resolver fixtures for `nodenext` scenarios |
| Monorepo linker variance (pnpm/Yarn PnP) | Physical FS differs from logical dependency graph | PnP and pnpm symlink fixtures in golden corpus |
| File watcher reliability | Native events may drop/merge in real environments | Overflow detection + auto-rescan + poll fallback tests |
| Path canonicalization cross-platform | Case/sep/symlink differences break stable IDs | Cross-OS identity conformance tests |
| Overlay abuse | Manual edges can hide detector weaknesses | Report discovered-vs-overlay ratio, warn when overlay-heavy |
| Snapshot corruption / partial writes | Local cache integrity impacts trust | Transactional writes + checksum validation |
| Large repo memory pressure | Degrades UX and crashes analysis | Staged loading + backpressure + hard memory guardrails |
| Inaccurate confidence messaging | Destroys trust faster than missing features | Strict confidence policy + UI language review |
| Portable identity vs privacy | IDs with absolute paths leak machine paths | Fix before persistence ships |
| Type vs runtime surface in TS | `types`/`typings`/`typesVersions` matter | Add to surface model early |
| Cargo resolver semantics | Omitting resolver behavior misleads Rust users | Show effective features/resolver semantics in profile |
| Yarn PnP corpus verification | Multi-project/workspace nuance | Include in compatibility report until proven on real repos |
| ELK licensing | EPL-2.0 (not MIT) — enterprise review issue | Conscious decision, not surprise |
| macOS desktop E2E automation | Weaker than Windows/Linux in Tauri | Plan manual smoke + core-heavy automation |
| SQLite portability | WAL files must be treated as unit | Transactional writes, handle busy states |

### 11.2 Path & Filesystem Reality

Address before first real scan:
- Symlink loops, symlinked workspace packages
- Case-insensitive vs case-sensitive filesystems
- Windows path normalization and drive letters
- Unicode paths
- Generated files tracked by git
- Submodules, worktrees

---

## 12. Build Sequence

### 12.1 Recommended Order (Trust-First)

1. Lock identity model, compatibility report schema, core API boundary
2. Workspace discovery + compatibility report + package topology before file graphs
3. Rust + TS detectors with unsupported-construct inventories before canvas work
4. Overlays and merged query views before persistence
5. Persistence and stable snapshot metadata before watch mode
6. Watch mode and Git-backed change overlay after graph truth is stable
7. Downstream impact, public-surface lenses, editor round-trip
8. Signing, updater, support docs, corpus expansion before external beta

**One sentence:** Ship the trust model first, workflows second, convenience surfaces last.

### 12.2 Critical Path Dependencies

1. Identity scheme MUST be designed before first scan
2. Detector trait MUST be defined before implementing any detector
3. Graph overlay model MUST be in data model from day one
4. ELK + React Flow prototype MUST happen early (M3) — highest-risk frontend integration
5. Streaming pipeline (Channel<T>) MUST be validated early (M4)

### 12.3 Phase Structure

**Phase A — Trustworthy POC Core:**
- Workspace discovery, config parse, compatibility report
- Package/crate graph, file graph, graph health
- React Flow overview, detail panel, JSON export

**Phase B — Value Layer:**
- Saved views, public surface lens
- Filtered static downstream impact
- Editor open/reveal
- Better unresolved reason reporting

**Phase C — Operational Maturity:**
- SQLite snapshots, lineage tracking
- Watch mode, change overlay
- Release channels, optional updater

**Phase D — External Surfaces:**
- CLI stabilization, thin MCP adapter
- Docs site, broader ecosystem integrations

### 12.4 90-Day Execution Plan

**Days 1–30:**
1. Finalize architecture and contracts (core, detector trait, identity, overlay schema)
2. Implement workspace discovery + compatibility report foundation
3. Build streaming scaffolding and phased render path
4. Stand up golden corpus harness with 3–5 representative repos

**Days 31–60:**
1. Implement Rust + TS POC detectors with unsupported inventory
2. Ship core graph health panel and evidence inspector
3. Implement watch-mode prototype with fallback logic
4. Establish CI split (fast, integration, nightly)

**Days 61–90:**
1. Move TS resolver toward MVP-grade (`oxc_resolver` integration)
2. Add SQLite snapshots and baseline diff plumbing
3. Stand up signed multi-platform release pipeline + updater flow
4. Run trust-focused user testing on impact/diff/compatibility workflows

---

## 13. Project Management

### 13.1 Feature Definition of Done

Every feature affecting graph correctness requires:
- Golden corpus coverage or fixture addition
- Health/compatibility behavior updated
- Docs updated
- Failure behavior defined
- Performance budget checked

Every feature PR answers: what graph invariant changed, what profile semantics changed, what unsupported constructs added/removed, what corpus cases added, what performance budget affected.

### 13.2 Recommended Issue Taxonomy

- `area:core`, `area:ui`, `area:detectors`, `area:config`, `area:docs`, `area:release`
- `phase:poc`, `phase:mvp`
- `risk:trust`, `risk:performance`, `risk:cross-platform`

### 13.3 Engineering Hygiene

- API/schema versioning for IPC and snapshot files
- Mandatory migration notes for persisted format changes
- Explicit "risk section" in PR template for detector/resolution logic changes
- ADR required for changes to: core graph model, identity, profile semantics, config overlay semantics, persistence model, external API surface

---

## 14. Future-Proofing Notes

### 14.1 WASM Compatibility

Design `codeatlas-core` for WASM from start (not a POC/MVP goal):
- Avoid platform-specific APIs
- Keep filesystem I/O behind a `trait FileSystem`
- Avoid `tokio` in core's public API
- Don't depend on `cargo_metadata` (subprocess) in core — define `WorkspaceMetadata` struct that caller populates

### 14.2 Plugin System (Platform)

- WASM plugins via wasmtime/Extism (safety, cross-language, distribution)
- Rhai scripting for lightweight config rules
- Internal `Detector` trait maps to WASM interface (`.wit` file)

### 14.3 External Index Ingestion

Keep path open for importing SCIP or similar code intelligence formats. Strategically important for expanding language support without first-party analyzers.

### 14.4 Open Source Licensing

- MIT for `codeatlas-core` and desktop app (maximizes adoption)
- Add `LICENSE` file + SPDX headers

---

## 15. Source References

### Core Platform
- Tauri v2 docs: https://v2.tauri.app/
- Tauri capabilities: https://v2.tauri.app/security/capabilities/
- Tauri updater: https://v2.tauri.app/plugin/updater/
- Tauri distribution: https://v2.tauri.app/distribute/
- Tauri signing (macOS/Windows): https://v2.tauri.app/distribute/sign/
- Tauri GitHub pipelines: https://v2.tauri.app/distribute/pipelines/github/
- Tauri WebDriver: https://v2.tauri.app/develop/tests/webdriver/

### Graph UI & Layout
- React Flow: https://reactflow.dev/
- React Flow performance: https://reactflow.dev/learn/advanced-use/performance
- React Flow testing: https://reactflow.dev/learn/advanced-use/testing
- ELK reference: https://eclipse.dev/elk/reference.html
- ELK/elkjs repo: https://github.com/kieler/elkjs

### Parsing & Resolution
- tree-sitter: https://tree-sitter.github.io/
- Oxc: https://oxc.rs/
- Oxc resolver: https://github.com/oxc-project/oxc-resolver
- TypeScript moduleResolution: https://www.typescriptlang.org/tsconfig/#moduleResolution
- TypeScript project references: https://www.typescriptlang.org/docs/handbook/project-references.html
- Node.js packages: https://nodejs.org/api/packages.html
- pnpm workspaces: https://pnpm.io/workspaces
- Yarn PnP: https://yarnpkg.com/features/pnp

### Rust & Analysis
- cargo metadata: https://doc.rust-lang.org/cargo/commands/cargo-metadata.html
- Cargo features: https://doc.rust-lang.org/cargo/reference/features.html
- petgraph StableGraph: https://docs.rs/petgraph/latest/petgraph/stable_graph/struct.StableGraph.html
- rust-analyzer architecture: https://rust-analyzer.github.io/book/contributing/architecture.html
- Sourcegraph SCIP: https://github.com/sourcegraph/scip
- notify crate: https://docs.rs/notify/latest/notify/
- ignore crate: https://docs.rs/ignore

### Editor & Desktop
- VS Code API: https://code.visualstudio.com/api/references/vscode-api
- VS Code extension host: https://code.visualstudio.com/api/advanced-topics/extension-host
- MCP specification: https://modelcontextprotocol.io/specification/

### Data & Storage
- SQLite WAL: https://www.sqlite.org/wal.html
- SQLite FTS5: https://www.sqlite.org/fts5.html
