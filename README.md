# Code Atlas

A local-first desktop app that turns a codebase into an interactive, zoomable architecture map. Point it at a repo, get a navigable nested graph — packages, modules, files, and their dependencies — without sending code anywhere.

**Status: WIP — product requirements and research are still being finalized. No application code yet.**

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
- [`research/consolidated-technical-decisions.md`](research/consolidated-technical-decisions.md) — Technical decisions with rationale
- [`research/consolidated-market-and-product.md`](research/consolidated-market-and-product.md) — Market analysis and product positioning

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
