# Research: Tauri POC Foundations

**Date:** 2026-03-17
**Scope:** Tauri best practices, March 2026 baseline, macOS Rust/Tauri install verification, and a testable architecture for a network code visualization proof of concept.

## Current baseline

- Tauri should be started on the current stable v2 line.
- Registry checks on 2026-03-17 returned:
  - `@tauri-apps/cli`: `2.10.1`
  - `@tauri-apps/api`: `2.10.1`
  - `create-tauri-app`: `4.6.2`
- The latest Rust stable release announcement on 2026-03-17 is `1.94.0`, released on 2026-03-05.

## Local macOS findings

- Host: `macOS 26.3` on `arm64`
- Xcode is installed at `/Applications/Xcode.app/Contents/Developer`
- Node is installed: `v24.13.0`
- pnpm is installed: `10.30.3`
- Rust is not installed yet:
  - `rustup`: missing
  - `rustc`: missing
  - `cargo`: missing
- `pnpm dlx @tauri-apps/cli@latest info` confirms the same: Xcode is present, Rust is missing.

## Recommended technical direction

### 1. Stack baseline

- Use `Rust stable` via `rustup`, not an ad hoc Homebrew Rust install.
- Start on `Tauri v2`, not v1.
- Use the React + TypeScript Tauri template and keep the project aligned with the local rules:
  - React 19
  - TypeScript 5.x
  - Tailwind CSS v4
  - pnpm
- After scaffolding, add a `rust-toolchain.toml` so the team and CI share the same toolchain.
- Prefer pinning the project to a known stable Rust release after initial bring-up. As of 2026-03-17, that is `1.94.0`.

### 2. POC architecture for a network visualization app

- Keep rendering and interaction in the webview.
- Keep graph parsing, indexing, filtering, and expensive layout or analysis in Rust.
- Treat the Rust side as the source of truth for application state that matters across windows or sessions.
- Treat the frontend as a projection layer over typed graph/view-model data.

Recommended shape:

- `src-tauri/`:
  - `graph_core`: pure graph data types, reducers, filtering, layout orchestration, serializers
  - `app_state`: shared Tauri-managed state
  - `commands`: thin IPC entry points
  - `events` or `streaming`: progress and delta emission back to the frontend
- frontend:
  - `features/graph/`: rendering adapter, viewport state, user preferences
  - `features/filters/`: search, category toggles, graph preferences
  - `lib/tauri/`: typed invoke/event wrappers only

Why this matters:

- Tauri's core process is the only component with full OS access and is the right place for global state and business-sensitive logic.
- The webview stays responsive if heavy work remains off the UI thread.
- Pure Rust and pure TypeScript modules stay easy to unit test.

### 3. IPC and state best practices

- Use `invoke` for request/response commands.
- Use Rust-to-frontend events or channels for progress, graph chunk streaming, and long-running work updates.
- Keep command handlers thin. Push real logic into normal Rust functions or modules.
- Use strongly typed DTOs across the boundary. Keep naming predictable, for example Rust structs serialized to `camelCase`.
- Avoid putting secrets or sensitive filesystem/process logic in the frontend.

Good fit for this POC:

- Frontend asks Rust for an initial graph snapshot.
- Rust emits layout progress and graph deltas while parsing or recomputing.
- Frontend applies deltas and keeps viewport-only state local.

### 4. Security defaults

- Use Tauri v2 capabilities and permissions from the start.
- Grant only the permissions the app actually needs per window.
- Avoid broad shell/process/file permissions in the first POC unless the visualization absolutely depends on them.
- Add an explicit CSP in Tauri config and keep it tight.
- Avoid CDN-loaded scripts in the webview.

For this app, a good starting point is:

- local bundled assets only
- no shell plugin
- no opener/process permissions unless there is a clear use case
- no remote script execution

### 5. Visualization library direction

Recommended default for this POC:

- `Sigma.js` + `Graphology`

Why:

- It is a strong fit for larger, denser network visualization and smooth zoom/pan behavior.
- It matches the "flexible and dynamic depending on graph size" goal better than node-editor-oriented tools.
- Graphology gives a useful graph model plus layout ecosystem.

Good alternative:

- `Cytoscape.js`

Why:

- Mature graph-focused library
- Built-in gestures and graph algorithms
- Good when you want strong graph analysis features and rich styling quickly

Not my default for this concept:

- `React Flow`

Why:

- Excellent for node editors and workflow builders
- Less natural for dense network maps or very large graph canvases

### 6. Testability strategy

- Keep the graph domain model and layout orchestration pure and deterministic where possible.
- Unit test Rust graph reducers, filters, serializers, and layout input/output transformations.
- Unit test frontend state transforms separately from the renderer.
- Use Vitest with Tauri's mock APIs for frontend IPC and event tests.
- Use Tauri event mocking for Rust-emitted event flows.
- Use Tauri window mocking for multi-window logic if that appears in the POC.

Important constraint:

- Tauri desktop WebDriver support does not currently cover macOS because WKWebView has no driver tool.
- For desktop E2E automation, plan on Linux and Windows CI coverage, not macOS desktop WebDriver.
- On macOS, rely on unit tests, Rust tests, mocked frontend tests, and manual smoke tests.

## macOS install and verification checklist

### Verify Apple toolchain

```bash
xcode-select -p
xcodebuild -version
```

If desktop-only, Tauri docs say Command Line Tools are enough:

```bash
xcode-select --install
```

If iOS is a future target, keep full Xcode installed and launch it once after install.

### Install Rust the supported way

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup update stable
rustup default stable
```

### Install standard developer components

```bash
rustup component add rustfmt clippy rust-analyzer
```

### Verify Rust

```bash
rustup --version
rustc --version
cargo --version
rustup show
```

### Optional Apple Silicon extras

If you want to build Intel macOS artifacts from Apple Silicon during local work or CI:

```bash
rustup target add x86_64-apple-darwin
```

If iOS is later added:

```bash
rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim
brew install cocoapods
```

### Verify Tauri prerequisites after Rust is installed

```bash
pnpm dlx @tauri-apps/cli@latest info
```

Expected outcome:

- Xcode or Xcode Command Line Tools: installed
- `rustup`, `rustc`, `cargo`: installed
- Node and package manager detected

## Suggested starting milestone

1. Scaffold a React + TypeScript + Tauri v2 app with pnpm.
2. Pin Rust with `rust-toolchain.toml`.
3. Build the first vertical slice:
   - load a graph fixture
   - render a zoomable/pannable network view
   - apply a few graph preferences
   - round-trip one Rust command and one Rust-emitted event
4. Add tests before adding more features:
   - Rust unit tests for graph transforms
   - Vitest tests for IPC adapters and view-state reducers
5. Add capability files and CSP before expanding plugin use.

## Sources

- Tauri prerequisites: https://v2.tauri.app/start/prerequisites/
- Tauri CLI reference: https://v2.tauri.app/reference/cli/
- Tauri process model: https://v2.tauri.app/concept/process-model/
- Tauri calling the frontend from Rust: https://v2.tauri.app/develop/calling-frontend/
- Tauri mock APIs for tests: https://v2.tauri.app/develop/tests/mocking/
- Tauri WebDriver support: https://v2.tauri.app/develop/tests/webdriver/
- Tauri CSP guidance: https://v2.tauri.app/security/csp/
- Tauri GitHub pipeline guide: https://v2.tauri.app/distribute/pipelines/github/
- Rust install page: https://www.rust-lang.org/tools/install
- rustup installer: https://rustup.rs/
- Rust release announcements: https://blog.rust-lang.org/releases/
- Latest Rust stable release on 2026-03-17: https://blog.rust-lang.org/2026/03/05/Rust-1.94.0/
- Sigma.js docs: https://www.sigmajs.org/docs/
- Cytoscape.js docs: https://js.cytoscape.org/
- React Flow: https://reactflow.dev/
