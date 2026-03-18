//! Identity scheme for the Code Atlas graph.
//!
//! **MaterializedKey** uniquely identifies a node within a single scan/snapshot.
//! Format: `{language}:{entity_kind}:{relative_path}` — no workspace root,
//! portable and privacy-safe from the start.
//!
//! **EdgeId** uniquely identifies an edge as a hash of
//! `(source_key, target_key, edge_kind, edge_category)`. This supports
//! parallel edges (value + type-only between the same nodes).
//!
//! **Path normalization** ensures all paths use forward slashes,
//! no trailing slash, no `./` prefix.

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};

use super::types::{EdgeCategory, EdgeKind, EntityKind, Language};

// ---------------------------------------------------------------------------
// MaterializedKey
// ---------------------------------------------------------------------------

/// Current-location identity for a graph node.
///
/// Format: `{language}:{entity_kind}:{relative_path}`
///
/// - `language` — the programming language (rust, typescript, etc.)
/// - `entity_kind` — structural kind (package, module, file)
/// - `relative_path` — workspace-root-relative path, normalized
///
/// No workspace root is stored in the key — it is session metadata
/// on `AnalysisHost`, not baked into identities.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterializedKey {
    pub language: Language,
    pub entity_kind: EntityKind,
    pub relative_path: String,
}

impl MaterializedKey {
    /// Create a new `MaterializedKey` with a normalized path.
    pub fn new(language: Language, entity_kind: EntityKind, relative_path: &str) -> Self {
        Self {
            language,
            entity_kind,
            relative_path: normalize_path(relative_path),
        }
    }
}

impl fmt::Display for MaterializedKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.language, self.entity_kind, self.relative_path)
    }
}

// ---------------------------------------------------------------------------
// EdgeId
// ---------------------------------------------------------------------------

/// Deterministic identity for a graph edge.
///
/// Computed as a hash of `(source_key, target_key, edge_kind, edge_category)`.
/// This supports parallel edges between the same pair of nodes (e.g., a
/// `value` import and a `type_only` import) and per-edge overlay suppression.
///
/// `EdgeIndex` from petgraph is internal-only; external references use `EdgeId`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeId(pub String);

impl EdgeId {
    /// Create a deterministic `EdgeId` from the edge's defining components.
    pub fn new(
        source: &MaterializedKey,
        target: &MaterializedKey,
        kind: EdgeKind,
        category: EdgeCategory,
    ) -> Self {
        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        target.hash(&mut hasher);
        kind.hash(&mut hasher);
        category.hash(&mut hasher);
        Self(format!("{:016x}", hasher.finish()))
    }
}

impl fmt::Display for EdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Path normalization
// ---------------------------------------------------------------------------

/// Normalize a path for use in identity keys.
///
/// Policy:
/// - Forward slashes only (even on Windows).
/// - No trailing slash.
/// - No `./` prefix.
/// - Preserves original case (case-sensitive comparison, Linux behavior).
/// - Does NOT resolve symlinks (callers should resolve before calling).
///
/// Uses `camino::Utf8Path` internally for UTF-8 safety.
pub fn normalize_path(path: &str) -> String {
    // Replace backslashes with forward slashes
    let normalized = path.replace('\\', "/");

    // Use camino to normalize path components (handles `..`, `.`, etc.)
    let utf8_path = Utf8Path::new(&normalized);
    let mut components = Vec::new();
    for component in utf8_path.components() {
        match component {
            camino::Utf8Component::Normal(c) => components.push(c.to_string()),
            camino::Utf8Component::ParentDir => {
                components.pop();
            }
            camino::Utf8Component::CurDir => {} // skip `.`
            camino::Utf8Component::RootDir => {} // skip leading `/`
            camino::Utf8Component::Prefix(_) => {} // skip Windows prefix
        }
    }

    components.join("/")
}

/// Convert a `camino::Utf8PathBuf` to a normalized relative path string.
pub fn normalize_utf8_path(path: &Utf8Path) -> String {
    normalize_path(path.as_str())
}

/// Create a normalized `Utf8PathBuf` from a string.
pub fn normalized_utf8_path_buf(path: &str) -> Utf8PathBuf {
    Utf8PathBuf::from(normalize_path(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Path normalization
    // -----------------------------------------------------------------------

    #[test]
    fn normalize_removes_dot_prefix() {
        assert_eq!(normalize_path("./src/lib.rs"), "src/lib.rs");
    }

    #[test]
    fn normalize_converts_backslashes() {
        assert_eq!(normalize_path("src\\graph\\mod.rs"), "src/graph/mod.rs");
    }

    #[test]
    fn normalize_removes_trailing_slash() {
        assert_eq!(normalize_path("src/graph/"), "src/graph");
    }

    #[test]
    fn normalize_resolves_parent_dirs() {
        assert_eq!(normalize_path("src/graph/../config/mod.rs"), "src/config/mod.rs");
    }

    #[test]
    fn normalize_idempotent() {
        let path = "src/graph/types.rs";
        assert_eq!(normalize_path(path), path);
        assert_eq!(normalize_path(&normalize_path(path)), path);
    }

    #[test]
    fn normalize_empty_stays_empty() {
        assert_eq!(normalize_path(""), "");
    }

    // -----------------------------------------------------------------------
    // MaterializedKey
    // -----------------------------------------------------------------------

    #[test]
    fn materialized_key_display() {
        let key = MaterializedKey::new(Language::Rust, EntityKind::File, "src/lib.rs");
        assert_eq!(key.to_string(), "rust:file:src/lib.rs");
    }

    #[test]
    fn materialized_key_normalizes_path() {
        let key = MaterializedKey::new(Language::TypeScript, EntityKind::Module, "./src/graph/");
        assert_eq!(key.relative_path, "src/graph");
        assert_eq!(key.to_string(), "typescript:module:src/graph");
    }

    #[test]
    fn materialized_key_equality() {
        let a = MaterializedKey::new(Language::Rust, EntityKind::Package, "crates/core");
        let b = MaterializedKey::new(Language::Rust, EntityKind::Package, "./crates/core");
        assert_eq!(a, b, "normalized paths should produce equal keys");
    }

    #[test]
    fn materialized_key_inequality_on_language() {
        let a = MaterializedKey::new(Language::Rust, EntityKind::File, "src/lib.rs");
        let b = MaterializedKey::new(Language::TypeScript, EntityKind::File, "src/lib.rs");
        assert_ne!(a, b);
    }

    #[test]
    fn materialized_key_inequality_on_entity_kind() {
        let a = MaterializedKey::new(Language::Rust, EntityKind::File, "src/lib.rs");
        let b = MaterializedKey::new(Language::Rust, EntityKind::Module, "src/lib.rs");
        assert_ne!(a, b);
    }

    #[test]
    fn materialized_key_hashing_consistency() {
        use std::collections::HashSet;

        let a = MaterializedKey::new(Language::Rust, EntityKind::File, "src/lib.rs");
        let b = MaterializedKey::new(Language::Rust, EntityKind::File, "./src/lib.rs");

        let mut set = HashSet::new();
        set.insert(a.clone());
        assert!(set.contains(&b), "equal keys must hash the same");
    }

    #[test]
    fn materialized_key_serde_round_trip() {
        let key = MaterializedKey::new(Language::Rust, EntityKind::Package, "crates/core");
        let json = serde_json::to_string(&key).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed.get("language").is_some());
        assert!(parsed.get("entityKind").is_some());
        assert!(parsed.get("relativePath").is_some());

        let deserialized: MaterializedKey = serde_json::from_str(&json).unwrap();
        assert_eq!(key, deserialized);
    }

    // -----------------------------------------------------------------------
    // EdgeId
    // -----------------------------------------------------------------------

    #[test]
    fn edge_id_deterministic() {
        let src = MaterializedKey::new(Language::Rust, EntityKind::File, "src/lib.rs");
        let tgt = MaterializedKey::new(Language::Rust, EntityKind::File, "src/graph/mod.rs");

        let id1 = EdgeId::new(&src, &tgt, EdgeKind::Imports, EdgeCategory::Value);
        let id2 = EdgeId::new(&src, &tgt, EdgeKind::Imports, EdgeCategory::Value);
        assert_eq!(id1, id2);
    }

    #[test]
    fn edge_id_differs_by_category() {
        let src = MaterializedKey::new(Language::TypeScript, EntityKind::File, "src/index.ts");
        let tgt = MaterializedKey::new(Language::TypeScript, EntityKind::File, "src/types.ts");

        let value_edge = EdgeId::new(&src, &tgt, EdgeKind::Imports, EdgeCategory::Value);
        let type_edge = EdgeId::new(&src, &tgt, EdgeKind::Imports, EdgeCategory::TypeOnly);
        assert_ne!(value_edge, type_edge, "parallel edges with different categories get different IDs");
    }

    #[test]
    fn edge_id_differs_by_kind() {
        let src = MaterializedKey::new(Language::Rust, EntityKind::Package, "crates/a");
        let tgt = MaterializedKey::new(Language::Rust, EntityKind::Package, "crates/b");

        let depends = EdgeId::new(&src, &tgt, EdgeKind::DependsOn, EdgeCategory::Normal);
        let manual = EdgeId::new(&src, &tgt, EdgeKind::Manual, EdgeCategory::Normal);
        assert_ne!(depends, manual);
    }

    #[test]
    fn edge_id_direction_matters() {
        let a = MaterializedKey::new(Language::Rust, EntityKind::File, "src/a.rs");
        let b = MaterializedKey::new(Language::Rust, EntityKind::File, "src/b.rs");

        let ab = EdgeId::new(&a, &b, EdgeKind::Imports, EdgeCategory::Value);
        let ba = EdgeId::new(&b, &a, EdgeKind::Imports, EdgeCategory::Value);
        assert_ne!(ab, ba, "edge direction must affect identity");
    }
}
