//! Graph profile management.
//!
//! A profile captures the build context that determines the graph:
//! package manager, resolution mode, features, condition sets, etc.
//! The POC uses a workspace-level profile auto-detected from manifests.

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::graph::types::Language;
use crate::workspace::WorkspaceInfo;

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

    /// Auto-detect a profile from workspace discovery results.
    ///
    /// Extracts languages, package manager, resolution mode, and
    /// Cargo features from the workspace metadata. Produces a
    /// fingerprint for change detection.
    pub fn detect_from_workspace(workspace: &WorkspaceInfo) -> Self {
        let mut languages: Vec<Language> = Vec::new();
        let mut package_manager: Option<String> = None;
        let mut resolution_mode: Option<String> = None;
        let mut cargo_features: Vec<String> = Vec::new();

        // Detect from Cargo metadata
        if let Some(ref cargo) = workspace.cargo {
            if !languages.contains(&Language::Rust) {
                languages.push(Language::Rust);
            }
            package_manager = Some("cargo".to_string());

            // Collect all features from workspace packages (default features)
            for pkg in &cargo.packages {
                for feature in &pkg.features {
                    if feature == "default" {
                        continue;
                    }
                    if !cargo_features.contains(feature) {
                        cargo_features.push(feature.clone());
                    }
                }
            }
            cargo_features.sort();
        }

        // Detect from JS metadata
        if let Some(ref js) = workspace.js {
            if !languages.contains(&Language::TypeScript) {
                languages.push(Language::TypeScript);
            }
            // JS package manager takes precedence if Cargo is also present
            // (for display — this is a mixed workspace)
            if package_manager.is_none() {
                package_manager = Some(js.package_manager.clone());
            } else {
                // Mixed workspace — show both
                package_manager =
                    Some(format!("cargo + {}", js.package_manager));
            }

            // Resolution mode from root tsconfig
            if let Some(ref tsconfig) = js.root_tsconfig {
                resolution_mode = tsconfig.module_resolution.clone();
            }
        }

        // If no metadata but packages exist, infer languages from packages
        if languages.is_empty() {
            for pkg in &workspace.packages {
                if !languages.contains(&pkg.language) {
                    languages.push(pkg.language);
                }
            }
        }

        languages.sort_by_key(|l| format!("{l:?}"));

        let fingerprint = compute_fingerprint(&languages, &package_manager, &resolution_mode, &cargo_features);

        Self {
            languages,
            package_manager,
            resolution_mode,
            cargo_features,
            fingerprint,
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

/// Compute a deterministic fingerprint from profile data.
fn compute_fingerprint(
    languages: &[Language],
    package_manager: &Option<String>,
    resolution_mode: &Option<String>,
    cargo_features: &[String],
) -> ProfileFingerprint {
    let mut hasher = DefaultHasher::new();
    for lang in languages {
        format!("{lang:?}").hash(&mut hasher);
    }
    package_manager.hash(&mut hasher);
    resolution_mode.hash(&mut hasher);
    for f in cargo_features {
        f.hash(&mut hasher);
    }
    ProfileFingerprint(format!("{:016x}", hasher.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::*;
    use camino::Utf8PathBuf;

    #[test]
    fn detect_empty_workspace() {
        let ws = WorkspaceInfo {
            root: Utf8PathBuf::from("/tmp/empty"),
            kind: WorkspaceKind::Single,
            packages: Vec::new(),
            cargo: None,
            js: None,
        };
        let profile = GraphProfile::detect_from_workspace(&ws);
        assert!(profile.languages.is_empty());
        assert!(profile.package_manager.is_none());
        assert!(profile.resolution_mode.is_none());
    }

    #[test]
    fn detect_cargo_workspace() {
        let ws = WorkspaceInfo {
            root: Utf8PathBuf::from("/tmp/rust-project"),
            kind: WorkspaceKind::Cargo,
            packages: vec![WorkspacePackage {
                name: "my-crate".to_string(),
                relative_path: "crates/my-crate".to_string(),
                language: Language::Rust,
            }],
            cargo: Some(CargoWorkspaceMeta {
                workspace_root: Utf8PathBuf::from("/tmp/rust-project"),
                packages: vec![CargoPackageInfo {
                    name: "my-crate".to_string(),
                    version: "0.1.0".to_string(),
                    manifest_path: Utf8PathBuf::from("/tmp/rust-project/crates/my-crate/Cargo.toml"),
                    has_build_script: false,
                    is_proc_macro: false,
                    features: vec!["serde".to_string()],
                    dependencies: Vec::new(),
                    targets: Vec::new(),
                }],
            }),
            js: None,
        };
        let profile = GraphProfile::detect_from_workspace(&ws);
        assert_eq!(profile.languages, vec![Language::Rust]);
        assert_eq!(profile.package_manager, Some("cargo".to_string()));
        assert!(profile.cargo_features.contains(&"serde".to_string()));
    }

    #[test]
    fn detect_mixed_workspace() {
        let ws = WorkspaceInfo {
            root: Utf8PathBuf::from("/tmp/mixed"),
            kind: WorkspaceKind::Mixed,
            packages: Vec::new(),
            cargo: Some(CargoWorkspaceMeta {
                workspace_root: Utf8PathBuf::from("/tmp/mixed"),
                packages: Vec::new(),
            }),
            js: Some(JsWorkspaceMeta {
                package_manager: "pnpm".to_string(),
                packages: Vec::new(),
                root_tsconfig: Some(TsconfigInfo {
                    path: Utf8PathBuf::from("/tmp/mixed/tsconfig.json"),
                    module_resolution: Some("bundler".to_string()),
                    has_project_references: false,
                    has_paths: true,
                    has_base_url: false,
                }),
                has_pnp: false,
            }),
        };
        let profile = GraphProfile::detect_from_workspace(&ws);
        assert!(profile.languages.contains(&Language::Rust));
        assert!(profile.languages.contains(&Language::TypeScript));
        assert_eq!(profile.package_manager, Some("cargo + pnpm".to_string()));
        assert_eq!(profile.resolution_mode, Some("bundler".to_string()));
    }

    #[test]
    fn fingerprint_is_deterministic() {
        let ws = WorkspaceInfo {
            root: Utf8PathBuf::from("/tmp/test"),
            kind: WorkspaceKind::Cargo,
            packages: Vec::new(),
            cargo: Some(CargoWorkspaceMeta {
                workspace_root: Utf8PathBuf::from("/tmp/test"),
                packages: Vec::new(),
            }),
            js: None,
        };
        let p1 = GraphProfile::detect_from_workspace(&ws);
        let p2 = GraphProfile::detect_from_workspace(&ws);
        assert_eq!(p1.fingerprint, p2.fingerprint);
    }
}
