/**
 * Configuration, profile, and health types — mirrors Rust serde output.
 */

import type { Language } from "./graph";

// ---------------------------------------------------------------------------
// SupportStatus
// ---------------------------------------------------------------------------

export type SupportStatus = "supported" | "partial" | "unsupported";

// ---------------------------------------------------------------------------
// CompatibilityDetail
// ---------------------------------------------------------------------------

export interface CompatibilityDetail {
	readonly feature: string;
	readonly status: SupportStatus;
	readonly explanation: string;
}

// ---------------------------------------------------------------------------
// CompatibilityAssessment
// ---------------------------------------------------------------------------

export interface CompatibilityAssessment {
	readonly language: Language;
	readonly status: SupportStatus;
	readonly details: readonly CompatibilityDetail[];
}

// ---------------------------------------------------------------------------
// CompatibilityReport
// ---------------------------------------------------------------------------

export interface CompatibilityReport {
	readonly assessments: readonly CompatibilityAssessment[];
	readonly isProvisional: boolean;
}

// ---------------------------------------------------------------------------
// ProfileFingerprint
// ---------------------------------------------------------------------------

export type ProfileFingerprint = string;

// ---------------------------------------------------------------------------
// GraphProfile
// ---------------------------------------------------------------------------

export interface GraphProfile {
	readonly languages: readonly Language[];
	readonly packageManager: string | null;
	readonly resolutionMode: string | null;
	readonly cargoFeatures: readonly string[];
	readonly fingerprint: ProfileFingerprint;
}

// ---------------------------------------------------------------------------
// GraphHealth
// ---------------------------------------------------------------------------

export interface GraphHealth {
	readonly totalNodes: number;
	readonly resolvedEdges: number;
	readonly unresolvedImports: number;
	readonly parseFailures: number;
	readonly unsupportedConstructs: number;
}
