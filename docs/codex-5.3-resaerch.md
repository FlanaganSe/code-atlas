# Code Atlas Deep Technical Research (Codex 5.4)

Date: March 18, 2026  
Scope: Deep buildout strategy for `docs/prd.md` with implementation, architecture, tooling, risk, delivery, and product-value guidance.

---

## 1. Executive Direction

The PRD’s core thesis is strong and technically differentiated: local-first, evidence-backed graphing with compatibility-first trust semantics. The highest-value path is to **double down on correctness, profile transparency, and deterministic reproducibility** before adding breadth.

Top-level recommendation:

1. Keep `codeatlas-core` as a Tauri-independent engine from day zero.
2. Build strict contracts around graph identity, evidence provenance, and compatibility reporting.
3. Ship POC quickly, but architect storage/diff/watch path in a way that avoids rewrite at MVP.
4. Treat updater/signing/release engineering as first-class platform work, not polish.
5. Make “trust UX” (what’s supported, what’s partial, what’s unknown) the center of product value.

---

## 2. Decision Matrix (Adopt / Phase-In / Defer)

### 2.1 Core Architecture

| Area | Recommendation | Why |
|---|---|---|
| Core analysis engine | **Adopt now:** separate Rust workspace crate(s), no Tauri deps | Enables desktop + CLI + MCP + CI analysis reuse without architectural debt |
| Detector model | **Adopt now:** registry + trait per detector, compatibility assessment required | Aligns with PRD trust model and prevents invasive rewrites |
| Graph layering | **Adopt now:** immutable discovered layer + explicit overlay layer | Preserves provenance and auditability |
| IPC | **Adopt now:** small invoke payloads + channel streaming phases | Better UX and memory behavior for large graphs |
| UI layout | **Adopt now:** React Flow + ELK in worker, DOM-size-driven | Compound graph layout requires measured node dimensions |

### 2.2 Language/Resolution

| Area | Recommendation | Why |
|---|---|---|
| TS/JS parser | **POC:** tree-sitter, **MVP:** `oxc_parser` | Fast bootstrap now, high-fidelity speed path at MVP |
| TS/JS resolver | **MVP:** `oxc_resolver` | Handles modern exports/imports conditions, tsconfig paths/references, and PnP scenarios |
| Rust workspace discovery | **Adopt now:** `cargo metadata` with explicit profile flags | Canonical source for members/features/target-aware deps |
| Rust feature semantics | **Adopt now:** resolver v2 aware modeling | Avoids feature-unification misinterpretation in multi-target builds |

### 2.3 Persistence/Search

| Area | Recommendation | Why |
|---|---|---|
| Snapshot store | **MVP:** SQLite (WAL mode), normalized graph tables | Local-first, robust, transactional snapshots |
| Text search | **Defer to platform:** FTS5 cross-snapshot search | Keep MVP focused on graph trust and impact workflows |
| Cache | **Adopt now:** content-hash + profile-hash + detector-version keyed | Deterministic incremental rebuild and invalidation behavior |

### 2.4 Testing/Verification

| Area | Recommendation | Why |
|---|---|---|
| Graph correctness | **Adopt now:** golden corpus harness + stable expected outputs | Product moat depends on trust, not feature count |
| Property tests | **Adopt now:** identity invariants + overlay/discovery separation | Catches subtle regressions early |
| E2E desktop UI | **Phase-in:** WebDriver for Linux/Windows, smoke alternatives for macOS | Tauri desktop WebDriver support is Linux/Windows-only |
| Perf testing | **Adopt now:** budgets and regression gates in CI | Prevents scale regressions from becoming architectural blockers |

### 2.5 Distribution

| Area | Recommendation | Why |
|---|---|---|
| Release transport | **Start:** GitHub Releases + signed updater artifacts | Fastest reliable path for initial distribution |
| Signing | **Adopt now:** automated macOS + Windows signing in CI | Trust + install friction reduction |
| Update channels | **Adopt now:** stable + beta channels | Safe rollout and fast feedback cycles |

---

## 3. Proposed System Architecture

### 3.1 Repository Shape

Recommended target layout:

```text
/apps
  /desktop            # Tauri shell + React UI
  /cli                # codeatlas CLI binary
  /mcp-adapter        # thin MCP interface over CLI/engine
/crates
  /codeatlas-core     # graph domain, pipeline orchestration
  /detector-rust      # rust detector implementation
  /detector-ts        # ts/js detector implementation
  /graph-diff         # diff semantics (MVP+)
  /snapshot-store     # sqlite persistence
  /config             # .codeatlas.yaml schema + merge logic
  /compat-report      # support contracts and health reporting
/corpus
  /golden             # curated repos + expected artifacts
/docs
  /architecture.md
  /decisions.md
  /runbooks/*
```

### 3.2 Core Pipeline (Contract-first)

Pipeline stages:

1. Workspace discovery (Rust + JS/TS ecosystems).
2. Profile resolution (workspace scope, features, target conditions).
3. Detector compatibility pass (before full graph build).
4. Detector execution into discovered graph partitions.
5. Overlay application (manual add/suppress with explicit badges).
6. Health computation + unsupported construct inventory.
7. Streamed phased graph publication to UI.
8. Optional snapshot persist + diff indexing.

Each stage should have:

- typed request/response contracts
- deterministic ordering guarantees
- structured diagnostics
- per-stage timing + counts

### 3.3 Domain Model Recommendations

#### Node identity

Use two identities:

- `stable_id`: deterministic hash over logical identity tuple
- `instance_id`: run-specific identity for UI/session operations

Logical tuple should include at least:

- `repo_fingerprint`
- `workspace_member`
- `language`
- `entity_kind`
- canonicalized relative path
- optional symbol key

#### Edge model

Edge fields:

- `edge_id`
- `source_node_id`
- `target_node_id`
- `semantic_category` (`runtime`, `type_only`, `dev`, `build`, `manual`, etc.)
- `evidence_class` (`parsed`, `manifest`, `config_overlay`)
- `resolution_method` (`path`, `exports`, `tsconfig_paths`, `cargo_metadata`, etc.)
- `confidence` (enum, not float)
- `overlay_status` (`discovered`, `suppressed`, `manual`)
- `why` (compact evidence trace)

#### Unsupported construct model

Treat unsupported constructs as first-class entities with:

- detector
- location
- reason code
- potential mitigation
- confidence impact

This is necessary to avoid false certainty.

### 3.4 Storage Model (MVP)

SQLite schema shape:

- `snapshots`
- `profiles`
- `nodes`
- `edges`
- `edge_evidence`
- `unsupported_constructs`
- `health_metrics`
- `diff_indices`

Operational recommendations:

- WAL mode
- explicit checkpoint strategy
- deterministic write ordering
- content-addressed dedupe for large evidence strings

Use snapshot-level immutable records; diffs are derived, not mutative updates.

---

## 4. Tooling & Library Recommendations

### 4.1 Rust Core

Adopt:

- `petgraph::StableGraph` for in-memory graph core.
- `cargo_metadata` for Rust workspace/dependency topology.
- `notify` with `recommended_watcher` + `PollWatcher` fallback strategy.
- `serde` + explicit schema versioning for on-disk artifacts.

Rationale:

- StableGraph preserves unrelated indices on removals, useful for graph evolution.
- `cargo metadata` gives machine-readable resolved graph and dependency kinds.
- notify backends have known reliability caveats in network FS/container/large-dir cases, so fallback modes are mandatory.

### 4.2 TS/JS Analysis

Adopt path:

- POC: tree-sitter-based import extraction with explicit limitations.
- MVP: `oxc_parser` + `oxc_resolver`.

Why:

- `oxc_parser` supports modern JS/TS/JSX/TSX syntax and is optimized for speed.
- `oxc_resolver` explicitly targets Node/TS ecosystem realities: tsconfig paths/references, exports/imports conditions, and Yarn PnP considerations.

### 4.3 Resolution Semantics Guardrails

Implement explicit condition-set profiles:

- Node ESM-like: `["node","import"]`
- Node CJS-like: `["node","require"]`
- Bundler-like: configurable condition stack

Also:

- Capture the exact condition order used.
- Persist condition set on snapshots.
- Show it in UI compatibility/profile panel.

This prevents “same repo, different graph” confusion.

### 4.4 Frontend

Adopt:

- React 19 + TypeScript
- React Flow v12
- ELK worker layout pipeline
- minimal predictable state container (Zustand or equivalent reducer model)

Rules:

- layout on worker thread only
- never block render with full graph relayout
- re-layout incrementally by changed subgraph when possible

---

## 5. Compatibility-First Trust System

The PRD’s strongest strategic move is compatibility-first UX. Operationalize with strict contracts.

Detector contract should return:

- `support_level`: `supported | partial | unsupported`
- `capability_flags`: e.g. `handles_exports_conditions`, `handles_dynamic_import`, etc.
- `known_gaps`: structured machine-readable codes
- `repo_specific_findings`: concrete encountered constructs

Compatibility report should ship before full graph.

UX rules:

- No “green” overall status if unsupported constructs were found in scanned scope.
- Every unresolved edge must carry a reason.
- Every suppression must remain auditable.

---

## 6. Diff & Impact Intelligence Design

### 6.1 Diff semantics

MVP diff should distinguish:

- graph-shaping change (manifest, lock, tsconfig, Cargo features)
- structural code change
- non-graph change

Never overstate confidence:

- static downstream impact should be labeled static-only
- include filtering by edge category (runtime/type/dev/build/manual)

### 6.2 Branch comparison behavior

Implement:

- explicit base-ref selection
- cached baseline snapshot by profile hash
- branch switch invalidation rules

Key risk to avoid: comparing two snapshots produced by different effective profile/condition sets without warning.

---

## 7. Watch Mode & Incremental Analysis

### 7.1 Watch strategy

Use layered watch behavior:

1. Native watcher primary backend.
2. Debounced event coalescer.
3. Poll fallback in known-bad environments.
4. Hard recovery mode on overflow or dropped-event detection.

### 7.2 Change classification

Classify file events into:

- `manifest_graph_shaping`
- `resolver_graph_shaping`
- `code_structural`
- `non_graph`

Use this class to decide:

- full rebuild
- workspace/member rebuild
- file-level incremental refresh

### 7.3 Reliability controls

- track event lag and overflow counters
- detect impossible sequences
- auto-trigger safety full-rescan when confidence drops

---

## 8. Distribution, Updating, and Release Engineering

### 8.1 Initial distribution recommendation

Start with:

- GitHub Releases as artifact host
- Tauri updater static JSON + signatures
- platform installers: macOS app bundles/tar for updater, Windows NSIS/MSI, Linux AppImage/deb/rpm

### 8.2 Signing and updater essentials

Critical facts:

- Tauri updater requires signatures and this cannot be disabled.
- macOS notarization is required for Developer ID distribution; free Apple accounts cannot notarize.
- Windows code signing significantly affects SmartScreen experience; EV certs get immediate reputation.

### 8.3 CI strategy

Recommended CI tracks:

1. `ci-fast`: lint/typecheck/unit
2. `ci-integration`: detector integration + golden subset
3. `release`: signed multi-platform artifacts + updater JSON + checks
4. `nightly`: full corpus + perf + flaky detector diagnostics

Release workflow notes:

- Use GH Actions matrix builds with explicit write permissions for release job.
- Linux Arm runner options exist, but emulated builds can be slow/costly.

### 8.4 Channel strategy

Two channels:

- `stable`: production users
- `beta`: pre-release and power users

Use separate updater endpoints/manifests, and include channel in app diagnostics UI.

---

## 9. Security and Privacy Architecture

### 9.1 Security posture

Use Tauri capability/permission model aggressively:

- minimum plugin permissions
- scoped file access
- no accidental command overexposure

### 9.2 Privacy guarantees

Product-level commitments:

- no network during analysis by default
- no source-content telemetry by default
- explicit opt-in for diagnostics export
- redaction path for logs/screenshots if support bundle is generated

### 9.3 Supply-chain and build integrity

Adopt:

- lockfile pinning
- `cargo audit` + `cargo deny` + `pnpm audit` in CI
- signed release artifacts
- reproducibility checks for core binaries

---

## 10. Testing and Quality Plan

### 10.1 Correctness pyramid

1. Unit tests (parsers/adapters/mappers)
2. Property tests (identity/diff invariants)
3. Golden corpus tests (truth repos)
4. Integration tests (end-to-end scanner pipelines)
5. UI behavior tests (graph interactions)
6. E2E desktop tests where platform support exists

### 10.2 Golden corpus strategy

Required dimensions:

- Rust workspaces with features/targets
- TS monorepos with project refs + exports/imports + aliases
- mixed-language repos
- overlays and suppressions
- graph-shaping branch deltas

Store for each corpus case:

- expected compatibility report
- expected unresolved/unsupported inventory
- expected graph metrics
- expected key edge samples

### 10.3 Performance budgets

Define budgets early:

- first compatibility report latency
- first visible graph latency
- full graph completion latency
- watch-mode delta update latency
- memory budget by repo-size bucket

Fail PRs that regress budgets beyond threshold.

### 10.4 E2E caveat

Tauri WebDriver desktop support is currently Linux/Windows; treat macOS desktop E2E via alternate smoke strategy until tooling improves.

---

## 11. Product Value Maximization (User Outcomes)

### 11.1 What users will pay attention to first

1. “Can I trust this graph?”
2. “Can I answer impact questions fast?”
3. “Can I open this in my editor immediately?”
4. “Does it stay correct as my branch changes?”

Optimize first-run UX around these questions.

### 11.2 High-value UX features to prioritize early

- Compatibility report with concrete repo findings
- Edge category filters with clear semantics
- “Why this edge exists” evidence drawer
- Branch diff narrative (“what changed and why graph moved”)
- Fast jump-to-code (path + line + column)

### 11.3 Avoid early UX traps

- Overly smooth visuals masking low-confidence analysis
- Giant default graph dumps without guided entry points
- Pretending dynamic/runtime links are statically proven

---

## 12. Project Management & Documentation Operating Model

### 12.1 Planning cadence

Use repeating cycles per milestone:

1. RFC/plan in `.plans/`
2. implementation slices with explicit acceptance tests
3. decision capture in `docs/decisions.md`
4. post-milestone risk review

### 12.2 Documentation set (recommended)

Create and maintain:

- `docs/architecture.md` (current state + context map)
- `docs/compatibility-contract.md`
- `docs/graph-schema.md`
- `docs/release-runbook.md`
- `docs/testing-strategy.md`
- `docs/troubleshooting.md`

### 12.3 Engineering hygiene

Enforce:

- API/schema versioning for IPC and snapshot files
- mandatory migration notes for any persisted format changes
- explicit “risk section” in PR template for detector/resolution logic changes

---

## 13. Unknown-Unknowns Risk Ledger (High Signal)

| Risk | Why it matters | Early probe |
|---|---|---|
| TS condition-resolution drift | Different condition stacks produce materially different graphs | Snapshot and display condition set in every run |
| `exports` ordering subtleties | Order is semantically meaningful; mistakes silently alter resolution | Unit tests with fixture packages focused on condition order |
| CJS↔ESM edge cases (Node 22+ behavior) | `require` and ESM interop can alter reachable modules | Add dedicated resolver fixtures for `nodenext` scenarios |
| Monorepo linker variance (pnpm/Yarn PnP) | Physical FS layout differs from logical dependency graph | Include PnP and pnpm symlink fixtures in golden corpus |
| File watcher reliability | Native events may drop/merge in real-world environments | Overflow detection + auto-rescan + poll fallback tests |
| Path canonicalization cross-platform | Case/sep/symlink differences can break stable IDs | Cross-OS identity conformance tests |
| Overlay abuse | Manual edges can hide detector weaknesses | Report discovered-vs-overlay ratio and warn when overlay-heavy |
| Snapshot corruption / partial writes | Local cache integrity impacts trust | Transactional writes + checksum validation |
| Large repo memory pressure | Can degrade UX and crash analysis | Staged loading + backpressure + hard memory guardrails |
| Inaccurate confidence messaging | Destroys trust faster than missing features | Strict confidence policy + UI language review |

---

## 14. Explicit De-Scopes (for focus and risk control)

Defer until Platform phase unless forced by a customer requirement:

- cross-repo/global graph federation
- cloud sync and collaborative sessions
- runtime instrumentation as primary dependency source
- broad language support beyond prioritized detectors
- AI-generated architecture narratives as core workflow

Reason: these are multiplicative complexity vectors that can dilute core moat (trustworthy local static graph + impact workflows).

---

## 15. Recommended 90-Day Execution Plan

### Days 1–30

1. Finalize architecture and contracts (`core`, detector trait, identity, overlay schema).
2. Implement workspace discovery + compatibility report foundation.
3. Build streaming scaffolding and phased render path.
4. Stand up golden corpus harness with 3–5 representative repos.

### Days 31–60

1. Implement Rust + TS POC detectors with explicit unsupported inventory.
2. Ship core graph health panel and evidence inspector.
3. Implement watch-mode prototype with fallback logic.
4. Establish CI split (`fast`, `integration`, `nightly`).

### Days 61–90

1. Move TS resolver stack toward MVP-grade behavior (`oxc_resolver` integration path).
2. Add SQLite snapshots and baseline diff plumbing.
3. Stand up signed multi-platform release pipeline + updater flow.
4. Run trust-focused user testing on impact/diff/compatibility workflows.

---

## 16. Source-Backed Facts Used

Note: Some recommendations below are direct from sources; others are informed architectural inferences.

### Tauri / distribution / security

- [S1] Tauri calling Rust from frontend (commands/events/channels): https://v2.tauri.app/develop/calling-rust/
- [S2] Tauri updater plugin docs (signatures required, static JSON/server flows): https://v2.tauri.app/plugin/updater/
- [S3] Updater source markdown (signature cannot be disabled; required JSON keys): https://raw.githubusercontent.com/tauri-apps/tauri-docs/v2/src/content/docs/plugin/updater.mdx
- [S4] Tauri permissions model and capabilities references: https://v2.tauri.app/security/permissions/
- [S5] Tauri GitHub pipeline guide (matrix examples, permissions, arm runner notes): https://v2.tauri.app/distribute/pipelines/github/
- [S6] Tauri WebDriver guide (desktop support caveat): https://v2.tauri.app/develop/tests/webdriver/
- [S7] Tauri macOS signing/notarization guide: https://v2.tauri.app/distribute/sign/macos/
- [S8] Tauri Windows signing guide (SmartScreen and cert implications): https://v2.tauri.app/distribute/sign/windows/
- [S9] Tauri Windows installer guide (MSI/NSIS caveats): https://v2.tauri.app/distribute/windows-installer/

### TS/Node resolution semantics

- [S10] TypeScript `moduleResolution` options (`node16`/`nodenext`/`bundler`): https://www.typescriptlang.org/tsconfig/moduleResolution.html
- [S11] TypeScript modules reference (Node 22+ interop behavior in `nodenext`, imports/exports resolution details): https://www.typescriptlang.org/docs/handbook/modules/reference.html
- [S12] TypeScript project references: https://www.typescriptlang.org/docs/handbook/project-references.html
- [S13] Node package exports/imports and conditional ordering semantics: https://nodejs.org/api/packages.html

### Rust graph/detector stack

- [S14] Cargo metadata command and JSON model: https://doc.rust-lang.org/cargo/commands/cargo-metadata.html
- [S15] Cargo feature resolver behavior and resolver v2: https://doc.rust-lang.org/cargo/reference/features.html
- [S16] Oxc parser docs: https://docs.rs/oxc_parser/latest/oxc_parser/
- [S17] Oxc resolver README/docs: https://raw.githubusercontent.com/oxc-project/oxc-resolver/main/README.md
- [S18] petgraph StableGraph behavior: https://docs.rs/petgraph/latest/petgraph/stable_graph/struct.StableGraph.html
- [S19] notify crate known watcher limitations and fallback implications: https://docs.rs/notify/latest/notify/

### Package manager and local storage realities

- [S20] pnpm symlinked/hardlinked node_modules structure: https://pnpm.io/symlinked-node-modules-structure
- [S21] Yarn Plug’n’Play behavior (no `node_modules`, `.pnp.cjs`): https://yarnpkg.com/features/pnp
- [S22] SQLite WAL tradeoffs and host constraints: https://www.sqlite.org/wal.html
- [S23] SQLite FTS5 reference (for platform-phase search): https://www.sqlite.org/fts5.html

### Product source

- [P1] Project PRD (primary internal source): `/Users/seanflanagan/proj/tauri-poc-zoom-thing/docs/prd.md`

---

## 17. Inferences vs Direct Source Claims

Direct source claims are used for ecosystem/platform behavior (updater signatures, signing requirements, resolver semantics, watcher caveats, etc.).

The following are architectural inferences (not quoted as hard requirements from one source):

1. Exact crate/module decomposition and folder layout.
2. Proposed DB schema specifics.
3. Risk-prioritized 90-day plan.
4. Product/UX prioritization ordering.
5. CI lane design and promotion gates.

These inferences are intentionally aligned with PRD constraints and source-verified platform behavior.

