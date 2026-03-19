//! `ArchGraph` — the two-layer architecture graph model.
//!
//! Wraps petgraph's `StableGraph` with domain invariants:
//! - No duplicate `MaterializedKey`s in the discovered layer.
//! - The overlay layer cannot mutate discovered edges.
//! - All external references use `MaterializedKey` / `EdgeId`,
//!   not petgraph's internal `NodeIndex` / `EdgeIndex`.

use petgraph::stable_graph::{EdgeIndex, NodeIndex, StableGraph};
use petgraph::Directed;
use std::collections::HashMap;

use super::identity::{EdgeId, MaterializedKey};
use super::overlay::GraphOverlay;
use super::types::{EdgeData, NodeData};
use super::GraphError;

/// The two-layer architecture graph.
///
/// The **discovered layer** contains nodes and edges found by scanning.
/// The **overlay layer** contains manual edges, suppressions, and metadata
/// from `.codeatlas.yaml`. The overlay never mutates the discovered layer.
#[derive(Clone)]
pub struct ArchGraph {
    /// The discovered graph — immutable once built.
    discovered: StableGraph<NodeData, EdgeData, Directed>,

    /// Fast lookup from MaterializedKey to petgraph NodeIndex.
    node_index: HashMap<MaterializedKey, NodeIndex>,

    /// Fast lookup from EdgeId to petgraph EdgeIndex.
    edge_index: HashMap<EdgeId, EdgeIndex>,

    /// Configuration overlay (manual edges, suppressions, metadata).
    overlay: GraphOverlay,
}

impl ArchGraph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self {
            discovered: StableGraph::new(),
            node_index: HashMap::new(),
            edge_index: HashMap::new(),
            overlay: GraphOverlay::default(),
        }
    }

    // -----------------------------------------------------------------------
    // Discovered layer — node operations
    // -----------------------------------------------------------------------

    /// Add a node to the discovered layer.
    ///
    /// Returns an error if a node with the same `MaterializedKey` already exists.
    pub fn add_node(&mut self, data: NodeData) -> Result<NodeIndex, GraphError> {
        let key = data.materialized_key.clone();

        if self.node_index.contains_key(&key) {
            return Err(GraphError::DuplicateNode { key });
        }

        let idx = self.discovered.add_node(data);
        self.node_index.insert(key, idx);
        Ok(idx)
    }

    /// Get a node's data by its `MaterializedKey`.
    pub fn node(&self, key: &MaterializedKey) -> Option<&NodeData> {
        self.node_index
            .get(key)
            .and_then(|idx| self.discovered.node_weight(*idx))
    }

    /// Get the petgraph `NodeIndex` for a `MaterializedKey`.
    pub fn node_index(&self, key: &MaterializedKey) -> Option<NodeIndex> {
        self.node_index.get(key).copied()
    }

    /// Number of nodes in the discovered layer.
    pub fn node_count(&self) -> usize {
        self.discovered.node_count()
    }

    /// Iterate over all discovered nodes.
    pub fn nodes(&self) -> impl Iterator<Item = &NodeData> {
        self.discovered.node_weights()
    }

    // -----------------------------------------------------------------------
    // Discovered layer — edge operations
    // -----------------------------------------------------------------------

    /// Add an edge to the discovered layer.
    ///
    /// Both source and target nodes must exist. Returns an error if either
    /// node is missing or if an edge with the same `EdgeId` already exists.
    pub fn add_edge(&mut self, data: EdgeData) -> Result<EdgeIndex, GraphError> {
        let source_idx = self.node_index.get(&data.source_key).ok_or_else(|| {
            GraphError::NodeNotFound {
                key: data.source_key.clone(),
            }
        })?;
        let target_idx = self.node_index.get(&data.target_key).ok_or_else(|| {
            GraphError::NodeNotFound {
                key: data.target_key.clone(),
            }
        })?;

        if self.edge_index.contains_key(&data.edge_id) {
            return Err(GraphError::DuplicateEdge {
                edge_id: data.edge_id.clone(),
            });
        }

        let edge_id = data.edge_id.clone();
        let idx = self.discovered.add_edge(*source_idx, *target_idx, data);
        self.edge_index.insert(edge_id, idx);
        Ok(idx)
    }

    /// Get an edge's data by its `EdgeId`.
    pub fn edge(&self, edge_id: &EdgeId) -> Option<&EdgeData> {
        self.edge_index
            .get(edge_id)
            .and_then(|idx| self.discovered.edge_weight(*idx))
    }

    /// Number of edges in the discovered layer.
    pub fn edge_count(&self) -> usize {
        self.discovered.edge_count()
    }

    /// Iterate over all discovered edges.
    pub fn edges(&self) -> impl Iterator<Item = &EdgeData> {
        self.discovered.edge_weights()
    }

    // -----------------------------------------------------------------------
    // Overlay layer
    // -----------------------------------------------------------------------

    /// Get a reference to the overlay.
    pub fn overlay(&self) -> &GraphOverlay {
        &self.overlay
    }

    /// Get a mutable reference to the overlay.
    ///
    /// This is the ONLY way to modify overlay data. The overlay
    /// operates on `EdgeId` and `MaterializedKey` references, not on
    /// the `StableGraph` directly, so it cannot mutate discovered data.
    pub fn overlay_mut(&mut self) -> &mut GraphOverlay {
        &mut self.overlay
    }

    /// Replace the entire overlay.
    pub fn set_overlay(&mut self, overlay: GraphOverlay) {
        self.overlay = overlay;
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Check whether a `MaterializedKey` exists in the discovered graph.
    pub fn contains_node(&self, key: &MaterializedKey) -> bool {
        self.node_index.contains_key(key)
    }

    /// Check whether an `EdgeId` exists in the discovered graph.
    pub fn contains_edge(&self, edge_id: &EdgeId) -> bool {
        self.edge_index.contains_key(edge_id)
    }
}

impl Default for ArchGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ArchGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArchGraph")
            .field("node_count", &self.node_count())
            .field("edge_count", &self.edge_count())
            .field("overlay_manual_edges", &self.overlay.manual_edges.len())
            .field("overlay_suppressions", &self.overlay.suppressions.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::identity::EdgeId;
    use crate::graph::types::*;

    fn make_node(lang: Language, kind: EntityKind, path: &str) -> NodeData {
        NodeData {
            materialized_key: MaterializedKey::new(lang, kind, path),
            lineage_key: None,
            label: path.to_string(),
            kind: match kind {
                EntityKind::Package => NodeKind::Package,
                EntityKind::Module => NodeKind::Module,
                EntityKind::File => NodeKind::File,
            },
            language: lang,
            parent_key: None,
        }
    }

    fn make_edge(
        source: &MaterializedKey,
        target: &MaterializedKey,
        kind: EdgeKind,
        category: EdgeCategory,
    ) -> EdgeData {
        EdgeData {
            edge_id: EdgeId::new(source, target, kind, category),
            source_key: source.clone(),
            target_key: target.clone(),
            kind,
            category,
            confidence: Confidence::Syntactic,
            source_location: None,
            resolution_method: None,
            overlay_status: OverlayStatus::None,
        }
    }

    #[test]
    fn add_node_succeeds() {
        let mut graph = ArchGraph::new();
        let node = make_node(Language::Rust, EntityKind::Package, "crates/core");
        assert!(graph.add_node(node).is_ok());
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn duplicate_node_rejected() {
        let mut graph = ArchGraph::new();
        let node1 = make_node(Language::Rust, EntityKind::Package, "crates/core");
        let node2 = make_node(Language::Rust, EntityKind::Package, "crates/core");

        assert!(graph.add_node(node1).is_ok());
        let err = graph.add_node(node2).unwrap_err();
        assert!(
            matches!(err, GraphError::DuplicateNode { .. }),
            "expected DuplicateNode error, got: {err:?}"
        );
    }

    #[test]
    fn add_edge_succeeds() {
        let mut graph = ArchGraph::new();
        let n1 = make_node(Language::Rust, EntityKind::File, "src/a.rs");
        let n2 = make_node(Language::Rust, EntityKind::File, "src/b.rs");
        let k1 = n1.materialized_key.clone();
        let k2 = n2.materialized_key.clone();

        graph.add_node(n1).unwrap();
        graph.add_node(n2).unwrap();

        let edge = make_edge(&k1, &k2, EdgeKind::Imports, EdgeCategory::Value);
        assert!(graph.add_edge(edge).is_ok());
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn edge_to_missing_node_rejected() {
        let mut graph = ArchGraph::new();
        let n1 = make_node(Language::Rust, EntityKind::File, "src/a.rs");
        let k1 = n1.materialized_key.clone();
        let k2 = MaterializedKey::new(Language::Rust, EntityKind::File, "src/missing.rs");

        graph.add_node(n1).unwrap();

        let edge = make_edge(&k1, &k2, EdgeKind::Imports, EdgeCategory::Value);
        let err = graph.add_edge(edge).unwrap_err();
        assert!(matches!(err, GraphError::NodeNotFound { .. }));
    }

    #[test]
    fn duplicate_edge_rejected() {
        let mut graph = ArchGraph::new();
        let n1 = make_node(Language::Rust, EntityKind::File, "src/a.rs");
        let n2 = make_node(Language::Rust, EntityKind::File, "src/b.rs");
        let k1 = n1.materialized_key.clone();
        let k2 = n2.materialized_key.clone();

        graph.add_node(n1).unwrap();
        graph.add_node(n2).unwrap();

        let edge1 = make_edge(&k1, &k2, EdgeKind::Imports, EdgeCategory::Value);
        let edge2 = make_edge(&k1, &k2, EdgeKind::Imports, EdgeCategory::Value);
        assert!(graph.add_edge(edge1).is_ok());
        let err = graph.add_edge(edge2).unwrap_err();
        assert!(matches!(err, GraphError::DuplicateEdge { .. }));
    }

    #[test]
    fn parallel_edges_with_different_categories_allowed() {
        let mut graph = ArchGraph::new();
        let n1 = make_node(Language::TypeScript, EntityKind::File, "src/index.ts");
        let n2 = make_node(Language::TypeScript, EntityKind::File, "src/types.ts");
        let k1 = n1.materialized_key.clone();
        let k2 = n2.materialized_key.clone();

        graph.add_node(n1).unwrap();
        graph.add_node(n2).unwrap();

        let value_edge = make_edge(&k1, &k2, EdgeKind::Imports, EdgeCategory::Value);
        let type_edge = make_edge(&k1, &k2, EdgeKind::Imports, EdgeCategory::TypeOnly);

        assert!(graph.add_edge(value_edge).is_ok());
        assert!(graph.add_edge(type_edge).is_ok());
        assert_eq!(graph.edge_count(), 2);
    }

    #[test]
    fn overlay_cannot_mutate_discovered_nodes() {
        let mut graph = ArchGraph::new();
        let node = make_node(Language::Rust, EntityKind::Package, "crates/core");
        let key = node.materialized_key.clone();
        graph.add_node(node).unwrap();

        // Overlay can add metadata but cannot remove or modify the discovered node
        graph.overlay_mut().metadata.insert(
            key.clone(),
            super::super::overlay::NodeMetadata {
                tags: vec!["public".to_string()],
                layer: Some("core".to_string()),
                owner: None,
            },
        );

        // Node still exists unchanged in discovered layer
        assert!(graph.contains_node(&key));
        assert_eq!(graph.node(&key).unwrap().label, "crates/core");
    }

    #[test]
    fn overlay_suppression_does_not_remove_edge() {
        let mut graph = ArchGraph::new();
        let n1 = make_node(Language::Rust, EntityKind::File, "src/a.rs");
        let n2 = make_node(Language::Rust, EntityKind::File, "src/b.rs");
        let k1 = n1.materialized_key.clone();
        let k2 = n2.materialized_key.clone();

        graph.add_node(n1).unwrap();
        graph.add_node(n2).unwrap();

        let edge = make_edge(&k1, &k2, EdgeKind::Imports, EdgeCategory::Value);
        let edge_id = edge.edge_id.clone();
        graph.add_edge(edge).unwrap();

        // Suppress the edge in overlay
        graph.overlay_mut().suppressions.insert(
            edge_id.clone(),
            super::super::overlay::SuppressionReason {
                reason: "dead code".to_string(),
            },
        );

        // Edge still exists in discovered layer
        assert!(graph.contains_edge(&edge_id));
        assert!(graph.edge(&edge_id).is_some());
        assert_eq!(graph.edge_count(), 1, "suppression must not remove edge");
    }

    #[test]
    fn node_lookup_by_key() {
        let mut graph = ArchGraph::new();
        let node = make_node(Language::Rust, EntityKind::Package, "crates/core");
        let key = node.materialized_key.clone();

        graph.add_node(node).unwrap();

        let found = graph.node(&key).unwrap();
        assert_eq!(found.label, "crates/core");
        assert_eq!(found.language, Language::Rust);
    }
}
