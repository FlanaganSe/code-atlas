# Code Atlas

A local-first desktop app that turns a codebase into an interactive, zoomable architecture map. Point it at a repo, get a navigable nested graph — packages, modules, files, and their dependencies — without sending code anywhere.

**Status: POC complete (M1–M9).** Rust + TypeScript scanning, interactive compound graph visualization, health/provenance/overlay, golden corpus validation.

## Goal

Build a proof of concept that validates:
- Parsing a real Rust/TypeScript project with tree-sitter
- Building a hierarchical graph in Rust (petgraph)
- Rendering it as a zoomable, nested compound visualization (React Flow + ELK)
- Adaptive expand/collapse so the graph never becomes an unreadable hairball

## Stack

Tauri v2 (Rust backend + React/TypeScript frontend), React Flow v12, ELK.js, petgraph, tree-sitter.

## Docs

- [`docs/prd.md`](docs/prd.md) — Product requirements (POC / MVP / Vision phases)
- [`docs/research-implementation.md`](docs/research-implementation.md) — Library APIs, architecture patterns, implementation research
- [`docs/research-strategy.md`](docs/research-strategy.md) — Product direction, competitive landscape, strategy
- [`docs/SYSTEM.md`](docs/SYSTEM.md) — System architecture documentation
- [`docs/decisions.md`](docs/decisions.md) — Architectural Decision Records

## Development

```bash
pnpm dev       # Tauri dev (launches app with hot reload)
pnpm build     # Production build
pnpm test      # Frontend tests (Vitest)
cargo test     # Rust tests
```

### Prerequisites

- Rust (stable) via rustup
- Node.js 22+ and pnpm
- macOS: Xcode Command Line Tools (`xcode-select --install`)
