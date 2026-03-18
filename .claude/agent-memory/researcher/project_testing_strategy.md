---
name: project_testing_strategy
description: Testing strategy research (March 2026): 3-layer approach for Tauri v2 graph viz POC — Rust unit/integration/bench, frontend jsdom+mockIPC, visual regression deferred; E2E skipped on macOS
type: project
---

Testing strategy decided for this Tauri v2 + React/TS graph visualization POC.

**Why:** macOS (darwin 25.3.0) blocks official `tauri-driver` WebDriver E2E (no WKWebView driver). Visualization testability is critical for comparing layout algorithms.

**3-layer strategy:**

1. **Rust unit + integration + benchmarks** — `mod tests` + `proptest` for property testing; `tauri::test::mock_builder()` + `assert_ipc_response` for IPC integration; `criterion` benchmarks at 10/100/1000 nodes; `rstest` for parameterized layout A/B comparison. These are the primary POC comparison tools.

2. **Frontend unit tests (jsdom)** — `mockIPC()` intercepts all `invoke()` calls; graph state as pure functions (zoom reducer, data transforms); `toMatchInlineSnapshot()` for structural regression; `test.each()` for parameterized configs; `fast-check` for property-based testing.

3. **Visual regression (deferred)** — Vitest browser mode (Playwright, stable in v4.0 Oct 2025) + `toMatchScreenshot()`. Only invest when a layout algorithm is selected for production.

**E2E skipped entirely for POC.** CrabNebula fork supports macOS but requires paid subscription + `tauri-plugin-automation` in source.

**How to apply:** When setting up tests: configure Vitest with jsdom + `clearMocks()` in `afterEach`; add `tauri = { features = ["test"] }` in Cargo.toml; add `criterion` and `proptest` as dev-dependencies; use `rstest` for parameterized Rust tests.

Key constraint: any test calling `invoke()` in jsdom must call `mockIPC()` first or it will throw.
