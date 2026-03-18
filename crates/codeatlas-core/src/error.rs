/// Top-level error type for `codeatlas-core`.
///
/// Each module defines its own error enum for domain-specific failures.
/// This top-level enum aggregates them for API consumers that want
/// a single error type.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("graph error: {0}")]
    Graph(#[from] crate::graph::GraphError),

    #[error("config error: {0}")]
    Config(#[from] crate::config::ConfigError),

    #[error("detector error: {0}")]
    Detector(#[from] crate::detector::DetectorError),
}
