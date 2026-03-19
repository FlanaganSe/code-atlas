//! Tauri commands — thin wrappers over `codeatlas-core`.
//!
//! All analysis logic lives in `codeatlas-core`. These commands
//! provide the IPC bridge: open file dialog, invoke discovery,
//! start/cancel scans, and return serialized results.

use codeatlas_core::{
    CompatibilityReport, DiscoveryResult, GraphHealth, ScanPhase, ScanSink,
};
use codeatlas_core::graph::types::{EdgeData, NodeData, ParseFailure, UnsupportedConstruct};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use tokio_util::sync::CancellationToken;

use crate::AppState;

// ---------------------------------------------------------------------------
// ScanEvent — transport envelope (lives in tauri shell, NOT in core)
// ---------------------------------------------------------------------------

/// Events streamed to the frontend during a scan via `Channel<ScanEvent>`.
#[derive(Clone, Serialize)]
#[serde(tag = "event", content = "data", rename_all = "camelCase")]
pub enum ScanEvent {
    CompatibilityReport {
        scan_id: String,
        report: CompatibilityReport,
    },
    Phase {
        scan_id: String,
        phase: ScanPhase,
        nodes: Vec<NodeData>,
        edges: Vec<EdgeData>,
    },
    Health {
        scan_id: String,
        health: GraphHealth,
    },
    Progress {
        scan_id: String,
        scanned: usize,
        total: usize,
    },
    /// Detailed scan findings: unsupported constructs and parse failures.
    Details {
        scan_id: String,
        unsupported_constructs: Vec<UnsupportedConstruct>,
        parse_failures: Vec<ParseFailure>,
    },
    /// Overlay data: manual edges from config and suppressed edge IDs.
    Overlay {
        scan_id: String,
        manual_edges: Vec<EdgeData>,
        suppressed_edge_ids: Vec<String>,
    },
    Complete {
        scan_id: String,
    },
    Error {
        scan_id: String,
        message: String,
    },
}

// ---------------------------------------------------------------------------
// ChannelSink — adapts ScanSink to Channel<ScanEvent>
// ---------------------------------------------------------------------------

/// Adapts the domain-level `ScanSink` trait to Tauri's `Channel<ScanEvent>`.
struct ChannelSink {
    scan_id: String,
    channel: Channel<ScanEvent>,
}

impl ScanSink for ChannelSink {
    fn on_compatibility(&self, report: CompatibilityReport) {
        let _ = self.channel.send(ScanEvent::CompatibilityReport {
            scan_id: self.scan_id.clone(),
            report,
        });
    }

    fn on_phase(&self, phase: ScanPhase, nodes: Vec<NodeData>, edges: Vec<EdgeData>) {
        let _ = self.channel.send(ScanEvent::Phase {
            scan_id: self.scan_id.clone(),
            phase,
            nodes,
            edges,
        });
    }

    fn on_health(&self, health: GraphHealth) {
        let _ = self.channel.send(ScanEvent::Health {
            scan_id: self.scan_id.clone(),
            health,
        });
    }

    fn on_progress(&self, scanned: usize, total: usize) {
        let _ = self.channel.send(ScanEvent::Progress {
            scan_id: self.scan_id.clone(),
            scanned,
            total,
        });
    }

    fn on_details(
        &self,
        unsupported_constructs: Vec<UnsupportedConstruct>,
        parse_failures: Vec<ParseFailure>,
    ) {
        let _ = self.channel.send(ScanEvent::Details {
            scan_id: self.scan_id.clone(),
            unsupported_constructs,
            parse_failures,
        });
    }

    fn on_overlay(&self, manual_edges: Vec<EdgeData>, suppressed_edge_ids: Vec<String>) {
        let _ = self.channel.send(ScanEvent::Overlay {
            scan_id: self.scan_id.clone(),
            manual_edges,
            suppressed_edge_ids,
        });
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Open a native directory picker dialog.
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
#[tauri::command]
pub async fn discover_workspace(
    path: String,
    state: State<'_, AppState>,
) -> Result<DiscoveryResult, String> {
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

    let mut guard = state
        .host
        .lock()
        .map_err(|e| format!("lock error: {e}"))?;
    *guard = updated_host;

    Ok(result)
}

/// Start a scan of the workspace at the given path.
///
/// Results stream to the frontend via `on_event` channel.
#[tauri::command]
pub async fn start_scan(
    scan_id: String,
    on_event: Channel<ScanEvent>,
    state: State<'_, AppState>,
) -> Result<(), String> {

    // Cancel any existing scan
    {
        let mut token_guard = state
            .cancel_token
            .lock()
            .map_err(|e| format!("lock error: {e}"))?;
        if let Some(old_token) = token_guard.take() {
            old_token.cancel();
        }
    }

    // Create new cancellation token
    let cancel = CancellationToken::new();
    {
        let mut token_guard = state
            .cancel_token
            .lock()
            .map_err(|e| format!("lock error: {e}"))?;
        *token_guard = Some(cancel.clone());
    }

    // Clone state needed for the scan
    let host = state
        .host
        .lock()
        .map_err(|e| format!("lock error: {e}"))?
        .clone();

    let workspace = host
        .workspace()
        .cloned()
        .ok_or_else(|| "no workspace discovered — run discover_workspace first".to_string())?;
    let config = host.config().clone();
    let profile = host.profile().clone();

    let scan_id_clone = scan_id.clone();
    let on_event_clone = on_event.clone();

    // Run scan in blocking thread
    let scan_result = tokio::task::spawn_blocking(move || {
        let detectors: Vec<Box<dyn codeatlas_core::detector::Detector>> = vec![
            Box::new(codeatlas_core::RustDetectorType),
            Box::new(codeatlas_core::TypeScriptDetectorType),
        ];

        let sink = ChannelSink {
            scan_id: scan_id_clone,
            channel: on_event_clone,
        };

        codeatlas_core::run_scan(&workspace, &profile, &config, &detectors, &sink, &cancel)
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?;

    match scan_result {
        Ok(results) => {
            // Apply results back to host
            let mut guard = state
                .host
                .lock()
                .map_err(|e| format!("lock error: {e}"))?;
            guard
                .apply_scan_results(&results)
                .map_err(|e| format!("apply results error: {e}"))?;

            let _ = on_event.send(ScanEvent::Complete {
                scan_id: scan_id.clone(),
            });
            Ok(())
        }
        Err(codeatlas_core::ScanError::Cancelled) => {
            let _ = on_event.send(ScanEvent::Error {
                scan_id: scan_id.clone(),
                message: "scan cancelled".to_string(),
            });
            Ok(())
        }
        Err(e) => {
            let _ = on_event.send(ScanEvent::Error {
                scan_id: scan_id.clone(),
                message: e.to_string(),
            });
            Err(e.to_string())
        }
    }
}

/// Cancel an in-progress scan.
#[tauri::command]
pub async fn cancel_scan(state: State<'_, AppState>) -> Result<(), String> {
    let mut token_guard = state
        .cancel_token
        .lock()
        .map_err(|e| format!("lock error: {e}"))?;
    if let Some(token) = token_guard.take() {
        token.cancel();
    }
    Ok(())
}
