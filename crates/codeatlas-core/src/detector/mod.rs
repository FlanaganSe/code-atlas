//! Detector trait and registry for language/framework analysis modules.
//!
//! Each detector discovers nodes and edges for a specific language or
//! framework. The trait is internal in the POC; a public plugin API
//! is deferred to the Platform phase.
//!
//! Detectors operate in two phases:
//! 1. `compatibility()` — assess what this detector can/cannot analyze (fast, no parsing).
//! 2. `detect()` — discover nodes and edges, streaming results through `DetectorSink`.

pub mod rust;
pub mod typescript;

use crate::config::RepoConfig;
use crate::graph::types::{EdgeData, Language, NodeData, ParseFailure, UnsupportedConstruct};
use crate::health::compatibility::{CompatibilityDetail, SupportStatus};
use crate::profile::GraphProfile;
use crate::workspace::WorkspaceInfo;

pub use rust::RustDetector;
pub use typescript::TypeScriptDetector;

/// Errors that can occur during detection.
#[derive(Debug, thiserror::Error)]
pub enum DetectorError {
    #[error("detector '{name}' failed: {reason}")]
    DetectionFailed { name: String, reason: String },

    #[error("parse error in {path}: {reason}")]
    ParseError { path: String, reason: String },
}

/// A detector module that discovers nodes and edges in a repository.
///
/// Implementations are registered in a `DetectorRegistry` and invoked
/// during scanning. Each detector handles one language/framework.
pub trait Detector: Send + Sync {
    /// Human-readable name (e.g., "rust-cargo", "typescript-imports").
    fn name(&self) -> &str;

    /// What language/framework this detector handles.
    fn language(&self) -> Language;

    /// Whether this detector applies to the given workspace.
    fn applies_to(&self, workspace: &WorkspaceInfo) -> bool;

    /// Report what this detector can and cannot analyze (for the compatibility report).
    ///
    /// This runs before `detect()` and produces the initial (provisional)
    /// compatibility assessment.
    fn compatibility(&self, workspace: &WorkspaceInfo) -> CompatibilityAssessment;

    /// Discover nodes and edges, streaming results through `sink`.
    ///
    /// Returns a summary of what was found and what couldn't be analyzed.
    fn detect(
        &self,
        workspace: &WorkspaceInfo,
        profile: &GraphProfile,
        config: &RepoConfig,
        sink: &dyn DetectorSink,
    ) -> Result<DetectorReport, DetectorError>;
}

/// Sink for streaming detection results.
///
/// This is a domain-level trait — no transport or IPC concepts.
/// The Tauri shell provides a `ChannelSink` adapter that bridges
/// this to `Channel<ScanEvent>`.
pub trait DetectorSink: Send + Sync {
    /// Report discovered nodes.
    fn on_nodes(&self, nodes: Vec<NodeData>);

    /// Report discovered edges.
    fn on_edges(&self, edges: Vec<EdgeData>);
}

/// A detector's contribution to the compatibility report.
///
/// Contains the language-level support status and specific details
/// about what is and isn't supported.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityAssessment {
    /// The language this assessment covers.
    pub language: Language,

    /// Overall support status for this language.
    pub status: SupportStatus,

    /// Specific details about what is/isn't supported.
    pub details: Vec<CompatibilityDetail>,
}

/// Summary of a detection pass.
///
/// Contains counts and lists of what was found and what couldn't be analyzed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectorReport {
    /// Number of nodes discovered.
    pub nodes_discovered: usize,

    /// Number of edges discovered.
    pub edges_discovered: usize,

    /// Constructs detected but not modeled.
    pub unsupported_constructs: Vec<UnsupportedConstruct>,

    /// Files that could not be fully parsed.
    pub parse_failures: Vec<ParseFailure>,
}

// A no-op sink useful for testing.
impl DetectorSink for () {
    fn on_nodes(&self, _nodes: Vec<NodeData>) {}
    fn on_edges(&self, _edges: Vec<EdgeData>) {}
}

// Ensure Detector is object-safe for the registry.
// The `dyn Detector` usage below proves this at compile time.
fn _assert_detector_object_safe(_: &dyn Detector) {}
fn _assert_sink_object_safe(_: &dyn DetectorSink) {}
