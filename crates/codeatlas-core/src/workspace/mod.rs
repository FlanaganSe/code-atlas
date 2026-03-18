//! Workspace discovery.
//!
//! Detects the workspace structure (Cargo workspace, pnpm workspace,
//! npm/yarn workspace) from a given directory. Implementation deferred
//! to M2 — this module defines the types.

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

/// Information about a discovered workspace.
///
/// Populated by workspace discovery in M2. Used by detectors to
/// understand the repository structure before scanning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInfo {
    /// Absolute path to the workspace root.
    pub root: Utf8PathBuf,

    /// What kind of workspace was detected.
    pub kind: WorkspaceKind,

    /// Discovered workspace packages/crates.
    pub packages: Vec<WorkspacePackage>,
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
    pub language: crate::graph::types::Language,
}
