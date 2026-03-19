//! TypeScript/JavaScript detector — workspace package discovery + tree-sitter parsing.
//!
//! M2: implements `compatibility()` (structural/manifest-level assessment).
//! M5: implements `detect()` (tree-sitter parsing, import resolution, value/type-only classification).

use std::collections::HashMap;

use camino::{Utf8Path, Utf8PathBuf};
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

use crate::config::RepoConfig;
use crate::graph::identity::{EdgeId, MaterializedKey};
use crate::graph::types::{
    Confidence, EdgeCategory, EdgeData, EdgeKind, EntityKind, Language, NodeData, NodeKind,
    OverlayStatus, ParseFailure, SourceLocation, UnresolvedImport, UnresolvedReason,
    UnsupportedConstruct, UnsupportedConstructType,
};
use crate::health::compatibility::{CompatibilityDetail, SupportStatus};
use crate::profile::GraphProfile;
use crate::workspace::{JsWorkspaceMeta, TsconfigInfo, WorkspaceInfo};

use super::{CompatibilityAssessment, Detector, DetectorError, DetectorReport, DetectorSink};

// ---------------------------------------------------------------------------
// tree-sitter query constants
// ---------------------------------------------------------------------------

/// Query for all import statements (captures the source string).
const IMPORT_QUERY: &str = r#"
(import_statement
  source: (string) @source
) @import
"#;

/// Query for statement-level `import type` (the anonymous "type" literal).
const IMPORT_TYPE_QUERY: &str = r#"
(import_statement
  "type"
  source: (string) @source
) @type_import
"#;

/// Query for re-exports: `export { ... } from '...'` and `export * from '...'`.
const REEXPORT_QUERY: &str = r#"
(export_statement
  source: (string) @source
) @reexport
"#;

/// Query for dynamic imports: `import('...')`.
const DYNAMIC_IMPORT_QUERY: &str = r#"
(call_expression
  function: (import)
  arguments: (arguments (string) @source)
) @dynamic_import
"#;

/// Query for `require()` calls.
const REQUIRE_QUERY: &str = r#"
(call_expression
  function: (identifier) @fn_name
  arguments: (arguments (string) @source)
  (#eq? @fn_name "require")
) @require_call
"#;

// ---------------------------------------------------------------------------
// TsConfigResolver
// ---------------------------------------------------------------------------

/// POC-scope TypeScript import resolver.
///
/// Resolves import specifiers using:
/// 1. Workspace package names (bare specifiers)
/// 2. tsconfig `paths` aliases
/// 3. Relative paths with extension probing
/// 4. tsconfig `baseUrl` for non-relative specifiers
///
/// Does NOT resolve: `exports`/`imports` conditions, PnP, project references,
/// `node_modules`, or `require()` paths.
pub struct TsConfigResolver {
    /// tsconfig `compilerOptions.baseUrl`, resolved to absolute path.
    base_url: Option<Utf8PathBuf>,
    /// tsconfig `compilerOptions.paths` — (pattern_prefix, pattern_suffix, replacement_templates).
    paths: Vec<PathMapping>,
    /// Workspace package name → package root directory (absolute).
    workspace_packages: HashMap<String, Utf8PathBuf>,
}

/// A single tsconfig paths mapping entry.
struct PathMapping {
    /// The prefix before the `*` wildcard (e.g., `@/` for `@/*`).
    prefix: String,
    /// The suffix after the `*` wildcard (empty for patterns like `@/*`).
    suffix: String,
    /// Replacement templates (e.g., `["./src/*"]`).
    replacements: Vec<String>,
}

/// Result of import resolution.
#[derive(Debug)]
#[allow(dead_code)] // Unresolved reason string is kept for diagnostics
pub(crate) enum ResolveResult {
    /// Resolved to a workspace package.
    Package(MaterializedKey),
    /// Resolved to a specific file.
    File(MaterializedKey),
    /// Could not resolve — with a human-readable reason.
    Unresolved(String),
}

/// Extension probing order for TypeScript module resolution.
const EXTENSION_PROBE_ORDER: &[&str] = &[
    ".ts", ".tsx", ".js", ".jsx",
    "/index.ts", "/index.tsx", "/index.js", "/index.jsx",
];

impl TsConfigResolver {
    /// Build a resolver from workspace metadata and tsconfig info.
    pub fn new(
        js_meta: &JsWorkspaceMeta,
        tsconfig: Option<&TsconfigInfo>,
        workspace_root: &Utf8Path,
    ) -> Self {
        let mut workspace_packages = HashMap::new();
        for pkg in &js_meta.packages {
            let pkg_dir = workspace_root.join(&pkg.relative_path);
            workspace_packages.insert(pkg.name.clone(), pkg_dir);
        }

        let (base_url, paths) = if let Some(tsconfig) = tsconfig {
            let tsconfig_dir = tsconfig
                .path
                .parent()
                .unwrap_or(workspace_root);

            let base_url = Self::read_base_url(tsconfig_dir);
            let paths = Self::read_paths(tsconfig_dir);
            (base_url, paths)
        } else {
            (None, Vec::new())
        };

        Self {
            base_url,
            paths,
            workspace_packages,
        }
    }

    /// Resolve an import specifier from a given file.
    ///
    /// Returns the resolved target or an unresolved reason.
    pub(crate) fn resolve(
        &self,
        specifier: &str,
        from_file: &Utf8Path,
        workspace_root: &Utf8Path,
    ) -> ResolveResult {
        // 1. Bare specifier matching a workspace package name
        if let Some(pkg_dir) = self.workspace_packages.get(specifier) {
            let relative = pkg_dir
                .strip_prefix(workspace_root)
                .unwrap_or(pkg_dir);
            return ResolveResult::Package(MaterializedKey::new(
                Language::TypeScript,
                EntityKind::Package,
                relative.as_str(),
            ));
        }

        // Also check if specifier starts with a workspace package name + /
        // e.g., "@fixture/shared/utils" → resolve to the package
        for (name, pkg_dir) in &self.workspace_packages {
            if specifier.starts_with(name.as_str())
                && specifier.get(name.len()..name.len() + 1) == Some("/")
            {
                let relative = pkg_dir
                    .strip_prefix(workspace_root)
                    .unwrap_or(pkg_dir);
                return ResolveResult::Package(MaterializedKey::new(
                    Language::TypeScript,
                    EntityKind::Package,
                    relative.as_str(),
                ));
            }
        }

        // 2. tsconfig paths alias
        for mapping in &self.paths {
            if let Some(resolved) =
                self.try_path_mapping(mapping, specifier, workspace_root)
            {
                return resolved;
            }
        }

        // 3. Relative path
        if specifier.starts_with("./") || specifier.starts_with("../") {
            return self.resolve_relative(specifier, from_file, workspace_root);
        }

        // 4. baseUrl
        if let Some(ref base) = self.base_url
            && let Some(resolved) = self.probe_extensions(base, specifier, workspace_root)
        {
            return resolved;
        }

        // 5. Unresolved — external package or unknown specifier
        if specifier.starts_with('@') || !specifier.contains('/') || specifier.contains("node_modules") {
            ResolveResult::Unresolved(format!(
                "external package (not in workspace): {specifier}"
            ))
        } else {
            ResolveResult::Unresolved(format!(
                "could not resolve specifier: {specifier}"
            ))
        }
    }

    /// Try to resolve a specifier through a tsconfig paths mapping.
    fn try_path_mapping(
        &self,
        mapping: &PathMapping,
        specifier: &str,
        workspace_root: &Utf8Path,
    ) -> Option<ResolveResult> {
        // Check if specifier matches the pattern
        if !specifier.starts_with(&mapping.prefix) {
            return None;
        }
        if !mapping.suffix.is_empty() && !specifier.ends_with(&mapping.suffix) {
            return None;
        }

        // Extract the wildcard capture
        let capture_start = mapping.prefix.len();
        let capture_end = specifier.len() - mapping.suffix.len();
        if capture_start > capture_end {
            return None;
        }
        let capture = &specifier[capture_start..capture_end];

        // Try each replacement template
        for template in &mapping.replacements {
            let replaced = template.replace('*', capture);
            // The replacement is relative to tsconfig dir (which is where base_url/paths resolve from)
            // Try resolving as a relative path from the tsconfig directory
            let tsconfig_dir = self.base_url.as_deref().unwrap_or(workspace_root);
            if let Some(resolved) = self.probe_extensions(tsconfig_dir, &replaced, workspace_root) {
                return Some(resolved);
            }
        }

        None
    }

    /// Resolve a relative import specifier.
    fn resolve_relative(
        &self,
        specifier: &str,
        from_file: &Utf8Path,
        workspace_root: &Utf8Path,
    ) -> ResolveResult {
        let from_dir = from_file.parent().unwrap_or(Utf8Path::new(""));

        // If specifier already has an extension, try it directly
        let candidate = from_dir.join(specifier);
        if candidate.exists() && candidate.is_file() {
            let relative = candidate
                .strip_prefix(workspace_root)
                .unwrap_or(&candidate);
            return ResolveResult::File(MaterializedKey::new(
                Language::TypeScript,
                EntityKind::File,
                relative.as_str(),
            ));
        }

        // Extension probing
        if let Some(resolved) = self.probe_extensions(from_dir, specifier, workspace_root) {
            return resolved;
        }

        ResolveResult::Unresolved(format!(
            "no matching file for relative import: {specifier}"
        ))
    }

    /// Try appending standard extensions to find a file.
    fn probe_extensions(
        &self,
        base_dir: &Utf8Path,
        specifier: &str,
        workspace_root: &Utf8Path,
    ) -> Option<ResolveResult> {
        // Strip leading ./ from specifier for joining
        let clean_spec = specifier.strip_prefix("./").unwrap_or(specifier);

        for ext in EXTENSION_PROBE_ORDER {
            let candidate = base_dir.join(format!("{clean_spec}{ext}"));
            if candidate.exists() && candidate.is_file() {
                let relative = candidate
                    .strip_prefix(workspace_root)
                    .unwrap_or(&candidate);
                return Some(ResolveResult::File(MaterializedKey::new(
                    Language::TypeScript,
                    EntityKind::File,
                    relative.as_str(),
                )));
            }
        }

        None
    }

    /// Read `baseUrl` from a tsconfig's directory (actually reads the file).
    fn read_base_url(tsconfig_dir: &Utf8Path) -> Option<Utf8PathBuf> {
        let tsconfig_path = tsconfig_dir.join("tsconfig.json");
        let content = std::fs::read_to_string(tsconfig_path.as_std_path()).ok()?;
        let stripped = crate::workspace::javascript::strip_jsonc_comments(&content);
        let tsconfig: serde_json::Value = serde_json::from_str(&stripped).ok()?;
        let base_url_str = tsconfig
            .get("compilerOptions")?
            .get("baseUrl")?
            .as_str()?;
        Some(tsconfig_dir.join(base_url_str))
    }

    /// Read `paths` from a tsconfig's directory.
    fn read_paths(tsconfig_dir: &Utf8Path) -> Vec<PathMapping> {
        let tsconfig_path = tsconfig_dir.join("tsconfig.json");
        let content = match std::fs::read_to_string(tsconfig_path.as_std_path()) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let stripped = crate::workspace::javascript::strip_jsonc_comments(&content);
        let tsconfig: serde_json::Value = match serde_json::from_str(&stripped) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        let paths_obj = match tsconfig
            .get("compilerOptions")
            .and_then(|co| co.get("paths"))
            .and_then(|p| p.as_object())
        {
            Some(obj) => obj,
            None => return Vec::new(),
        };

        let mut mappings = Vec::new();
        for (pattern, replacements) in paths_obj {
            let replacement_strs: Vec<String> = replacements
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            // Split pattern on `*`
            if let Some(star_pos) = pattern.find('*') {
                let prefix = pattern[..star_pos].to_string();
                let suffix = pattern[star_pos + 1..].to_string();
                mappings.push(PathMapping {
                    prefix,
                    suffix,
                    replacements: replacement_strs,
                });
            } else {
                // Exact match (no wildcard)
                mappings.push(PathMapping {
                    prefix: pattern.clone(),
                    suffix: String::new(),
                    replacements: replacement_strs,
                });
            }
        }

        mappings
    }
}

// ---------------------------------------------------------------------------
// TypeScriptDetector
// ---------------------------------------------------------------------------

/// TypeScript/JavaScript detector.
pub struct TypeScriptDetector;

impl Detector for TypeScriptDetector {
    fn name(&self) -> &str {
        "typescript-imports"
    }

    fn language(&self) -> Language {
        Language::TypeScript
    }

    fn applies_to(&self, workspace: &WorkspaceInfo) -> bool {
        workspace.js.is_some()
    }

    fn compatibility(&self, workspace: &WorkspaceInfo) -> CompatibilityAssessment {
        let js = match &workspace.js {
            Some(j) => j,
            None => {
                return CompatibilityAssessment {
                    language: Language::TypeScript,
                    status: SupportStatus::Unsupported,
                    details: vec![CompatibilityDetail {
                        feature: "JS/TS workspace".to_string(),
                        status: SupportStatus::Unsupported,
                        explanation: "No JS/TS workspace detected (no package.json with workspaces or pnpm-workspace.yaml)".to_string(),
                    }],
                };
            }
        };

        let mut details = Vec::new();
        let mut has_partial = false;
        let mut has_unsupported = false;

        // Workspace structure
        details.push(CompatibilityDetail {
            feature: "Workspace package discovery".to_string(),
            status: SupportStatus::Supported,
            explanation: format!(
                "{} package(s) discovered via {} workspace",
                js.packages.len(),
                js.package_manager
            ),
        });

        // Package manager
        details.push(CompatibilityDetail {
            feature: format!("Package manager: {}", js.package_manager),
            status: SupportStatus::Supported,
            explanation: format!("{} workspace structure detected", js.package_manager),
        });

        // Yarn PnP detection
        if js.has_pnp {
            has_unsupported = true;
            details.push(CompatibilityDetail {
                feature: "Yarn Plug'n'Play".to_string(),
                status: SupportStatus::Unsupported,
                explanation: "Yarn PnP (.pnp.cjs) detected. PnP module resolution is not supported in POC — node_modules resolution assumed.".to_string(),
            });
        }

        // tsconfig analysis
        if let Some(ref tsconfig) = js.root_tsconfig {
            if let Some(ref mode) = tsconfig.module_resolution {
                details.push(CompatibilityDetail {
                    feature: format!("Module resolution: {mode}"),
                    status: SupportStatus::Supported,
                    explanation: format!("moduleResolution \"{mode}\" detected in tsconfig.json"),
                });
            }

            if tsconfig.has_paths {
                details.push(CompatibilityDetail {
                    feature: "tsconfig paths".to_string(),
                    status: SupportStatus::Supported,
                    explanation: "compilerOptions.paths detected — basic path alias resolution supported".to_string(),
                });
            }

            if tsconfig.has_project_references {
                has_partial = true;
                details.push(CompatibilityDetail {
                    feature: "TypeScript project references".to_string(),
                    status: SupportStatus::Partial,
                    explanation: "tsconfig references detected. Cross-project reference resolution is not fully supported in POC.".to_string(),
                });
            }
        } else {
            details.push(CompatibilityDetail {
                feature: "tsconfig.json".to_string(),
                status: SupportStatus::Partial,
                explanation: "No tsconfig.json found at workspace root — module resolution defaults may differ from actual project configuration.".to_string(),
            });
            has_partial = true;
        }

        // Check for package.json exports/imports fields
        let packages_with_exports: Vec<&str> = js
            .packages
            .iter()
            .filter(|p| p.has_exports_field)
            .map(|p| p.name.as_str())
            .collect();
        if !packages_with_exports.is_empty() {
            has_partial = true;
            details.push(CompatibilityDetail {
                feature: "package.json exports conditions".to_string(),
                status: SupportStatus::Partial,
                explanation: format!(
                    "Exports field found in: {}. Condition-based resolution (import/require/node/browser) is not evaluated in POC — first export path assumed.",
                    packages_with_exports.join(", ")
                ),
            });
        }

        let packages_with_imports: Vec<&str> = js
            .packages
            .iter()
            .filter(|p| p.has_imports_field)
            .map(|p| p.name.as_str())
            .collect();
        if !packages_with_imports.is_empty() {
            has_partial = true;
            details.push(CompatibilityDetail {
                feature: "package.json imports (#imports)".to_string(),
                status: SupportStatus::Partial,
                explanation: format!(
                    "Imports field (# aliases) found in: {}. Internal import aliases are not resolved in POC.",
                    packages_with_imports.join(", ")
                ),
            });
        }

        // Check for mixed module types
        let module_types: Vec<&str> = js
            .packages
            .iter()
            .filter_map(|p| p.module_type.as_deref())
            .collect();
        let has_esm = module_types.contains(&"module");
        let has_cjs = module_types.contains(&"commonjs") || module_types.len() < js.packages.len();
        if has_esm && has_cjs {
            has_partial = true;
            details.push(CompatibilityDetail {
                feature: "Mixed ESM/CJS modules".to_string(),
                status: SupportStatus::Partial,
                explanation: "Mixed module types detected (some ESM, some CJS). Per-package resolution profiles are not available in POC — workspace-level profile used.".to_string(),
            });
        }

        // Import parsing
        details.push(CompatibilityDetail {
            feature: "Import/export parsing".to_string(),
            status: SupportStatus::Supported,
            explanation: "value vs type-only import classification via tree-sitter — will be available after full scan".to_string(),
        });

        let status = if has_unsupported {
            SupportStatus::Unsupported
        } else if has_partial {
            SupportStatus::Partial
        } else {
            SupportStatus::Supported
        };

        CompatibilityAssessment {
            language: Language::TypeScript,
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
        let js = workspace.js.as_ref().ok_or_else(|| {
            DetectorError::DetectionFailed {
                name: self.name().to_string(),
                reason: "no JS/TS workspace metadata available".to_string(),
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

        // Build resolver
        let resolver = TsConfigResolver::new(
            js,
            js.root_tsconfig.as_ref(),
            &workspace.root,
        );

        // Phase 1: Package topology
        let (pkg_nodes, pkg_edges) =
            detect_package_topology(js, &workspace.root, &mut report)?;
        let phase1_node_count = pkg_nodes.len();
        let phase1_edge_count = pkg_edges.len();
        sink.on_nodes(pkg_nodes);
        sink.on_edges(pkg_edges);

        // Phase 2: Module structure
        let (mod_nodes, mod_edges) = detect_module_structure(
            js,
            &workspace.root,
            ignore_set.as_ref(),
            &mut report,
        )?;
        let phase2_node_count = mod_nodes.len();
        let phase2_edge_count = mod_edges.len();
        sink.on_nodes(mod_nodes);
        sink.on_edges(mod_edges);

        // Phase 3: File-level nodes and import edges
        let (file_nodes, file_edges) = detect_file_edges(
            js,
            &workspace.root,
            ignore_set.as_ref(),
            &resolver,
            &mut report,
        )?;
        let phase3_node_count = file_nodes.len();
        let phase3_edge_count = file_edges.len();
        sink.on_nodes(file_nodes);
        sink.on_edges(file_edges);

        report.nodes_discovered = phase1_node_count + phase2_node_count + phase3_node_count;
        report.edges_discovered = phase1_edge_count + phase2_edge_count + phase3_edge_count;

        // Detect unsupported constructs at workspace level
        detect_workspace_level_constructs(js, &workspace.root, &mut report);

        Ok(report)
    }
}

// ---------------------------------------------------------------------------
// Phase 1: Package topology
// ---------------------------------------------------------------------------

/// Extract package nodes and inter-package dependency edges from package.json manifests.
fn detect_package_topology(
    js: &JsWorkspaceMeta,
    workspace_root: &Utf8Path,
    report: &mut DetectorReport,
) -> Result<(Vec<NodeData>, Vec<EdgeData>), DetectorError> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // Build name → relative_path lookup
    let pkg_name_to_path: HashMap<&str, &str> = js
        .packages
        .iter()
        .map(|p| (p.name.as_str(), p.relative_path.as_str()))
        .collect();

    for pkg in &js.packages {
        let key = MaterializedKey::new(
            Language::TypeScript,
            EntityKind::Package,
            &pkg.relative_path,
        );

        nodes.push(NodeData {
            materialized_key: key.clone(),
            lineage_key: None,
            label: pkg.name.clone(),
            kind: NodeKind::Package,
            language: Language::TypeScript,
            parent_key: None,
        });

        // Detect exports conditions as unsupported
        if pkg.has_exports_field {
            report.unsupported_constructs.push(UnsupportedConstruct {
                construct_type: UnsupportedConstructType::ExportsCondition,
                location: SourceLocation {
                    path: Utf8PathBuf::from(&pkg.relative_path).join("package.json"),
                    start_line: 1,
                    end_line: 1,
                },
                impact: format!(
                    "package.json exports conditions in {} — condition-based resolution not evaluated in POC",
                    pkg.name
                ),
                how_to_address: "Imports resolve to the package root; subpath exports are not followed".to_string(),
            });
        }

        // Read package.json dependencies to find workspace-internal edges
        let pkg_json_path = workspace_root.join(&pkg.relative_path).join("package.json");
        let deps = read_package_dependencies(&pkg_json_path);

        for (dep_name, dep_category) in &deps {
            if let Some(target_path) = pkg_name_to_path.get(dep_name.as_str()) {
                let target_key = MaterializedKey::new(
                    Language::TypeScript,
                    EntityKind::Package,
                    target_path,
                );

                edges.push(EdgeData {
                    edge_id: EdgeId::new(&key, &target_key, EdgeKind::DependsOn, *dep_category),
                    source_key: key.clone(),
                    target_key,
                    kind: EdgeKind::DependsOn,
                    category: *dep_category,
                    confidence: Confidence::Structural,
                    source_location: None,
                    resolution_method: Some("package.json".to_string()),
                    overlay_status: OverlayStatus::None,
                });
            }
        }
    }

    Ok((nodes, edges))
}

/// Read dependencies from a package.json file.
///
/// Returns (dependency_name, EdgeCategory) pairs for workspace-relevant deps.
fn read_package_dependencies(pkg_json_path: &Utf8Path) -> Vec<(String, EdgeCategory)> {
    let mut deps = Vec::new();

    let content = match std::fs::read_to_string(pkg_json_path.as_std_path()) {
        Ok(c) => c,
        Err(_) => return deps,
    };

    let pkg: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return deps,
    };

    // dependencies → Normal
    if let Some(obj) = pkg.get("dependencies").and_then(|v| v.as_object()) {
        for name in obj.keys() {
            deps.push((name.clone(), EdgeCategory::Normal));
        }
    }

    // devDependencies → Dev
    if let Some(obj) = pkg.get("devDependencies").and_then(|v| v.as_object()) {
        for name in obj.keys() {
            deps.push((name.clone(), EdgeCategory::Dev));
        }
    }

    // peerDependencies → Normal (treat as normal for POC)
    if let Some(obj) = pkg.get("peerDependencies").and_then(|v| v.as_object()) {
        for name in obj.keys() {
            deps.push((name.clone(), EdgeCategory::Normal));
        }
    }

    deps
}

// ---------------------------------------------------------------------------
// Phase 2: Module structure
// ---------------------------------------------------------------------------

/// Discover directory-as-module hierarchy for each workspace package.
fn detect_module_structure(
    js: &JsWorkspaceMeta,
    workspace_root: &Utf8Path,
    ignore_set: Option<&globset::GlobSet>,
    _report: &mut DetectorReport,
) -> Result<(Vec<NodeData>, Vec<EdgeData>), DetectorError> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for pkg in &js.packages {
        let pkg_dir = workspace_root.join(&pkg.relative_path);
        let pkg_key = MaterializedKey::new(
            Language::TypeScript,
            EntityKind::Package,
            &pkg.relative_path,
        );

        // Find source root: try src/, then package root
        let source_root = find_source_root(&pkg_dir);
        if !source_root.exists() {
            continue;
        }

        // Walk directories and create module nodes
        discover_modules_recursive(
            &source_root,
            workspace_root,
            &pkg_key,
            ignore_set,
            &mut nodes,
            &mut edges,
        );
    }

    Ok((nodes, edges))
}

/// Find the source root directory for a package.
///
/// Checks for `src/` first, falls back to the package root.
fn find_source_root(pkg_dir: &Utf8Path) -> Utf8PathBuf {
    let src_dir = pkg_dir.join("src");
    if src_dir.exists() {
        src_dir
    } else {
        pkg_dir.to_owned()
    }
}

/// Recursively discover directories as module nodes.
fn discover_modules_recursive(
    dir: &Utf8Path,
    workspace_root: &Utf8Path,
    parent_key: &MaterializedKey,
    ignore_set: Option<&globset::GlobSet>,
    nodes: &mut Vec<NodeData>,
    edges: &mut Vec<EdgeData>,
) {
    let entries = match std::fs::read_dir(dir.as_std_path()) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let utf8_path = match Utf8PathBuf::try_from(path) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let dir_name = utf8_path
            .file_name()
            .unwrap_or("");

        // Skip common non-source directories
        if matches!(dir_name, "node_modules" | "dist" | "build" | ".git" | "__pycache__") {
            continue;
        }

        let relative = utf8_path
            .strip_prefix(workspace_root)
            .unwrap_or(&utf8_path);

        if is_ignored(relative.as_str(), ignore_set) {
            continue;
        }

        // Check if directory contains any source files
        if !dir_contains_ts_files(&utf8_path) {
            continue;
        }

        let mod_key = MaterializedKey::new(
            Language::TypeScript,
            EntityKind::Module,
            relative.as_str(),
        );

        nodes.push(NodeData {
            materialized_key: mod_key.clone(),
            lineage_key: None,
            label: dir_name.to_string(),
            kind: NodeKind::Module,
            language: Language::TypeScript,
            parent_key: Some(parent_key.clone()),
        });

        edges.push(EdgeData {
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
            confidence: Confidence::Structural,
            source_location: None,
            resolution_method: Some("filesystem".to_string()),
            overlay_status: OverlayStatus::None,
        });

        // Recurse into subdirectories
        discover_modules_recursive(
            &utf8_path,
            workspace_root,
            &mod_key,
            ignore_set,
            nodes,
            edges,
        );
    }
}

/// Check if a directory contains TypeScript/JavaScript source files.
fn dir_contains_ts_files(dir: &Utf8Path) -> bool {
    let entries = match std::fs::read_dir(dir.as_std_path()) {
        Ok(e) => e,
        Err(_) => return false,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file()
            && let Some(ext) = path.extension()
            && matches!(ext.to_str(), Some("ts" | "tsx" | "js" | "jsx"))
        {
            return true;
        }
        // Also check subdirectories recursively
        if path.is_dir() {
            let utf8 = match Utf8PathBuf::try_from(path) {
                Ok(p) => p,
                Err(_) => continue,
            };
            if dir_contains_ts_files(&utf8) {
                return true;
            }
        }
    }

    false
}

// ---------------------------------------------------------------------------
// Phase 3: File-level nodes and import edges
// ---------------------------------------------------------------------------

/// Discover file nodes and import edges from tree-sitter parsing.
fn detect_file_edges(
    js: &JsWorkspaceMeta,
    workspace_root: &Utf8Path,
    ignore_set: Option<&globset::GlobSet>,
    resolver: &TsConfigResolver,
    report: &mut DetectorReport,
) -> Result<(Vec<NodeData>, Vec<EdgeData>), DetectorError> {
    let ts_lang: tree_sitter::Language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
    let tsx_lang: tree_sitter::Language = tree_sitter_typescript::LANGUAGE_TSX.into();

    // Pre-compile queries for both parsers
    let ts_queries = compile_queries(&ts_lang, "typescript")?;
    let tsx_queries = compile_queries(&tsx_lang, "typescript-tsx")?;

    let mut all_nodes = Vec::new();
    let mut all_edges = Vec::new();

    for pkg in &js.packages {
        let pkg_dir = workspace_root.join(&pkg.relative_path);
        let pkg_key = MaterializedKey::new(
            Language::TypeScript,
            EntityKind::Package,
            &pkg.relative_path,
        );

        // Walk source files
        let source_files = walk_ts_files(&pkg_dir, workspace_root, ignore_set);

        for file_path in &source_files {
            let file_relative = file_path
                .strip_prefix(workspace_root)
                .unwrap_or(file_path);

            let file_key = MaterializedKey::new(
                Language::TypeScript,
                EntityKind::File,
                file_relative.as_str(),
            );

            // Determine parent
            let parent_key = find_parent_key(
                file_path,
                &pkg_dir,
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
                language: Language::TypeScript,
                parent_key: Some(parent_key.clone()),
            });

            // Contains edge
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

            // Parse file with tree-sitter
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

            // Select parser based on extension
            let is_tsx = file_path.extension() == Some("tsx") || file_path.extension() == Some("jsx");
            let (lang, queries) = if is_tsx {
                (&tsx_lang, &tsx_queries)
            } else {
                (&ts_lang, &ts_queries)
            };

            let mut parser = Parser::new();
            if parser.set_language(lang).is_err() {
                report.parse_failures.push(ParseFailure {
                    path: file_relative.to_owned(),
                    reason: "could not set tree-sitter language".to_string(),
                });
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

            // Extract imports and create edges
            let import_edges = extract_import_edges(
                &tree,
                &source,
                queries,
                &file_key,
                file_path,
                file_relative,
                workspace_root,
                resolver,
                report,
            );
            all_edges.extend(import_edges);
        }
    }

    Ok((all_nodes, all_edges))
}

/// Pre-compiled tree-sitter queries for TypeScript.
struct TsQueries {
    import: Query,
    import_type: Query,
    reexport: Query,
    dynamic_import: Query,
    require: Query,
}

/// Compile all tree-sitter queries for a given language.
fn compile_queries(
    language: &tree_sitter::Language,
    name: &str,
) -> Result<TsQueries, DetectorError> {
    let make_err = |query_name: &str, e: tree_sitter::QueryError| DetectorError::DetectionFailed {
        name: format!("typescript-imports ({name})"),
        reason: format!("invalid {query_name} query: {e}"),
    };

    Ok(TsQueries {
        import: Query::new(language, IMPORT_QUERY).map_err(|e| make_err("import", e))?,
        import_type: Query::new(language, IMPORT_TYPE_QUERY)
            .map_err(|e| make_err("import_type", e))?,
        reexport: Query::new(language, REEXPORT_QUERY).map_err(|e| make_err("reexport", e))?,
        dynamic_import: Query::new(language, DYNAMIC_IMPORT_QUERY)
            .map_err(|e| make_err("dynamic_import", e))?,
        require: Query::new(language, REQUIRE_QUERY).map_err(|e| make_err("require", e))?,
    })
}

/// Extract import edges from a parsed TypeScript file.
#[allow(clippy::too_many_arguments)]
fn extract_import_edges(
    tree: &tree_sitter::Tree,
    source: &str,
    queries: &TsQueries,
    file_key: &MaterializedKey,
    file_path: &Utf8Path,
    file_relative: &Utf8Path,
    workspace_root: &Utf8Path,
    resolver: &TsConfigResolver,
    report: &mut DetectorReport,
) -> Vec<EdgeData> {
    let mut edges = Vec::new();

    // Collect type-import source strings (statement-level `import type`)
    let type_import_sources = collect_type_import_sources(tree, source, &queries.import_type);

    // Process all import statements
    let source_idx = queries.import
        .capture_index_for_name("source")
        .unwrap_or(u32::MAX);

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&queries.import, tree.root_node(), source.as_bytes());

    while let Some(m) = matches.next() {
        for capture in m.captures {
            if capture.index != source_idx {
                continue;
            }

            let raw_source = match capture.node.utf8_text(source.as_bytes()) {
                Ok(s) => strip_quotes(s),
                Err(_) => continue,
            };

            let start_line = capture.node.start_position().row as u32 + 1;
            let end_line = capture.node.end_position().row as u32 + 1;

            // Determine if this is a type-only import
            let is_type_only = type_import_sources.contains(&raw_source);

            // Also check for inline type specifiers: `import { type Foo, Bar }`
            // If ALL specifiers are type-only, treat as type-only.
            // If ANY specifier is a value, treat as value.
            // The statement-level `import type` takes precedence.
            let category = if is_type_only {
                EdgeCategory::TypeOnly
            } else {
                // Check inline type specifiers on the import_statement node
                let import_node = capture.node.parent();
                if let Some(import_stmt) = import_node {
                    if has_only_type_specifiers(import_stmt) {
                        EdgeCategory::TypeOnly
                    } else {
                        EdgeCategory::Value
                    }
                } else {
                    EdgeCategory::Value
                }
            };

            match resolver.resolve(&raw_source, file_path, workspace_root) {
                ResolveResult::Package(target_key) => {
                    edges.push(EdgeData {
                        edge_id: EdgeId::new(file_key, &target_key, EdgeKind::Imports, category),
                        source_key: file_key.clone(),
                        target_key,
                        kind: EdgeKind::Imports,
                        category,
                        confidence: Confidence::Structural,
                        source_location: Some(SourceLocation {
                            path: file_relative.to_owned(),
                            start_line,
                            end_line,
                        }),
                        resolution_method: Some("workspace package".to_string()),
                        overlay_status: OverlayStatus::None,
                    });
                }
                ResolveResult::File(target_key) => {
                    edges.push(EdgeData {
                        edge_id: EdgeId::new(file_key, &target_key, EdgeKind::Imports, category),
                        source_key: file_key.clone(),
                        target_key,
                        kind: EdgeKind::Imports,
                        category,
                        confidence: Confidence::ResolverAware,
                        source_location: Some(SourceLocation {
                            path: file_relative.to_owned(),
                            start_line,
                            end_line,
                        }),
                        resolution_method: Some("tsconfig path resolution".to_string()),
                        overlay_status: OverlayStatus::None,
                    });
                }
                ResolveResult::Unresolved(reason_str) => {
                    // Track unresolved imports with structured reason
                    let reason = if reason_str.contains("external package") {
                        UnresolvedReason::ExternalPackage
                    } else if reason_str.contains("could not resolve") {
                        UnresolvedReason::NoMatchingFile
                    } else {
                        UnresolvedReason::Other(reason_str)
                    };
                    report.unresolved_imports.push(UnresolvedImport {
                        source_file: file_relative.to_string(),
                        specifier: raw_source.clone(),
                        reason,
                    });
                }
            }
        }
    }

    // Process re-exports
    extract_reexport_edges(
        tree,
        source,
        &queries.reexport,
        file_key,
        file_path,
        file_relative,
        workspace_root,
        resolver,
        &mut edges,
    );

    // Detect dynamic imports
    detect_dynamic_imports(tree, source, &queries.dynamic_import, file_relative, report);

    // Detect require() calls
    detect_require_calls(tree, source, &queries.require, file_relative, report);

    edges
}

/// Collect all source strings from `import type { ... } from '...'` statements.
fn collect_type_import_sources(
    tree: &tree_sitter::Tree,
    source: &str,
    import_type_query: &Query,
) -> std::collections::HashSet<String> {
    let mut type_sources = std::collections::HashSet::new();
    let source_idx = import_type_query
        .capture_index_for_name("source")
        .unwrap_or(u32::MAX);

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(import_type_query, tree.root_node(), source.as_bytes());

    while let Some(m) = matches.next() {
        for capture in m.captures {
            if capture.index == source_idx
                && let Ok(s) = capture.node.utf8_text(source.as_bytes())
            {
                type_sources.insert(strip_quotes(s));
            }
        }
    }

    type_sources
}

/// Check if an `import_statement` node has only type specifiers (no value specifiers).
///
/// For `import { type Foo, Bar }`, this returns false (Bar is a value).
/// For `import { type Foo, type Bar }`, this returns true.
fn has_only_type_specifiers(import_stmt: tree_sitter::Node) -> bool {
    // Find the import_clause → named_imports → import_specifiers
    let mut has_any_specifier = false;
    let mut all_type = true;

    for i in 0..import_stmt.child_count() as u32 {
        let Some(child) = import_stmt.child(i) else {
            continue;
        };
        if child.kind() == "import_clause" {
            for j in 0..child.child_count() as u32 {
                let Some(clause_child) = child.child(j) else {
                    continue;
                };
                if clause_child.kind() == "named_imports" {
                    for k in 0..clause_child.child_count() as u32 {
                        let Some(spec) = clause_child.child(k) else {
                            continue;
                        };
                        if spec.kind() == "import_specifier" {
                            has_any_specifier = true;
                            let mut is_type = false;
                            for l in 0..spec.child_count() as u32 {
                                let Some(type_child) = spec.child(l) else {
                                    continue;
                                };
                                if type_child.kind() == "type" && !type_child.is_named() {
                                    is_type = true;
                                    break;
                                }
                            }
                            if !is_type {
                                all_type = false;
                            }
                        }
                    }
                }
                // Also handle namespace imports: `import * as foo`
                if clause_child.kind() == "namespace_import" {
                    has_any_specifier = true;
                    all_type = false;
                }
            }
        }
    }

    has_any_specifier && all_type
}

/// Extract re-export edges.
#[allow(clippy::too_many_arguments)]
fn extract_reexport_edges(
    tree: &tree_sitter::Tree,
    source: &str,
    reexport_query: &Query,
    file_key: &MaterializedKey,
    file_path: &Utf8Path,
    file_relative: &Utf8Path,
    workspace_root: &Utf8Path,
    resolver: &TsConfigResolver,
    edges: &mut Vec<EdgeData>,
) {
    let source_idx = reexport_query
        .capture_index_for_name("source")
        .unwrap_or(u32::MAX);
    let reexport_idx = reexport_query
        .capture_index_for_name("reexport")
        .unwrap_or(u32::MAX);

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(reexport_query, tree.root_node(), source.as_bytes());

    while let Some(m) = matches.next() {
        let mut raw_source = String::new();
        let mut start_line = 0u32;
        let mut end_line = 0u32;
        let mut export_node = None;

        for capture in m.captures {
            if capture.index == source_idx {
                raw_source = match capture.node.utf8_text(source.as_bytes()) {
                    Ok(s) => strip_quotes(s),
                    Err(_) => continue,
                };
                start_line = capture.node.start_position().row as u32 + 1;
                end_line = capture.node.end_position().row as u32 + 1;
            }
            if capture.index == reexport_idx {
                export_node = Some(capture.node);
            }
        }

        if raw_source.is_empty() {
            continue;
        }

        // Determine if this is `export type { ... } from '...'`
        let is_type_only = if let Some(node) = export_node {
            has_type_keyword_child(node)
        } else {
            false
        };

        let category = if is_type_only {
            EdgeCategory::TypeOnly
        } else {
            EdgeCategory::Value
        };

        let edge_kind = EdgeKind::ReExports;

        match resolver.resolve(&raw_source, file_path, workspace_root) {
            ResolveResult::Package(target_key) | ResolveResult::File(target_key) => {
                edges.push(EdgeData {
                    edge_id: EdgeId::new(file_key, &target_key, edge_kind, category),
                    source_key: file_key.clone(),
                    target_key,
                    kind: edge_kind,
                    category,
                    confidence: Confidence::ResolverAware,
                    source_location: Some(SourceLocation {
                        path: file_relative.to_owned(),
                        start_line,
                        end_line,
                    }),
                    resolution_method: Some("tsconfig path resolution".to_string()),
                    overlay_status: OverlayStatus::None,
                });
            }
            ResolveResult::Unresolved(_) => {
                // Unresolved re-exports are not added as edges
            }
        }
    }
}

/// Check if a node has an anonymous "type" keyword child (for `export type`).
fn has_type_keyword_child(node: tree_sitter::Node) -> bool {
    for i in 0..node.child_count() as u32 {
        if let Some(child) = node.child(i)
            && child.kind() == "type"
            && !child.is_named()
        {
            return true;
        }
    }
    false
}

/// Detect dynamic `import()` calls and add to unsupported constructs.
fn detect_dynamic_imports(
    tree: &tree_sitter::Tree,
    source: &str,
    dynamic_query: &Query,
    file_relative: &Utf8Path,
    report: &mut DetectorReport,
) {
    let source_idx = dynamic_query
        .capture_index_for_name("source")
        .unwrap_or(u32::MAX);

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(dynamic_query, tree.root_node(), source.as_bytes());

    while let Some(m) = matches.next() {
        for capture in m.captures {
            if capture.index == source_idx {
                let specifier = capture
                    .node
                    .utf8_text(source.as_bytes())
                    .unwrap_or("unknown");
                let start_line = capture.node.start_position().row as u32 + 1;
                let end_line = capture.node.end_position().row as u32 + 1;

                report.unsupported_constructs.push(UnsupportedConstruct {
                    construct_type: UnsupportedConstructType::DynamicImport,
                    location: SourceLocation {
                        path: file_relative.to_owned(),
                        start_line,
                        end_line,
                    },
                    impact: format!(
                        "dynamic import({}) — runtime-determined, cannot be statically resolved",
                        specifier
                    ),
                    how_to_address: "Add manual edges in .codeatlas.yaml if the import target is known".to_string(),
                });

                // Dynamic imports are also unresolved
                report.unresolved_imports.push(UnresolvedImport {
                    source_file: file_relative.to_string(),
                    specifier: strip_quotes(specifier),
                    reason: UnresolvedReason::DynamicImport,
                });
            }
        }
    }
}

/// Detect `require()` calls and add to unsupported constructs.
fn detect_require_calls(
    tree: &tree_sitter::Tree,
    source: &str,
    require_query: &Query,
    file_relative: &Utf8Path,
    report: &mut DetectorReport,
) {
    let source_idx = require_query
        .capture_index_for_name("source")
        .unwrap_or(u32::MAX);

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(require_query, tree.root_node(), source.as_bytes());

    while let Some(m) = matches.next() {
        for capture in m.captures {
            if capture.index == source_idx {
                let specifier = capture
                    .node
                    .utf8_text(source.as_bytes())
                    .unwrap_or("unknown");
                let start_line = capture.node.start_position().row as u32 + 1;
                let end_line = capture.node.end_position().row as u32 + 1;

                report.unsupported_constructs.push(UnsupportedConstruct {
                    construct_type: UnsupportedConstructType::CommonJsRequire,
                    location: SourceLocation {
                        path: file_relative.to_owned(),
                        start_line,
                        end_line,
                    },
                    impact: format!(
                        "require({}) — CommonJS require() is not resolved in POC",
                        specifier
                    ),
                    how_to_address: "Convert to ESM import or add manual edges".to_string(),
                });

                // require() calls are also unresolved
                report.unresolved_imports.push(UnresolvedImport {
                    source_file: file_relative.to_string(),
                    specifier: strip_quotes(specifier),
                    reason: UnresolvedReason::CommonJsRequire,
                });
            }
        }
    }
}

/// Detect workspace-level unsupported constructs.
fn detect_workspace_level_constructs(
    js: &JsWorkspaceMeta,
    workspace_root: &Utf8Path,
    report: &mut DetectorReport,
) {
    // Project references
    if let Some(ref tsconfig) = js.root_tsconfig
        && tsconfig.has_project_references
    {
        report.unsupported_constructs.push(UnsupportedConstruct {
            construct_type: UnsupportedConstructType::ProjectReferences,
            location: SourceLocation {
                path: tsconfig.path.strip_prefix(workspace_root)
                    .unwrap_or(&tsconfig.path)
                    .to_owned(),
                start_line: 1,
                end_line: 1,
            },
            impact: "TypeScript project references detected — cross-project reference resolution is not supported in POC".to_string(),
            how_to_address: "Import resolution falls back to workspace-level tsconfig".to_string(),
        });
    }

    // Yarn PnP
    if js.has_pnp {
        report.unsupported_constructs.push(UnsupportedConstruct {
            construct_type: UnsupportedConstructType::YarnPnp,
            location: SourceLocation {
                path: Utf8PathBuf::from(".pnp.cjs"),
                start_line: 1,
                end_line: 1,
            },
            impact: "Yarn Plug'n'Play detected — PnP module resolution is not supported".to_string(),
            how_to_address: "node_modules resolution is assumed instead of PnP".to_string(),
        });
    }
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

/// Walk all TypeScript/JavaScript source files in a package directory.
fn walk_ts_files(
    pkg_dir: &Utf8Path,
    workspace_root: &Utf8Path,
    ignore_set: Option<&globset::GlobSet>,
) -> Vec<Utf8PathBuf> {
    let mut files = Vec::new();

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
        let ext_str = ext.to_str().unwrap_or("");
        if !matches!(ext_str, "ts" | "tsx" | "js" | "jsx") {
            continue;
        }

        let utf8_path = match Utf8PathBuf::try_from(path.to_path_buf()) {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Skip common non-source directories
        let path_str = utf8_path.as_str();
        if path_str.contains("/node_modules/")
            || path_str.contains("/dist/")
            || path_str.contains("/build/")
            || path_str.contains("/.next/")
        {
            continue;
        }

        let relative = utf8_path
            .strip_prefix(workspace_root)
            .unwrap_or(&utf8_path);
        if is_ignored(relative.as_str(), ignore_set) {
            continue;
        }

        files.push(utf8_path);
    }

    files.sort();
    files
}

/// Find the parent key for a file (module directory or package).
fn find_parent_key(
    file_path: &Utf8Path,
    pkg_dir: &Utf8Path,
    workspace_root: &Utf8Path,
    pkg_key: &MaterializedKey,
) -> MaterializedKey {
    let file_dir = file_path.parent().unwrap_or(Utf8Path::new(""));

    // If file is directly in src/ or the package root, parent is the package
    let src_dir = pkg_dir.join("src");
    if file_dir == src_dir || file_dir == pkg_dir {
        return pkg_key.clone();
    }

    // Otherwise parent is the containing directory (module)
    let dir_relative = file_dir
        .strip_prefix(workspace_root)
        .unwrap_or(file_dir);

    MaterializedKey::new(
        Language::TypeScript,
        EntityKind::Module,
        dir_relative.as_str(),
    )
}

/// Strip surrounding quotes from a string literal.
fn strip_quotes(s: &str) -> String {
    let trimmed = s.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

/// Check if a relative path should be ignored.
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

    // -----------------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------------

    fn make_js_workspace(
        packages: Vec<JsPackageInfo>,
        tsconfig: Option<TsconfigInfo>,
        has_pnp: bool,
    ) -> WorkspaceInfo {
        WorkspaceInfo {
            root: Utf8PathBuf::from("/tmp/test"),
            kind: WorkspaceKind::Pnpm,
            packages: packages
                .iter()
                .map(|p| WorkspacePackage {
                    name: p.name.clone(),
                    relative_path: p.relative_path.clone(),
                    language: Language::TypeScript,
                })
                .collect(),
            cargo: None,
            js: Some(JsWorkspaceMeta {
                package_manager: "pnpm".to_string(),
                packages,
                root_tsconfig: tsconfig,
                has_pnp,
            }),
        }
    }

    fn ts_parser() -> (Parser, tree_sitter::Language) {
        let language: tree_sitter::Language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
        let mut parser = Parser::new();
        parser.set_language(&language).expect("set language");
        (parser, language)
    }

    fn tsx_parser() -> (Parser, tree_sitter::Language) {
        let language: tree_sitter::Language = tree_sitter_typescript::LANGUAGE_TSX.into();
        let mut parser = Parser::new();
        parser.set_language(&language).expect("set language");
        (parser, language)
    }

    // -----------------------------------------------------------------------
    // M2 compatibility tests (preserved)
    // -----------------------------------------------------------------------

    #[test]
    fn supported_basic_workspace() {
        let ws = make_js_workspace(
            vec![JsPackageInfo {
                name: "@app/web".to_string(),
                relative_path: "apps/web".to_string(),
                has_exports_field: false,
                has_imports_field: false,
                module_type: Some("module".to_string()),
            }],
            Some(TsconfigInfo {
                path: Utf8PathBuf::from("/tmp/test/tsconfig.json"),
                module_resolution: Some("bundler".to_string()),
                has_project_references: false,
                has_paths: true,
                has_base_url: false,
            }),
            false,
        );

        let detector = TypeScriptDetector;
        assert!(detector.applies_to(&ws));

        let assessment = detector.compatibility(&ws);
        assert_eq!(assessment.language, Language::TypeScript);
        assert_eq!(assessment.status, SupportStatus::Supported);
    }

    #[test]
    fn partial_with_exports_field() {
        let ws = make_js_workspace(
            vec![JsPackageInfo {
                name: "@app/sdk".to_string(),
                relative_path: "packages/sdk".to_string(),
                has_exports_field: true,
                has_imports_field: false,
                module_type: Some("module".to_string()),
            }],
            Some(TsconfigInfo {
                path: Utf8PathBuf::from("/tmp/test/tsconfig.json"),
                module_resolution: Some("bundler".to_string()),
                has_project_references: false,
                has_paths: false,
                has_base_url: false,
            }),
            false,
        );

        let assessment = TypeScriptDetector.compatibility(&ws);
        assert_eq!(assessment.status, SupportStatus::Partial);
        assert!(assessment
            .details
            .iter()
            .any(|d| d.feature.contains("exports")));
    }

    #[test]
    fn unsupported_with_pnp() {
        let ws = make_js_workspace(
            vec![JsPackageInfo {
                name: "@app/web".to_string(),
                relative_path: "apps/web".to_string(),
                has_exports_field: false,
                has_imports_field: false,
                module_type: Some("module".to_string()),
            }],
            None,
            true,
        );

        let assessment = TypeScriptDetector.compatibility(&ws);
        assert_eq!(assessment.status, SupportStatus::Unsupported);
    }

    #[test]
    fn partial_with_project_references() {
        let ws = make_js_workspace(
            vec![],
            Some(TsconfigInfo {
                path: Utf8PathBuf::from("/tmp/test/tsconfig.json"),
                module_resolution: Some("nodenext".to_string()),
                has_project_references: true,
                has_paths: false,
                has_base_url: false,
            }),
            false,
        );

        let assessment = TypeScriptDetector.compatibility(&ws);
        assert_eq!(assessment.status, SupportStatus::Partial);
    }

    #[test]
    fn no_js_workspace_returns_unsupported() {
        let ws = WorkspaceInfo {
            root: Utf8PathBuf::from("/tmp/empty"),
            kind: WorkspaceKind::Single,
            packages: Vec::new(),
            cargo: None,
            js: None,
        };

        let detector = TypeScriptDetector;
        assert!(!detector.applies_to(&ws));

        let assessment = detector.compatibility(&ws);
        assert_eq!(assessment.status, SupportStatus::Unsupported);
    }

    #[test]
    fn partial_with_mixed_module_types() {
        let ws = make_js_workspace(
            vec![
                JsPackageInfo {
                    name: "@app/web".to_string(),
                    relative_path: "apps/web".to_string(),
                    has_exports_field: false,
                    has_imports_field: false,
                    module_type: Some("module".to_string()),
                },
                JsPackageInfo {
                    name: "@app/legacy".to_string(),
                    relative_path: "packages/legacy".to_string(),
                    has_exports_field: false,
                    has_imports_field: false,
                    module_type: Some("commonjs".to_string()),
                },
            ],
            Some(TsconfigInfo {
                path: Utf8PathBuf::from("/tmp/test/tsconfig.json"),
                module_resolution: Some("bundler".to_string()),
                has_project_references: false,
                has_paths: false,
                has_base_url: false,
            }),
            false,
        );

        let assessment = TypeScriptDetector.compatibility(&ws);
        assert_eq!(assessment.status, SupportStatus::Partial);
        assert!(assessment
            .details
            .iter()
            .any(|d| d.feature.contains("Mixed ESM/CJS")));
    }

    // -----------------------------------------------------------------------
    // M5 tree-sitter query validation tests
    // -----------------------------------------------------------------------

    #[test]
    fn ts_query_regular_import() {
        let source = r#"import { Foo } from './bar';"#;
        let (mut parser, language) = ts_parser();
        let tree = parser.parse(source, None).expect("parse");

        let query = Query::new(&language, IMPORT_QUERY).expect("import query");
        let source_idx = query.capture_index_for_name("source").expect("source capture");

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        let mut sources = Vec::new();

        while let Some(m) = matches.next() {
            for capture in m.captures {
                if capture.index == source_idx {
                    sources.push(capture.node.utf8_text(source.as_bytes()).unwrap().to_string());
                }
            }
        }

        assert_eq!(sources, vec!["'./bar'"]);

        // Verify NO type keyword matched
        let type_query = Query::new(&language, IMPORT_TYPE_QUERY).expect("type query");
        let mut cursor2 = QueryCursor::new();
        let mut type_matches = cursor2.matches(&type_query, tree.root_node(), source.as_bytes());
        assert!(type_matches.next().is_none(), "regular import should NOT match import_type query");
    }

    #[test]
    fn ts_query_import_type() {
        let source = r#"import type { Foo } from './bar';"#;
        let (mut parser, language) = ts_parser();
        let tree = parser.parse(source, None).expect("parse");

        // Should match import_type query
        let type_query = Query::new(&language, IMPORT_TYPE_QUERY).expect("type query");
        let source_idx = type_query.capture_index_for_name("source").expect("source capture");

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&type_query, tree.root_node(), source.as_bytes());
        let mut sources = Vec::new();

        while let Some(m) = matches.next() {
            for capture in m.captures {
                if capture.index == source_idx {
                    sources.push(capture.node.utf8_text(source.as_bytes()).unwrap().to_string());
                }
            }
        }

        assert_eq!(sources, vec!["'./bar'"], "import type should capture source");
    }

    #[test]
    fn ts_query_inline_type_specifiers() {
        let source = r#"import { type Foo, Bar } from './bar';"#;
        let (mut parser, language) = ts_parser();
        let tree = parser.parse(source, None).expect("parse");

        // Should NOT match import_type query (it's not statement-level type)
        let type_query = Query::new(&language, IMPORT_TYPE_QUERY).expect("type query");
        let mut cursor = QueryCursor::new();
        let mut type_matches = cursor.matches(&type_query, tree.root_node(), source.as_bytes());
        assert!(type_matches.next().is_none(), "inline type should NOT match statement-level import_type");

        // Should match regular import query
        let import_query = Query::new(&language, IMPORT_QUERY).expect("import query");
        let mut cursor2 = QueryCursor::new();
        let mut matches = cursor2.matches(&import_query, tree.root_node(), source.as_bytes());
        assert!(matches.next().is_some(), "should match regular import");

        // Verify inline type detection via AST walking
        let root = tree.root_node();
        let import_stmt = root.child(0).expect("import statement");
        assert_eq!(import_stmt.kind(), "import_statement");

        // has_only_type_specifiers should return false (Bar is a value)
        assert!(!has_only_type_specifiers(import_stmt),
            "mixed import with Bar should not be type-only");
    }

    #[test]
    fn ts_query_all_inline_type_specifiers() {
        let source = r#"import { type Foo, type Bar } from './baz';"#;
        let (mut parser, _language) = ts_parser();
        let tree = parser.parse(source, None).expect("parse");

        let root = tree.root_node();
        let import_stmt = root.child(0).expect("import statement");
        assert!(has_only_type_specifiers(import_stmt),
            "all-type specifiers should be type-only");
    }

    #[test]
    fn ts_query_reexport() {
        let source = r#"export { Foo } from './bar';"#;
        let (mut parser, language) = ts_parser();
        let tree = parser.parse(source, None).expect("parse");

        let query = Query::new(&language, REEXPORT_QUERY).expect("reexport query");
        let source_idx = query.capture_index_for_name("source").expect("source capture");

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        let mut sources = Vec::new();

        while let Some(m) = matches.next() {
            for capture in m.captures {
                if capture.index == source_idx {
                    sources.push(strip_quotes(capture.node.utf8_text(source.as_bytes()).unwrap()));
                }
            }
        }

        assert_eq!(sources, vec!["./bar"]);
    }

    #[test]
    fn ts_query_type_reexport() {
        let source = r#"export type { Foo } from './bar';"#;
        let (mut parser, language) = ts_parser();
        let tree = parser.parse(source, None).expect("parse");

        let query = Query::new(&language, REEXPORT_QUERY).expect("reexport query");
        let reexport_idx = query.capture_index_for_name("reexport").expect("reexport capture");

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

        let m = matches.next().expect("should match");
        let export_node = m.captures.iter().find(|c| c.index == reexport_idx).expect("reexport capture");
        assert!(has_type_keyword_child(export_node.node), "export type should have type keyword");
    }

    #[test]
    fn ts_query_namespace_reexport() {
        let source = r#"export * from './bar';"#;
        let (mut parser, language) = ts_parser();
        let tree = parser.parse(source, None).expect("parse");

        let query = Query::new(&language, REEXPORT_QUERY).expect("reexport query");
        let source_idx = query.capture_index_for_name("source").expect("source capture");

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        let mut sources = Vec::new();

        while let Some(m) = matches.next() {
            for capture in m.captures {
                if capture.index == source_idx {
                    sources.push(strip_quotes(capture.node.utf8_text(source.as_bytes()).unwrap()));
                }
            }
        }

        assert_eq!(sources, vec!["./bar"]);
    }

    #[test]
    fn ts_query_dynamic_import() {
        let source = r#"const x = import('./bar');"#;
        let (mut parser, language) = ts_parser();
        let tree = parser.parse(source, None).expect("parse");

        let query = Query::new(&language, DYNAMIC_IMPORT_QUERY).expect("dynamic import query");
        let source_idx = query.capture_index_for_name("source").expect("source capture");

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        let mut sources = Vec::new();

        while let Some(m) = matches.next() {
            for capture in m.captures {
                if capture.index == source_idx {
                    sources.push(strip_quotes(capture.node.utf8_text(source.as_bytes()).unwrap()));
                }
            }
        }

        assert_eq!(sources, vec!["./bar"]);
    }

    #[test]
    fn ts_query_require_call() {
        let source = r#"const x = require('./bar');"#;
        let (mut parser, language) = ts_parser();
        let tree = parser.parse(source, None).expect("parse");

        let query = Query::new(&language, REQUIRE_QUERY).expect("require query");
        let source_idx = query.capture_index_for_name("source").expect("source capture");

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        let mut sources = Vec::new();

        while let Some(m) = matches.next() {
            for capture in m.captures {
                if capture.index == source_idx {
                    sources.push(strip_quotes(capture.node.utf8_text(source.as_bytes()).unwrap()));
                }
            }
        }

        assert_eq!(sources, vec!["./bar"]);
    }

    #[test]
    fn tsx_parser_handles_jsx() {
        let source = r#"
import { Button } from './components';

export function App() {
    return <Button onClick={() => console.log("clicked")}>Click me</Button>;
}
"#;
        let (mut parser, language) = tsx_parser();
        let tree = parser.parse(source, None).expect("parse TSX");

        // Should capture the import
        let query = Query::new(&language, IMPORT_QUERY).expect("import query for TSX");
        let source_idx = query.capture_index_for_name("source").expect("source capture");

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        let mut sources = Vec::new();

        while let Some(m) = matches.next() {
            for capture in m.captures {
                if capture.index == source_idx {
                    sources.push(strip_quotes(capture.node.utf8_text(source.as_bytes()).unwrap()));
                }
            }
        }

        assert_eq!(sources, vec!["./components"]);
    }

    // -----------------------------------------------------------------------
    // Strip quotes tests
    // -----------------------------------------------------------------------

    #[test]
    fn strip_single_quotes() {
        assert_eq!(strip_quotes("'./bar'"), "./bar");
    }

    #[test]
    fn strip_double_quotes() {
        assert_eq!(strip_quotes("\"./bar\""), "./bar");
    }

    // -----------------------------------------------------------------------
    // Detector integration tests
    // -----------------------------------------------------------------------

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

    #[test]
    fn detect_ts_monorepo_fixture() {
        let fixture_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("project root")
            .join("tests/fixtures/ts-monorepo");

        if !fixture_dir.exists() {
            return;
        }

        let ws = crate::workspace::discover_workspace(&fixture_dir)
            .expect("discovery should succeed");

        let config = RepoConfig::default_config();
        let profile = GraphProfile::empty();
        let detector = TypeScriptDetector;
        let sink = CollectingSink::default();

        let report = detector
            .detect(&ws, &profile, &config, &sink)
            .expect("detect should succeed");

        let nodes = sink.nodes();
        let edges = sink.edges();

        // Should discover package nodes
        let pkg_nodes: Vec<_> = nodes.iter().filter(|n| n.kind == NodeKind::Package).collect();
        assert!(
            pkg_nodes.len() >= 2,
            "should find at least 2 packages (@fixture/shared, @fixture/app), got {}",
            pkg_nodes.len()
        );

        // Should find inter-package dependency edge (app depends on shared)
        let dep_edges: Vec<_> = edges.iter().filter(|e| e.kind == EdgeKind::DependsOn).collect();
        assert!(!dep_edges.is_empty(), "should find inter-package dependency edges");

        // Should find file nodes
        let file_nodes: Vec<_> = nodes.iter().filter(|n| n.kind == NodeKind::File).collect();
        assert!(!file_nodes.is_empty(), "should find file nodes");

        // Should find import edges
        let import_edges: Vec<_> = edges.iter()
            .filter(|e| e.kind == EdgeKind::Imports || e.kind == EdgeKind::ReExports)
            .collect();
        assert!(!import_edges.is_empty(), "should find import/re-export edges");

        // Should have type-only edges (import type in fixtures)
        let type_only_edges: Vec<_> = import_edges.iter()
            .filter(|e| e.category == EdgeCategory::TypeOnly)
            .collect();
        assert!(!type_only_edges.is_empty(), "should find type-only import edges");

        // Should have value edges (regular imports)
        let value_edges: Vec<_> = import_edges.iter()
            .filter(|e| e.category == EdgeCategory::Value)
            .collect();
        assert!(!value_edges.is_empty(), "should find value import edges");

        // Should detect dynamic imports as unsupported
        let dynamic_imports: Vec<_> = report.unsupported_constructs.iter()
            .filter(|c| c.construct_type == UnsupportedConstructType::DynamicImport)
            .collect();
        assert!(!dynamic_imports.is_empty(), "should detect dynamic imports");

        // Should report counts
        assert!(report.nodes_discovered > 0);
        assert!(report.edges_discovered > 0);
    }

    #[test]
    fn detect_ts_unsupported_fixture() {
        let fixture_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("project root")
            .join("tests/fixtures/ts-unsupported");

        if !fixture_dir.exists() {
            return;
        }

        let ws = crate::workspace::discover_workspace(&fixture_dir)
            .expect("discovery should succeed");

        let config = RepoConfig::default_config();
        let profile = GraphProfile::empty();
        let detector = TypeScriptDetector;
        let sink = CollectingSink::default();

        let report = detector
            .detect(&ws, &profile, &config, &sink)
            .expect("detect should succeed");

        // Should detect require() calls
        let require_constructs: Vec<_> = report.unsupported_constructs.iter()
            .filter(|c| c.construct_type == UnsupportedConstructType::CommonJsRequire)
            .collect();
        assert!(!require_constructs.is_empty(), "should detect require() calls");

        // Should detect exports conditions
        let exports_constructs: Vec<_> = report.unsupported_constructs.iter()
            .filter(|c| c.construct_type == UnsupportedConstructType::ExportsCondition)
            .collect();
        assert!(!exports_constructs.is_empty(), "should detect exports conditions");

        // Should detect project references
        let ref_constructs: Vec<_> = report.unsupported_constructs.iter()
            .filter(|c| c.construct_type == UnsupportedConstructType::ProjectReferences)
            .collect();
        assert!(!ref_constructs.is_empty(), "should detect project references");

        // Should detect dynamic imports
        let dynamic_constructs: Vec<_> = report.unsupported_constructs.iter()
            .filter(|c| c.construct_type == UnsupportedConstructType::DynamicImport)
            .collect();
        assert!(!dynamic_constructs.is_empty(), "should detect dynamic imports");
    }
}
