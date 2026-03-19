//! Graph health metrics.
//!
//! Provides a snapshot of the graph's completeness and quality:
//! total nodes, resolved edges, unresolved imports, parse failures,
//! and unsupported constructs.

use serde::{Deserialize, Serialize};

use crate::graph::types::UnresolvedImport;

/// Summary health metrics for the architecture graph.
///
/// Displayed as a header metric in the UI so users can immediately
/// gauge how complete and trustworthy the graph is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphHealth {
    /// Total number of nodes in the graph.
    pub total_nodes: usize,

    /// Number of edges successfully resolved.
    pub resolved_edges: usize,

    /// Number of imports that could not be resolved to a target.
    pub unresolved_imports: usize,

    /// Number of files with parse failures.
    pub parse_failures: usize,

    /// Number of unsupported constructs detected.
    pub unsupported_constructs: usize,

    /// Detailed list of unresolved imports with reasons.
    pub unresolved_import_details: Vec<UnresolvedImport>,
}

impl GraphHealth {
    /// Create a health snapshot with all zeros.
    pub fn empty() -> Self {
        Self {
            total_nodes: 0,
            resolved_edges: 0,
            unresolved_imports: 0,
            parse_failures: 0,
            unsupported_constructs: 0,
            unresolved_import_details: Vec::new(),
        }
    }
}
