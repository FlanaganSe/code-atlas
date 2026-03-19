//! Cargo workspace discovery via `cargo_metadata`.
//!
//! Locates `Cargo.toml` by walking upward from the selected directory,
//! then runs `cargo metadata` to extract workspace structure, packages,
//! dependencies, targets, and features.

use camino::{Utf8Path, Utf8PathBuf};

use super::{
    CargoDependencyInfo, CargoDependencyKind, CargoDiscoveryResult, CargoPackageInfo,
    CargoTargetInfo, CargoWorkspaceMeta, WorkspaceError, WorkspacePackage,
};
use crate::graph::types::Language;

/// Discover a Cargo workspace starting from `dir`.
///
/// Walks upward looking for `Cargo.toml`. If found, runs `cargo metadata`
/// to extract workspace information. Returns `None` if no `Cargo.toml`
/// is found in `dir` or any parent.
///
/// **Blocking**: This calls `cargo metadata` which can take 2-10s on
/// first run. The caller should use `spawn_blocking` if on an async runtime.
pub(crate) fn discover_cargo_workspace(
    dir: &Utf8Path,
) -> Result<Option<CargoDiscoveryResult>, WorkspaceError> {
    let cargo_toml = match find_cargo_toml(dir) {
        Some(path) => path,
        None => return Ok(None),
    };

    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(&cargo_toml)
        .exec()
        .map_err(|e| WorkspaceError::CargoMetadataFailed {
            reason: e.to_string(),
        })?;

    let workspace_root =
        Utf8PathBuf::from(metadata.workspace_root.as_str());
    let workspace_members: std::collections::HashSet<_> =
        metadata.workspace_members.iter().collect();

    let mut cargo_packages = Vec::new();
    let mut ws_packages = Vec::new();

    for pkg in &metadata.packages {
        // Only include workspace members, not external dependencies
        if !workspace_members.contains(&pkg.id) {
            continue;
        }

        let has_build_script = pkg.targets.iter().any(|t| t.is_custom_build());

        let is_proc_macro = pkg.targets.iter().any(|t| t.is_proc_macro());

        let features: Vec<String> = pkg.features.keys().cloned().collect();

        let dependencies: Vec<CargoDependencyInfo> = pkg
            .dependencies
            .iter()
            .map(|dep| CargoDependencyInfo {
                name: dep.name.clone(),
                kind: match dep.kind {
                    cargo_metadata::DependencyKind::Normal => CargoDependencyKind::Normal,
                    cargo_metadata::DependencyKind::Development => CargoDependencyKind::Dev,
                    cargo_metadata::DependencyKind::Build => CargoDependencyKind::Build,
                    _ => CargoDependencyKind::Normal,
                },
                is_optional: dep.optional,
            })
            .collect();

        let targets: Vec<CargoTargetInfo> = pkg
            .targets
            .iter()
            .map(|t| CargoTargetInfo {
                name: t.name.clone(),
                kinds: t.kind.iter().map(|k| format!("{k}")).collect(),
                src_path: Utf8PathBuf::from(t.src_path.as_str()),
            })
            .collect();

        let manifest_path = Utf8PathBuf::from(pkg.manifest_path.as_str());

        // Compute relative path from workspace root to package directory
        let pkg_dir = manifest_path
            .parent()
            .unwrap_or(Utf8Path::new(""));
        let relative_path = pkg_dir
            .strip_prefix(&workspace_root)
            .unwrap_or(pkg_dir)
            .to_string();

        cargo_packages.push(CargoPackageInfo {
            name: pkg.name.clone(),
            version: pkg.version.to_string(),
            manifest_path,
            has_build_script,
            is_proc_macro,
            features,
            dependencies,
            targets,
        });

        ws_packages.push(WorkspacePackage {
            name: pkg.name.clone(),
            relative_path,
            language: Language::Rust,
        });
    }

    Ok(Some(CargoDiscoveryResult {
        packages: ws_packages,
        cargo_meta: CargoWorkspaceMeta {
            workspace_root,
            packages: cargo_packages,
        },
    }))
}

/// Walk upward from `dir` looking for a `Cargo.toml` file.
///
/// Returns the path to the first `Cargo.toml` found, preferring
/// workspace roots (those containing `[workspace]`).
fn find_cargo_toml(dir: &Utf8Path) -> Option<Utf8PathBuf> {
    let mut current = Some(dir);
    let mut found_single: Option<Utf8PathBuf> = None;

    while let Some(d) = current {
        let candidate = d.join("Cargo.toml");
        if candidate.exists() {
            // Check if it's a workspace root
            if let Ok(contents) = std::fs::read_to_string(&candidate) {
                if contents.contains("[workspace]") {
                    return Some(candidate);
                }
                // Remember the first Cargo.toml we find (single crate)
                if found_single.is_none() {
                    found_single = Some(candidate);
                }
            }
        }
        current = d.parent();
    }

    // If no workspace root found, use the first Cargo.toml (single crate)
    found_single
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_this_projects_workspace() {
        // Use this project's own Cargo.toml as a test fixture
        let project_root = Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("should find project root");

        let result = discover_cargo_workspace(project_root)
            .expect("discovery should succeed")
            .expect("should find Cargo workspace");

        // Should find workspace members
        assert!(
            !result.packages.is_empty(),
            "should discover workspace packages"
        );

        // Should find codeatlas-core
        assert!(
            result
                .packages
                .iter()
                .any(|p| p.name == "codeatlas-core"),
            "should find codeatlas-core package"
        );

        // Should find codeatlas-tauri
        assert!(
            result
                .packages
                .iter()
                .any(|p| p.name == "codeatlas-tauri"),
            "should find codeatlas-tauri package"
        );

        // All packages should be Rust
        assert!(
            result.packages.iter().all(|p| p.language == Language::Rust),
            "all Cargo packages should be Rust"
        );

        // Cargo metadata should be populated
        assert!(!result.cargo_meta.packages.is_empty());
        assert!(!result.cargo_meta.workspace_root.as_str().is_empty());
    }

    #[test]
    fn find_cargo_toml_from_subdirectory() {
        // Start from codeatlas-core/src and should still find root Cargo.toml
        let src_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let found = find_cargo_toml(&src_dir);
        assert!(found.is_some(), "should find Cargo.toml from subdirectory");
    }

    #[test]
    fn no_cargo_toml_in_temp_dir() {
        let result = discover_cargo_workspace(Utf8Path::new("/tmp"));
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn discover_fixture_rust_workspace() {
        let fixture_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("project root")
            .join("tests/fixtures/rust-workspace");

        if !fixture_dir.exists() {
            return; // Skip if fixtures not yet created
        }

        let result = discover_cargo_workspace(&fixture_dir)
            .expect("discovery should succeed")
            .expect("should find Cargo workspace");

        // Should find fixture-core and fixture-cli
        let names: Vec<&str> = result.packages.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"fixture-core"), "should find fixture-core");
        assert!(names.contains(&"fixture-cli"), "should find fixture-cli");

        // fixture-cli should have a build.rs
        let cli_pkg = result
            .cargo_meta
            .packages
            .iter()
            .find(|p| p.name == "fixture-cli")
            .expect("should have fixture-cli in cargo meta");
        assert!(cli_pkg.has_build_script, "fixture-cli should have build.rs");
    }
}
