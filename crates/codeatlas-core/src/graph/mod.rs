//! Graph model for the Code Atlas architecture graph.
//!
//! The graph is organized in two layers:
//! - **Discovered layer**: nodes and edges found by scanning source code and manifests.
//! - **Overlay layer**: manual edges, suppressions, and metadata from `.codeatlas.yaml`.
//!
//! The overlay never mutates the discovered layer — it supplements it.

pub mod arch_graph;
pub mod identity;
pub mod overlay;
pub mod query;
pub mod types;

pub use arch_graph::ArchGraph;
pub use identity::{normalize_path, EdgeId, MaterializedKey};
pub use overlay::GraphOverlay;
pub use types::*;

/// Errors that can occur during graph operations.
#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("duplicate node: key {key} already exists")]
    DuplicateNode { key: MaterializedKey },

    #[error("node not found: key {key}")]
    NodeNotFound { key: MaterializedKey },

    #[error("duplicate edge: id {edge_id} already exists")]
    DuplicateEdge { edge_id: EdgeId },
}
