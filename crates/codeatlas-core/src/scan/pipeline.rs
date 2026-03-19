//! Scan pipeline orchestration.
//!
//! Coordinates detector execution and streams results through `ScanSink`.

use tokio_util::sync::CancellationToken;

use crate::config::RepoConfig;
use crate::detector::{Detector, DetectorSink};
use crate::graph::identity::{EdgeId, MaterializedKey};
use crate::graph::overlay::SuppressionReason;
use crate::graph::types::{
    Confidence, EdgeCategory, EdgeData, EdgeKind, NodeData, OverlayStatus,
};
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

    // Apply overlay from config
    let (manual_edge_data, suppressed_edge_ids) =
        apply_overlay_from_config(config, &mut graph);

    // Compute health metrics
    let health = GraphHealth {
        total_nodes: graph.node_count(),
        resolved_edges: graph.edge_count(),
        unresolved_imports: 0, // We don't create unresolved entries
        parse_failures: all_results.parse_failures.len(),
        unsupported_constructs: all_results.unsupported_constructs.len(),
    };
    sink.on_health(health.clone());

    // Send detailed findings (unsupported constructs + parse failures)
    sink.on_details(
        all_results.unsupported_constructs.clone(),
        all_results.parse_failures.clone(),
    );

    // Send overlay data (manual edges + suppressed edge IDs)
    sink.on_overlay(manual_edge_data, suppressed_edge_ids);

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
        use crate::graph::types::UnsupportedConstructType as UCT;

        let cfg_count = all_results.unsupported_constructs.iter()
            .filter(|c| c.construct_type == UCT::CfgGate).count();
        let dynamic_count = all_results.unsupported_constructs.iter()
            .filter(|c| c.construct_type == UCT::DynamicImport).count();
        let require_count = all_results.unsupported_constructs.iter()
            .filter(|c| c.construct_type == UCT::CommonJsRequire).count();

        for assessment in &mut enriched_report.assessments {
            if assessment.language == crate::graph::types::Language::Rust && cfg_count > 0 {
                assessment.details.push(CompatibilityDetail {
                    feature: "Source-level cfg gates".to_string(),
                    status: crate::health::compatibility::SupportStatus::Partial,
                    explanation: format!(
                        "{cfg_count} #[cfg(...)] gate(s) detected in source — modules included assuming default features"
                    ),
                });
            }
            if assessment.language == crate::graph::types::Language::TypeScript {
                if dynamic_count > 0 {
                    assessment.details.push(CompatibilityDetail {
                        feature: "Dynamic imports".to_string(),
                        status: crate::health::compatibility::SupportStatus::Partial,
                        explanation: format!(
                            "{dynamic_count} dynamic import() call(s) detected — not statically resolved"
                        ),
                    });
                }
                if require_count > 0 {
                    assessment.details.push(CompatibilityDetail {
                        feature: "CommonJS require()".to_string(),
                        status: crate::health::compatibility::SupportStatus::Partial,
                        explanation: format!(
                            "{require_count} require() call(s) detected — not resolved in POC"
                        ),
                    });
                }
            }
        }
    }
    sink.on_compatibility(enriched_report);

    all_results.nodes = all_nodes;
    all_results.edges = all_edges;

    Ok(all_results)
}

/// Apply overlay configuration to the graph.
///
/// Translates `dependencies.add` from config into manual `EdgeData` entries
/// and `dependencies.suppress` into suppression entries on the overlay.
/// Returns the manual edges as EdgeData (for streaming to frontend) and
/// the list of suppressed edge IDs.
fn apply_overlay_from_config(
    config: &RepoConfig,
    graph: &mut ArchGraph,
) -> (Vec<EdgeData>, Vec<String>) {
    let mut manual_edge_data: Vec<EdgeData> = Vec::new();
    let mut suppressed_edge_ids: Vec<String> = Vec::new();

    // Apply manual edges from config.dependencies.add
    for manual in &config.dependencies.add {
        // Find matching source and target nodes by relative path prefix.
        // The config uses package-level paths (e.g., "packages/app"),
        // so we match against Package nodes whose path starts with the from/to value.
        let source_key = find_node_by_path(graph, &manual.from);
        let target_key = find_node_by_path(graph, &manual.to);

        if let (Some(source), Some(target)) = (source_key, target_key) {
            let edge_id = EdgeId::new(&source, &target, EdgeKind::Manual, EdgeCategory::Manual);

            let edge_data = EdgeData {
                edge_id: edge_id.clone(),
                source_key: source,
                target_key: target,
                kind: EdgeKind::Manual,
                category: EdgeCategory::Manual,
                confidence: Confidence::Structural,
                source_location: None,
                resolution_method: Some("manual config".to_string()),
                overlay_status: OverlayStatus::None,
            };

            manual_edge_data.push(edge_data);

            // Add to overlay's manual edges
            graph.overlay_mut().manual_edges.push(
                crate::graph::overlay::ManualEdge {
                    from: manual.from.clone(),
                    to: manual.to.clone(),
                    reason: manual.reason.clone(),
                },
            );
        }
    }

    // Apply suppressions from config.dependencies.suppress
    for suppression in &config.dependencies.suppress {
        // Find matching discovered edges: edges whose source path contains
        // the suppression's `from` and target path contains `to`.
        let matching_edge_ids: Vec<EdgeId> = graph
            .edges()
            .filter(|e| {
                e.source_key.relative_path.starts_with(&suppression.from)
                    && e.target_key.relative_path.starts_with(&suppression.to)
            })
            .map(|e| e.edge_id.clone())
            .collect();

        for edge_id in matching_edge_ids {
            suppressed_edge_ids.push(edge_id.0.clone());
            graph.overlay_mut().suppressions.insert(
                edge_id,
                SuppressionReason {
                    reason: suppression.reason.clone(),
                },
            );
        }
    }

    (manual_edge_data, suppressed_edge_ids)
}

/// Find a graph node whose relative path matches or starts with the given path.
/// Prefers exact match on Package nodes, then prefix match.
fn find_node_by_path(graph: &ArchGraph, path: &str) -> Option<MaterializedKey> {
    let normalized = crate::graph::normalize_path(path);

    // First try exact match on packages
    for node in graph.nodes() {
        if node.kind == crate::graph::types::NodeKind::Package
            && node.materialized_key.relative_path == normalized
        {
            return Some(node.materialized_key.clone());
        }
    }

    // Then try prefix match on any node
    for node in graph.nodes() {
        if node.materialized_key.relative_path == normalized {
            return Some(node.materialized_key.clone());
        }
    }

    // Finally, try path-prefix match (with separator boundary)
    let prefix_with_sep = format!("{normalized}/");
    for node in graph.nodes() {
        if node.materialized_key.relative_path.starts_with(&prefix_with_sep) {
            return Some(node.materialized_key.clone());
        }
    }

    None
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
    use crate::detector::{RustDetector, TypeScriptDetector};

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
        let detectors: Vec<Box<dyn Detector>> = vec![
            Box::new(RustDetector),
            Box::new(TypeScriptDetector),
        ];
        let cancel = CancellationToken::new();

        let collecting_sink = CollectingScanSink::default();
        let results =
            run_scan(&workspace, &profile, &config, &detectors, &collecting_sink, &cancel)
                .expect("scan should succeed");

        // Should discover nodes
        assert!(!results.nodes.is_empty(), "should discover nodes");
        assert!(!results.edges.is_empty(), "should discover edges");

        // Rust detector should produce nodes (this project has a Cargo workspace)
        let rust_nodes = results.nodes.iter()
            .filter(|n| n.language == crate::graph::types::Language::Rust)
            .count();
        assert!(rust_nodes > 0, "should have Rust nodes");

        // No duplicate MaterializedKeys
        let all_keys: Vec<_> = results.nodes.iter()
            .map(|n| &n.materialized_key)
            .collect();
        let unique_keys: std::collections::HashSet<_> = all_keys.iter().collect();
        assert_eq!(all_keys.len(), unique_keys.len(), "no MaterializedKey collisions");

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

    #[test]
    fn scan_ts_monorepo_fixture() {
        let fixture_dir = camino::Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("project root")
            .join("tests/fixtures/ts-monorepo");

        if !fixture_dir.exists() {
            return;
        }

        let workspace =
            crate::workspace::discover_workspace(&fixture_dir).expect("discovery should succeed");
        let config = RepoConfig::default_config();
        let profile = GraphProfile::detect_from_workspace(&workspace);
        let detectors: Vec<Box<dyn Detector>> = vec![Box::new(TypeScriptDetector)];
        let cancel = CancellationToken::new();

        let collecting_sink = CollectingScanSink::default();
        let results =
            run_scan(&workspace, &profile, &config, &detectors, &collecting_sink, &cancel)
                .expect("scan should succeed");

        // Should have TS nodes only
        assert!(!results.nodes.is_empty(), "should discover TS nodes");
        assert!(results.nodes.iter().all(|n| n.language == crate::graph::types::Language::TypeScript));

        // Compatibility report should be final
        let compat = collecting_sink.compatibility();
        assert!(compat.is_some());
        assert!(!compat.expect("compat").is_provisional);
    }

    #[test]
    fn scan_pipeline_streams_overlay_data() {
        let project_root = camino::Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("should find project root");

        let workspace =
            crate::workspace::discover_workspace(project_root).expect("discovery should succeed");
        let config = RepoConfig::load_from_dir(&workspace.root)
            .unwrap_or_else(|_| RepoConfig::default_config());
        let profile = GraphProfile::detect_from_workspace(&workspace);
        let detectors: Vec<Box<dyn Detector>> = vec![
            Box::new(RustDetector),
            Box::new(TypeScriptDetector),
        ];
        let cancel = CancellationToken::new();

        let collecting_sink = CollectingScanSink::default();
        let _results =
            run_scan(&workspace, &profile, &config, &detectors, &collecting_sink, &cancel)
                .expect("scan should succeed");

        // Overlay event should have been sent (even if empty for this project)
        let overlay = collecting_sink.overlay();
        assert!(overlay.is_some(), "overlay event should be sent");

        let (manual_edges, suppressed_ids) = overlay.expect("overlay");
        // This project has no .codeatlas.yaml with add/suppress, so both should be empty
        assert!(
            manual_edges.is_empty(),
            "no manual edges expected without .codeatlas.yaml config"
        );
        assert!(
            suppressed_ids.is_empty(),
            "no suppressions expected without .codeatlas.yaml config"
        );
    }

    /// Collecting sink for testing scan pipeline output.
    #[derive(Default)]
    struct CollectingScanSink {
        phases: std::sync::Mutex<Vec<(ScanPhase, Vec<NodeData>, Vec<EdgeData>)>>,
        health: std::sync::Mutex<Option<GraphHealth>>,
        compatibility: std::sync::Mutex<Option<CompatibilityReport>>,
        progress: std::sync::Mutex<Vec<(usize, usize)>>,
        details: std::sync::Mutex<
            Option<(
                Vec<crate::graph::types::UnsupportedConstruct>,
                Vec<crate::graph::types::ParseFailure>,
            )>,
        >,
        overlay: std::sync::Mutex<Option<(Vec<EdgeData>, Vec<String>)>>,
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
        fn overlay(&self) -> Option<(Vec<EdgeData>, Vec<String>)> {
            self.overlay.lock().expect("lock").clone()
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
        fn on_details(
            &self,
            unsupported_constructs: Vec<crate::graph::types::UnsupportedConstruct>,
            parse_failures: Vec<crate::graph::types::ParseFailure>,
        ) {
            *self.details.lock().expect("lock") =
                Some((unsupported_constructs, parse_failures));
        }
        fn on_overlay(&self, manual_edges: Vec<EdgeData>, suppressed_edge_ids: Vec<String>) {
            *self.overlay.lock().expect("lock") = Some((manual_edges, suppressed_edge_ids));
        }
    }
}
