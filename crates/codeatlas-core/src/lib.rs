//! `codeatlas-core` — standalone analysis library for Code Atlas.
//!
//! This crate contains all scanning, graph building, profile management,
//! compatibility reporting, overlay management, and query logic.
//! It has **zero dependency on Tauri, serde_json, or any IPC/transport crate**.
//!
//! # Architecture
//!
//! The crate is organized into modules:
//!
//! - [`graph`] — Graph model: `ArchGraph`, `NodeData`, `EdgeData`, identity scheme, overlay.
//! - [`detector`] — `Detector` trait and `DetectorSink` for language/framework analysis.
//! - [`config`] — `.codeatlas.yaml` parsing and validation.
//! - [`profile`] — Graph profile (build context) management.
//! - [`health`] — Compatibility report and graph health metrics.
//! - [`workspace`] — Workspace discovery types.
//! - [`scan`] — Scan orchestration, domain result types, `ScanSink` trait.
//! - [`error`] — Top-level error types.
//!
//! # Core API Pattern
//!
//! Following rust-analyzer's architecture invariant:
//! - **`AnalysisHost`** — mutable handle, accepts changes.
//! - **`Analysis`** — immutable snapshot, safe for concurrent queries.
//!
//! Both are defined here but minimally implemented in M1.

pub mod config;
pub mod detector;
pub mod error;
pub mod graph;
pub mod health;
pub mod profile;
pub mod scan;
pub mod workspace;

// Re-export primary types for convenience.
pub use config::RepoConfig;
pub use error::CoreError;
pub use graph::{ArchGraph, EdgeId, GraphOverlay, MaterializedKey};
pub use health::{CompatibilityReport, GraphHealth};
pub use profile::GraphProfile;
pub use scan::{ScanPhase, ScanResults, ScanSink};
pub use workspace::WorkspaceInfo;
