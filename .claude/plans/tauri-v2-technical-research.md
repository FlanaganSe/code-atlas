# Tauri v2 Technical Research

Research date: 2026-03-18

## 1. Tauri v2 with Cargo Workspace

### Virtual workspace Cargo.toml (project root)
```toml
[workspace]
members = ["codeatlas-core", "codeatlas-tauri"]
resolver = "2"
```

### codeatlas-core/Cargo.toml
```toml
[package]
name = "codeatlas-core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
```

### codeatlas-tauri/Cargo.toml (the Tauri app, replaces src-tauri)
```toml
[package]
name = "codeatlas-tauri"
version = "0.1.0"
edition = "2021"

[dependencies]
codeatlas-core = { path = "../codeatlas-core" }
tauri = { version = "2.10", features = [] }
tauri-build = { version = "2.0", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[build-dependencies]
tauri-build = { version = "2.0", features = [] }
```

### Key facts
- tauri.conf.json must be next to the Tauri app's Cargo.toml
- `tauri dev` watches the Tauri crate AND its workspace dependencies for hot reload
- Target dir is at workspace root when using a workspace
- The Tauri app folder does NOT need to be named `src-tauri`; you configure it via `tauri.conf.json` location

### tauri.conf.json awareness
When using a non-standard folder name, run tauri CLI from the Tauri app dir or use `--config` flag.

## 2. tauri-specta v2

### Status: RELEASE CANDIDATE (NOT STABLE)
- Latest version: **2.0.0-rc.21** (released January 13, 2025)
- No stable 2.0.0 exists
- All 2.x releases are pre-release

### Exact Cargo.toml dependencies
```toml
tauri = "2.0"
specta = "=2.0.0-rc.21"
specta-typescript = "0.0.9"
tauri-specta = { version = "=2.0.0-rc.21", features = ["derive", "typescript"] }
```
IMPORTANT: Use `=` prefix to pin exact RC versions.

### Channel<T> support: NOT YET AVAILABLE
- docs.rs page says "Coming soon..." for Channel support
- Tracked at GitHub issue (referenced as #111)
- No workaround exists within tauri-specta itself

### Known issues
- rc.21 was broken due to Tauri upstream changes; requires patch from tauri-apps/tauri#12371
- specta_util resolution failures reported with Tauri v2.2.0+

### Setup code
```rust
#[tauri::command]
#[specta::specta]
fn hello_world(my_name: String) -> String {
    format!("Hello, {my_name}!")
}

let mut builder = tauri_specta::Builder::<tauri::Wry>::new()
    .commands(tauri_specta::collect_commands![hello_world,]);

#[cfg(debug_assertions)]
builder
    .export(specta_typescript::Typescript::default(), "../src/bindings.ts")
    .expect("Failed to export typescript bindings");

tauri::Builder::default()
    .invoke_handler(builder.invoke_handler())
    .setup(move |app| {
        builder.mount_events(app);
        Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
```

## 3. Tauri v2 Channel<T>

### Rust side
```rust
use tauri::ipc::Channel;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
enum DownloadEvent<'a> {
    Started {
        url: &'a str,
        download_id: usize,
        content_length: usize,
    },
    Progress {
        download_id: usize,
        chunk_length: usize,
    },
    Finished {
        download_id: usize,
    },
}

#[tauri::command]
fn download(app: tauri::AppHandle, url: String, on_event: Channel<DownloadEvent>) {
    on_event.send(DownloadEvent::Started {
        url: &url,
        download_id: 1,
        content_length: 1000,
    }).unwrap();

    on_event.send(DownloadEvent::Finished { download_id: 1 }).unwrap();
}
```

### TypeScript side
```typescript
import { invoke, Channel } from '@tauri-apps/api/core';

type DownloadEvent =
  | { event: 'started'; data: { url: string; downloadId: number; contentLength: number } }
  | { event: 'progress'; data: { downloadId: number; chunkLength: number } }
  | { event: 'finished'; data: { downloadId: number } };

const onEvent = new Channel<DownloadEvent>();
onEvent.onmessage = (message) => {
  console.log(`got download event ${message.event}`);
};

await invoke('download', {
  url: 'https://example.com/file',
  onEvent,
});
```

### Key facts
- Channel is in `tauri::ipc::Channel<T>`
- T must implement `IpcResponse` (which Serialize satisfies)
- Channel is a command argument -- the frontend creates it and passes it in
- `send()` signature: `pub fn send(&self, data: TSend) -> Result<()> where TSend: IpcResponse`
- Channels are ordered and fast; preferred over events for streaming
- Known memory leak issue reported (GitHub #13133) when listening to channel events

## 4. Tauri Plugin Installation

### plugin-dialog

**Rust (Cargo.toml):**
```toml
tauri-plugin-dialog = "2.6"   # latest: 2.6.0 on crates.io
```
Or: `cargo add tauri-plugin-dialog`

**Frontend (package.json):**
```
pnpm add @tauri-apps/plugin-dialog    # latest: 2.6.0 on npm
```

**Rust registration:**
```rust
tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
```

**Default permissions (no config needed):**
allow-ask, allow-confirm, allow-message, allow-save, allow-open

### plugin-shell

**Rust (Cargo.toml):**
```toml
tauri-plugin-shell = "2.3"    # latest: 2.3.4 on crates.io (docs.rs shows 2.3.4)
```
Or: `cargo add tauri-plugin-shell`

**Frontend (package.json):**
```
pnpm add @tauri-apps/plugin-shell     # latest: 2.3.5 on npm
```

**Rust registration:**
```rust
tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
```

**Capabilities config (src-tauri/capabilities/default.json):**
```json
{
  "identifier": "main-capability",
  "permissions": [
    "shell:allow-open",
    {
      "identifier": "shell:allow-execute",
      "allow": [{ "name": "exec-sh", "cmd": "sh" }]
    }
  ]
}
```
Default permission only includes allow-open for http(s)://, tel:, mailto: links.

## 5. Tauri v2 Scaffolding

### Command
```bash
pnpm create tauri-app
```
create-tauri-app latest version: **4.6.2**

### Interactive prompts
1. Project name (default: tauri-app)
2. Identifier (default: com.tauri-app.app)
3. Frontend language: Rust / TypeScript-JavaScript / .NET
4. Package manager: pnpm, yarn, npm, bun
5. UI Template: Vanilla, Vue, Svelte, React, Solid, Angular, Preact
6. UI flavor: TypeScript or JavaScript

### One-liner (non-interactive)
```bash
pnpm create tauri-app my-app --template react-ts --manager pnpm
```

### Resulting structure
```
my-app/
  package.json
  index.html
  src/
    main.tsx
  src-tauri/
    Cargo.toml
    Cargo.lock
    build.rs
    tauri.conf.json
    src/
      main.rs
      lib.rs
    icons/
    capabilities/
      default.json
```

### After scaffolding
```bash
cd my-app
pnpm install
pnpm tauri dev
```

### Current version matrix (as of 2026-03-18)
| Package | Version |
|---------|---------|
| tauri (crate) | 2.10.3 |
| tauri-build (crate) | 2.x (match tauri) |
| @tauri-apps/api (npm) | 2.10.1 |
| @tauri-apps/cli (npm) | 2.10.1 |
| create-tauri-app (npm) | 4.6.2 |
| tauri-plugin-dialog (crate) | 2.6.0 |
| @tauri-apps/plugin-dialog (npm) | 2.6.0 |
| tauri-plugin-shell (crate) | 2.3.4 |
| @tauri-apps/plugin-shell (npm) | 2.3.5 |
| tauri-specta (crate) | 2.0.0-rc.21 |
| specta (crate) | 2.0.0-rc.21 |
| specta-typescript (crate) | 0.0.9 |
