//! Scan orchestration and domain result types.
//!
//! The scan module coordinates detectors and collects results.
//! It exports domain-level types (`ScanResults`, `ScanPhase`) and the
//! `ScanSink` trait. The `ScanEvent` transport envelope belongs in
//! `codeatlas-tauri`, not here — this preserves the core/shell boundary.

pub mod pipeline;

pub use pipeline::{run_scan, ScanError};

use serde::{Deserialize, Serialize};

use crate::graph::types::{EdgeData, NodeData, ParseFailure, UnresolvedImport, UnsupportedConstruct};
use crate::health::compatibility::CompatibilityReport;
use crate::health::graph_health::GraphHealth;

/// Phases of a scan, delivered progressively.
///
/// The frontend renders each phase as it arrives, giving users
/// package topology first, then module structure, then file-level edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanPhase {
    /// Top-level packages/crates discovered.
    PackageTopology,
    /// Modules within packages discovered.
    ModuleStructure,
    /// File-level edges (imports, re-exports) resolved.
    FileEdges,
}

/// Aggregate results from a complete scan.
///
/// This is the domain-level result type. It contains everything
/// the scan discovered, independent of how it was streamed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResults {
    pub nodes: Vec<NodeData>,
    pub edges: Vec<EdgeData>,
    pub unsupported_constructs: Vec<UnsupportedConstruct>,
    pub parse_failures: Vec<ParseFailure>,
    pub unresolved_imports: Vec<UnresolvedImport>,
}

/// Domain-level sink for streaming scan output.
///
/// This is the interface between the scan pipeline and consumers.
/// The Tauri shell implements this to bridge to `Channel<ScanEvent>`.
/// Test harnesses can implement this for verification.
///
/// **This trait has zero transport concepts** — no JSON, no channels,
/// no IPC. It uses domain terminology only.
pub trait ScanSink: Send + Sync {
    /// Report compatibility assessment.
    fn on_compatibility(&self, report: CompatibilityReport);

    /// Report a batch of discovered nodes and edges for a scan phase.
    fn on_phase(&self, phase: ScanPhase, nodes: Vec<NodeData>, edges: Vec<EdgeData>);

    /// Report graph health metrics.
    fn on_health(&self, health: GraphHealth);

    /// Report scan progress.
    fn on_progress(&self, scanned: usize, total: usize);

    /// Report detailed scan findings: unsupported constructs, parse failures, and unresolved imports.
    fn on_details(
        &self,
        unsupported_constructs: Vec<UnsupportedConstruct>,
        parse_failures: Vec<ParseFailure>,
        unresolved_imports: Vec<UnresolvedImport>,
    );

    /// Report overlay data: manual edges from config and suppressed edge IDs.
    fn on_overlay(&self, manual_edges: Vec<EdgeData>, suppressed_edge_ids: Vec<String>);
}

// No-op sink for testing.
impl ScanSink for () {
    fn on_compatibility(&self, _report: CompatibilityReport) {}
    fn on_phase(&self, _phase: ScanPhase, _nodes: Vec<NodeData>, _edges: Vec<EdgeData>) {}
    fn on_health(&self, _health: GraphHealth) {}
    fn on_progress(&self, _scanned: usize, _total: usize) {}
    fn on_details(
        &self,
        _unsupported_constructs: Vec<UnsupportedConstruct>,
        _parse_failures: Vec<ParseFailure>,
        _unresolved_imports: Vec<UnresolvedImport>,
    ) {
    }
    fn on_overlay(&self, _manual_edges: Vec<EdgeData>, _suppressed_edge_ids: Vec<String>) {}
}
