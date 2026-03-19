/**
 * Scan event types — mirrors Rust serde output.
 *
 * ScanPhase and ScanResults are domain types from codeatlas-core.
 * ScanEvent is the transport envelope from codeatlas-tauri.
 */

import type { CompatibilityReport, GraphHealth } from "./config";
import type { EdgeData, NodeData, ParseFailure, UnsupportedConstruct } from "./graph";

// ---------------------------------------------------------------------------
// ScanPhase
// ---------------------------------------------------------------------------

export type ScanPhase = "packageTopology" | "moduleStructure" | "fileEdges";

// ---------------------------------------------------------------------------
// ScanStatus
// ---------------------------------------------------------------------------

export type ScanStatus = "idle" | "scanning" | "complete" | "error" | "cancelled";

// ---------------------------------------------------------------------------
// ScanEvent (transport envelope — from codeatlas-tauri)
// ---------------------------------------------------------------------------

/** Discriminated union of scan events delivered via Channel<T>. */
export type ScanEvent =
	| {
			readonly event: "compatibilityReport";
			readonly data: {
				readonly scanId: string;
				readonly report: CompatibilityReport;
			};
	  }
	| {
			readonly event: "phase";
			readonly data: {
				readonly scanId: string;
				readonly phase: ScanPhase;
				readonly nodes: readonly NodeData[];
				readonly edges: readonly EdgeData[];
			};
	  }
	| {
			readonly event: "health";
			readonly data: {
				readonly scanId: string;
				readonly health: GraphHealth;
			};
	  }
	| {
			readonly event: "progress";
			readonly data: {
				readonly scanId: string;
				readonly scanned: number;
				readonly total: number;
			};
	  }
	| {
			readonly event: "complete";
			readonly data: {
				readonly scanId: string;
			};
	  }
	| {
			readonly event: "details";
			readonly data: {
				readonly scanId: string;
				readonly unsupportedConstructs: readonly UnsupportedConstruct[];
				readonly parseFailures: readonly ParseFailure[];
			};
	  }
	| {
			readonly event: "overlay";
			readonly data: {
				readonly scanId: string;
				readonly manualEdges: readonly EdgeData[];
				readonly suppressedEdgeIds: readonly string[];
			};
	  }
	| {
			readonly event: "error";
			readonly data: {
				readonly scanId: string;
				readonly message: string;
			};
	  };
