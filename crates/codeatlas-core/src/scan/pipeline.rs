//! Scan pipeline orchestration.
//!
//! Coordinates detector execution and streams results through `ScanSink`.

use tokio_util::sync::CancellationToken;

use crate::config::RepoConfig;
use crate::detector::{Detector, DetectorSink};
use crate::graph::types::{EdgeData, NodeData};
use crate::graph::ArchGraph;
use crate::health::compatibility::{CompatibilityDetail, CompatibilityReport};
use crate::health::graph_health::GraphHealth;
use crate::profile::GraphProfile;
use crate::workspace::WorkspaceInfo;

use super::{ScanPhase, ScanResults, ScanSink};

/// Errors that can occur during a scan pipeline run.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("scan cancelled")]
    Cancelled,
    #[error("detector error: {0}")]
    Detector(#[from] crate::detector::DetectorError),
    #[error("graph error: {0}")]
    Graph(#[from] crate::graph::GraphError),
}

/// Run the full scan pipeline.
///
/// Orchestrates detector execution, streams results through `sink`,
/// merges everything into an `ArchGraph`, and returns aggregate results.
///
/// The `cancel` token is checked between phases and can abort the scan.
pub fn run_scan(
    workspace: &WorkspaceInfo,
    profile: &GraphProfile,
    config: &RepoConfig,
    detectors: &[Box<dyn Detector>],
    sink: &dyn ScanSink,
    cancel: &CancellationToken,
) -> Result<ScanResults, ScanError> {
    let mut all_nodes: Vec<NodeData> = Vec::new();
    let mut all_edges: Vec<EdgeData> = Vec::new();
    let mut all_results = ScanResults {
        nodes: Vec::new(),
        edges: Vec::new(),
        unsupported_constructs: Vec::new(),
        parse_failures: Vec::new(),
    };

    // Phase-aware collecting sink that streams to the real sink in phases
    let phase_sink = PhaseSink::new();

    let applicable_detectors: Vec<_> = detectors
        .iter()
        .filter(|d| d.applies_to(workspace))
        .collect();

    let total = applicable_detectors.len();
    sink.on_progress(0, total);

    for (i, detector) in applicable_detectors.iter().enumerate() {
        if cancel.is_cancelled() {
            return Err(ScanError::Cancelled);
        }

        // Run the detector
        let report = detector.detect(workspace, profile, config, &phase_sink)?;

        all_results
            .unsupported_constructs
            .extend(report.unsupported_constructs);
        all_results.parse_failures.extend(report.parse_failures);

        sink.on_progress(i + 1, total);
    }

    // Collect all accumulated nodes and edges from the phase sink
    let (collected_nodes, collected_edges) = phase_sink.take();
    all_nodes.extend(collected_nodes);
    all_edges.extend(collected_edges);

    if cancel.is_cancelled() {
        return Err(ScanError::Cancelled);
    }

    // Stream phases to sink
    // Phase 1: Package topology
    let (pkg_nodes, pkg_edges): (Vec<_>, Vec<_>) = {
        let pkg_nodes: Vec<_> = all_nodes
            .iter()
            .filter(|n| n.kind == crate::graph::types::NodeKind::Package)
            .cloned()
            .collect();
        let pkg_node_keys: std::collections::HashSet<_> = pkg_nodes
            .iter()
            .map(|n| &n.materialized_key)
            .collect();
        let pkg_edges: Vec<_> = all_edges
            .iter()
            .filter(|e| {
                e.kind == crate::graph::types::EdgeKind::DependsOn
                    || (e.kind == crate::graph::types::EdgeKind::Contains
                        && pkg_node_keys.contains(&e.source_key)
                        && pkg_node_keys.contains(&e.target_key))
            })
            .cloned()
            .collect();
        (pkg_nodes, pkg_edges)
    };
    sink.on_phase(ScanPhase::PackageTopology, pkg_nodes, pkg_edges);

    if cancel.is_cancelled() {
        return Err(ScanError::Cancelled);
    }

    // Phase 2: Module structure (using HashSet for O(1) lookups)
    let (mod_nodes, mod_edges): (Vec<_>, Vec<_>) = {
        let mod_nodes: Vec<_> = all_nodes
            .iter()
            .filter(|n| n.kind == crate::graph::types::NodeKind::Module)
            .cloned()
            .collect();
        let mod_node_keys: std::collections::HashSet<_> = mod_nodes
            .iter()
            .map(|n| &n.materialized_key)
            .collect();
        let mod_edges: Vec<_> = all_edges
            .iter()
            .filter(|e| {
                e.kind == crate::graph::types::EdgeKind::Contains
                    && mod_node_keys.contains(&e.target_key)
            })
            .cloned()
            .collect();
        (mod_nodes, mod_edges)
    };
    sink.on_phase(ScanPhase::ModuleStructure, mod_nodes, mod_edges);

    if cancel.is_cancelled() {
        return Err(ScanError::Cancelled);
    }

    // Phase 3: File edges (using HashSet for O(1) lookups)
    let (file_nodes, file_edges): (Vec<_>, Vec<_>) = {
        let file_nodes: Vec<_> = all_nodes
            .iter()
            .filter(|n| n.kind == crate::graph::types::NodeKind::File)
            .cloned()
            .collect();
        let file_node_keys: std::collections::HashSet<_> = file_nodes
            .iter()
            .map(|n| &n.materialized_key)
            .collect();
        let file_edges: Vec<_> = all_edges
            .iter()
            .filter(|e| {
                e.kind == crate::graph::types::EdgeKind::Imports
                    || e.kind == crate::graph::types::EdgeKind::ReExports
                    || (e.kind == crate::graph::types::EdgeKind::Contains
                        && file_node_keys.contains(&e.target_key))
            })
            .cloned()
            .collect();
        (file_nodes, file_edges)
    };
    sink.on_phase(ScanPhase::FileEdges, file_nodes, file_edges);

    // Merge into ArchGraph
    let mut graph = ArchGraph::new();
    for node in &all_nodes {
        // Skip duplicates (can happen from overlapping detector output)
        if graph.contains_node(&node.materialized_key) {
            continue;
        }
        graph.add_node(node.clone())?;
    }
    for edge in &all_edges {
        if graph.contains_edge(&edge.edge_id) {
            continue;
        }
        // Skip edges where endpoints don't exist
        if !graph.contains_node(&edge.source_key) || !graph.contains_node(&edge.target_key) {
            continue;
        }
        graph.add_edge(edge.clone())?;
    }

    // Compute health metrics
    let health = GraphHealth {
        total_nodes: graph.node_count(),
        resolved_edges: graph.edge_count(),
        unresolved_imports: 0, // We don't create unresolved entries
        parse_failures: all_results.parse_failures.len(),
        unsupported_constructs: all_results.unsupported_constructs.len(),
    };
    sink.on_health(health.clone());

    // Enrich compatibility report
    let mut enriched_report = CompatibilityReport {
        assessments: detectors
            .iter()
            .filter(|d| d.applies_to(workspace))
            .map(|d| d.compatibility(workspace))
            .collect(),
        is_provisional: false,
    };
    // Add source-level findings to the report
    if !all_results.unsupported_constructs.is_empty() {
        for assessment in &mut enriched_report.assessments {
            let cfg_count = all_results
                .unsupported_constructs
                .iter()
                .filter(|c| {
                    c.construct_type == crate::graph::types::UnsupportedConstructType::CfgGate
                })
                .count();
            if cfg_count > 0 {
                assessment.details.push(CompatibilityDetail {
                    feature: "Source-level cfg gates".to_string(),
                    status: crate::health::compatibility::SupportStatus::Partial,
                    explanation: format!(
                        "{cfg_count} #[cfg(...)] gate(s) detected in source — modules included assuming default features"
                    ),
                });
            }
        }
    }
    sink.on_compatibility(enriched_report);

    all_results.nodes = all_nodes;
    all_results.edges = all_edges;

    Ok(all_results)
}

/// Internal sink that collects nodes and edges from detectors.
///
/// The detector emits via `DetectorSink::on_nodes` / `on_edges`.
/// This collects them and also forwards to the outer `ScanSink`.
struct PhaseSink {
    nodes: std::sync::Mutex<Vec<NodeData>>,
    edges: std::sync::Mutex<Vec<EdgeData>>,
}

impl PhaseSink {
    fn new() -> Self {
        Self {
            nodes: std::sync::Mutex::new(Vec::new()),
            edges: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn take(self) -> (Vec<NodeData>, Vec<EdgeData>) {
        let nodes = self.nodes.into_inner().unwrap_or_default();
        let edges = self.edges.into_inner().unwrap_or_default();
        (nodes, edges)
    }
}

impl DetectorSink for PhaseSink {
    fn on_nodes(&self, nodes: Vec<NodeData>) {
        self.nodes
            .lock()
            .expect("PhaseSink nodes mutex poisoned")
            .extend(nodes);
    }

    fn on_edges(&self, edges: Vec<EdgeData>) {
        self.edges
            .lock()
            .expect("PhaseSink edges mutex poisoned")
            .extend(edges);
    }
}

// Ensure PhaseSink is Send + Sync for the DetectorSink trait bound
fn _assert_phase_sink_send_sync(_: &PhaseSink) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detector::RustDetector;

    #[test]
    fn scan_pipeline_on_this_project() {
        let project_root = camino::Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("should find project root");

        let workspace =
            crate::workspace::discover_workspace(project_root).expect("discovery should succeed");
        let config = RepoConfig::load_from_dir(&workspace.root).unwrap_or_else(|_| RepoConfig::default_config());
        let profile = GraphProfile::detect_from_workspace(&workspace);
        let detectors: Vec<Box<dyn Detector>> = vec![Box::new(RustDetector)];
        let cancel = CancellationToken::new();

        let collecting_sink = CollectingScanSink::default();
        let results =
            run_scan(&workspace, &profile, &config, &detectors, &collecting_sink, &cancel)
                .expect("scan should succeed");

        // Should discover nodes
        assert!(!results.nodes.is_empty(), "should discover nodes");
        assert!(!results.edges.is_empty(), "should discover edges");

        // Should have streamed phases
        let phases = collecting_sink.phases();
        assert!(
            phases.len() >= 3,
            "should stream at least 3 phases, got {}",
            phases.len()
        );

        // Should have health
        let health = collecting_sink.health();
        assert!(health.is_some(), "should report health");
        let h = health.expect("health");
        assert!(h.total_nodes > 0);
        assert!(h.resolved_edges > 0);

        // Compatibility report should be non-provisional
        let compat = collecting_sink.compatibility();
        assert!(compat.is_some(), "should report compatibility");
        assert!(!compat.expect("compat").is_provisional);
    }

    #[test]
    fn scan_cancellation_works() {
        let project_root = camino::Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("should find project root");

        let workspace =
            crate::workspace::discover_workspace(project_root).expect("discovery should succeed");
        let config = RepoConfig::default_config();
        let profile = GraphProfile::empty();
        let detectors: Vec<Box<dyn Detector>> = vec![Box::new(RustDetector)];
        let cancel = CancellationToken::new();

        // Cancel immediately
        cancel.cancel();

        let result = run_scan(&workspace, &profile, &config, &detectors, &(), &cancel);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ScanError::Cancelled));
    }

    /// Collecting sink for testing scan pipeline output.
    #[derive(Default)]
    struct CollectingScanSink {
        phases: std::sync::Mutex<Vec<(ScanPhase, Vec<NodeData>, Vec<EdgeData>)>>,
        health: std::sync::Mutex<Option<GraphHealth>>,
        compatibility: std::sync::Mutex<Option<CompatibilityReport>>,
        progress: std::sync::Mutex<Vec<(usize, usize)>>,
    }

    impl CollectingScanSink {
        fn phases(&self) -> Vec<(ScanPhase, Vec<NodeData>, Vec<EdgeData>)> {
            self.phases.lock().expect("lock").clone()
        }
        fn health(&self) -> Option<GraphHealth> {
            self.health.lock().expect("lock").clone()
        }
        fn compatibility(&self) -> Option<CompatibilityReport> {
            self.compatibility.lock().expect("lock").clone()
        }
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
    }
}
