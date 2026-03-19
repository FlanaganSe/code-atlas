/**
 * Zustand store for scan lifecycle state.
 *
 * Tracks active scan ID, status, progress, compatibility report, and health.
 * Events from stale scans (mismatched scanId) are silently dropped.
 */

import { create } from "zustand";
import type { CompatibilityReport, GraphHealth, UnresolvedImport } from "@/types/config";
import type { ParseFailure, UnsupportedConstruct } from "@/types/graph";
import type { ScanEvent, ScanStatus } from "@/types/scan";

export interface ScanStore {
	activeScanId: string | null;
	scanStatus: ScanStatus;
	progress: { scanned: number; total: number } | null;
	compatibilityReport: CompatibilityReport | null;
	graphHealth: GraphHealth | null;
	unsupportedConstructs: readonly UnsupportedConstruct[];
	parseFailures: readonly ParseFailure[];
	unresolvedImports: readonly UnresolvedImport[];
	error: string | null;
	scanPath: string | null;

	startScan: (scanId: string) => void;
	setScanPath: (path: string) => void;
	handleScanEvent: (event: ScanEvent) => void;
	reset: () => void;
}

export const useScanStore = create<ScanStore>()((set, get) => ({
	activeScanId: null,
	scanStatus: "idle",
	progress: null,
	compatibilityReport: null,
	graphHealth: null,
	unsupportedConstructs: [],
	parseFailures: [],
	unresolvedImports: [],
	error: null,
	scanPath: null,

	setScanPath: (path: string) => {
		set({ scanPath: path });
	},

	startScan: (scanId: string) => {
		set({
			activeScanId: scanId,
			scanStatus: "scanning",
			progress: null,
			error: null,
		});
	},

	handleScanEvent: (event: ScanEvent) => {
		const state = get();

		// Extract scanId from event data
		const eventScanId = event.data && "scanId" in event.data ? event.data.scanId : null;

		// Drop stale events
		if (eventScanId && eventScanId !== state.activeScanId) {
			return;
		}

		switch (event.event) {
			case "compatibilityReport":
				set({ compatibilityReport: event.data.report });
				break;
			case "health":
				set({ graphHealth: event.data.health });
				break;
			case "progress":
				set({
					progress: {
						scanned: event.data.scanned,
						total: event.data.total,
					},
				});
				break;
			case "complete":
				set({ scanStatus: "complete", progress: null });
				break;
			case "error":
				set({
					scanStatus: "error",
					error: event.data.message,
					progress: null,
				});
				break;
			case "details":
				set({
					unsupportedConstructs: event.data.unsupportedConstructs,
					parseFailures: event.data.parseFailures,
					unresolvedImports: event.data.unresolvedImports,
				});
				break;
			case "phase":
				// Phase events are handled by the graph store, not here
				break;
			case "overlay":
				// Overlay events are handled by the graph store, not here
				break;
		}
	},

	reset: () => {
		set({
			activeScanId: null,
			scanStatus: "idle",
			progress: null,
			compatibilityReport: null,
			graphHealth: null,
			unsupportedConstructs: [],
			parseFailures: [],
			unresolvedImports: [],
			error: null,
			scanPath: null,
		});
	},
}));
