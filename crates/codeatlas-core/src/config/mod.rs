//! Configuration parsing and validation for `.codeatlas.yaml`.
//!
//! Note on serde_yaml: serde_yaml 0.9 is deprecated upstream but still
//! functional and battle-tested. For the POC we use it as-is; migration
//! to serde_yml or another format can happen in a later milestone if needed.

pub mod schema;

pub use schema::RepoConfig;

/// Errors from config parsing and validation.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to parse config: {reason}")]
    ParseError { reason: String },

    #[error("unsupported config version: {version} (expected 1)")]
    UnsupportedVersion { version: u32 },

    #[error("I/O error reading config: {0}")]
    Io(#[from] std::io::Error),
}
