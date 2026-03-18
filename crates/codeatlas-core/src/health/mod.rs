//! Health and compatibility reporting.
//!
//! Two surfaces:
//! - **Compatibility report**: per-language assessment of what can/cannot be analyzed.
//! - **Graph health**: runtime metrics on the graph's completeness.

pub mod compatibility;
pub mod graph_health;

pub use compatibility::CompatibilityReport;
pub use graph_health::GraphHealth;
