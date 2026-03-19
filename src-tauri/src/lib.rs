mod commands;

use std::sync::Mutex;

/// Tauri managed state wrapping the mutable AnalysisHost.
///
/// Persists across commands so that `Analysis::snapshot()` returns
/// accumulated state (workspace info, scan results in M4+).
pub struct AppState {
    pub host: Mutex<codeatlas_core::AnalysisHost>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            host: Mutex::new(codeatlas_core::AnalysisHost::new()),
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_directory,
            commands::discover_workspace,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
