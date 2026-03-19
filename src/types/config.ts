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

// ---------------------------------------------------------------------------
// WorkspaceKind
// ---------------------------------------------------------------------------

export type WorkspaceKind = "cargo" | "pnpm" | "npmYarn" | "mixed" | "single";

// ---------------------------------------------------------------------------
// WorkspacePackage
// ---------------------------------------------------------------------------

export interface WorkspacePackage {
	readonly name: string;
	readonly relativePath: string;
	readonly language: Language;
}

// ---------------------------------------------------------------------------
// WorkspaceInfo
// ---------------------------------------------------------------------------

export interface WorkspaceInfo {
	readonly root: string;
	readonly kind: WorkspaceKind;
	readonly packages: readonly WorkspacePackage[];
	readonly cargo: CargoWorkspaceMeta | null;
	readonly js: JsWorkspaceMeta | null;
}

// ---------------------------------------------------------------------------
// Cargo workspace metadata
// ---------------------------------------------------------------------------

export interface CargoWorkspaceMeta {
	readonly workspaceRoot: string;
	readonly packages: readonly CargoPackageInfo[];
}

export interface CargoPackageInfo {
	readonly name: string;
	readonly version: string;
	readonly manifestPath: string;
	readonly hasBuildScript: boolean;
	readonly isProcMacro: boolean;
	readonly features: readonly string[];
	readonly dependencies: readonly CargoDependencyInfo[];
	readonly targets: readonly CargoTargetInfo[];
}

export interface CargoDependencyInfo {
	readonly name: string;
	readonly kind: "normal" | "dev" | "build";
	readonly isOptional: boolean;
}

export interface CargoTargetInfo {
	readonly name: string;
	readonly kinds: readonly string[];
	readonly srcPath: string;
}

// ---------------------------------------------------------------------------
// JS/TS workspace metadata
// ---------------------------------------------------------------------------

export interface JsWorkspaceMeta {
	readonly packageManager: string;
	readonly packages: readonly JsPackageInfo[];
	readonly rootTsconfig: TsconfigInfo | null;
	readonly hasPnp: boolean;
}

export interface JsPackageInfo {
	readonly name: string;
	readonly relativePath: string;
	readonly hasExportsField: boolean;
	readonly hasImportsField: boolean;
	readonly moduleType: string | null;
}

export interface TsconfigInfo {
	readonly path: string;
	readonly moduleResolution: string | null;
	readonly hasProjectReferences: boolean;
	readonly hasPaths: boolean;
	readonly hasBaseUrl: boolean;
}

// ---------------------------------------------------------------------------
// RepoConfig (simplified — config sections for display)
// ---------------------------------------------------------------------------

export interface RepoConfig {
	readonly version: number;
	readonly ignore: readonly string[];
	readonly entrypoints: readonly Entrypoint[];
}

export interface Entrypoint {
	readonly path: string;
	readonly kind: "app" | "public-api" | "binary";
}

// ---------------------------------------------------------------------------
// DiscoveryResult — returned by discover_workspace command
// ---------------------------------------------------------------------------

export interface DiscoveryResult {
	readonly workspace: WorkspaceInfo;
	readonly config: RepoConfig;
	readonly profile: GraphProfile;
	readonly compatibility: CompatibilityReport;
	readonly nonFunctionalConfigSections: readonly string[];
}
