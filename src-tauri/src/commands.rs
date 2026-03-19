//! Tauri commands — thin wrappers over `codeatlas-core`.
//!
//! All analysis logic lives in `codeatlas-core`. These commands
//! provide the IPC bridge: open file dialog, invoke discovery,
//! and return serialized results.

use codeatlas_core::DiscoveryResult;
use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::AppState;

/// Open a native directory picker dialog.
///
/// Returns the selected directory path, or an error if the user
/// cancels or the dialog fails.
#[tauri::command]
pub async fn open_directory(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    app.dialog()
        .file()
        .pick_folder(move |folder| {
            let path = folder.map(|f| f.to_string());
            let _ = tx.send(path);
        });

    rx.await.map_err(|e| format!("dialog error: {e}"))
}

/// Discover workspace structure at the given directory path.
///
/// Runs workspace discovery (cargo_metadata, JS workspace detection),
/// loads `.codeatlas.yaml`, detects the graph profile, and runs
/// detector compatibility assessments.
///
/// Uses `spawn_blocking` because `cargo_metadata` is synchronous
/// and can take 2-10s on first run. The AnalysisHost is persisted
/// as Tauri managed state so snapshot() carries accumulated state.
#[tauri::command]
pub async fn discover_workspace(
    path: String,
    state: State<'_, AppState>,
) -> Result<DiscoveryResult, String> {
    // Clone the host out of managed state so we can move it into spawn_blocking.
    // NOTE: This has a TOCTOU race if multiple discover_workspace calls run
    // concurrently — the last to finish wins. Acceptable for POC since there's
    // only one UI button. M4 should switch to tokio::sync::Mutex.
    let mut host = state
        .host
        .lock()
        .map_err(|e| format!("lock error: {e}"))?
        .clone();

    let (result, updated_host) = tokio::task::spawn_blocking(move || {
        let dir = camino::Utf8Path::new(&path);
        let res = host.discover_workspace(dir);
        (res, host)
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?;

    let result = result.map_err(|e| format!("discovery error: {e}"))?;

    // Write the updated host back to managed state
    let mut guard = state
        .host
        .lock()
        .map_err(|e| format!("lock error: {e}"))?;
    *guard = updated_host;

    Ok(result)
}
