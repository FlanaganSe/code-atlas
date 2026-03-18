//! Graph profile management.
//!
//! A profile captures the build context that determines the graph:
//! package manager, resolution mode, features, condition sets, etc.
//! The POC uses a workspace-level profile auto-detected from manifests.

use serde::{Deserialize, Serialize};

use crate::graph::types::Language;

/// The active graph profile — captures the build context that
/// parameterizes the architecture graph.
///
/// In the POC, this is workspace-level (not per-package).
/// A mixed ESM/CJS monorepo gets a single auto-detected profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphProfile {
    /// Languages detected in the workspace.
    pub languages: Vec<Language>,

    /// Package manager (e.g., "cargo", "pnpm", "npm", "yarn").
    pub package_manager: Option<String>,

    /// TypeScript module resolution mode (e.g., "bundler", "nodenext").
    pub resolution_mode: Option<String>,

    /// Cargo features enabled for the scan.
    pub cargo_features: Vec<String>,

    /// Content hash fingerprint for caching/comparison.
    pub fingerprint: ProfileFingerprint,
}

impl GraphProfile {
    /// Create a default empty profile.
    pub fn empty() -> Self {
        Self {
            languages: Vec::new(),
            package_manager: None,
            resolution_mode: None,
            cargo_features: Vec::new(),
            fingerprint: ProfileFingerprint::empty(),
        }
    }
}

/// Content hash of the profile for caching and comparison.
///
/// Allows detecting when the build context has changed and
/// a rescan is needed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileFingerprint(pub String);

impl ProfileFingerprint {
    /// Create an empty fingerprint (no profile detected yet).
    pub fn empty() -> Self {
        Self(String::new())
    }
}
