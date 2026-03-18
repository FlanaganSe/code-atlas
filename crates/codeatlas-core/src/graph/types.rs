//! Domain types for the Code Atlas architecture graph.
//!
//! These types form the core data model. They are serializable to JSON
//! with `camelCase` field names to match the manually-written TypeScript types.
//! Enums use adjacently tagged serde: `#[serde(tag = "type", content = "data")]`.

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use std::fmt;

use super::identity::{EdgeId, MaterializedKey};

// ---------------------------------------------------------------------------
// Language
// ---------------------------------------------------------------------------

/// Programming language detected in the repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "rust")]
    Rust,
    #[serde(rename = "typescript")]
    TypeScript,
    #[serde(rename = "javascript")]
    JavaScript,
    #[serde(rename = "unknown")]
    Unknown,
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rust => write!(f, "rust"),
            Self::TypeScript => write!(f, "typescript"),
            Self::JavaScript => write!(f, "javascript"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

// ---------------------------------------------------------------------------
// EntityKind
// ---------------------------------------------------------------------------

/// The structural kind of an entity within a repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntityKind {
    /// A workspace package (Cargo crate, npm package).
    Package,
    /// A module within a package (Rust `mod`, TS directory-as-module).
    Module,
    /// A single source file.
    File,
}

impl fmt::Display for EntityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Package => write!(f, "package"),
            Self::Module => write!(f, "module"),
            Self::File => write!(f, "file"),
        }
    }
}

// ---------------------------------------------------------------------------
// NodeKind
// ---------------------------------------------------------------------------

/// Visual/semantic kind of a graph node. Determines rendering behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeKind {
    Package,
    Module,
    File,
}

// ---------------------------------------------------------------------------
// NodeData
// ---------------------------------------------------------------------------

/// A node in the architecture graph.
///
/// Every node is uniquely identified by its `MaterializedKey`.
/// The `lineage_key` tracks logical identity across renames (MVP).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeData {
    /// Unique identity within a single scan/snapshot.
    pub materialized_key: MaterializedKey,

    /// Logical identity that survives renames. Always `None` in POC.
    pub lineage_key: Option<LineageKey>,

    /// Human-readable label (e.g. crate name, file name).
    pub label: String,

    /// Structural kind of this node.
    pub kind: NodeKind,

    /// Language this node belongs to.
    pub language: Language,

    /// Key of the parent node (`None` for top-level packages).
    pub parent_key: Option<MaterializedKey>,
}

// ---------------------------------------------------------------------------
// LineageKey
// ---------------------------------------------------------------------------

/// UUID-based identity that survives renames/moves.
///
/// In the POC this is always `None` — lineage tracking activates
/// in MVP when SQLite persistence lands.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineageKey(pub String);

// ---------------------------------------------------------------------------
// EdgeKind
// ---------------------------------------------------------------------------

/// How two nodes are related.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EdgeKind {
    /// A source-level import relationship.
    Imports,
    /// A re-export (`pub use`, `export { ... } from`).
    ReExports,
    /// Structural containment (parent → child).
    Contains,
    /// Manifest-level dependency (Cargo.toml, package.json).
    DependsOn,
    /// Manually declared via `.codeatlas.yaml`.
    Manual,
}

// ---------------------------------------------------------------------------
// EdgeCategory
// ---------------------------------------------------------------------------

/// Semantic role of an edge — orthogonal to how it was discovered.
///
/// Edge categories enable filtering: "show me only runtime imports",
/// "highlight dev-only edges", etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EdgeCategory {
    /// Runtime import present in compiled output.
    Value,
    /// `import type` / `export type` — elided at compile time.
    TypeOnly,
    /// From `devDependencies` or `[dev-dependencies]`.
    Dev,
    /// From `[build-dependencies]` or build tooling.
    Build,
    /// Edge exists only in test code.
    Test,
    /// From `peerDependencies`.
    Peer,
    /// Standard runtime dependency (default).
    Normal,
    /// Added via `.codeatlas.yaml` overlay.
    Manual,
}

// ---------------------------------------------------------------------------
// Confidence
// ---------------------------------------------------------------------------

/// How confident we are in an edge's correctness.
///
/// Ordered from least to most confident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Confidence {
    /// Directory hierarchy only.
    Structural,
    /// Parsed import statement.
    Syntactic,
    /// Import resolved to target via path/workspace resolution.
    ResolverAware,
    /// Type-aware analysis (post-MVP).
    Semantic,
    /// Observed at runtime (Vision).
    Runtime,
}

// ---------------------------------------------------------------------------
// SourceLocation
// ---------------------------------------------------------------------------

/// Source-code location where a relationship was detected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceLocation {
    /// Path relative to workspace root.
    pub path: Utf8PathBuf,
    /// 1-based start line.
    pub start_line: u32,
    /// 1-based end line.
    pub end_line: u32,
}

// ---------------------------------------------------------------------------
// OverlayStatus
// ---------------------------------------------------------------------------

/// Whether an edge has been affected by a configuration overlay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum OverlayStatus {
    /// Not affected by any overlay.
    None,
    /// Suppressed in the default view, with a reason.
    Suppressed { reason: String },
}

// ---------------------------------------------------------------------------
// EdgeData
// ---------------------------------------------------------------------------

/// An edge in the architecture graph.
///
/// Every edge carries evidence metadata (kind, category, confidence,
/// source location, resolution method) for trust and transparency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeData {
    /// Deterministic identity: hash of (source_key, target_key, kind, category).
    pub edge_id: EdgeId,

    /// Key of the source node.
    pub source_key: MaterializedKey,

    /// Key of the target node.
    pub target_key: MaterializedKey,

    /// How these nodes are related.
    pub kind: EdgeKind,

    /// Semantic role of this relationship.
    pub category: EdgeCategory,

    /// How confident we are in this edge.
    pub confidence: Confidence,

    /// Where in source code this relationship was detected.
    pub source_location: Option<SourceLocation>,

    /// Which resolver/method produced this edge.
    pub resolution_method: Option<String>,

    /// Overlay status (suppressed, etc.).
    pub overlay_status: OverlayStatus,
}

// ---------------------------------------------------------------------------
// UnsupportedConstruct
// ---------------------------------------------------------------------------

/// A language construct detected but not modeled by the scanner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsupportedConstruct {
    /// Category of unsupported construct.
    pub construct_type: UnsupportedConstructType,

    /// Where it was detected.
    pub location: SourceLocation,

    /// What the graph might be missing because of this.
    pub impact: String,

    /// Guidance on how to address this limitation.
    pub how_to_address: String,
}

/// Categories of unsupported constructs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UnsupportedConstructType {
    CfgGate,
    BuildScript,
    ProcMacro,
    DynamicImport,
    FrameworkConvention,
    ExportsCondition,
}

// ---------------------------------------------------------------------------
// ParseFailure
// ---------------------------------------------------------------------------

/// A file that the parser could not fully process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseFailure {
    /// Path relative to workspace root.
    pub path: Utf8PathBuf,
    /// Human-readable description of the failure.
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_display() {
        assert_eq!(Language::Rust.to_string(), "rust");
        assert_eq!(Language::TypeScript.to_string(), "typescript");
        assert_eq!(Language::JavaScript.to_string(), "javascript");
        assert_eq!(Language::Unknown.to_string(), "unknown");
    }

    #[test]
    fn language_serde_matches_display() {
        // Serde output must match Display (lowercase) so MaterializedKey string
        // representation and JSON serialization use the same language names.
        let check = |lang: Language| {
            let json = serde_json::to_string(&lang).unwrap();
            let expected = format!("\"{}\"", lang);
            assert_eq!(json, expected, "serde and Display must match for {lang:?}");
        };
        check(Language::Rust);
        check(Language::TypeScript);
        check(Language::JavaScript);
        check(Language::Unknown);
    }

    #[test]
    fn entity_kind_display() {
        assert_eq!(EntityKind::Package.to_string(), "package");
        assert_eq!(EntityKind::Module.to_string(), "module");
        assert_eq!(EntityKind::File.to_string(), "file");
    }

    #[test]
    fn confidence_ordering() {
        assert!(Confidence::Structural < Confidence::Syntactic);
        assert!(Confidence::Syntactic < Confidence::ResolverAware);
        assert!(Confidence::ResolverAware < Confidence::Semantic);
        assert!(Confidence::Semantic < Confidence::Runtime);
    }

    #[test]
    fn node_data_serde_round_trip() {
        use crate::graph::identity::MaterializedKey;

        let node = NodeData {
            materialized_key: MaterializedKey::new(
                Language::Rust,
                EntityKind::Package,
                "crates/core",
            ),
            lineage_key: None,
            label: "codeatlas-core".to_string(),
            kind: NodeKind::Package,
            language: Language::Rust,
            parent_key: None,
        };

        let json = serde_json::to_string(&node).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Verify camelCase field names
        assert!(parsed.get("materializedKey").is_some());
        assert!(parsed.get("lineageKey").is_some());
        assert!(parsed.get("parentKey").is_some());

        // Round-trip
        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(node, deserialized);
    }

    #[test]
    fn edge_data_serde_round_trip() {
        use crate::graph::identity::{EdgeId, MaterializedKey};

        let source = MaterializedKey::new(Language::Rust, EntityKind::File, "src/lib.rs");
        let target = MaterializedKey::new(Language::Rust, EntityKind::File, "src/graph/mod.rs");

        let edge = EdgeData {
            edge_id: EdgeId::new(&source, &target, EdgeKind::Imports, EdgeCategory::Value),
            source_key: source,
            target_key: target,
            kind: EdgeKind::Imports,
            category: EdgeCategory::Value,
            confidence: Confidence::Syntactic,
            source_location: None,
            resolution_method: Some("tree-sitter".to_string()),
            overlay_status: OverlayStatus::None,
        };

        let json = serde_json::to_string(&edge).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Verify camelCase
        assert!(parsed.get("edgeId").is_some());
        assert!(parsed.get("sourceKey").is_some());
        assert!(parsed.get("targetKey").is_some());
        assert!(parsed.get("resolutionMethod").is_some());
        assert!(parsed.get("overlayStatus").is_some());

        // Round-trip
        let deserialized: EdgeData = serde_json::from_str(&json).unwrap();
        assert_eq!(edge, deserialized);
    }

    #[test]
    fn overlay_status_tagged_enum_format() {
        let suppressed = OverlayStatus::Suppressed {
            reason: "dead code".to_string(),
        };
        let json = serde_json::to_string(&suppressed).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Adjacently tagged: {"type": "suppressed", "data": {"reason": "dead code"}}
        assert_eq!(parsed["type"], "suppressed");
        assert_eq!(parsed["data"]["reason"], "dead code");

        let none = OverlayStatus::None;
        let json = serde_json::to_string(&none).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "none");
    }
}
