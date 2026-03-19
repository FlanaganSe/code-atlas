import { beforeEach, describe, expect, it } from "vitest";
import { useScanStore } from "@/store/scan-store";
import type { ScanEvent } from "@/types/scan";

describe("useScanStore", () => {
	beforeEach(() => {
		useScanStore.setState({
			activeScanId: null,
			scanStatus: "idle",
			progress: null,
			compatibilityReport: null,
			graphHealth: null,
			unsupportedConstructs: [],
			parseFailures: [],
			error: null,
		});
	});

	it("starts a scan with correct state", () => {
		useScanStore.getState().startScan("scan-123");
		const state = useScanStore.getState();
		expect(state.activeScanId).toBe("scan-123");
		expect(state.scanStatus).toBe("scanning");
		expect(state.progress).toBeNull();
		expect(state.error).toBeNull();
	});

	it("handles progress events", () => {
		useScanStore.getState().startScan("scan-123");

		const event: ScanEvent = {
			event: "progress",
			data: { scanId: "scan-123", scanned: 1, total: 3 },
		};
		useScanStore.getState().handleScanEvent(event);

		const state = useScanStore.getState();
		expect(state.progress).toEqual({ scanned: 1, total: 3 });
	});

	it("handles complete events", () => {
		useScanStore.getState().startScan("scan-123");

		const event: ScanEvent = {
			event: "complete",
			data: { scanId: "scan-123" },
		};
		useScanStore.getState().handleScanEvent(event);

		const state = useScanStore.getState();
		expect(state.scanStatus).toBe("complete");
		expect(state.progress).toBeNull();
	});

	it("handles error events", () => {
		useScanStore.getState().startScan("scan-123");

		const event: ScanEvent = {
			event: "error",
			data: { scanId: "scan-123", message: "something failed" },
		};
		useScanStore.getState().handleScanEvent(event);

		const state = useScanStore.getState();
		expect(state.scanStatus).toBe("error");
		expect(state.error).toBe("something failed");
	});

	it("handles health events", () => {
		useScanStore.getState().startScan("scan-123");

		const event: ScanEvent = {
			event: "health",
			data: {
				scanId: "scan-123",
				health: {
					totalNodes: 10,
					resolvedEdges: 5,
					unresolvedImports: 1,
					parseFailures: 0,
					unsupportedConstructs: 2,
				},
			},
		};
		useScanStore.getState().handleScanEvent(event);

		const state = useScanStore.getState();
		expect(state.graphHealth).toBeDefined();
		expect(state.graphHealth?.totalNodes).toBe(10);
	});

	it("drops events from stale scans", () => {
		useScanStore.getState().startScan("scan-123");

		const staleEvent: ScanEvent = {
			event: "complete",
			data: { scanId: "old-scan" },
		};
		useScanStore.getState().handleScanEvent(staleEvent);

		// Status should still be scanning, not complete
		expect(useScanStore.getState().scanStatus).toBe("scanning");
	});

	it("handles compatibility report events", () => {
		useScanStore.getState().startScan("scan-123");

		const event: ScanEvent = {
			event: "compatibilityReport",
			data: {
				scanId: "scan-123",
				report: {
					assessments: [],
					isProvisional: false,
				},
			},
		};
		useScanStore.getState().handleScanEvent(event);

		const state = useScanStore.getState();
		expect(state.compatibilityReport).toBeDefined();
		expect(state.compatibilityReport?.isProvisional).toBe(false);
	});

	it("resets to initial state", () => {
		useScanStore.getState().startScan("scan-123");
		useScanStore.getState().reset();

		const state = useScanStore.getState();
		expect(state.activeScanId).toBeNull();
		expect(state.scanStatus).toBe("idle");
	});

	// M6: Details event handling
	it("handles details events with unsupported constructs and parse failures", () => {
		useScanStore.getState().startScan("scan-123");

		const event: ScanEvent = {
			event: "details",
			data: {
				scanId: "scan-123",
				unsupportedConstructs: [
					{
						constructType: "cfgGate",
						location: { path: "src/lib.rs", startLine: 10, endLine: 12 },
						impact: "Module may be missing from graph",
						howToAddress: "Add manual edge in .codeatlas.yaml",
					},
				],
				parseFailures: [{ path: "src/broken.rs", reason: "syntax error" }],
			},
		};
		useScanStore.getState().handleScanEvent(event);

		const state = useScanStore.getState();
		expect(state.unsupportedConstructs).toHaveLength(1);
		expect(state.unsupportedConstructs[0].constructType).toBe("cfgGate");
		expect(state.parseFailures).toHaveLength(1);
		expect(state.parseFailures[0].path).toBe("src/broken.rs");
	});

	// M6: Compatibility report provisional → final transition
	it("updates compatibility report from provisional to final", () => {
		useScanStore.getState().startScan("scan-123");

		// First: provisional report
		useScanStore.getState().handleScanEvent({
			event: "compatibilityReport",
			data: {
				scanId: "scan-123",
				report: {
					assessments: [
						{
							language: "rust",
							status: "supported",
							details: [{ feature: "Workspace detection", status: "supported", explanation: "ok" }],
						},
					],
					isProvisional: true,
				},
			},
		});
		expect(useScanStore.getState().compatibilityReport?.isProvisional).toBe(true);

		// Second: enriched final report
		useScanStore.getState().handleScanEvent({
			event: "compatibilityReport",
			data: {
				scanId: "scan-123",
				report: {
					assessments: [
						{
							language: "rust",
							status: "partial",
							details: [
								{ feature: "Workspace detection", status: "supported", explanation: "ok" },
								{
									feature: "Source-level cfg gates",
									status: "partial",
									explanation: "2 cfg gates detected",
								},
							],
						},
					],
					isProvisional: false,
				},
			},
		});
		const state = useScanStore.getState();
		expect(state.compatibilityReport?.isProvisional).toBe(false);
		expect(state.compatibilityReport?.assessments[0].details).toHaveLength(2);
	});

	// M6: Reset clears details
	it("reset clears unsupported constructs and parse failures", () => {
		useScanStore.getState().startScan("scan-123");
		useScanStore.getState().handleScanEvent({
			event: "details",
			data: {
				scanId: "scan-123",
				unsupportedConstructs: [
					{
						constructType: "dynamicImport",
						location: { path: "src/index.ts", startLine: 5, endLine: 5 },
						impact: "Import not resolved",
						howToAddress: "N/A",
					},
				],
				parseFailures: [],
			},
		});
		expect(useScanStore.getState().unsupportedConstructs).toHaveLength(1);

		useScanStore.getState().reset();
		expect(useScanStore.getState().unsupportedConstructs).toHaveLength(0);
		expect(useScanStore.getState().parseFailures).toHaveLength(0);
	});
});
