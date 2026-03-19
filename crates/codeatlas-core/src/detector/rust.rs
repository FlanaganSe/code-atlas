//! Rust detector — `cargo_metadata` + tree-sitter analysis.
//!
//! M2: implements `compatibility()` (structural/manifest-level assessment).
//! M4: implements `detect()` (tree-sitter parsing, edge discovery).

use std::collections::HashMap;

use camino::{Utf8Path, Utf8PathBuf};
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

use crate::config::RepoConfig;
use crate::graph::identity::{normalize_path, EdgeId, MaterializedKey};
use crate::graph::types::{
    Confidence, EdgeCategory, EdgeData, EdgeKind, EntityKind, Language, NodeData, NodeKind,
    OverlayStatus, ParseFailure, SourceLocation, UnresolvedImport, UnresolvedReason,
    UnsupportedConstruct, UnsupportedConstructType,
};
use crate::health::compatibility::{CompatibilityDetail, SupportStatus};
use crate::profile::GraphProfile;
use crate::workspace::{CargoDependencyKind, CargoPackageInfo, CargoWorkspaceMeta, WorkspaceInfo};

use super::{CompatibilityAssessment, Detector, DetectorError, DetectorReport, DetectorSink};

// ---------------------------------------------------------------------------
// tree-sitter query constants
// ---------------------------------------------------------------------------

/// Query for `mod foo;` declarations.
const MOD_QUERY: &str = "(mod_item name: (identifier) @mod_name)";

/// Query for `use ...;` declarations.
const USE_QUERY: &str = "(use_declaration argument: (_) @use_path)";

/// Query for `#[cfg(...)]` attributes.
const CFG_ATTR_QUERY: &str = r#"
(attribute_item
  (attribute
    (identifier) @attr_name
    (#eq? @attr_name "cfg")))
"#;

/// Query for `include!(...)` macro invocations.
const INCLUDE_QUERY: &str = r#"
(macro_invocation
  macro: (identifier) @macro_name
  (#eq? @macro_name "include"))
"#;

// ---------------------------------------------------------------------------
// RustDetector
// ---------------------------------------------------------------------------

/// Rust detector using `cargo_metadata` and tree-sitter.
pub struct RustDetector;

impl Detector for RustDetector {
    fn name(&self) -> &str {
        "rust-cargo"
    }

    fn language(&self) -> Language {
        Language::Rust
    }

    fn applies_to(&self, workspace: &WorkspaceInfo) -> bool {
        workspace.cargo.is_some()
    }

    fn compatibility(&self, workspace: &WorkspaceInfo) -> CompatibilityAssessment {
        let cargo = match &workspace.cargo {
            Some(c) => c,
            None => {
                return CompatibilityAssessment {
                    language: Language::Rust,
                    status: SupportStatus::Unsupported,
                    details: vec![CompatibilityDetail {
                        feature: "Cargo workspace".to_string(),
                        status: SupportStatus::Unsupported,
                        explanation: "No Cargo.toml found in workspace".to_string(),
                    }],
                };
            }
        };

        let mut details = Vec::new();
        let mut has_partial = false;

        // Workspace structure — always supported if we got here
        details.push(CompatibilityDetail {
            feature: "Cargo workspace structure".to_string(),
            status: SupportStatus::Supported,
            explanation: format!(
                "{} workspace package(s) discovered via cargo_metadata",
                cargo.packages.len()
            ),
        });

        // Inter-crate dependencies — supported
        details.push(CompatibilityDetail {
            feature: "Inter-crate dependencies".to_string(),
            status: SupportStatus::Supported,
            explanation: "Normal/dev/build dependency kinds extracted from cargo_metadata"
                .to_string(),
        });

        // Check for build.rs files
        let build_script_crates: Vec<&str> = cargo
            .packages
            .iter()
            .filter(|p| p.has_build_script)
            .map(|p| p.name.as_str())
            .collect();
        if !build_script_crates.is_empty() {
            has_partial = true;
            details.push(CompatibilityDetail {
                feature: "build.rs scripts".to_string(),
                status: SupportStatus::Partial,
                explanation: format!(
                    "Found build.rs in: {}. Generated code and custom cfg flags from build scripts are not analyzed.",
                    build_script_crates.join(", ")
                ),
            });
        }

        // Check for proc-macro crates
        let proc_macro_crates: Vec<&str> = cargo
            .packages
            .iter()
            .filter(|p| p.is_proc_macro)
            .map(|p| p.name.as_str())
            .collect();
        if !proc_macro_crates.is_empty() {
            has_partial = true;
            details.push(CompatibilityDetail {
                feature: "Procedural macros".to_string(),
                status: SupportStatus::Partial,
                explanation: format!(
                    "Proc-macro crate(s): {}. Macro expansion output is not analyzed — only the crate dependency is captured.",
                    proc_macro_crates.join(", ")
                ),
            });
        }

        // Check for cfg usage in features
        let total_features: usize = cargo.packages.iter().map(|p| p.features.len()).sum();
        if total_features > 0 {
            details.push(CompatibilityDetail {
                feature: "Cargo features / cfg gates".to_string(),
                status: SupportStatus::Partial,
                explanation: format!(
                    "{total_features} feature(s) defined across workspace. Default features are assumed; non-default cfg gates are not evaluated."
                ),
            });
            has_partial = true;
        }

        // Module structure — supported (via tree-sitter)
        details.push(CompatibilityDetail {
            feature: "Module hierarchy (mod/use/pub use)".to_string(),
            status: SupportStatus::Supported,
            explanation: "Parsed via tree-sitter — will be available after full scan".to_string(),
        });

        let status = if has_partial {
            SupportStatus::Partial
        } else {
            SupportStatus::Supported
        };

        CompatibilityAssessment {
            language: Language::Rust,
            status,
            details,
        }
    }

    #[tracing::instrument(skip(self, workspace, _profile, config, sink))]
    fn detect(
        &self,
        workspace: &WorkspaceInfo,
        _profile: &GraphProfile,
        config: &RepoConfig,
        sink: &dyn DetectorSink,
    ) -> Result<DetectorReport, DetectorError> {
        let cargo = workspace.cargo.as_ref().ok_or_else(|| {
            DetectorError::DetectionFailed {
                name: self.name().to_string(),
                reason: "no Cargo workspace metadata available".to_string(),
            }
        })?;

        let ignore_set = config.ignore_glob_set();

        let mut report = DetectorReport {
            nodes_discovered: 0,
            edges_discovered: 0,
            unsupported_constructs: Vec::new(),
            parse_failures: Vec::new(),
            unresolved_imports: Vec::new(),
        };

        // Phase 1: Package topology from cargo_metadata
        let (pkg_nodes, pkg_edges) =
            detect_package_topology(cargo, &workspace.root, &mut report)?;
        let phase1_node_count = pkg_nodes.len();
        let phase1_edge_count = pkg_edges.len();
        sink.on_nodes(pkg_nodes);
        sink.on_edges(pkg_edges);

        // Phase 2: Module structure from tree-sitter
        let (mod_nodes, mod_edges) = detect_module_structure(
            cargo,
            &workspace.root,
            ignore_set.as_ref(),
            &mut report,
        )?;
        let phase2_node_count = mod_nodes.len();
        let phase2_edge_count = mod_edges.len();
        sink.on_nodes(mod_nodes);
        sink.on_edges(mod_edges);

        // Phase 3: File-level nodes and import edges from tree-sitter
        let (file_nodes, file_edges) = detect_file_edges(
            cargo,
            &workspace.root,
            ignore_set.as_ref(),
            &mut report,
        )?;
        let phase3_node_count = file_nodes.len();
        let phase3_edge_count = file_edges.len();
        sink.on_nodes(file_nodes);
        sink.on_edges(file_edges);

        report.nodes_discovered = phase1_node_count + phase2_node_count + phase3_node_count;
        report.edges_discovered = phase1_edge_count + phase2_edge_count + phase3_edge_count;

        Ok(report)
    }
}

// ---------------------------------------------------------------------------
// Phase 1: Package topology
// ---------------------------------------------------------------------------

/// Extract package nodes and inter-package dependency edges from cargo_metadata.
fn detect_package_topology(
    cargo: &CargoWorkspaceMeta,
    workspace_root: &Utf8Path,
    report: &mut DetectorReport,
) -> Result<(Vec<NodeData>, Vec<EdgeData>), DetectorError> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // Create a package name -> relative_path map for MaterializedKey construction
    let pkg_relative_paths: HashMap<&str, String> = cargo
        .packages
        .iter()
        .map(|p| {
            let pkg_dir = p
                .manifest_path
                .parent()
                .unwrap_or(Utf8Path::new(""));
            let relative = pkg_dir
                .strip_prefix(workspace_root)
                .unwrap_or(pkg_dir)
                .to_string();
            (p.name.as_str(), relative)
        })
        .collect();

    // Create a node for each workspace package
    for pkg in &cargo.packages {
        let relative_path = pkg_relative_paths
            .get(pkg.name.as_str())
            .cloned()
            .unwrap_or_default();

        let key = MaterializedKey::new(Language::Rust, EntityKind::Package, &relative_path);

        nodes.push(NodeData {
            materialized_key: key,
            lineage_key: None,
            label: pkg.name.clone(),
            kind: NodeKind::Package,
            language: Language::Rust,
            parent_key: None,
        });

        // Detect unsupported constructs at package level
        if pkg.has_build_script {
            let build_rs_path = Utf8PathBuf::from(&relative_path).join("build.rs");
            report.unsupported_constructs.push(UnsupportedConstruct {
                construct_type: UnsupportedConstructType::BuildScript,
                location: SourceLocation {
                    path: build_rs_path,
                    start_line: 1,
                    end_line: 1,
                },
                impact: format!(
                    "build.rs in {} may generate code or set cfg flags not visible to static analysis",
                    pkg.name
                ),
                how_to_address: "Review build.rs output and add manual edges if needed".to_string(),
            });
        }

        if pkg.is_proc_macro {
            let src_path = pkg
                .targets
                .iter()
                .find(|t| t.kinds.iter().any(|k| k == "proc-macro"))
                .map(|t| {
                    t.src_path
                        .strip_prefix(workspace_root)
                        .unwrap_or(&t.src_path)
                        .to_owned()
                })
                .unwrap_or_else(|| Utf8PathBuf::from(&relative_path).join("src/lib.rs"));

            report.unsupported_constructs.push(UnsupportedConstruct {
                construct_type: UnsupportedConstructType::ProcMacro,
                location: SourceLocation {
                    path: src_path,
                    start_line: 1,
                    end_line: 1,
                },
                impact: format!(
                    "proc-macro crate {} — macro expansion output is not analyzed",
                    pkg.name
                ),
                how_to_address:
                    "Dependency edges to the proc-macro crate are captured, but expanded code is not"
                        .to_string(),
            });
        }
    }

    // Create inter-package dependency edges
    let pkg_names: std::collections::HashSet<&str> =
        cargo.packages.iter().map(|p| p.name.as_str()).collect();

    for pkg in &cargo.packages {
        let source_path = pkg_relative_paths
            .get(pkg.name.as_str())
            .cloned()
            .unwrap_or_default();
        let source_key =
            MaterializedKey::new(Language::Rust, EntityKind::Package, &source_path);

        for dep in &pkg.dependencies {
            // Only create edges to workspace-internal packages
            if !pkg_names.contains(dep.name.as_str()) {
                continue;
            }

            let target_path = pkg_relative_paths
                .get(dep.name.as_str())
                .cloned()
                .unwrap_or_default();
            let target_key =
                MaterializedKey::new(Language::Rust, EntityKind::Package, &target_path);

            let category = match dep.kind {
                CargoDependencyKind::Normal => EdgeCategory::Normal,
                CargoDependencyKind::Dev => EdgeCategory::Dev,
                CargoDependencyKind::Build => EdgeCategory::Build,
            };

            edges.push(EdgeData {
                edge_id: EdgeId::new(&source_key, &target_key, EdgeKind::DependsOn, category),
                source_key: source_key.clone(),
                target_key,
                kind: EdgeKind::DependsOn,
                category,
                confidence: Confidence::Structural,
                source_location: None,
                resolution_method: Some("cargo_metadata".to_string()),
                overlay_status: OverlayStatus::None,
            });
        }
    }

    Ok((nodes, edges))
}

// ---------------------------------------------------------------------------
// Phase 2: Module structure
// ---------------------------------------------------------------------------

/// Discover module hierarchy by parsing `mod` declarations with tree-sitter.
fn detect_module_structure(
    cargo: &CargoWorkspaceMeta,
    workspace_root: &Utf8Path,
    ignore_set: Option<&globset::GlobSet>,
    report: &mut DetectorReport,
) -> Result<(Vec<NodeData>, Vec<EdgeData>), DetectorError> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let language: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
    let mod_query = Query::new(&language, MOD_QUERY).map_err(|e| {
        DetectorError::DetectionFailed {
            name: "rust-cargo".to_string(),
            reason: format!("invalid mod query: {e}"),
        }
    })?;

    for pkg in &cargo.packages {
        let pkg_dir = pkg
            .manifest_path
            .parent()
            .unwrap_or(Utf8Path::new(""));
        let pkg_relative = pkg_dir
            .strip_prefix(workspace_root)
            .unwrap_or(pkg_dir);
        let pkg_key = MaterializedKey::new(
            Language::Rust,
            EntityKind::Package,
            pkg_relative.as_str(),
        );

        // Find the lib or bin entry point
        let entry_points = find_entry_points(pkg);

        for entry_path in &entry_points {
            if !entry_path.exists() {
                continue;
            }

            let entry_relative = entry_path
                .strip_prefix(workspace_root)
                .unwrap_or(entry_path);

            if is_ignored(entry_relative.as_str(), ignore_set) {
                continue;
            }

            let entry_dir = entry_path.parent().unwrap_or(Utf8Path::new(""));

            // Recursively discover modules from the entry point
            discover_modules_recursive(
                entry_path,
                entry_dir,
                workspace_root,
                &pkg_key,
                &language,
                &mod_query,
                ignore_set,
                &mut nodes,
                &mut edges,
                report,
            );
        }
    }

    Ok((nodes, edges))
}

/// Find entry point source files for a package (lib.rs or main.rs).
fn find_entry_points(pkg: &CargoPackageInfo) -> Vec<Utf8PathBuf> {
    let mut entry_points = Vec::new();

    // Prefer lib target, then bin targets
    for target in &pkg.targets {
        let is_lib = target.kinds.iter().any(|k| k == "lib" || k == "proc-macro");
        let is_bin = target.kinds.iter().any(|k| k == "bin");
        if is_lib || is_bin {
            entry_points.push(target.src_path.clone());
        }
    }

    // Fallback: common locations
    if entry_points.is_empty() {
        let pkg_dir = pkg.manifest_path.parent().unwrap_or(Utf8Path::new(""));
        let lib_rs = pkg_dir.join("src/lib.rs");
        let main_rs = pkg_dir.join("src/main.rs");
        if lib_rs.exists() {
            entry_points.push(lib_rs);
        }
        if main_rs.exists() {
            entry_points.push(main_rs);
        }
    }

    entry_points
}

/// Recursively discover `mod` declarations in a Rust source file.
#[allow(clippy::too_many_arguments)]
fn discover_modules_recursive(
    file_path: &Utf8Path,
    source_dir: &Utf8Path,
    workspace_root: &Utf8Path,
    parent_key: &MaterializedKey,
    language: &tree_sitter::Language,
    mod_query: &Query,
    ignore_set: Option<&globset::GlobSet>,
    nodes: &mut Vec<NodeData>,
    edges: &mut Vec<EdgeData>,
    report: &mut DetectorReport,
) {
    let source = match std::fs::read_to_string(file_path.as_std_path()) {
        Ok(s) => s,
        Err(_) => return,
    };

    let mut parser = Parser::new();
    if parser.set_language(language).is_err() {
        return;
    }

    let tree = match parser.parse(&source, None) {
        Some(t) => t,
        None => {
            report.parse_failures.push(ParseFailure {
                path: file_path
                    .strip_prefix(workspace_root)
                    .unwrap_or(file_path)
                    .to_owned(),
                reason: "tree-sitter parse returned None".to_string(),
            });
            return;
        }
    };

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(mod_query, tree.root_node(), source.as_bytes());

    while let Some(m) = matches.next() {
        for capture in m.captures {
            let mod_name = match capture.node.utf8_text(source.as_bytes()) {
                Ok(s) => s.to_string(),
                Err(_) => continue,
            };

            // Resolve mod to a file path: foo.rs or foo/mod.rs
            let mod_file = source_dir.join(format!("{mod_name}.rs"));
            let mod_dir_file = source_dir.join(&mod_name).join("mod.rs");

            let mod_file_exists = mod_file.exists();
            let mod_dir_file_exists = mod_dir_file.exists();

            let resolved_path = if mod_file_exists {
                Some(mod_file)
            } else if mod_dir_file_exists {
                Some(mod_dir_file)
            } else {
                None
            };

            let Some(resolved) = resolved_path else {
                continue;
            };

            let mod_relative = resolved
                .strip_prefix(workspace_root)
                .unwrap_or(&resolved);

            if is_ignored(mod_relative.as_str(), ignore_set) {
                continue;
            }

            // Module key: use parent/name as the module path
            let parent_rel = source_dir
                .strip_prefix(workspace_root)
                .unwrap_or(source_dir);
            let mod_key_str =
                normalize_path(&format!("{}/{}", parent_rel.as_str(), mod_name));

            let mod_key =
                MaterializedKey::new(Language::Rust, EntityKind::Module, &mod_key_str);

            nodes.push(NodeData {
                materialized_key: mod_key.clone(),
                lineage_key: None,
                label: mod_name.clone(),
                kind: NodeKind::Module,
                language: Language::Rust,
                parent_key: Some(parent_key.clone()),
            });

            let contains_edge = EdgeData {
                edge_id: EdgeId::new(
                    parent_key,
                    &mod_key,
                    EdgeKind::Contains,
                    EdgeCategory::Normal,
                ),
                source_key: parent_key.clone(),
                target_key: mod_key.clone(),
                kind: EdgeKind::Contains,
                category: EdgeCategory::Normal,
                confidence: Confidence::Syntactic,
                source_location: Some(SourceLocation {
                    path: file_path
                        .strip_prefix(workspace_root)
                        .unwrap_or(file_path)
                        .to_owned(),
                    start_line: capture.node.start_position().row as u32 + 1,
                    end_line: capture.node.end_position().row as u32 + 1,
                }),
                resolution_method: Some("tree-sitter mod_item".to_string()),
                overlay_status: OverlayStatus::None,
            };
            edges.push(contains_edge);

            // Recursively discover nested modules
            let next_source_dir = if mod_dir_file_exists {
                source_dir.join(&mod_name)
            } else {
                source_dir.to_owned()
            };

            discover_modules_recursive(
                &resolved,
                &next_source_dir,
                workspace_root,
                &mod_key,
                language,
                mod_query,
                ignore_set,
                nodes,
                edges,
                report,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 3: File-level nodes and import edges
// ---------------------------------------------------------------------------

/// Discover file nodes and import edges from `use` declarations.
fn detect_file_edges(
    cargo: &CargoWorkspaceMeta,
    workspace_root: &Utf8Path,
    ignore_set: Option<&globset::GlobSet>,
    report: &mut DetectorReport,
) -> Result<(Vec<NodeData>, Vec<EdgeData>), DetectorError> {
    let language: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
    let use_query = Query::new(&language, USE_QUERY).map_err(|e| {
        DetectorError::DetectionFailed {
            name: "rust-cargo".to_string(),
            reason: format!("invalid use query: {e}"),
        }
    })?;
    let cfg_query = Query::new(&language, CFG_ATTR_QUERY).map_err(|e| {
        DetectorError::DetectionFailed {
            name: "rust-cargo".to_string(),
            reason: format!("invalid cfg query: {e}"),
        }
    })?;
    let include_query = Query::new(&language, INCLUDE_QUERY).map_err(|e| {
        DetectorError::DetectionFailed {
            name: "rust-cargo".to_string(),
            reason: format!("invalid include query: {e}"),
        }
    })?;

    // Build a map: crate_name -> package relative path (for resolving cross-crate use)
    let crate_to_path: HashMap<String, String> = cargo
        .packages
        .iter()
        .map(|p| {
            let pkg_dir = p.manifest_path.parent().unwrap_or(Utf8Path::new(""));
            let relative = pkg_dir
                .strip_prefix(workspace_root)
                .unwrap_or(pkg_dir)
                .to_string();
            // Cargo crate names use underscores in `use` statements
            (p.name.replace('-', "_"), relative)
        })
        .collect();

    // Build a map: dep name -> category for cross-crate edge categories
    let mut dep_categories: HashMap<(String, String), EdgeCategory> = HashMap::new();
    for pkg in &cargo.packages {
        let source_crate = pkg.name.replace('-', "_");
        for dep in &pkg.dependencies {
            let target_crate = dep.name.replace('-', "_");
            let category = match dep.kind {
                CargoDependencyKind::Normal => EdgeCategory::Normal,
                CargoDependencyKind::Dev => EdgeCategory::Dev,
                CargoDependencyKind::Build => EdgeCategory::Build,
            };
            dep_categories.insert((source_crate.clone(), target_crate), category);
        }
    }

    let mut all_nodes = Vec::new();
    let mut all_edges = Vec::new();

    for pkg in &cargo.packages {
        let pkg_dir = pkg.manifest_path.parent().unwrap_or(Utf8Path::new(""));
        let pkg_relative = pkg_dir
            .strip_prefix(workspace_root)
            .unwrap_or(pkg_dir);
        let pkg_key = MaterializedKey::new(
            Language::Rust,
            EntityKind::Package,
            pkg_relative.as_str(),
        );
        let crate_name = pkg.name.replace('-', "_");

        // Walk .rs files in this package
        let rs_files = walk_rs_files(pkg_dir, workspace_root, ignore_set);

        for file_path in &rs_files {
            let file_relative = file_path
                .strip_prefix(workspace_root)
                .unwrap_or(file_path);

            let file_key = MaterializedKey::new(
                Language::Rust,
                EntityKind::File,
                file_relative.as_str(),
            );

            // Determine parent: find the module this file belongs to
            let parent_key = find_parent_module_key(
                file_path,
                pkg_dir,
                workspace_root,
                &pkg_key,
            );

            let file_label = file_path
                .file_name()
                .unwrap_or(file_relative.as_str());

            all_nodes.push(NodeData {
                materialized_key: file_key.clone(),
                lineage_key: None,
                label: file_label.to_string(),
                kind: NodeKind::File,
                language: Language::Rust,
                parent_key: Some(parent_key.clone()),
            });

            // Contains edge from parent to file
            all_edges.push(EdgeData {
                edge_id: EdgeId::new(
                    &parent_key,
                    &file_key,
                    EdgeKind::Contains,
                    EdgeCategory::Normal,
                ),
                source_key: parent_key.clone(),
                target_key: file_key.clone(),
                kind: EdgeKind::Contains,
                category: EdgeCategory::Normal,
                confidence: Confidence::Structural,
                source_location: None,
                resolution_method: Some("filesystem".to_string()),
                overlay_status: OverlayStatus::None,
            });

            // Parse the file for use declarations and unsupported constructs
            let source = match std::fs::read_to_string(file_path.as_std_path()) {
                Ok(s) => s,
                Err(_) => {
                    report.parse_failures.push(ParseFailure {
                        path: file_relative.to_owned(),
                        reason: "could not read file".to_string(),
                    });
                    continue;
                }
            };

            let mut parser = Parser::new();
            if parser.set_language(&language).is_err() {
                continue;
            }

            let tree = match parser.parse(&source, None) {
                Some(t) => t,
                None => {
                    report.parse_failures.push(ParseFailure {
                        path: file_relative.to_owned(),
                        reason: "tree-sitter parse returned None".to_string(),
                    });
                    continue;
                }
            };

            // Extract use declarations
            let import_edges = extract_use_edges(
                &tree,
                &source,
                &use_query,
                &file_key,
                file_relative,
                &crate_name,
                pkg_relative,
                workspace_root,
                &crate_to_path,
                &dep_categories,
                report,
            );
            all_edges.extend(import_edges);

            // Detect #[cfg(...)] on mod items
            detect_cfg_constructs(
                &tree,
                &source,
                &cfg_query,
                file_relative,
                report,
            );

            // Detect include!() macros
            detect_include_constructs(
                &tree,
                &source,
                &include_query,
                file_relative,
                report,
            );
        }
    }

    Ok((all_nodes, all_edges))
}

/// Walk all `.rs` files in a package directory, respecting ignore patterns.
fn walk_rs_files(
    pkg_dir: &Utf8Path,
    workspace_root: &Utf8Path,
    ignore_set: Option<&globset::GlobSet>,
) -> Vec<Utf8PathBuf> {
    let mut files = Vec::new();

    // Use the ignore crate for .gitignore-aware walking
    let walker = ignore::WalkBuilder::new(pkg_dir.as_std_path())
        .hidden(true)
        .git_ignore(true)
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension() else {
            continue;
        };
        if ext != "rs" {
            continue;
        }

        let utf8_path = match Utf8PathBuf::try_from(path.to_path_buf()) {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Check custom ignore patterns
        let relative = utf8_path
            .strip_prefix(workspace_root)
            .unwrap_or(&utf8_path);
        if is_ignored(relative.as_str(), ignore_set) {
            continue;
        }

        // Skip build.rs — it's detected as unsupported, not as a regular file
        if utf8_path.file_name() == Some("build.rs") {
            continue;
        }

        files.push(utf8_path);
    }

    files.sort();
    files
}

/// Extract import edges from `use` declarations in a parsed file.
#[allow(clippy::too_many_arguments)]
fn extract_use_edges(
    tree: &tree_sitter::Tree,
    source: &str,
    use_query: &Query,
    file_key: &MaterializedKey,
    file_relative: &Utf8Path,
    crate_name: &str,
    pkg_relative: &Utf8Path,
    workspace_root: &Utf8Path,
    crate_to_path: &HashMap<String, String>,
    dep_categories: &HashMap<(String, String), EdgeCategory>,
    report: &mut DetectorReport,
) -> Vec<EdgeData> {
    let mut edges = Vec::new();
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(use_query, tree.root_node(), source.as_bytes());

    while let Some(m) = matches.next() {
        for capture in m.captures {
            let use_path = match capture.node.utf8_text(source.as_bytes()) {
                Ok(s) => s.to_string(),
                Err(_) => continue,
            };

            // Skip grouped/tree imports like `use crate::{foo, bar}`
            // These produce a use_tree_list node with braces in the text.
            // File/module granularity doesn't need to decompose them.
            if use_path.contains('{') || use_path.contains('}') {
                continue;
            }

            // Skip glob imports like `use crate::foo::*`
            if use_path.contains('*') {
                continue;
            }

            let start_line = capture.node.start_position().row as u32 + 1;
            let end_line = capture.node.end_position().row as u32 + 1;

            let resolved = resolve_use_path(
                &use_path,
                crate_name,
                pkg_relative,
                file_relative,
                workspace_root,
                crate_to_path,
            );

            let Some((target_key, is_cross_crate, target_crate)) = resolved else {
                // Track the unresolved import with reason
                let segments: Vec<&str> = use_path.split("::").collect();
                let first = segments.first().copied().unwrap_or("");
                let reason = match first {
                    "crate" | "super" | "self" => UnresolvedReason::UnresolvablePath,
                    _ => {
                        if crate_to_path.contains_key(first) {
                            UnresolvedReason::UnresolvablePath
                        } else {
                            UnresolvedReason::ExternalCrate
                        }
                    }
                };
                report.unresolved_imports.push(UnresolvedImport {
                    source_file: file_relative.to_string(),
                    specifier: use_path.clone(),
                    reason,
                });
                continue;
            };

            let category = if is_cross_crate {
                if let Some(tc) = &target_crate {
                    dep_categories
                        .get(&(crate_name.to_string(), tc.clone()))
                        .copied()
                        .unwrap_or(EdgeCategory::Normal)
                } else {
                    EdgeCategory::Normal
                }
            } else {
                EdgeCategory::Normal
            };

            let edge_kind = EdgeKind::Imports;

            edges.push(EdgeData {
                edge_id: EdgeId::new(file_key, &target_key, edge_kind, category),
                source_key: file_key.clone(),
                target_key,
                kind: edge_kind,
                category,
                confidence: if is_cross_crate {
                    Confidence::Structural
                } else {
                    Confidence::Syntactic
                },
                source_location: Some(SourceLocation {
                    path: file_relative.to_owned(),
                    start_line,
                    end_line,
                }),
                resolution_method: Some("tree-sitter use_declaration".to_string()),
                overlay_status: OverlayStatus::None,
            });
        }
    }

    edges
}

/// Resolve a `use` path to a target `MaterializedKey`.
///
/// Returns `(target_key, is_cross_crate, target_crate_name)` or `None` if unresolvable.
fn resolve_use_path(
    use_path: &str,
    _crate_name: &str,
    pkg_relative: &Utf8Path,
    file_relative: &Utf8Path,
    _workspace_root: &Utf8Path,
    crate_to_path: &HashMap<String, String>,
) -> Option<(MaterializedKey, bool, Option<String>)> {
    let segments: Vec<&str> = use_path.split("::").collect();
    if segments.is_empty() {
        return None;
    }

    let first = segments[0];

    match first {
        "crate" => {
            // use crate::foo::bar → resolve within current crate
            if segments.len() < 2 {
                return None;
            }
            let module_path = normalize_path(&format!(
                "{}/src/{}",
                pkg_relative.as_str(),
                segments[1]
            ));
            Some((
                MaterializedKey::new(Language::Rust, EntityKind::Module, &module_path),
                false,
                None,
            ))
        }
        "super" => {
            // use super::foo → resolve relative to parent module
            let file_dir = file_relative.parent()?;
            let parent_dir = file_dir.parent()?;
            if segments.len() < 2 {
                return None;
            }
            let module_path = normalize_path(&format!(
                "{}/{}",
                parent_dir.as_str(),
                segments[1]
            ));
            Some((
                MaterializedKey::new(Language::Rust, EntityKind::Module, &module_path),
                false,
                None,
            ))
        }
        "self" => {
            // use self::foo → resolve within current module
            let file_dir = file_relative.parent()?;
            if segments.len() < 2 {
                return None;
            }
            let module_path = normalize_path(&format!(
                "{}/{}",
                file_dir.as_str(),
                segments[1]
            ));
            Some((
                MaterializedKey::new(Language::Rust, EntityKind::Module, &module_path),
                false,
                None,
            ))
        }
        _ => {
            // use some_crate::foo → cross-crate dependency
            crate_to_path.get(first).map(|target_pkg_path| {
                (
                    MaterializedKey::new(
                        Language::Rust,
                        EntityKind::Package,
                        target_pkg_path,
                    ),
                    true,
                    Some(first.to_string()),
                )
            })
        }
    }
}

/// Detect `#[cfg(...)]` attributes on `mod` items.
///
/// Finds `#[cfg(...)]` attribute_item nodes and checks if their next
/// sibling is a `mod_item`. This is how tree-sitter-rust structures
/// outer attributes on declarations.
fn detect_cfg_constructs(
    tree: &tree_sitter::Tree,
    source: &str,
    cfg_query: &Query,
    file_relative: &Utf8Path,
    report: &mut DetectorReport,
) {
    let attr_name_idx = cfg_query
        .capture_index_for_name("attr_name")
        .unwrap_or(u32::MAX);
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(cfg_query, tree.root_node(), source.as_bytes());

    while let Some(m) = matches.next() {
        for capture in m.captures {
            if capture.index != attr_name_idx {
                continue;
            }

            // Walk up to the attribute_item node
            let attr_item = match capture.node.parent().and_then(|p| p.parent()) {
                Some(n) if n.kind() == "attribute_item" => n,
                _ => continue,
            };

            // Check if the next sibling is a mod_item
            let next_sibling = attr_item.next_named_sibling();
            let is_on_mod = next_sibling
                .as_ref()
                .is_some_and(|s| s.kind() == "mod_item");

            if !is_on_mod {
                continue;
            }

            // Get the mod name from the sibling
            let mod_name = next_sibling
                .and_then(|s| s.child_by_field_name("name"))
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .unwrap_or("unknown");

            let start_line = attr_item.start_position().row as u32 + 1;
            let end_line = attr_item.end_position().row as u32 + 1;

            report.unsupported_constructs.push(UnsupportedConstruct {
                construct_type: UnsupportedConstructType::CfgGate,
                location: SourceLocation {
                    path: file_relative.to_owned(),
                    start_line,
                    end_line,
                },
                impact: format!(
                    "#[cfg(...)] on module `{mod_name}` — module may be conditionally compiled"
                ),
                how_to_address:
                    "Module is included in graph assuming default features are active"
                        .to_string(),
            });
        }
    }
}

/// Detect `include!(...)` macro invocations.
fn detect_include_constructs(
    tree: &tree_sitter::Tree,
    source: &str,
    include_query: &Query,
    file_relative: &Utf8Path,
    report: &mut DetectorReport,
) {
    let macro_name_idx = include_query
        .capture_index_for_name("macro_name")
        .unwrap_or(u32::MAX);
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(include_query, tree.root_node(), source.as_bytes());

    while let Some(m) = matches.next() {
        for capture in m.captures {
            if capture.index == macro_name_idx {
                let start_line = capture.node.start_position().row as u32 + 1;
                let end_line = capture.node.end_position().row as u32 + 1;

                report.unsupported_constructs.push(UnsupportedConstruct {
                    construct_type: UnsupportedConstructType::IncludeMacro,
                    location: SourceLocation {
                        path: file_relative.to_owned(),
                        start_line,
                        end_line,
                    },
                    impact: "include!() macro includes external file content not visible to static analysis".to_string(),
                    how_to_address: "Review included file and add manual edges if needed".to_string(),
                });
            }
        }
    }
}

/// Find the parent module key for a file.
fn find_parent_module_key(
    file_path: &Utf8Path,
    pkg_dir: &Utf8Path,
    workspace_root: &Utf8Path,
    pkg_key: &MaterializedKey,
) -> MaterializedKey {
    let file_dir = file_path.parent().unwrap_or(Utf8Path::new(""));

    // If the file is directly in src/, the parent is the package
    let src_dir = pkg_dir.join("src");
    if file_dir == src_dir {
        return pkg_key.clone();
    }

    // Otherwise the parent is the containing module directory
    let dir_relative = file_dir
        .strip_prefix(workspace_root)
        .unwrap_or(file_dir);

    MaterializedKey::new(
        Language::Rust,
        EntityKind::Module,
        dir_relative.as_str(),
    )
}

/// Check if a path should be ignored by the custom ignore patterns.
fn is_ignored(relative_path: &str, ignore_set: Option<&globset::GlobSet>) -> bool {
    match ignore_set {
        Some(gs) => gs.is_match(relative_path),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::*;

    fn make_cargo_workspace(packages: Vec<CargoPackageInfo>) -> WorkspaceInfo {
        WorkspaceInfo {
            root: Utf8PathBuf::from("/tmp/test"),
            kind: WorkspaceKind::Cargo,
            packages: packages
                .iter()
                .map(|p| WorkspacePackage {
                    name: p.name.clone(),
                    relative_path: format!("crates/{}", p.name),
                    language: Language::Rust,
                })
                .collect(),
            cargo: Some(CargoWorkspaceMeta {
                workspace_root: Utf8PathBuf::from("/tmp/test"),
                packages,
            }),
            js: None,
        }
    }

    #[test]
    fn supported_basic_workspace() {
        let ws = make_cargo_workspace(vec![CargoPackageInfo {
            name: "my-lib".to_string(),
            version: "0.1.0".to_string(),
            manifest_path: Utf8PathBuf::from("/tmp/test/Cargo.toml"),
            has_build_script: false,
            is_proc_macro: false,
            features: Vec::new(),
            dependencies: Vec::new(),
            targets: Vec::new(),
        }]);

        let detector = RustDetector;
        assert!(detector.applies_to(&ws));

        let assessment = detector.compatibility(&ws);
        assert_eq!(assessment.language, Language::Rust);
        assert_eq!(assessment.status, SupportStatus::Supported);
    }

    #[test]
    fn partial_with_build_script() {
        let ws = make_cargo_workspace(vec![CargoPackageInfo {
            name: "my-lib".to_string(),
            version: "0.1.0".to_string(),
            manifest_path: Utf8PathBuf::from("/tmp/test/Cargo.toml"),
            has_build_script: true,
            is_proc_macro: false,
            features: Vec::new(),
            dependencies: Vec::new(),
            targets: Vec::new(),
        }]);

        let assessment = RustDetector.compatibility(&ws);
        assert_eq!(assessment.status, SupportStatus::Partial);
        assert!(assessment
            .details
            .iter()
            .any(|d| d.feature.contains("build.rs")));
    }

    #[test]
    fn partial_with_proc_macro() {
        let ws = make_cargo_workspace(vec![CargoPackageInfo {
            name: "my-macro".to_string(),
            version: "0.1.0".to_string(),
            manifest_path: Utf8PathBuf::from("/tmp/test/Cargo.toml"),
            has_build_script: false,
            is_proc_macro: true,
            features: Vec::new(),
            dependencies: Vec::new(),
            targets: Vec::new(),
        }]);

        let assessment = RustDetector.compatibility(&ws);
        assert_eq!(assessment.status, SupportStatus::Partial);
        assert!(assessment
            .details
            .iter()
            .any(|d| d.feature.contains("Procedural macros")));
    }

    #[test]
    fn no_cargo_metadata_returns_unsupported() {
        let ws = WorkspaceInfo {
            root: Utf8PathBuf::from("/tmp/empty"),
            kind: WorkspaceKind::Single,
            packages: Vec::new(),
            cargo: None,
            js: None,
        };

        let detector = RustDetector;
        assert!(!detector.applies_to(&ws));

        let assessment = detector.compatibility(&ws);
        assert_eq!(assessment.status, SupportStatus::Unsupported);
    }

    // --- M4 tree-sitter tests ---

    #[test]
    fn tree_sitter_extracts_mod_declarations() {
        let source = r#"
mod foo;
mod bar;
pub mod baz;
"#;
        let language: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&language).expect("set language");
        let tree = parser.parse(source, None).expect("parse");

        let query = Query::new(&language, MOD_QUERY).expect("mod query");
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        let mut mod_names = Vec::new();

        while let Some(m) = matches.next() {
            for capture in m.captures {
                let name = capture.node.utf8_text(source.as_bytes()).expect("text");
                mod_names.push(name.to_string());
            }
        }

        assert_eq!(mod_names, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn tree_sitter_extracts_use_declarations() {
        let source = r#"
use crate::foo::bar;
use super::baz;
use std::collections::HashMap;
pub use crate::graph::types::NodeData;
"#;
        let language: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&language).expect("set language");
        let tree = parser.parse(source, None).expect("parse");

        let query = Query::new(&language, USE_QUERY).expect("use query");
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        let mut use_paths = Vec::new();

        while let Some(m) = matches.next() {
            for capture in m.captures {
                let path = capture.node.utf8_text(source.as_bytes()).expect("text");
                use_paths.push(path.to_string());
            }
        }

        assert_eq!(use_paths.len(), 4);
        assert!(use_paths[0].contains("crate::foo::bar"));
        assert!(use_paths[1].contains("super::baz"));
        assert!(use_paths[2].contains("std::collections::HashMap"));
        assert!(use_paths[3].contains("crate::graph::types::NodeData"));
    }

    #[test]
    fn resolve_use_path_crate_internal() {
        let crate_to_path = HashMap::new();
        let result = resolve_use_path(
            "crate::graph::types",
            "codeatlas_core",
            Utf8Path::new("crates/codeatlas-core"),
            Utf8Path::new("crates/codeatlas-core/src/lib.rs"),
            Utf8Path::new("/workspace"),
            &crate_to_path,
        );
        assert!(result.is_some());
        let (key, is_cross, _) = result.expect("should resolve");
        assert!(!is_cross);
        assert_eq!(key.entity_kind, EntityKind::Module);
        assert!(key.relative_path.contains("graph"));
    }

    #[test]
    fn resolve_use_path_cross_crate() {
        let mut crate_to_path = HashMap::new();
        crate_to_path.insert("other_crate".to_string(), "crates/other".to_string());

        let result = resolve_use_path(
            "other_crate::thing",
            "my_crate",
            Utf8Path::new("crates/my-crate"),
            Utf8Path::new("crates/my-crate/src/lib.rs"),
            Utf8Path::new("/workspace"),
            &crate_to_path,
        );
        assert!(result.is_some());
        let (key, is_cross, target_crate) = result.expect("should resolve");
        assert!(is_cross);
        assert_eq!(key.entity_kind, EntityKind::Package);
        assert_eq!(key.relative_path, "crates/other");
        assert_eq!(target_crate, Some("other_crate".to_string()));
    }

    #[test]
    fn resolve_use_path_super() {
        let crate_to_path = HashMap::new();
        let result = resolve_use_path(
            "super::sibling",
            "my_crate",
            Utf8Path::new("crates/my-crate"),
            Utf8Path::new("crates/my-crate/src/graph/types.rs"),
            Utf8Path::new("/workspace"),
            &crate_to_path,
        );
        assert!(result.is_some());
        let (key, is_cross, _) = result.expect("should resolve");
        assert!(!is_cross);
        assert_eq!(key.entity_kind, EntityKind::Module);
        assert!(key.relative_path.contains("sibling"));
    }

    #[test]
    fn resolve_use_path_external_crate_returns_none() {
        let crate_to_path = HashMap::new();
        let result = resolve_use_path(
            "serde::Serialize",
            "my_crate",
            Utf8Path::new("crates/my-crate"),
            Utf8Path::new("crates/my-crate/src/lib.rs"),
            Utf8Path::new("/workspace"),
            &crate_to_path,
        );
        assert!(result.is_none());
    }

    #[test]
    fn detect_cfg_on_mod_item() {
        let source = r#"
#[cfg(feature = "serde-support")]
mod serde_impl;
"#;
        let language: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&language).expect("set language");
        let tree = parser.parse(source, None).expect("parse");

        let query = Query::new(&language, CFG_ATTR_QUERY).expect("cfg query");
        let mut report = DetectorReport {
            nodes_discovered: 0,
            edges_discovered: 0,
            unsupported_constructs: Vec::new(),
            parse_failures: Vec::new(),
            unresolved_imports: Vec::new(),
        };

        detect_cfg_constructs(
            &tree,
            source,
            &query,
            Utf8Path::new("src/lib.rs"),
            &mut report,
        );

        assert_eq!(report.unsupported_constructs.len(), 1);
        assert_eq!(
            report.unsupported_constructs[0].construct_type,
            UnsupportedConstructType::CfgGate
        );
    }

    #[test]
    fn edge_category_from_dep_kind() {
        assert_eq!(
            match CargoDependencyKind::Normal {
                CargoDependencyKind::Normal => EdgeCategory::Normal,
                CargoDependencyKind::Dev => EdgeCategory::Dev,
                CargoDependencyKind::Build => EdgeCategory::Build,
            },
            EdgeCategory::Normal
        );
        assert_eq!(
            match CargoDependencyKind::Dev {
                CargoDependencyKind::Normal => EdgeCategory::Normal,
                CargoDependencyKind::Dev => EdgeCategory::Dev,
                CargoDependencyKind::Build => EdgeCategory::Build,
            },
            EdgeCategory::Dev
        );
        assert_eq!(
            match CargoDependencyKind::Build {
                CargoDependencyKind::Normal => EdgeCategory::Normal,
                CargoDependencyKind::Dev => EdgeCategory::Dev,
                CargoDependencyKind::Build => EdgeCategory::Build,
            },
            EdgeCategory::Build
        );
    }

    #[test]
    fn detect_on_this_projects_workspace() {
        let project_root = Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("should find project root");

        let ws = crate::workspace::discover_workspace(project_root)
            .expect("discovery should succeed");

        let config = RepoConfig::default_config();
        let profile = GraphProfile::empty();
        let detector = RustDetector;

        // Use a collecting sink to verify results
        let sink = CollectingSink::default();
        let report = detector
            .detect(&ws, &profile, &config, &sink)
            .expect("detect should succeed");

        // Should discover nodes and edges
        assert!(report.nodes_discovered > 0, "should discover nodes");
        assert!(report.edges_discovered > 0, "should discover edges");

        let nodes = sink.nodes();
        let edges = sink.edges();

        // Should find package nodes
        let pkg_nodes: Vec<_> = nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Package)
            .collect();
        assert!(
            pkg_nodes.len() >= 2,
            "should find at least 2 packages (codeatlas-core, codeatlas-tauri)"
        );

        // Should find module nodes
        let mod_nodes: Vec<_> = nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Module)
            .collect();
        assert!(!mod_nodes.is_empty(), "should find module nodes");

        // Should find file nodes
        let file_nodes: Vec<_> = nodes
            .iter()
            .filter(|n| n.kind == NodeKind::File)
            .collect();
        assert!(!file_nodes.is_empty(), "should find file nodes");

        // Should have dependency edges between packages
        let dep_edges: Vec<_> = edges
            .iter()
            .filter(|e| e.kind == EdgeKind::DependsOn)
            .collect();
        assert!(!dep_edges.is_empty(), "should find inter-package dependency edges");
    }

    /// A collecting sink for testing — stores all nodes and edges.
    #[derive(Default)]
    struct CollectingSink {
        nodes: std::sync::Mutex<Vec<NodeData>>,
        edges: std::sync::Mutex<Vec<EdgeData>>,
    }

    impl CollectingSink {
        fn nodes(&self) -> Vec<NodeData> {
            self.nodes.lock().expect("lock nodes").clone()
        }
        fn edges(&self) -> Vec<EdgeData> {
            self.edges.lock().expect("lock edges").clone()
        }
    }

    impl DetectorSink for CollectingSink {
        fn on_nodes(&self, nodes: Vec<NodeData>) {
            self.nodes.lock().expect("lock nodes").extend(nodes);
        }
        fn on_edges(&self, edges: Vec<EdgeData>) {
            self.edges.lock().expect("lock edges").extend(edges);
        }
    }
}
