//! JavaScript/TypeScript workspace discovery.
//!
//! Detects pnpm workspaces (`pnpm-workspace.yaml`) first, then falls
//! back to `package.json` `workspaces` field (npm/yarn). Reads
//! `tsconfig.json` for module resolution mode detection.

use camino::{Utf8Path, Utf8PathBuf};

use super::{
    JsDiscoveryResult, JsPackageInfo, JsWorkspaceMeta, TsconfigInfo, WorkspaceError,
    WorkspacePackage,
};
use crate::graph::types::Language;

/// Discover a JS/TS workspace starting from `dir`.
///
/// Check order:
/// 1. `pnpm-workspace.yaml` — pnpm doesn't use `package.json` `workspaces`
/// 2. `package.json` `workspaces` field — npm/yarn
///
/// Returns `None` if no JS workspace structure is found.
pub(crate) fn discover_js_workspace(
    dir: &Utf8Path,
) -> Result<Option<JsDiscoveryResult>, WorkspaceError> {
    // Check for pnpm workspace first
    let pnpm_workspace_path = dir.join("pnpm-workspace.yaml");
    if pnpm_workspace_path.exists() {
        return discover_pnpm_workspace(dir, &pnpm_workspace_path);
    }

    // Check for package.json with workspaces field
    let package_json_path = dir.join("package.json");
    if package_json_path.exists() {
        return discover_npm_workspace(dir, &package_json_path);
    }

    Ok(None)
}

/// Discover a pnpm workspace from `pnpm-workspace.yaml`.
fn discover_pnpm_workspace(
    root: &Utf8Path,
    pnpm_path: &Utf8Path,
) -> Result<Option<JsDiscoveryResult>, WorkspaceError> {
    let content = read_file(pnpm_path)?;

    // pnpm-workspace.yaml has a `packages` array of glob patterns
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&content).map_err(|e| WorkspaceError::YamlParse {
            path: pnpm_path.to_string(),
            reason: e.to_string(),
        })?;

    let package_globs = yaml
        .get("packages")
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let js_packages = resolve_package_globs(root, &package_globs)?;

    let root_tsconfig = read_tsconfig_info(root)?;
    let has_pnp = root.join(".pnp.cjs").exists() || root.join(".pnp.mjs").exists();

    let ws_packages: Vec<WorkspacePackage> = js_packages
        .iter()
        .map(|p| WorkspacePackage {
            name: p.name.clone(),
            relative_path: p.relative_path.clone(),
            language: Language::TypeScript, // Default to TS for JS workspaces
        })
        .collect();

    Ok(Some(JsDiscoveryResult {
        packages: ws_packages,
        js_meta: JsWorkspaceMeta {
            package_manager: "pnpm".to_string(),
            packages: js_packages,
            root_tsconfig,
            has_pnp,
        },
    }))
}

/// Discover an npm/yarn workspace from `package.json` `workspaces` field.
fn discover_npm_workspace(
    root: &Utf8Path,
    package_json_path: &Utf8Path,
) -> Result<Option<JsDiscoveryResult>, WorkspaceError> {
    let content = read_file(package_json_path)?;
    let pkg: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| WorkspaceError::JsonParse {
            path: package_json_path.to_string(),
            reason: e.to_string(),
        })?;

    // workspaces can be an array or an object with a packages field
    let workspace_globs = match pkg.get("workspaces") {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<_>>(),
        Some(serde_json::Value::Object(obj)) => obj
            .get("packages")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        _ => return Ok(None),
    };

    if workspace_globs.is_empty() {
        return Ok(None);
    }

    // Detect package manager from lock files
    let package_manager = detect_package_manager(root);

    let js_packages = resolve_package_globs(root, &workspace_globs)?;

    let root_tsconfig = read_tsconfig_info(root)?;
    let has_pnp = root.join(".pnp.cjs").exists() || root.join(".pnp.mjs").exists();

    let ws_packages: Vec<WorkspacePackage> = js_packages
        .iter()
        .map(|p| WorkspacePackage {
            name: p.name.clone(),
            relative_path: p.relative_path.clone(),
            language: Language::TypeScript,
        })
        .collect();

    Ok(Some(JsDiscoveryResult {
        packages: ws_packages,
        js_meta: JsWorkspaceMeta {
            package_manager,
            packages: js_packages,
            root_tsconfig,
            has_pnp,
        },
    }))
}

/// Resolve workspace package globs to actual packages.
///
/// Handles common patterns like `packages/*`, `apps/*`.
/// For each matching directory with a `package.json`, reads the
/// package name and metadata.
fn resolve_package_globs(
    root: &Utf8Path,
    globs: &[String],
) -> Result<Vec<JsPackageInfo>, WorkspaceError> {
    let mut packages = Vec::new();

    for glob in globs {
        // Skip negation patterns (e.g., "!packages/internal")
        if glob.starts_with('!') {
            continue;
        }

        // Handle simple glob patterns: "dir/*" or "dir/**"
        let base_dir = glob
            .trim_end_matches("/**")
            .trim_end_matches("/*")
            .trim_end_matches('*');

        let search_dir = root.join(base_dir);
        if !search_dir.exists() {
            continue;
        }

        // If the glob ends with /*, list immediate children
        // If it's a specific path, check just that path
        if glob.contains('*') {
            let entries = std::fs::read_dir(search_dir.as_std_path()).map_err(|e| {
                WorkspaceError::Io {
                    path: search_dir.to_string(),
                    source: e,
                }
            })?;

            for entry in entries {
                let entry = entry.map_err(|e| WorkspaceError::Io {
                    path: search_dir.to_string(),
                    source: e,
                })?;
                let path = entry.path();
                if path.is_dir() {
                    let utf8_path =
                        Utf8PathBuf::try_from(path.clone()).map_err(|e| WorkspaceError::Io {
                            path: path.display().to_string(),
                            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
                        })?;
                    if let Some(pkg) = read_js_package_info(root, &utf8_path)? {
                        packages.push(pkg);
                    }
                }
            }
        } else {
            // Specific directory path
            let pkg_dir = root.join(glob.as_str());
            if pkg_dir.exists()
                && let Some(pkg) = read_js_package_info(root, &pkg_dir)?
            {
                packages.push(pkg);
            }
        }
    }

    Ok(packages)
}

/// Read package.json in a directory and extract JS package info.
fn read_js_package_info(
    root: &Utf8Path,
    pkg_dir: &Utf8Path,
) -> Result<Option<JsPackageInfo>, WorkspaceError> {
    let package_json_path = pkg_dir.join("package.json");
    if !package_json_path.exists() {
        return Ok(None);
    }

    let content = read_file(&package_json_path)?;
    let pkg: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| WorkspaceError::JsonParse {
            path: package_json_path.to_string(),
            reason: e.to_string(),
        })?;

    let name = pkg
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let relative_path = pkg_dir
        .strip_prefix(root)
        .unwrap_or(pkg_dir)
        .to_string();

    let has_exports_field = pkg.get("exports").is_some();
    let has_imports_field = pkg.get("imports").is_some();
    let module_type = pkg
        .get("type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(Some(JsPackageInfo {
        name,
        relative_path,
        has_exports_field,
        has_imports_field,
        module_type,
    }))
}

/// Read tsconfig.json and extract relevant info.
fn read_tsconfig_info(dir: &Utf8Path) -> Result<Option<TsconfigInfo>, WorkspaceError> {
    let tsconfig_path = dir.join("tsconfig.json");
    if !tsconfig_path.exists() {
        return Ok(None);
    }

    let content = read_file(&tsconfig_path)?;

    // tsconfig.json may contain comments (JSONC), but serde_json doesn't
    // support those. Strip single-line comments as a basic workaround.
    let stripped = strip_jsonc_comments(&content);

    let tsconfig: serde_json::Value =
        serde_json::from_str(&stripped).map_err(|e| WorkspaceError::JsonParse {
            path: tsconfig_path.to_string(),
            reason: e.to_string(),
        })?;

    let compiler_options = tsconfig.get("compilerOptions");

    let module_resolution = compiler_options
        .and_then(|co| co.get("moduleResolution"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let has_project_references = tsconfig
        .get("references")
        .and_then(|v| v.as_array())
        .map(|arr| !arr.is_empty())
        .unwrap_or(false);

    let has_paths = compiler_options
        .and_then(|co| co.get("paths"))
        .is_some();

    let has_base_url = compiler_options
        .and_then(|co| co.get("baseUrl"))
        .is_some();

    Ok(Some(TsconfigInfo {
        path: tsconfig_path,
        module_resolution,
        has_project_references,
        has_paths,
        has_base_url,
    }))
}

/// Detect package manager from lock files.
fn detect_package_manager(root: &Utf8Path) -> String {
    if root.join("pnpm-lock.yaml").exists() {
        "pnpm".to_string()
    } else if root.join("yarn.lock").exists() {
        "yarn".to_string()
    } else if root.join("bun.lockb").exists() || root.join("bun.lock").exists() {
        "bun".to_string()
    } else {
        "npm".to_string()
    }
}

/// Strip single-line comments from JSONC content.
///
/// This is a simple heuristic — it handles `//` comments outside strings.
/// Not fully JSONC-compliant but sufficient for tsconfig.json files.
fn strip_jsonc_comments(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_string = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_string {
            result.push(ch);
            if ch == '\\' {
                // Skip escaped character
                if let Some(next) = chars.next() {
                    result.push(next);
                }
            } else if ch == '"' {
                in_string = false;
            }
        } else if ch == '"' {
            in_string = true;
            result.push(ch);
        } else if ch == '/' && chars.peek() == Some(&'/') {
            // Single-line comment — skip to end of line
            for c in chars.by_ref() {
                if c == '\n' {
                    result.push('\n');
                    break;
                }
            }
        } else if ch == '/' && chars.peek() == Some(&'*') {
            // Block comment — skip to */
            chars.next(); // consume *
            let mut prev = ' ';
            for c in chars.by_ref() {
                if prev == '*' && c == '/' {
                    break;
                }
                prev = c;
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Read a file as a UTF-8 string.
fn read_file(path: &Utf8Path) -> Result<String, WorkspaceError> {
    std::fs::read_to_string(path.as_std_path()).map_err(|e| WorkspaceError::Io {
        path: path.to_string(),
        source: e,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_jsonc_single_line_comments() {
        let input = r#"{
  // This is a comment
  "compilerOptions": {
    "target": "ES2020" // inline comment
  }
}"#;
        let result = strip_jsonc_comments(input);
        let parsed: serde_json::Value = serde_json::from_str(&result)
            .expect("should parse after stripping comments");
        assert!(parsed.get("compilerOptions").is_some());
    }

    #[test]
    fn strip_jsonc_block_comments() {
        let input = r#"{
  /* block comment */
  "key": "value"
}"#;
        let result = strip_jsonc_comments(input);
        let parsed: serde_json::Value = serde_json::from_str(&result)
            .expect("should parse after stripping block comments");
        assert_eq!(parsed.get("key").unwrap().as_str().unwrap(), "value");
    }

    #[test]
    fn detect_package_manager_from_this_project() {
        // This project uses pnpm
        let project_root = Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("should find project root");

        let pm = detect_package_manager(project_root);
        assert_eq!(pm, "pnpm");
    }

    #[test]
    fn read_this_projects_tsconfig() {
        let project_root = Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("should find project root");

        let tsconfig = read_tsconfig_info(project_root)
            .expect("should read tsconfig")
            .expect("tsconfig.json should exist");

        // This project's tsconfig should have some resolution mode
        assert!(tsconfig.path.as_str().contains("tsconfig.json"));
    }

    #[test]
    fn no_js_workspace_in_tmp() {
        let result = discover_js_workspace(Utf8Path::new("/tmp"));
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn discover_fixture_ts_monorepo() {
        let fixture_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("project root")
            .join("tests/fixtures/ts-monorepo");

        if !fixture_dir.exists() {
            return; // Skip if fixtures not yet created
        }

        let result = discover_js_workspace(&fixture_dir)
            .expect("discovery should succeed")
            .expect("should find JS workspace");

        // Should be pnpm workspace
        assert_eq!(result.js_meta.package_manager, "pnpm");

        // Should find packages
        let names: Vec<&str> = result.packages.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"@fixture/shared"), "should find @fixture/shared: got {names:?}");
        assert!(names.contains(&"@fixture/app"), "should find @fixture/app: got {names:?}");

        // Should detect tsconfig
        let tsconfig = result
            .js_meta
            .root_tsconfig
            .as_ref()
            .expect("should have root tsconfig");
        assert_eq!(tsconfig.module_resolution.as_deref(), Some("bundler"));
        assert!(tsconfig.has_paths);

        // @fixture/shared should have exports field
        let shared_pkg = result
            .js_meta
            .packages
            .iter()
            .find(|p| p.name == "@fixture/shared")
            .expect("should find @fixture/shared in js meta");
        assert!(shared_pkg.has_exports_field);
    }
}
