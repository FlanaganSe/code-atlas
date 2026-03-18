/**
 * Graph domain types — mirrors Rust serde output from `codeatlas-core`.
 *
 * IMPORTANT: These types are manually maintained (no tauri-specta).
 * Any change to Rust serde attributes must be reflected here.
 * Contract tests in Rust verify JSON shape matches these definitions.
 */

// ---------------------------------------------------------------------------
// Language
// ---------------------------------------------------------------------------

export type Language = "rust" | "typescript" | "javascript" | "unknown";

// ---------------------------------------------------------------------------
// EntityKind
// ---------------------------------------------------------------------------

export type EntityKind = "package" | "module" | "file";

// ---------------------------------------------------------------------------
// NodeKind
// ---------------------------------------------------------------------------

export type NodeKind = "package" | "module" | "file";

// ---------------------------------------------------------------------------
// MaterializedKey
// ---------------------------------------------------------------------------

/** Unique identity for a graph node within a single scan/snapshot. */
export interface MaterializedKey {
	readonly language: Language;
	readonly entityKind: EntityKind;
	readonly relativePath: string;
}

// ---------------------------------------------------------------------------
// LineageKey
// ---------------------------------------------------------------------------

/** UUID-based identity that survives renames. Always null in POC. */
export type LineageKey = string | null;

// ---------------------------------------------------------------------------
// NodeData
// ---------------------------------------------------------------------------

/** A node in the architecture graph. */
export interface NodeData {
	readonly materializedKey: MaterializedKey;
	readonly lineageKey: LineageKey;
	readonly label: string;
	readonly kind: NodeKind;
	readonly language: Language;
	readonly parentKey: MaterializedKey | null;
}

// ---------------------------------------------------------------------------
// EdgeKind
// ---------------------------------------------------------------------------

export type EdgeKind = "imports" | "reExports" | "contains" | "dependsOn" | "manual";

// ---------------------------------------------------------------------------
// EdgeCategory
// ---------------------------------------------------------------------------

export type EdgeCategory =
	| "value"
	| "typeOnly"
	| "dev"
	| "build"
	| "test"
	| "peer"
	| "normal"
	| "manual";

// ---------------------------------------------------------------------------
// Confidence
// ---------------------------------------------------------------------------

export type Confidence = "structural" | "syntactic" | "resolverAware" | "semantic" | "runtime";

// ---------------------------------------------------------------------------
// SourceLocation
// ---------------------------------------------------------------------------

export interface SourceLocation {
	readonly path: string;
	readonly startLine: number;
	readonly endLine: number;
}

// ---------------------------------------------------------------------------
// OverlayStatus (adjacently tagged enum)
// ---------------------------------------------------------------------------

export type OverlayStatus =
	| { readonly type: "none" }
	| { readonly type: "suppressed"; readonly data: { readonly reason: string } };

// ---------------------------------------------------------------------------
// EdgeId
// ---------------------------------------------------------------------------

/** Deterministic hash-based identity for a graph edge. */
export type EdgeId = string;

// ---------------------------------------------------------------------------
// EdgeData
// ---------------------------------------------------------------------------

/** An edge in the architecture graph. */
export interface EdgeData {
	readonly edgeId: EdgeId;
	readonly sourceKey: MaterializedKey;
	readonly targetKey: MaterializedKey;
	readonly kind: EdgeKind;
	readonly category: EdgeCategory;
	readonly confidence: Confidence;
	readonly sourceLocation: SourceLocation | null;
	readonly resolutionMethod: string | null;
	readonly overlayStatus: OverlayStatus;
}

// ---------------------------------------------------------------------------
// UnsupportedConstructType
// ---------------------------------------------------------------------------

export type UnsupportedConstructType =
	| "cfgGate"
	| "buildScript"
	| "procMacro"
	| "dynamicImport"
	| "frameworkConvention"
	| "exportsCondition";

// ---------------------------------------------------------------------------
// UnsupportedConstruct
// ---------------------------------------------------------------------------

export interface UnsupportedConstruct {
	readonly constructType: UnsupportedConstructType;
	readonly location: SourceLocation;
	readonly impact: string;
	readonly howToAddress: string;
}

// ---------------------------------------------------------------------------
// ParseFailure
// ---------------------------------------------------------------------------

export interface ParseFailure {
	readonly path: string;
	readonly reason: string;
}
