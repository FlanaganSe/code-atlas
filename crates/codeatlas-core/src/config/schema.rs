//! `.codeatlas.yaml` configuration schema.
//!
//! Defines the configuration file that repo owners use to supplement
//! the discovered graph with manual edges, suppressions, ignore paths,
//! entrypoints, and metadata.

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

    /// Parse and validate a `.codeatlas.yaml` string.
    pub fn parse(yaml: &str) -> Result<Self, super::ConfigError> {
        let config: Self =
            serde_yaml::from_str(yaml).map_err(|e| super::ConfigError::ParseError {
                reason: e.to_string(),
            })?;
        config.validate()?;
        Ok(config)
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
