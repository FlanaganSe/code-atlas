/**
 * Hook for managing Tauri scan lifecycle via Channel<ScanEvent>.
 *
 * Creates a Channel, dispatches events to both scan-store and graph-store,
 * and handles stale scan ID rejection.
 */

import { Channel, invoke } from "@tauri-apps/api/core";
import { useCallback, useRef } from "react";
import { useGraphStore } from "@/store/graph-store";
import { useScanStore } from "@/store/scan-store";
import type { ScanEvent } from "@/types/scan";

interface UseScanReturn {
	startScan: (path: string) => Promise<void>;
	cancelScan: () => Promise<void>;
}

export function useScan(): UseScanReturn {
	const scanIdRef = useRef<string | null>(null);

	const startScan = useCallback(async (path: string): Promise<void> => {
		const scanStore = useScanStore.getState();
		const graphStore = useGraphStore.getState();

		// Clear existing graph for a fresh scan
		graphStore.clearGraph();

		// Generate scan ID client-side for immediate stale rejection
		const scanId = crypto.randomUUID();
		scanIdRef.current = scanId;
		scanStore.startScan(scanId);

		const channel = new Channel<ScanEvent>();
		channel.onmessage = (event: ScanEvent) => {
			// Stale event rejection
			const currentScanId = useScanStore.getState().activeScanId;
			const eventScanId = event.data && "scanId" in event.data ? event.data.scanId : null;

			if (eventScanId && eventScanId !== currentScanId) {
				return;
			}

			// Dispatch to scan store (status, progress, health, compatibility)
			useScanStore.getState().handleScanEvent(event);

			// Dispatch phase events to graph store
			if (event.event === "phase") {
				useGraphStore
					.getState()
					.applyScanPhase(event.data.phase, event.data.nodes, event.data.edges);
			}
		};

		try {
			await invoke("start_scan", { path, onEvent: channel });
		} catch (err) {
			useScanStore.getState().handleScanEvent({
				event: "error",
				data: {
					scanId,
					message: err instanceof Error ? err.message : String(err),
				},
			});
		}
	}, []);

	const cancelScan = useCallback(async (): Promise<void> => {
		try {
			await invoke("cancel_scan");
			const scanStore = useScanStore.getState();
			if (scanStore.activeScanId) {
				scanStore.handleScanEvent({
					event: "error",
					data: {
						scanId: scanStore.activeScanId,
						message: "scan cancelled",
					},
				});
			}
		} catch {
			// Ignore cancel errors
		}
	}, []);

	return { startScan, cancelScan };
}
