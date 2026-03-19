//! Tauri commands — thin wrappers over `codeatlas-core`.
//!
//! All analysis logic lives in `codeatlas-core`. These commands
//! provide the IPC bridge: open file dialog, invoke discovery,
//! and return serialized results.

use codeatlas_core::DiscoveryResult;
use tauri_plugin_dialog::DialogExt;

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
/// and can take 2-10s on first run.
#[tauri::command]
pub async fn discover_workspace(path: String) -> Result<DiscoveryResult, String> {
    let result = tokio::task::spawn_blocking(move || {
        let dir = camino::Utf8Path::new(&path);
        let mut host = codeatlas_core::AnalysisHost::new();
        host.discover_workspace(dir)
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
    .map_err(|e| format!("discovery error: {e}"))?;

    Ok(result)
}
