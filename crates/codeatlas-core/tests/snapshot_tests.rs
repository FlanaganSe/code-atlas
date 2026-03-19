//! Snapshot tests for golden corpus fixtures.
//!
//! Each test scans a fixture directory, builds a `GraphSummary`,
//! and snapshots it via `insta::assert_json_snapshot!`. If the graph
//! output changes, `cargo insta review` shows exactly what moved.

use std::sync::Mutex;

use codeatlas_core::config::RepoConfig;
use codeatlas_core::detector::{Detector, RustDetector, TypeScriptDetector};
use codeatlas_core::graph::types::{
    EdgeData, NodeData, NodeKind, ParseFailure, UnresolvedImport, UnsupportedConstruct,
};
use codeatlas_core::health::compatibility::CompatibilityReport;
use codeatlas_core::health::graph_health::GraphHealth;
use codeatlas_core::profile::GraphProfile;
use codeatlas_core::scan::{ScanPhase, ScanSink};

use camino::Utf8Path;
use insta::assert_json_snapshot;
use serde::Serialize;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// GraphSummary — the shape we snapshot
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct GraphSummary {
    node_count: usize,
    edge_count: usize,
    packages: Vec<String>,
    modules: usize,
    files: usize,
    unsupported_count: usize,
    unresolved_count: usize,
    parse_failure_count: usize,
}

// ---------------------------------------------------------------------------
// CollectingScanSink — duplicated from pipeline tests (integration boundary)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct CollectingScanSink {
    phases: Mutex<Vec<(ScanPhase, Vec<NodeData>, Vec<EdgeData>)>>,
    health: Mutex<Option<GraphHealth>>,
    compatibility: Mutex<Option<CompatibilityReport>>,
    progress: Mutex<Vec<(usize, usize)>>,
    details: Mutex<Option<(Vec<UnsupportedConstruct>, Vec<ParseFailure>, Vec<UnresolvedImport>)>>,
    overlay: Mutex<Option<(Vec<EdgeData>, Vec<String>)>>,
}

impl ScanSink for CollectingScanSink {
    fn on_compatibility(&self, report: CompatibilityReport) {
        *self.compatibility.lock().expect("lock") = Some(report);
    }
    fn on_phase(&self, phase: ScanPhase, nodes: Vec<NodeData>, edges: Vec<EdgeData>) {
        self.phases.lock().expect("lock").push((phase, nodes, edges));
    }
    fn on_health(&self, health: GraphHealth) {
        *self.health.lock().expect("lock") = Some(health);
    }
    fn on_progress(&self, scanned: usize, total: usize) {
        self.progress.lock().expect("lock").push((scanned, total));
    }
    fn on_details(
        &self,
        unsupported_constructs: Vec<UnsupportedConstruct>,
        parse_failures: Vec<ParseFailure>,
        unresolved_imports: Vec<UnresolvedImport>,
    ) {
        *self.details.lock().expect("lock") =
            Some((unsupported_constructs, parse_failures, unresolved_imports));
    }
    fn on_overlay(&self, manual_edges: Vec<EdgeData>, suppressed_edge_ids: Vec<String>) {
        *self.overlay.lock().expect("lock") = Some((manual_edges, suppressed_edge_ids));
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fixture_path(name: &str) -> camino::Utf8PathBuf {
    let manifest_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("project root")
        .join("tests/fixtures")
        .join(name)
}

fn scan_fixture(
    fixture_name: &str,
    detectors: Vec<Box<dyn Detector>>,
) -> (codeatlas_core::scan::ScanResults, CollectingScanSink) {
    let dir = fixture_path(fixture_name);
    assert!(dir.exists(), "fixture directory not found: {dir}");

    let workspace =
        codeatlas_core::workspace::discover_workspace(&dir).expect("discovery should succeed");
    let config = RepoConfig::load_from_dir(&workspace.root)
        .unwrap_or_else(|_| RepoConfig::default_config());
    let profile = GraphProfile::detect_from_workspace(&workspace);
    let cancel = CancellationToken::new();
    let sink = CollectingScanSink::default();

    let results =
        codeatlas_core::scan::run_scan(&workspace, &profile, &config, &detectors, &sink, &cancel)
            .expect("scan should succeed");

    (results, sink)
}

fn build_summary(
    results: &codeatlas_core::scan::ScanResults,
    sink: &CollectingScanSink,
) -> GraphSummary {
    let mut packages: Vec<String> = results
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Package)
        .map(|n| n.label.clone())
        .collect();
    packages.sort();

    let modules = results
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Module)
        .count();

    let files = results
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::File)
        .count();

    let (unsupported_count, parse_failure_count, unresolved_count) = sink
        .details
        .lock()
        .expect("lock")
        .as_ref()
        .map(|(u, p, r)| (u.len(), p.len(), r.len()))
        .unwrap_or((0, 0, 0));

    GraphSummary {
        node_count: results.nodes.len(),
        edge_count: results.edges.len(),
        packages,
        modules,
        files,
        unsupported_count,
        unresolved_count,
        parse_failure_count,
    }
}

// ---------------------------------------------------------------------------
// Snapshot tests
// ---------------------------------------------------------------------------

#[test]
fn snapshot_rust_workspace_graph() {
    let dir = fixture_path("rust-workspace");
    if !dir.exists() {
        eprintln!("skipping: fixture not found at {dir}");
        return;
    }

    let (results, sink) = scan_fixture("rust-workspace", vec![Box::new(RustDetector)]);
    let summary = build_summary(&results, &sink);
    assert_json_snapshot!("rust_workspace_graph", summary);
}

#[test]
fn snapshot_ts_monorepo_graph() {
    let dir = fixture_path("ts-monorepo");
    if !dir.exists() {
        eprintln!("skipping: fixture not found at {dir}");
        return;
    }

    let (results, sink) = scan_fixture("ts-monorepo", vec![Box::new(TypeScriptDetector)]);
    let summary = build_summary(&results, &sink);
    assert_json_snapshot!("ts_monorepo_graph", summary);
}

#[test]
fn snapshot_rust_unsupported_graph() {
    let dir = fixture_path("rust-unsupported");
    if !dir.exists() {
        eprintln!("skipping: fixture not found at {dir}");
        return;
    }

    let (results, sink) = scan_fixture("rust-unsupported", vec![Box::new(RustDetector)]);
    let summary = build_summary(&results, &sink);
    assert_json_snapshot!("rust_unsupported_graph", summary);
}

#[test]
fn snapshot_ts_unsupported_graph() {
    let dir = fixture_path("ts-unsupported");
    if !dir.exists() {
        eprintln!("skipping: fixture not found at {dir}");
        return;
    }

    let (results, sink) = scan_fixture("ts-unsupported", vec![Box::new(TypeScriptDetector)]);
    let summary = build_summary(&results, &sink);
    assert_json_snapshot!("ts_unsupported_graph", summary);
}
