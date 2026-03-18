# Tauri Zoom Thing

A desktop application built with Tauri v2 featuring zoom/pan capabilities.

## What is Tauri?

Tauri is a framework for building lightweight, secure desktop applications with a Rust backend and web frontend. Think Electron but with a Rust core instead of Node.js — smaller binaries, lower memory usage, and better security.

**Key concepts:**
- **Frontend**: A standard web app (React, in our case) that runs in a native webview — not a bundled Chromium
- **Backend**: Rust code that handles system-level operations, exposed to the frontend via "commands" (like RPC calls)
- **IPC bridge**: The `@tauri-apps/api` package lets your TypeScript call Rust functions seamlessly

## Recommendations for Getting Started

### 1. Prerequisites
- **Rust** (stable): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Node.js 22+** and **pnpm**: `npm install -g pnpm`
- **macOS**: Xcode Command Line Tools (`xcode-select --install`)

### 2. Scaffold the Tauri project
```bash
pnpm create tauri-app --template react-ts
```
This generates the full project structure with Rust backend + React/TypeScript frontend pre-wired.

### 3. Project structure (after scaffolding)
```
src/              # React frontend (TypeScript)
src-tauri/        # Rust backend
  src/
    main.rs       # Tauri entry point
    lib.rs        # Your Rust commands live here
  Cargo.toml      # Rust dependencies
  tauri.conf.json # Tauri configuration
package.json      # Frontend dependencies
```

### 4. Key Tauri patterns to know

**Calling Rust from TypeScript:**
```rust
// src-tauri/src/lib.rs
#[tauri::command]
fn get_zoom_level() -> f64 {
    1.0
}
```
```typescript
// src/App.tsx
import { invoke } from '@tauri-apps/api/core';
const zoom = await invoke<number>('get_zoom_level');
```

**Tauri v2 permissions:** Tauri v2 uses a capability-based security model. You must declare which APIs your app can use in `src-tauri/capabilities/`. This is more work upfront but prevents accidental exposure.

### 5. Zoom feature considerations
- **CSS transforms** (`transform: scale()`) are the simplest approach for basic zoom on web content
- **Canvas-based zoom** (HTML Canvas or WebGL) gives you more control for custom rendering
- For a "Zoom-like" video conferencing feature, look into WebRTC + Tauri's window management APIs
- Tauri v2's `WebviewWindow` API allows multi-window setups if needed

### 6. Useful resources
- [Tauri v2 docs](https://v2.tauri.app)
- [Tauri v2 JavaScript API](https://v2.tauri.app/reference/javascript/)
- [Tauri v2 Rust API](https://docs.rs/tauri/latest/tauri/)

## Development
```bash
pnpm dev       # Start dev server with hot reload
pnpm build     # Production build
pnpm test      # Run frontend tests
cargo test     # Run Rust tests
```
