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
});
