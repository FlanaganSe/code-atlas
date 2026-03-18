//! Compatibility report — first-class product surface for trust.
//!
//! Before a developer invests time in the graph, the tool provides
//! an upfront report declaring what is Supported, Partially supported,
//! and Unsupported for the target repository.

use serde::{Deserialize, Serialize};

use crate::detector::CompatibilityAssessment;
use crate::graph::types::Language;

/// Support status for a language or feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SupportStatus {
    /// Fully supported — accurate graph expected.
    Supported,
    /// Partially supported — some constructs not modeled.
    Partial,
    /// Not supported — language/framework not analyzed.
    Unsupported,
}

/// A specific detail in a compatibility assessment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityDetail {
    /// What feature/construct this detail covers.
    pub feature: String,
    /// Support status for this specific feature.
    pub status: SupportStatus,
    /// Human-readable explanation.
    pub explanation: String,
}

/// Aggregate compatibility report for the entire workspace.
///
/// Contains per-language assessments from all detectors.
/// Lifecycle: starts as provisional (structural findings only),
/// becomes final after scanning completes (source-level findings added).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityReport {
    /// Per-language compatibility assessments.
    pub assessments: Vec<CompatibilityAssessment>,

    /// Whether this report is provisional (pre-scan) or final (post-scan).
    pub is_provisional: bool,
}

impl CompatibilityReport {
    /// Create an empty provisional report.
    pub fn provisional() -> Self {
        Self {
            assessments: Vec::new(),
            is_provisional: true,
        }
    }

    /// Get the overall status for a language.
    pub fn status_for(&self, language: Language) -> Option<SupportStatus> {
        self.assessments
            .iter()
            .find(|a| a.language == language)
            .map(|a| a.status)
    }
}
