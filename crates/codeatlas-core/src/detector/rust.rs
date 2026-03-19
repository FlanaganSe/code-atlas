//! Rust detector — `cargo_metadata` + tree-sitter analysis.
//!
//! M2: implements `compatibility()` (structural/manifest-level assessment).
//! M4: implements `detect()` (tree-sitter parsing, edge discovery).

use crate::config::RepoConfig;
use crate::graph::types::Language;
use crate::health::compatibility::{CompatibilityDetail, SupportStatus};
use crate::profile::GraphProfile;
use crate::workspace::WorkspaceInfo;

use super::{CompatibilityAssessment, Detector, DetectorError, DetectorReport, DetectorSink};

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
            explanation: "Normal/dev/build dependency kinds extracted from cargo_metadata".to_string(),
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

        // Check for cfg usage in features (heuristic: features with "/" indicate cfg-gated paths)
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

        // Module structure — supported (via tree-sitter in M4)
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

    fn detect(
        &self,
        _workspace: &WorkspaceInfo,
        _profile: &GraphProfile,
        _config: &RepoConfig,
        _sink: &dyn DetectorSink,
    ) -> Result<DetectorReport, DetectorError> {
        // Stub — detect() is implemented in M4
        Ok(DetectorReport {
            nodes_discovered: 0,
            edges_discovered: 0,
            unsupported_constructs: Vec::new(),
            parse_failures: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::*;
    use camino::Utf8PathBuf;

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
}
