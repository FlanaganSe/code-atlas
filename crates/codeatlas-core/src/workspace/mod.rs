//! Workspace discovery.
//!
//! Detects the workspace structure (Cargo workspace, pnpm workspace,
//! npm/yarn workspace) from a given directory. Populates [`WorkspaceInfo`]
//! with metadata that detectors use for compatibility assessment and
//! profile detection.

pub mod cargo;
pub mod javascript;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::graph::types::Language;

/// Errors from workspace discovery.
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("no Cargo.toml found in {path} or any parent directory")]
    CargoTomlNotFound { path: String },

    #[error("cargo metadata failed: {reason}")]
    CargoMetadataFailed { reason: String },

    #[error("I/O error reading {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },

    #[error("JSON parse error in {path}: {reason}")]
    JsonParse { path: String, reason: String },

    #[error("YAML parse error in {path}: {reason}")]
    YamlParse { path: String, reason: String },
}

/// Information about a discovered workspace.
///
/// Populated by workspace discovery. Used by detectors to understand
/// the repository structure before scanning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInfo {
    /// Absolute path to the workspace root.
    pub root: Utf8PathBuf,

    /// What kind of workspace was detected.
    pub kind: WorkspaceKind,

    /// Discovered workspace packages/crates.
    pub packages: Vec<WorkspacePackage>,

    /// Cargo-specific metadata (populated if Cargo workspace detected).
    pub cargo: Option<CargoWorkspaceMeta>,

    /// JS/TS-specific metadata (populated if JS workspace detected).
    pub js: Option<JsWorkspaceMeta>,
}

/// The kind of workspace detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceKind {
    /// Cargo workspace (`Cargo.toml` with `[workspace]`).
    Cargo,
    /// pnpm workspace (`pnpm-workspace.yaml`).
    Pnpm,
    /// npm/yarn workspace (`package.json` with `workspaces`).
    NpmYarn,
    /// Both Cargo and JS workspace detected.
    Mixed,
    /// No workspace structure detected (single package).
    Single,
}

/// A package within a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePackage {
    /// Package name.
    pub name: String,
    /// Path relative to workspace root.
    pub relative_path: String,
    /// What language this package is (Rust crate vs JS/TS package).
    pub language: Language,
}

// ---------------------------------------------------------------------------
// Cargo-specific metadata
// ---------------------------------------------------------------------------

/// Metadata extracted from `cargo metadata` for the Cargo workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CargoWorkspaceMeta {
    /// Workspace root from cargo_metadata.
    pub workspace_root: Utf8PathBuf,
    /// Per-package information.
    pub packages: Vec<CargoPackageInfo>,
}

/// Information about a single Cargo package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CargoPackageInfo {
    /// Crate name.
    pub name: String,
    /// Crate version.
    pub version: String,
    /// Path to Cargo.toml.
    pub manifest_path: Utf8PathBuf,
    /// Whether this package has a build.rs file.
    pub has_build_script: bool,
    /// Whether this package is a proc-macro crate.
    pub is_proc_macro: bool,
    /// Feature flags defined by this package.
    pub features: Vec<String>,
    /// Dependencies (name + kind).
    pub dependencies: Vec<CargoDependencyInfo>,
    /// Build targets.
    pub targets: Vec<CargoTargetInfo>,
}

/// A dependency of a Cargo package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CargoDependencyInfo {
    /// Dependency name.
    pub name: String,
    /// Dependency kind (normal, dev, build).
    pub kind: CargoDependencyKind,
    /// Whether this dependency is optional.
    pub is_optional: bool,
}

/// Kind of a Cargo dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CargoDependencyKind {
    Normal,
    Dev,
    Build,
}

/// A build target in a Cargo package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CargoTargetInfo {
    /// Target name.
    pub name: String,
    /// Target kinds (e.g., "lib", "bin", "proc-macro").
    pub kinds: Vec<String>,
    /// Source path.
    pub src_path: Utf8PathBuf,
}

// ---------------------------------------------------------------------------
// JS/TS-specific metadata
// ---------------------------------------------------------------------------

/// Metadata for a JS/TS workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsWorkspaceMeta {
    /// Detected package manager ("pnpm", "npm", "yarn").
    pub package_manager: String,
    /// Per-package information.
    pub packages: Vec<JsPackageInfo>,
    /// Root tsconfig info (if tsconfig.json exists at workspace root).
    pub root_tsconfig: Option<TsconfigInfo>,
    /// Whether Yarn PnP is detected (.pnp.cjs present).
    pub has_pnp: bool,
}

/// Information about a single JS/TS package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsPackageInfo {
    /// Package name from package.json.
    pub name: String,
    /// Path relative to workspace root.
    pub relative_path: String,
    /// Whether package.json has an `exports` field.
    pub has_exports_field: bool,
    /// Whether package.json has an `imports` field.
    pub has_imports_field: bool,
    /// Value of the `type` field ("module", "commonjs", or absent).
    pub module_type: Option<String>,
}

/// Information extracted from a tsconfig.json file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TsconfigInfo {
    /// Path to the tsconfig.json.
    pub path: Utf8PathBuf,
    /// `compilerOptions.moduleResolution` value.
    pub module_resolution: Option<String>,
    /// Whether `references` array is present.
    pub has_project_references: bool,
    /// Whether `compilerOptions.paths` is present.
    pub has_paths: bool,
    /// Whether `compilerOptions.baseUrl` is present.
    pub has_base_url: bool,
}

// ---------------------------------------------------------------------------
// Discovery orchestration
// ---------------------------------------------------------------------------

/// Discover the workspace structure at the given directory path.
///
/// Checks for Cargo workspace first, then JS workspace. If both
/// are found, returns a Mixed workspace with both metadata populated.
/// If neither is found, returns Single workspace kind.
///
/// This function performs synchronous I/O (including running
/// `cargo metadata`). The Tauri shell should call this via
/// `tokio::task::spawn_blocking`.
pub fn discover_workspace(dir: &Utf8Path) -> Result<WorkspaceInfo, WorkspaceError> {
    let dir = if dir.is_relative() {
        let abs = std::env::current_dir()
            .map_err(|e| WorkspaceError::Io {
                path: dir.to_string(),
                source: e,
            })?;
        Utf8PathBuf::try_from(abs).map_err(|e| WorkspaceError::Io {
            path: dir.to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
        })?
        .join(dir)
    } else {
        dir.to_path_buf()
    };

    let cargo_result = cargo::discover_cargo_workspace(&dir);
    let js_result = javascript::discover_js_workspace(&dir);

    match (cargo_result, js_result) {
        (Ok(Some(cargo_info)), Ok(Some(js_info))) => {
            // Mixed workspace — both Cargo and JS found
            let root = dir.clone();
            let mut packages = cargo_info.packages.clone();
            packages.extend(js_info.packages.clone());
            Ok(WorkspaceInfo {
                root,
                kind: WorkspaceKind::Mixed,
                packages,
                cargo: Some(cargo_info.cargo_meta),
                js: Some(js_info.js_meta),
            })
        }
        (Ok(Some(cargo_info)), Ok(None) | Err(_)) => {
            Ok(WorkspaceInfo {
                root: cargo_info.cargo_meta.workspace_root.clone(),
                kind: WorkspaceKind::Cargo,
                packages: cargo_info.packages,
                cargo: Some(cargo_info.cargo_meta),
                js: None,
            })
        }
        (Ok(None) | Err(_), Ok(Some(js_info))) => {
            let kind = match js_info.js_meta.package_manager.as_str() {
                "pnpm" => WorkspaceKind::Pnpm,
                _ => WorkspaceKind::NpmYarn,
            };
            Ok(WorkspaceInfo {
                root: dir,
                kind,
                packages: js_info.packages,
                cargo: None,
                js: Some(js_info.js_meta),
            })
        }
        (Ok(None), Ok(None)) => {
            Ok(WorkspaceInfo {
                root: dir,
                kind: WorkspaceKind::Single,
                packages: Vec::new(),
                cargo: None,
                js: None,
            })
        }
        (Err(e), Ok(None)) => Err(e),
        (Ok(None), Err(e)) => Err(e),
        (Err(e), Err(_)) => Err(e),
    }
}

/// Intermediate result from Cargo workspace discovery.
pub(crate) struct CargoDiscoveryResult {
    pub packages: Vec<WorkspacePackage>,
    pub cargo_meta: CargoWorkspaceMeta,
}

/// Intermediate result from JS workspace discovery.
pub(crate) struct JsDiscoveryResult {
    pub packages: Vec<WorkspacePackage>,
    pub js_meta: JsWorkspaceMeta,
}
