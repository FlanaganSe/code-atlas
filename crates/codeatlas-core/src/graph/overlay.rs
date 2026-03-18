//! Configuration overlay model for the architecture graph.
//!
//! The discovered graph is **immutable** — it represents what the scanner
//! actually found. Overlays supplement the graph with manually declared
//! edges, suppressions, and metadata without mutating discovered data.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::identity::{EdgeId, MaterializedKey};

/// Configuration overlay layer on top of the discovered graph.
///
/// Contains manual edges (declared in `.codeatlas.yaml`), edge suppressions,
/// and per-node metadata. None of these mutate the discovered graph —
/// they are additive annotations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphOverlay {
    /// Edges declared in `.codeatlas.yaml` that the scanner cannot observe.
    pub manual_edges: Vec<ManualEdge>,

    /// Discovered edges suppressed in the default view.
    /// Key is the `EdgeId` of the discovered edge.
    pub suppressions: HashMap<EdgeId, SuppressionReason>,

    /// Per-node metadata from configuration.
    pub metadata: HashMap<MaterializedKey, NodeMetadata>,
}

/// A manually declared edge from `.codeatlas.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualEdge {
    /// Source node path (relative to workspace root).
    pub from: String,
    /// Target node path (relative to workspace root).
    pub to: String,
    /// Human-readable reason why this edge was manually declared.
    pub reason: String,
}

/// Why an edge was suppressed in the default view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuppressionReason {
    /// Human-readable reason.
    pub reason: String,
}

/// Metadata attached to a node from `.codeatlas.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeMetadata {
    /// Organizational tags.
    pub tags: Vec<String>,
    /// Architectural layer name.
    pub layer: Option<String>,
    /// Team or individual owner.
    pub owner: Option<String>,
}
