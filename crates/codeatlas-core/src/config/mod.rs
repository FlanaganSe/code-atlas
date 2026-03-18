//! Configuration parsing and validation for `.codeatlas.yaml`.

pub mod schema;

pub use schema::RepoConfig;

/// Errors from config parsing and validation.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to parse config: {reason}")]
    ParseError { reason: String },

    #[error("unsupported config version: {version} (expected 1)")]
    UnsupportedVersion { version: u32 },
}
