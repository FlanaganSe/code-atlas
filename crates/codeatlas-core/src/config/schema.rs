//! `.codeatlas.yaml` configuration schema.
//!
//! Defines the configuration file that repo owners use to supplement
//! the discovered graph with manual edges, suppressions, ignore paths,
//! entrypoints, and metadata.

use camino::Utf8Path;
use serde::{Deserialize, Serialize};

/// Root configuration from `.codeatlas.yaml`.
///
/// Located at workspace root, versioned with the repo.
/// Schema version must be checked before parsing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoConfig {
    /// Schema version (must be 1 for POC).
    pub version: u32,

    /// Glob patterns for paths to ignore during scanning.
    #[serde(default)]
    pub ignore: Vec<String>,

    /// Explicit entrypoints and public API roots.
    #[serde(default)]
    pub entrypoints: Vec<Entrypoint>,

    /// Dependency overlays (manual edges and suppressions).
    #[serde(default)]
    pub dependencies: DependencyOverlays,

    /// Per-package metadata.
    #[serde(default)]
    pub packages: std::collections::HashMap<String, PackageMetadata>,

    /// Framework detector hints.
    #[serde(default)]
    pub frameworks: Vec<FrameworkHint>,

    /// Declarations about unsupported constructs.
    #[serde(default)]
    pub declarations: Vec<Declaration>,
}

impl RepoConfig {
    /// Create a minimal default config.
    pub fn default_config() -> Self {
        Self {
            version: 1,
            ignore: Vec::new(),
            entrypoints: Vec::new(),
            dependencies: DependencyOverlays::default(),
            packages: std::collections::HashMap::new(),
            frameworks: Vec::new(),
            declarations: Vec::new(),
        }
    }

    /// Load `.codeatlas.yaml` from a directory.
    ///
    /// If the file doesn't exist, returns a default config.
    /// If the file exists but is invalid, returns an error.
    pub fn load_from_dir(dir: &Utf8Path) -> Result<Self, super::ConfigError> {
        let config_path = dir.join(".codeatlas.yaml");
        if !config_path.exists() {
            return Ok(Self::default_config());
        }
        let contents = std::fs::read_to_string(config_path.as_std_path())?;
        Self::parse(&contents)
    }

    /// Parse and validate a `.codeatlas.yaml` string.
    pub fn parse(yaml: &str) -> Result<Self, super::ConfigError> {
        let config: Self =
            serde_yaml::from_str(yaml).map_err(|e| super::ConfigError::ParseError {
                reason: e.to_string(),
            })?;
        config.validate()?;
        Ok(config)
    }

    /// Build a [`globset::GlobSet`] from the ignore patterns.
    ///
    /// Returns `None` if there are no ignore patterns. The returned
    /// set can be used by file walkers to skip ignored paths.
    pub fn ignore_glob_set(&self) -> Option<globset::GlobSet> {
        if self.ignore.is_empty() {
            return None;
        }
        let mut builder = globset::GlobSetBuilder::new();
        for pattern in &self.ignore {
            if let Ok(glob) = globset::Glob::new(pattern) {
                builder.add(glob);
            }
        }
        builder.build().ok()
    }

    /// Check if a path should be ignored based on the config's ignore patterns.
    pub fn is_ignored(&self, path: &str) -> bool {
        self.ignore_glob_set()
            .map(|gs| gs.is_match(path))
            .unwrap_or(false)
    }

    /// Validate the config after parsing.
    fn validate(&self) -> Result<(), super::ConfigError> {
        if self.version != 1 {
            return Err(super::ConfigError::UnsupportedVersion {
                version: self.version,
            });
        }
        Ok(())
    }

    /// Summary of which config sections are recognized but not yet
    /// functional in the POC.
    ///
    /// Note: `dependencies` (manual edges and suppressions) became functional in M6.
    pub fn non_functional_sections(&self) -> Vec<&'static str> {
        let mut sections = Vec::new();
        if !self.packages.is_empty() {
            sections.push("packages (per-package metadata)");
        }
        if !self.frameworks.is_empty() {
            sections.push("frameworks (detector hints)");
        }
        if !self.declarations.is_empty() {
            sections.push("declarations (unsupported construct annotations)");
        }
        sections
    }
}

/// An explicit entrypoint or public API root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entrypoint {
    /// Path relative to workspace root.
    pub path: String,
    /// Kind of entrypoint.
    pub kind: EntrypointKind,
}

/// What kind of entrypoint this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntrypointKind {
    App,
    PublicApi,
    Binary,
}

/// Dependency overlay configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyOverlays {
    /// Manual edges: declare dependencies the scanner cannot observe.
    #[serde(default)]
    pub add: Vec<ManualEdgeConfig>,

    /// View-level suppressions: hide edges in default view.
    #[serde(default)]
    pub suppress: Vec<SuppressionConfig>,
}

/// Configuration for a manually declared edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualEdgeConfig {
    pub from: String,
    pub to: String,
    pub reason: String,
}

/// Configuration for a suppressed edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuppressionConfig {
    pub from: String,
    pub to: String,
    pub reason: String,
}

/// Per-package metadata from configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageMetadata {
    #[serde(default)]
    pub tags: Vec<String>,
    pub layer: Option<String>,
    pub owner: Option<String>,
}

/// Framework detector hint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameworkHint {
    #[serde(rename = "type")]
    pub hint_type: String,
    pub root: Option<String>,
    pub output: Option<String>,
    pub source: Option<String>,
}

/// Declaration about an unsupported construct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Declaration {
    #[serde(rename = "type")]
    pub declaration_type: String,
    pub path: String,
    pub note: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let yaml = "version: 1\n";
        let config = RepoConfig::parse(yaml).unwrap();
        assert_eq!(config.version, 1);
        assert!(config.ignore.is_empty());
        assert!(config.entrypoints.is_empty());
    }

    #[test]
    fn parse_full_config() {
        let yaml = r#"
version: 1
ignore:
  - "dist/**"
  - "node_modules/**"
  - "target/**"
entrypoints:
  - path: "apps/web/src/main.tsx"
    kind: app
  - path: "packages/sdk/src/index.ts"
    kind: public-api
dependencies:
  add:
    - from: "apps/web"
      to: "packages/config"
      reason: "Runtime config loaded via environment"
  suppress:
    - from: "packages/utils"
      to: "packages/legacy"
      reason: "Dead code pending removal"
packages:
  "packages/sdk":
    tags: [public, stable]
    layer: api
    owner: platform-team
frameworks:
  - type: next-pages
    root: apps/web
declarations:
  - type: convention-based-routing
    path: "apps/web/src/pages/**"
    note: "Next.js file-based routing"
"#;
        let config = RepoConfig::parse(yaml).unwrap();
        assert_eq!(config.ignore.len(), 3);
        assert_eq!(config.entrypoints.len(), 2);
        assert_eq!(config.entrypoints[0].kind, EntrypointKind::App);
        assert_eq!(config.entrypoints[1].kind, EntrypointKind::PublicApi);
        assert_eq!(config.dependencies.add.len(), 1);
        assert_eq!(config.dependencies.suppress.len(), 1);
        assert_eq!(config.packages.len(), 1);
        assert_eq!(config.frameworks.len(), 1);
        assert_eq!(config.declarations.len(), 1);
    }

    #[test]
    fn reject_unsupported_version() {
        let yaml = "version: 99\n";
        let err = RepoConfig::parse(yaml).unwrap_err();
        assert!(matches!(err, super::super::ConfigError::UnsupportedVersion { version: 99 }));
    }

    #[test]
    fn reject_invalid_yaml() {
        let yaml = "not: [valid: yaml: at: all";
        assert!(RepoConfig::parse(yaml).is_err());
    }

    #[test]
    fn default_config_is_valid() {
        let config = RepoConfig::default_config();
        assert_eq!(config.version, 1);
    }

    #[test]
    fn load_from_dir_missing_file_returns_default() {
        let config = RepoConfig::load_from_dir(Utf8Path::new("/tmp"))
            .expect("should return default config");
        assert_eq!(config.version, 1);
        assert!(config.ignore.is_empty());
    }

    #[test]
    fn ignore_pattern_matching() {
        let config = RepoConfig {
            version: 1,
            ignore: vec!["dist/**".to_string(), "node_modules/**".to_string()],
            ..RepoConfig::default_config()
        };

        assert!(config.is_ignored("dist/index.js"));
        assert!(config.is_ignored("node_modules/foo/bar.js"));
        assert!(!config.is_ignored("src/main.ts"));
    }

    #[test]
    fn ignore_glob_set_empty_patterns() {
        let config = RepoConfig::default_config();
        assert!(config.ignore_glob_set().is_none());
    }

    #[test]
    fn non_functional_sections_detected() {
        let yaml = r#"
version: 1
dependencies:
  add:
    - from: "a"
      to: "b"
      reason: "test"
packages:
  "my-pkg":
    tags: [api]
"#;
        let config = RepoConfig::parse(yaml).unwrap();
        let sections = config.non_functional_sections();
        // dependencies are now functional (M6)
        assert!(!sections.contains(&"dependencies (manual edges and suppressions)"));
        assert!(sections.contains(&"packages (per-package metadata)"));
    }

    #[test]
    fn config_serde_round_trip() {
        let config = RepoConfig {
            version: 1,
            ignore: vec!["dist/**".to_string()],
            entrypoints: vec![Entrypoint {
                path: "src/main.tsx".to_string(),
                kind: EntrypointKind::App,
            }],
            dependencies: DependencyOverlays {
                add: vec![ManualEdgeConfig {
                    from: "a".to_string(),
                    to: "b".to_string(),
                    reason: "test".to_string(),
                }],
                suppress: vec![],
            },
            packages: std::collections::HashMap::new(),
            frameworks: vec![],
            declarations: vec![],
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: RepoConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }
}
