//! TypeScript/JavaScript detector — workspace package discovery + tree-sitter parsing.
//!
//! M2: implements `compatibility()` (structural/manifest-level assessment).
//! M5: implements `detect()` (tree-sitter parsing, import resolution).

use crate::config::RepoConfig;
use crate::graph::types::Language;
use crate::health::compatibility::{CompatibilityDetail, SupportStatus};
use crate::profile::GraphProfile;
use crate::workspace::WorkspaceInfo;

use super::{CompatibilityAssessment, Detector, DetectorError, DetectorReport, DetectorSink};

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

        // Import parsing (will be available in M5)
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

    fn detect(
        &self,
        _workspace: &WorkspaceInfo,
        _profile: &GraphProfile,
        _config: &RepoConfig,
        _sink: &dyn DetectorSink,
    ) -> Result<DetectorReport, DetectorError> {
        // Stub — detect() is implemented in M5
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
}
