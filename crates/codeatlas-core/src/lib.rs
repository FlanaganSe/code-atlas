//! `codeatlas-core` — standalone analysis library for Code Atlas.
//!
//! This crate contains all scanning, graph building, profile management,
//! compatibility reporting, overlay management, and query logic.
//! It has **zero dependency on Tauri, serde_json, or any IPC/transport crate**.
//!
//! # Architecture
//!
//! The crate is organized into modules:
//!
//! - [`graph`] — Graph model: `ArchGraph`, `NodeData`, `EdgeData`, identity scheme, overlay.
//! - [`detector`] — `Detector` trait and `DetectorSink` for language/framework analysis.
//! - [`config`] — `.codeatlas.yaml` parsing and validation.
//! - [`profile`] — Graph profile (build context) management.
//! - [`health`] — Compatibility report and graph health metrics.
//! - [`workspace`] — Workspace discovery types and implementation.
//! - [`scan`] — Scan orchestration, domain result types, `ScanSink` trait.
//! - [`error`] — Top-level error types.
//!
//! # Core API Pattern
//!
//! Following rust-analyzer's architecture invariant:
//! - **`AnalysisHost`** — mutable handle, accepts changes.
//! - **`Analysis`** — immutable snapshot, safe for concurrent queries.

pub mod config;
pub mod detector;
pub mod error;
pub mod graph;
pub mod health;
pub mod profile;
pub mod scan;
pub mod workspace;

// Re-export primary types for convenience.
pub use config::RepoConfig;
pub use error::CoreError;
pub use graph::{ArchGraph, EdgeId, GraphOverlay, MaterializedKey};
pub use health::{CompatibilityReport, GraphHealth};
pub use profile::GraphProfile;
pub use scan::{ScanPhase, ScanResults, ScanSink};
pub use workspace::WorkspaceInfo;

use camino::Utf8Path;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use detector::{CompatibilityAssessment, Detector, RustDetector, TypeScriptDetector};
use health::compatibility::CompatibilityReport as CompatReport;

/// Result of workspace discovery — bundles all information needed
/// by the frontend to display the initial workspace state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryResult {
    /// Workspace structure information.
    pub workspace: WorkspaceInfo,
    /// Parsed `.codeatlas.yaml` config.
    pub config: RepoConfig,
    /// Auto-detected graph profile.
    pub profile: GraphProfile,
    /// Compatibility report (provisional — structural findings only).
    pub compatibility: CompatReport,
    /// Config sections recognized but not yet functional in POC.
    pub non_functional_config_sections: Vec<String>,
}

/// Mutable handle to the analysis engine.
///
/// Accepts workspace discovery, scan results, and config changes.
/// Use [`AnalysisHost::snapshot`] to get an immutable [`Analysis`]
/// for concurrent queries.
pub struct AnalysisHost {
    /// The architecture graph (populated by scanning in M4).
    #[expect(dead_code)]
    graph: ArchGraph,
    config: RepoConfig,
    profile: GraphProfile,
    compatibility: CompatReport,
    workspace: Option<WorkspaceInfo>,
}

impl AnalysisHost {
    /// Create a new, empty analysis host.
    pub fn new() -> Self {
        Self {
            graph: ArchGraph::new(),
            config: RepoConfig::default_config(),
            profile: GraphProfile::empty(),
            compatibility: CompatReport::provisional(),
            workspace: None,
        }
    }

    /// Discover workspace structure at the given directory path.
    ///
    /// This is the M2 entry point. It:
    /// 1. Discovers workspace structure (Cargo + JS)
    /// 2. Loads `.codeatlas.yaml` config
    /// 3. Auto-detects graph profile
    /// 4. Runs detector `compatibility()` assessments
    /// 5. Assembles the provisional compatibility report
    ///
    /// **Blocking**: this may run `cargo metadata` which takes 2-10s.
    /// The Tauri shell should call this via `tokio::task::spawn_blocking`.
    pub fn discover_workspace(
        &mut self,
        dir: &Utf8Path,
    ) -> Result<DiscoveryResult, CoreError> {
        // Step 1: Discover workspace structure
        let workspace = workspace::discover_workspace(dir)?;

        // Step 2: Load config from workspace root
        let config = RepoConfig::load_from_dir(&workspace.root)
            .unwrap_or_else(|e| {
                tracing::warn!("failed to load .codeatlas.yaml: {e}, using defaults");
                RepoConfig::default_config()
            });

        // Step 3: Detect profile from workspace
        let profile = GraphProfile::detect_from_workspace(&workspace);

        // Step 4: Run detector compatibility assessments
        let detectors: Vec<Box<dyn Detector>> = vec![
            Box::new(RustDetector),
            Box::new(TypeScriptDetector),
        ];

        let assessments: Vec<CompatibilityAssessment> = detectors
            .iter()
            .filter(|d| d.applies_to(&workspace))
            .map(|d| d.compatibility(&workspace))
            .collect();

        // Step 5: Build compatibility report
        let compatibility = CompatReport {
            assessments,
            is_provisional: true,
        };

        // Track non-functional config sections
        let non_functional_config_sections: Vec<String> = config
            .non_functional_sections()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        // Update host state
        self.workspace = Some(workspace.clone());
        self.config = config.clone();
        self.profile = profile.clone();
        self.compatibility = compatibility.clone();

        Ok(DiscoveryResult {
            workspace,
            config,
            profile,
            compatibility,
            non_functional_config_sections,
        })
    }

    /// Take an immutable snapshot for concurrent queries.
    pub fn snapshot(&self) -> Analysis {
        Analysis {
            config: Arc::new(self.config.clone()),
            profile: Arc::new(self.profile.clone()),
            compatibility: Arc::new(self.compatibility.clone()),
            workspace: self.workspace.as_ref().map(|w| Arc::new(w.clone())),
        }
    }
}

impl Default for AnalysisHost {
    fn default() -> Self {
        Self::new()
    }
}

/// Immutable snapshot of the analysis state.
///
/// Safe for concurrent queries across threads. All fields are
/// `Arc`-wrapped for cheap cloning.
#[derive(Clone)]
pub struct Analysis {
    config: Arc<RepoConfig>,
    profile: Arc<GraphProfile>,
    compatibility: Arc<CompatReport>,
    workspace: Option<Arc<WorkspaceInfo>>,
}

impl Analysis {
    /// Get the current compatibility report.
    pub fn compatibility(&self) -> &CompatReport {
        &self.compatibility
    }

    /// Get the detected graph profile.
    pub fn profile(&self) -> &GraphProfile {
        &self.profile
    }

    /// Get the workspace info (if discovery has run).
    pub fn workspace_info(&self) -> Option<&WorkspaceInfo> {
        self.workspace.as_deref()
    }

    /// Get the loaded config.
    pub fn config(&self) -> &RepoConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_host_new_has_empty_state() {
        let host = AnalysisHost::new();
        let snap = host.snapshot();
        assert!(snap.compatibility().is_provisional);
        assert!(snap.compatibility().assessments.is_empty());
        assert!(snap.profile().languages.is_empty());
        assert!(snap.workspace_info().is_none());
    }

    #[test]
    fn discovery_result_serde_round_trip() {
        let result = DiscoveryResult {
            workspace: WorkspaceInfo {
                root: camino::Utf8PathBuf::from("/tmp/test"),
                kind: workspace::WorkspaceKind::Cargo,
                packages: vec![workspace::WorkspacePackage {
                    name: "my-crate".to_string(),
                    relative_path: "crates/my-crate".to_string(),
                    language: graph::types::Language::Rust,
                }],
                cargo: None,
                js: None,
            },
            config: RepoConfig::default_config(),
            profile: GraphProfile::empty(),
            compatibility: CompatibilityReport::provisional(),
            non_functional_config_sections: Vec::new(),
        };

        let json = serde_json::to_string(&result).expect("should serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("should parse JSON");

        // Verify camelCase keys
        assert!(parsed.get("workspace").is_some());
        assert!(parsed.get("config").is_some());
        assert!(parsed.get("profile").is_some());
        assert!(parsed.get("compatibility").is_some());
        assert!(parsed.get("nonFunctionalConfigSections").is_some());

        // Workspace should have camelCase keys
        let ws = parsed.get("workspace").unwrap();
        assert!(ws.get("root").is_some());
        assert!(ws.get("kind").is_some());
        assert!(ws.get("packages").is_some());

        // Round-trip
        let deserialized: DiscoveryResult =
            serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(result.workspace.kind, deserialized.workspace.kind);
    }

    #[test]
    fn discover_this_projects_workspace() {
        let project_root = camino::Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("should find project root");

        let mut host = AnalysisHost::new();
        let result = host
            .discover_workspace(project_root)
            .expect("discovery should succeed");

        // Should find workspace packages
        assert!(!result.workspace.packages.is_empty());

        // Profile should detect Rust
        assert!(result.profile.languages.contains(&graph::types::Language::Rust));

        // Compatibility report should be provisional
        assert!(result.compatibility.is_provisional);

        // Rust detector should have run
        assert!(result
            .compatibility
            .assessments
            .iter()
            .any(|a| a.language == graph::types::Language::Rust));

        // Snapshot should reflect the discovery
        let snap = host.snapshot();
        assert!(snap.workspace_info().is_some());
        assert!(!snap.profile().languages.is_empty());
    }
}
